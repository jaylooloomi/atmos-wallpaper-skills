use crate::{
    asset_store, host_settings, library_archive, library_audit, library_move,
    portable_store::{safe_uint, Store},
};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const INVALID: &str = "LIBRARY_COMMAND_INVALID";

fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}

fn query(store: &Store, request: &str) -> Result<Option<Value>, String> {
    if let Some(r) = library_archive::query(store, request)? {
        return Ok(Some(r));
    }
    let name = format!(
        "requests/{}.json",
        asset_store::hash_text(&format!("library:{request}"))
    );
    let r = store.read_optional(&name)?;
    let intent;
    let pending = match r.as_ref() {
        Some(r) if r["pending"] != true => return Ok(Some(r["result"].clone())),
        Some(r) => r,
        None => {
            intent = store.read_optional("library-switch-intent.json")?;
            let Some(v) = intent.as_ref().filter(|v| v["requestId"] == request) else {
                return Ok(None);
            };
            v
        }
    };
    let p = library_audit::pointer(store)?;
    let mut result = pending["result"].clone();
    if result["path"] != p.root
        || result["revision"] != p.revision
        || pending["targetId"] != json!(p.id)
    {
        return Ok(None);
    }
    result["recovered"] = true.into();
    Ok(Some(result))
}

fn run(args: &[String], input: impl Read) -> Result<Value, String> {
    let action = args.get(1).map(String::as_str).ok_or(INVALID)?;
    let root = match &args[2..] {
        [] => host_settings::directory()?,
        [flag, path] if flag == "--store" => PathBuf::from(path),
        _ => return Err(INVALID.into()),
    };
    crate::portable_store::validate_root(&root)?;
    if action == "help" {
        return serde_json::from_str(include_str!("cli-catalogs/library-help.json"))
            .map_err(|_| "LIBRARY_OPERATION_FAILED".into());
    }
    let store = Store::new(&root)?;
    match action {
        "get" => {
            library_archive::recover(&store)?;
            let mut status = crate::native_control_http::library_status(&store)?;
            status.as_object_mut().unwrap().remove("job");
            return Ok(status);
        }
        "audit" => {
            let revision = library_audit::pointer(&store)?.revision;
            return library_audit::audit(&store, revision);
        }
        "result" | "move" | "reset" | "archive" => {}
        _ => return Err(INVALID.into()),
    }
    let mut bytes = Vec::new();
    input
        .take(32769)
        .read_to_end(&mut bytes)
        .map_err(|_| INVALID)?;
    if bytes.len() > 32768 {
        return Err(INVALID.into());
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| INVALID)?;
    if safe_uint(&v["schemaVersion"]) != Some(1) {
        return Err(INVALID.into());
    }
    let request = v["requestId"]
        .as_str()
        .filter(|s| crate::policy_store::valid_id(s))
        .ok_or(INVALID)?;
    if action == "result" {
        if !exact(&v, &["schemaVersion", "requestId"]) {
            return Err(INVALID.into());
        }
        let result = query(&store, request)?;
        return Ok(json!({"found":result.is_some(),"result":result}));
    }
    let keys = &["schemaVersion", "expectedRevision", "requestId"];
    let expected = safe_uint(&v["expectedRevision"]).ok_or(INVALID)?;
    match action {
        "move" if exact(&v, &[keys[0], keys[1], keys[2], "path"]) => {
            let path = Path::new(v["path"].as_str().ok_or(INVALID)?);
            library_move::relocate(&store, path, expected, request, |_, _| {})
        }
        "reset" if exact(&v, keys) => library_move::relocate(
            &store,
            &library_move::default_path(&store),
            expected,
            request,
            |_, _| {},
        ),
        "archive" if exact(&v, &[keys[0], keys[1], keys[2], "auditHash"]) => {
            library_archive::archive(
                &store,
                expected,
                request,
                v["auditHash"].as_str().ok_or(INVALID)?,
            )
        }
        _ => Err(INVALID.into()),
    }
}

pub(crate) fn command(args: &[String]) -> i32 {
    match run(args, std::io::stdin().lock()) {
        Ok(value) if args.get(1).is_some_and(|s| s == "help") => {
            println!("{value}");
            0
        }
        Ok(result) => {
            println!("{}", json!({"schemaVersion":1,"ok":true,"result":result}));
            0
        }
        Err(code) => {
            let code = if code.starts_with("LIBRARY_")
                || matches!(code.as_str(), "STORE_LOCKED" | "PATH_REJECTED")
            {
                code.as_str()
            } else {
                "LIBRARY_OPERATION_FAILED"
            };
            println!(
                "{}",
                json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
            );
            2
        }
    }
}
