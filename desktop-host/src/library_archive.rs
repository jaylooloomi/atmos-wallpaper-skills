//! Verified archive with a durable rollback intent before active files are removed.
use crate::{
    asset_store, library,
    library_audit::{self, File, Pointer},
    library_move,
    portable_store::{self, Store},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};

const INTENT: &str = "library-archive-intent.json";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Intent {
    schema_version: u32,
    state: String,
    owner_key: String,
    request_id: String,
    fingerprint: String,
    library: Pointer,
    files: Vec<File>,
    result: Value,
    #[serde(default, skip_serializing_if = "is_false")]
    recovered: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}
fn owner_key(store: &Store) -> String {
    asset_store::hash_text(&store.external_root().to_lowercase())
}
fn archive_path(request: &str) -> String {
    format!("library-archive/{}", asset_store::hash_text(request))
}
fn receipt(request: &str) -> String {
    format!(
        "requests/{}.json",
        asset_store::hash_text(&format!("library-archive:{request}"))
    )
}

fn read_intent(store: &Store) -> Result<Option<Intent>, String> {
    match asset_store::read_bytes(&store.path(INTENT)?, 4 * 1048576, "LIBRARY_ARCHIVE_INVALID") {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "LIBRARY_ARCHIVE_INVALID".into()),
        Err(e) if e == "ENOENT" => Ok(None),
        Err(e) => Err(e),
    }
}

fn hex(v: &str) -> bool {
    crate::desired_state::hex_id(&v.into())
}
fn validate(store: &Store, intent: &Intent) -> Result<(), String> {
    if intent.schema_version != 1
        || intent.state != "pending"
        || intent.owner_key != owner_key(store)
        || !crate::policy_store::valid_id(&intent.request_id)
        || !hex(&intent.fingerprint)
        || intent.files.len() > 20000
        || intent.library != library_audit::pointer(store)?
    {
        return Err("LIBRARY_ARCHIVE_INVALID".into());
    }
    let mut unique = BTreeSet::new();
    for f in &intent.files {
        let (id, ext) = f.name.split_once('.').ok_or("LIBRARY_ARCHIVE_INVALID")?;
        let limit = match ext {
            "png" => 16 * 1048576,
            "json" => 16384,
            _ => return Err("LIBRARY_ARCHIVE_INVALID".into()),
        };
        if !hex(id) || !hex(&f.sha256) || f.bytes > limit || !unique.insert(&f.name) {
            return Err("LIBRARY_ARCHIVE_INVALID".into());
        }
    }
    Ok(())
}

fn verified(path: &Path, entry: &File, code: &str) -> Result<Vec<u8>, String> {
    let data = asset_store::read_bytes(path, entry.bytes, code).map_err(|_| code)?;
    if data.len() != entry.bytes || asset_store::hash_bytes(&data) != entry.sha256 {
        return Err(code.into());
    }
    Ok(data)
}

fn ensure_copy(path: &Path, entry: &File, bytes: &[u8]) -> Result<(), String> {
    portable_store::no_links(path).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    if !path.exists() {
        portable_store::atomic_new_file(path, bytes)?;
    }
    verified(path, entry, "LIBRARY_ARCHIVE_CONFLICT")?;
    Ok(())
}

fn update_index(pointer: &Pointer, active: &Path) -> Result<(), String> {
    if pointer.id.is_some() {
        let files = library_audit::inventory(active, false)?;
        library_move::json_file(
            &Path::new(&pointer.root).join("index.json"),
            &json!({"schemaVersion":1,"files":files}),
        )?;
    }
    Ok(())
}

fn restore(store: &Store, intent: &mut Intent) -> Result<(), String> {
    validate(store, intent)?;
    library::local_path(&intent.library.root)?;
    let active = asset_store::library_path(store, "assets", true)?;
    let archive = store.path(&archive_path(&intent.request_id))?;
    for file in &intent.files {
        let bytes = verified(&archive.join(&file.name), file, "LIBRARY_ARCHIVE_CORRUPT")?;
        ensure_copy(&active.join(&file.name), file, &bytes)?;
    }
    update_index(&intent.library, &active)?;
    intent.state = "aborted".into();
    intent.recovered = true;
    store.atomic(INTENT, &serde_json::to_value(intent).unwrap())
}

pub(crate) fn recover(store: &Store) -> Result<(), String> {
    if read_intent(store)?.is_none_or(|i| i.state != "pending") {
        return Ok(());
    }
    let _guard = store.lock()?;
    if let Some(mut i) = read_intent(store)?.filter(|i| i.state == "pending") {
        restore(store, &mut i)?;
    }
    Ok(())
}

