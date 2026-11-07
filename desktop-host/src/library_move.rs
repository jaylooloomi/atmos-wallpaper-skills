//! Native, copy-verify-switch library relocation. The source is retained.
use crate::{
    asset_store, library, library_audit,
    portable_store::{self, Store, MAX_SAFE_INTEGER},
};
use serde_json::{json, Value};
use std::{
    fs,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};

pub(crate) fn default_path(store: &Store) -> PathBuf {
    store.root.with_file_name(format!(
        "{}-library",
        store.root.file_name().unwrap_or_default().to_string_lossy()
    ))
}

fn path_key(path: &Path) -> String {
    portable_store::external_path(path).to_lowercase()
}
fn inside(child: &str, parent: &str) -> bool {
    child == parent
        || child
            .strip_prefix(parent)
            .is_some_and(|tail| tail.starts_with('\\'))
}

fn target(store: &Store, raw: &Path, current: &Path, legacy: bool) -> Result<PathBuf, String> {
    let path = library::local_path(&portable_store::external_path(raw))?;
    portable_store::validate_root(&path).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    let parent = path.parent().ok_or("LIBRARY_PATH_UNSAFE")?;
    let parent = parent
        .canonicalize()
        .map_err(|_| "LIBRARY_PARENT_MISSING")?;
    let candidate = if path.exists() {
        path.canonicalize().map_err(|_| "LIBRARY_PATH_UNSAFE")?
    } else {
        parent.join(path.file_name().ok_or("LIBRARY_PATH_UNSAFE")?)
    };
    let key = path_key(&candidate);
    let old = current
        .canonicalize()
        .map(|p| path_key(&p))
        .unwrap_or_else(|_| path_key(current));
    if !legacy && key != old && (inside(&key, &old) || inside(&old, &key)) {
        return Err("LIBRARY_PATH_UNSAFE".into());
    }
    let exe = std::env::current_exe().map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    for forbidden in [Some(store.root.clone()), exe.parent().map(Path::to_owned)]
        .into_iter()
        .flatten()
    {
        let forbidden = forbidden
            .canonicalize()
            .map_err(|_| "LIBRARY_PATH_UNSAFE")?;
        let p = path_key(&forbidden);
        if inside(&key, &p) || inside(&p, &key) {
            return Err("LIBRARY_PATH_UNSAFE".into());
        }
    }
    for var in ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(var) {
            let p = PathBuf::from(p);
            if let Ok(p) = p.canonicalize() {
                let p = path_key(&p);
                if inside(&key, &p) || inside(&p, &key) {
                    return Err("LIBRARY_PATH_UNSAFE".into());
                }
            }
        }
    }
    Ok(candidate)
}

fn free_bytes(dir: &Path) -> Result<u64, String> {
    let path: Vec<_> = dir.as_os_str().encode_wide().chain([0]).collect();
    let mut free = 0;
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err("LIBRARY_UNAVAILABLE".into());
    }
    Ok(free)
}

pub(crate) fn json_file(path: &Path, value: &Value) -> Result<(), String> {
    portable_store::atomic_file(
        path,
        &serde_json::to_vec(value).map_err(|_| "LIBRARY_MOVE_FAILED")?,
    )
}

fn marker(dir: &Path, owner_key: &str) -> Result<Value, String> {
    let file = dir.join(".atmos-library.json");
    let value = if file.exists() {
        let bytes = asset_store::read_bytes(&file, 16384, "LIBRARY_OWNER_MISMATCH")?;
        serde_json::from_slice(&bytes).map_err(|_| "LIBRARY_OWNER_MISMATCH")?
    } else {
        let id = crate::native_http::random_hex(16).map_err(|_| "LIBRARY_MOVE_FAILED")?;
        let id = format!(
            "{}-{}-4{}-a{}-{}",
            &id[..8],
            &id[8..12],
            &id[13..16],
            &id[17..20],
            &id[20..]
        );
        let value = json!({"schemaVersion":1,"id":id,"storeKey":owner_key});
        json_file(&file, &value)?;
        value
    };
    let valid_id = value["id"].as_str().is_some_and(|s| {
        s.len() == 36
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || b == b'-')
    });
    if value["schemaVersion"] != 1 || value["storeKey"] != owner_key || !valid_id {
        return Err("LIBRARY_OWNER_MISMATCH".into());
    }
    Ok(value)
}