pub(crate) fn query(store: &Store, request: &str) -> Result<Option<Value>, String> {
    if let Some(r) = store.read_optional(&receipt(request))? {
        return Ok(Some(r["result"].clone()));
    }
    Ok(read_intent(store)?
        .filter(|i| i.state == "committed" && i.request_id == request)
        .map(|i| i.result))
}

pub(crate) fn archive(
    store: &Store,
    expected: u64,
    request: &str,
    audit_hash: &str,
) -> Result<Value, String> {
    if !crate::policy_store::valid_id(request)
        || !hex(audit_hash)
        || expected > portable_store::MAX_SAFE_INTEGER
    {
        return Err("LIBRARY_COMMAND_INVALID".into());
    }
    recover(store)?;
    let _guard = store.lock()?;
    let fingerprint = asset_store::hash_text(&format!(
        r#"{{"expectedRevision":{expected},"auditHash":"{audit_hash}"}}"#
    ));
    let receipt = receipt(request);
    if let Some(r) = store.read_optional(&receipt)? {
        if r["fingerprint"] != fingerprint {
            return Err("LIBRARY_REQUEST_CONFLICT".into());
        }
        let mut result = r["result"].clone();
        if !result.is_object() {
            return Err("LIBRARY_ARCHIVE_INVALID".into());
        }
        result["replayed"] = true.into();
        return Ok(result);
    }
    if let Some(old) =
        read_intent(store)?.filter(|i| i.state == "committed" && i.request_id == request)
    {
        if old.fingerprint != fingerprint {
            return Err("LIBRARY_REQUEST_CONFLICT".into());
        }
        store.atomic(
            &receipt,
            &json!({"fingerprint":fingerprint,"result":old.result}),
        )?;
        let mut result = old.result;
        result["replayed"] = true.into();
        result["recovered"] = true.into();
        return Ok(result);
    }
    let (audit, files) = library_audit::audit_locked(store, expected)?;
    if audit["auditHash"] != audit_hash {
        return Err("LIBRARY_AUDIT_STALE".into());
    }
    let library = library_audit::pointer(store)?;
    let active = asset_store::path(store, "assets")?;
    let archive = store.path(&archive_path(request))?;
    fs::create_dir_all(&archive).map_err(|_| "LIBRARY_ARCHIVE_FAILED")?;
    for file in &files {
        let bytes = verified(&active.join(&file.name), file, "LIBRARY_SOURCE_CHANGED")?;
        ensure_copy(&archive.join(&file.name), file, &bytes)?;
    }
    let (current, _) = library_audit::audit_locked(store, expected)?;
    if current["auditHash"] != audit_hash {
        return Err("LIBRARY_AUDIT_STALE".into());
    }
    let result = json!({"committed":true,"revision":expected,"archivedAssets":audit["candidateCount"],
        "archivedBytes":audit["candidateBytes"],"archivePath":portable_store::external_path(&archive),
        "permanentlyDeleted":false,"diskSpaceFreed":false});
    let mut intent = Intent {
        schema_version: 1,
        state: "pending".into(),
        owner_key: owner_key(store),
        request_id: request.into(),
        fingerprint: fingerprint.clone(),
        library,
        files,
        result: result.clone(),
        recovered: false,
    };
    store.atomic(INTENT, &serde_json::to_value(&intent).unwrap())?;
    let remove = (|| -> Result<(), String> {
        for file in &intent.files {
            let path = active.join(&file.name);
            verified(&path, file, "LIBRARY_SOURCE_CHANGED")?;
            portable_store::no_links(&path)?;
            fs::remove_file(path).map_err(|_| "LIBRARY_ARCHIVE_FAILED")?;
            #[cfg(feature = "http-fixture")]
            crate::library_crash_fixture::pause(&store.root, "archive-after-remove");
        }
        update_index(&intent.library, &active)?;
        intent.state = "committed".into();
        store.atomic(INTENT, &serde_json::to_value(&intent).unwrap())
    })();
    if let Err(error) = remove {
        intent.state = "pending".into();
        restore(store, &mut intent)?;
        return Err(error);
    }
    let mut result = result;
    if store
        .atomic(
            &receipt,
            &json!({"fingerprint":fingerprint,"result":result}),
        )
        .is_err()
    {
        result["recovered"] = true.into();
    }
    Ok(result)
}