pub(crate) fn relocate(
    store: &Store,
    raw: &Path,
    expected: u64,
    request: &str,
    mut progress: impl FnMut(usize, usize),
) -> Result<Value, String> {
    if !crate::policy_store::valid_id(request) || expected > MAX_SAFE_INTEGER {
        return Err("LIBRARY_COMMAND_INVALID".into());
    }
    crate::library_archive::recover(store)?;
    let _guard = store.lock()?;
    let (current, old_id, revision, legacy) = asset_store::library_config(store)?;
    let target = target(store, raw, &current, legacy)?;
    let external = portable_store::external_path(&target);
    // Match the legacy JSON.stringify field order for cross-runtime receipts.
    let fp = asset_store::hash_text(&format!(
        r#"{{"target":{},"expectedRevision":{expected}}}"#,
        serde_json::to_string(&external).unwrap()
    ));
    let receipt = format!(
        "requests/{}.json",
        asset_store::hash_text(&format!("library:{request}"))
    );
    if let Some(r) = store.read_optional(&receipt)? {
        if r["fingerprint"] != fp {
            return Err("LIBRARY_REQUEST_CONFLICT".into());
        }
        let committed = portable_store::external_path(&current) == external
            && r["result"]["revision"] == revision
            && r["targetId"] == old_id;
        if r["pending"] != true || committed {
            let mut result = r["result"].clone();
            if !result.is_object() {
                return Err("LIBRARY_CORRUPT".into());
            }
            if r["pending"] == true {
                store.atomic(&receipt, &json!({"fingerprint":fp,"result":result}))?;
                result["recovered"] = true.into();
            }
            result["replayed"] = true.into();
            return Ok(result);
        }
    }
    if revision != expected {
        return Err("LIBRARY_REVISION_CONFLICT".into());
    }
    if revision >= MAX_SAFE_INTEGER {
        return Err("LIBRARY_LIMIT".into());
    }
    let source = asset_store::path(store, "assets")?;
    let files = library_audit::inventory(&source, false)?;
    if !legacy && path_key(&current) == path_key(&target) {
        let result = json!({"committed":true,"revision":revision,"path":external,"copiedFiles":0,"sourceRetained":true});
        store.atomic(&receipt, &json!({"fingerprint":fp,"result":result}))?;
        return Ok(result);
    }
    let existing = target.exists();
    if existing {
        if !target.is_dir() {
            return Err("LIBRARY_PATH_UNSAFE".into());
        }
        let mut names = fs::read_dir(&target).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
        if let Some(entry) = names.next() {
            if !target.join(".atmos-library.json").is_file() {
                return Err("LIBRARY_TARGET_NOT_EMPTY".into());
            }
            for e in std::iter::once(entry).chain(names) {
                let e = e.map_err(|_| "LIBRARY_PATH_UNSAFE")?;
                if !e
                    .file_name()
                    .to_str()
                    .is_some_and(|n| [".atmos-library.json", "assets", "index.json"].contains(&n))
                {
                    return Err("LIBRARY_TARGET_NOT_EMPTY".into());
                }
                portable_store::no_links(&e.path()).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
            }
        }
    }
    let bytes = files.iter().map(|f| f.bytes as u64).sum::<u64>();
    if free_bytes(if existing {
        &target
    } else {
        target.parent().unwrap()
    })? < bytes + 1048576
    {
        return Err("LIBRARY_DISK_FULL".into());
    }
    if !existing {
        fs::create_dir(&target).map_err(|_| "LIBRARY_MOVE_FAILED")?;
    }
    let marker = marker(
        &target,
        &asset_store::hash_text(&store.external_root().to_lowercase()),
    )?;
    let dest = target.join("assets");
    portable_store::no_links(&dest).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    if !dest.exists() {
        fs::create_dir(&dest).map_err(|_| "LIBRARY_MOVE_FAILED")?;
    }
    library_audit::inventory(&dest, true)?;
    let mut job = json!({"schemaVersion":1,"ownerPid":std::process::id(),"requestId":request,"state":"copying",
                       "completed":0,"total":files.len(),"bytes":bytes,"sourceRetained":true});
    store.atomic("library-job.json", &job)?;
    let outcome: Result<Value, String> = (|| {
        progress(0, files.len());
        for (i, file) in files.iter().enumerate() {
            let data = asset_store::read_bytes(
                &source.join(&file.name),
                file.bytes,
                "LIBRARY_SOURCE_CHANGED",
            )?;
            if asset_store::hash_bytes(&data) != file.sha256 {
                return Err("LIBRARY_SOURCE_CHANGED".into());
            }
            let to = dest.join(&file.name);
            portable_store::no_links(&to).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
            if !to.exists() {
                portable_store::atomic_new_file(&to, &data)?;
            }
            let copied = asset_store::read_bytes(&to, file.bytes, "LIBRARY_TARGET_CONFLICT")?;
            if asset_store::hash_bytes(&copied) != file.sha256 {
                return Err("LIBRARY_TARGET_CONFLICT".into());
            }
            job["completed"] = (i + 1).into();
            progress(i + 1, files.len());
            #[cfg(feature = "http-fixture")]
            crate::library_crash_fixture::pause(&store.root, "move-after-copy");
        }
        let verified = library_audit::inventory(&dest, false)?;
        if !files.iter().all(|f| {
            verified
                .binary_search_by(|v| v.name.cmp(&f.name))
                .is_ok_and(|i| verified[i] == *f)
        }) {
            return Err("LIBRARY_COPY_CORRUPT".into());
        }
        if library_audit::inventory(&source, false)? != files {
            return Err("LIBRARY_SOURCE_CHANGED".into());
        }
        json_file(
            &target.join("index.json"),
            &json!({"schemaVersion":1,"files":verified}),
        )?;
        if !legacy {
            store.atomic("library-config.previous.json",&json!({"schemaVersion":1,"revision":revision,"root":portable_store::external_path(&current),"id":old_id}))?;
        }
        let result = json!({"committed":true,"revision":revision+1,"path":external,"copiedFiles":files.len(),"verifiedFiles":verified.len(),"sourceRetained":true});
        let target_id = &marker["id"];
        store.atomic(
            &receipt,
            &json!({"fingerprint":fp,"pending":true,"result":result,"targetId":target_id}),
        )?;
        store.atomic("library-switch-intent.json",&json!({"schemaVersion":1,"requestId":request,"fingerprint":fp,"result":result,"targetId":target_id}))?;
        #[cfg(feature = "http-fixture")]
        crate::library_crash_fixture::pause(&store.root, "move-before-switch");
        store.atomic(
            "library-config.json",
            &json!({"schemaVersion":1,"revision":revision+1,"root":external,"id":target_id}),
        )?;
        #[cfg(feature = "http-fixture")]
        crate::library_crash_fixture::pause(&store.root, "move-after-switch");
        // Pointer switched. The pending receipt is sufficient if finalization fails.
        let mut result = result;
        if store
            .atomic(&receipt, &json!({"fingerprint":fp,"result":result}))
            .is_err()
        {
            result["recovered"] = true.into();
        }
        Ok(result)
    })();
    match &outcome {
        Ok(result) => {
            job["state"] = "completed".into();
            job["result"] = result.clone();
        }
        Err(code) => {
            job["state"] = "failed".into();
            job["error"] = json!({"code":if code.starts_with("LIBRARY_"){code.as_str()}else{"LIBRARY_MOVE_FAILED"}});
        }
    }
    let _ = store.atomic("library-job.json", &job);
    outcome
}
