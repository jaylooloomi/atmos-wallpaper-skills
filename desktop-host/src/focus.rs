//! Session-bound global privacy policy. Uses the same owner .lock and JSON
//! receipt envelope as focus-core.mjs; never writes appearance or renderer ACKs.
use crate::portable_store::{safe_uint, Store, MAX_SAFE_INTEGER};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

fn mode(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| ["normal", "focus", "showcase"].contains(&s))
}
fn session_valid(s: &str) -> bool {
    !s.is_empty() && s.encode_utf16().count() <= 120
}
fn normalized_input(v: &Value) -> Result<Value, String> {
    if !v.as_object().is_some_and(|o| {
        o.len() == 4
            && ["schemaVersion", "requestId", "expectedRevision", "mode"]
                .iter()
                .all(|k| o.contains_key(*k))
    }) || safe_uint(&v["schemaVersion"]) != Some(1)
        || safe_uint(&v["expectedRevision"]).is_none()
        || !v["requestId"]
            .as_str()
            .is_some_and(crate::policy_store::valid_id)
        || !mode(&v["mode"])
    {
        return Err("FOCUS_INVALID".into());
    }
    Ok(
        json!({"schemaVersion":1,"requestId":v["requestId"],"expectedRevision":safe_uint(&v["expectedRevision"]).unwrap(),"mode":v["mode"]}),
    )
}
fn read(store: &Store, session: &str) -> Result<Value, String> {
    if !session_valid(session) {
        return Err("FOCUS_SESSION_INVALID".into());
    }
    let default = || json!({"schemaVersion":1,"sessionId":session,"revision":0,"mode":"normal","receipts":{}});
    let Some(v) = store.read_optional("focus-runtime.json")? else {
        return Ok(default());
    };
    if safe_uint(&v["schemaVersion"]) != Some(1)
        || !v["sessionId"].is_string()
        || safe_uint(&v["revision"]).is_none()
        || !mode(&v["mode"])
        || !v["receipts"].is_object()
    {
        return Err("FOCUS_CORRUPT".into());
    }
    if v["sessionId"] != session {
        return Ok(default());
    }
    Ok(v)
}
pub fn status(store: &Store, session: &str) -> Result<Value, String> {
    let s = read(store, session)?;
    Ok(
        json!({"schemaVersion":1,"revision":s["revision"],"mode":s["mode"],"scope":"all-atmos-views","persistent":false,"resetsAt":"host-restart"}),
    )
}
pub fn set(store: &Store, session: &str, input: &Value) -> Result<Value, String> {
    let input = normalized_input(input)?;
    let _guard = store.lock()?;
    let mut state = read(store, session)?;
    let id = input["requestId"].as_str().unwrap();
    // Narrow canonical domain: ASCII keys/IDs/modes and normalized safe integers.
    let fp = format!("{:x}", Sha256::digest(serde_json::to_vec(&input).unwrap()));
    if let Some(old) = state["receipts"].get(id) {
        if old["fingerprint"] != fp {
            return Err("REQUEST_CONFLICT".into());
        }
        let mut result = old["result"].clone();
        if !result.is_object() {
            return Err("FOCUS_CORRUPT".into());
        }
        result["replayed"] = json!(true);
        return Ok(result);
    }
    // Preserve legacy plain-object receipt lookup semantics for these reserved IDs.
    if [
        "constructor",
        "hasOwnProperty",
        "isPrototypeOf",
        "propertyIsEnumerable",
        "toString",
        "valueOf",
        "toLocaleString",
    ]
    .contains(&id)
    {
        return Err("REQUEST_CONFLICT".into());
    }
    let revision = safe_uint(&state["revision"]).unwrap();
    if safe_uint(&input["expectedRevision"]) != Some(revision) {
        return Err("FOCUS_REVISION_CONFLICT".into());
    }
    if state["receipts"].as_object().unwrap().len() >= 1000 {
        return Err("FOCUS_RECEIPT_LIMIT".into());
    }
    if revision >= MAX_SAFE_INTEGER {
        return Err("FOCUS_CORRUPT".into());
    }
    state["revision"] = json!(revision + 1);
    state["mode"] = input["mode"].clone();
    let result = json!({"saved":true,"schemaVersion":1,"revision":revision+1,"mode":state["mode"],"scope":"all-atmos-views","persistent":false,"resetsAt":"host-restart"});
    state["receipts"][id] = json!({"fingerprint":fp,"result":result});
    store.atomic("focus-runtime.json", &state)?;
    Ok(result)
}
fn host_session(store: &Store, now: u64) -> Result<String, String> {
    let v = store
        .read_optional("host-status.json")
        .map_err(|_| "FOCUS_HOST_UNAVAILABLE")?
        .ok_or("FOCUS_HOST_UNAVAILABLE")?;
    let pid = safe_uint(&v["pid"])
        .filter(|p| *p > 0 && *p <= u32::MAX as u64)
        .ok_or("FOCUS_HOST_UNAVAILABLE")? as u32;
    let captured = v["capturedAtMs"]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or("FOCUS_HOST_UNAVAILABLE")?;
    let age = now as f64 - captured;
    let session = v["hostSessionId"]
        .as_str()
        .ok_or("FOCUS_HOST_UNAVAILABLE")?;
    let mut parts = session.split('-');
    let first = parts.next().unwrap_or("");
    let second = parts.next().unwrap_or("");
    if safe_uint(&v["schemaVersion"]) != Some(1)
        || !crate::portable_store::process_alive(pid)
        || age < 0.0
        || age > 5000.0
        || v["state"] == "stopped"
        || first.is_empty()
        || second.is_empty()
        || !first.bytes().all(|b| b.is_ascii_digit())
        || !second.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_some()
    {
        return Err("FOCUS_HOST_UNAVAILABLE".into());
    }
    Ok(session.into())
}
pub fn execute(args: &[String], input: impl Read) -> Result<Value, String> {
    let (action, root) = crate::providers::arguments(args, "--focus-command")?;
    let store = Store::new(&root)?;
    let session = host_session(&store, crate::journal::now_ms())?;
    let value = if action == "get" {
        status(&store, &session)?
    } else {
        json!({"result":set(&store,&session,&crate::providers::read_input(input)?)?})
    };
    let mut output = json!({"apiVersion":1,"ok":true,"namespace":"focus","rendererAcknowledged":false,"hostVisibility":"unknown"});
    for (k, v) in value.as_object().unwrap() {
        output[k] = v.clone();
    }
    Ok(output)
}
pub fn command(args: &[String]) -> i32 {
    let (exit, value) = match execute(args, std::io::stdin().lock()) {
        Ok(v) => (0, v),
        Err(code) => crate::providers::error(&code),
    };
    println!("{value}");
    exit
}
pub fn quick_action(action: &str, root: &Path, session: &str) -> Result<(), String> {
    if !["normal", "focus", "showcase"].contains(&action) {
        return Err("QUICK_ACTION_INVALID".into());
    }
    let store = Store::new(root)?;
    let current = status(&store, session)?;
    let mut guid = unsafe { std::mem::zeroed() };
    if unsafe { windows_sys::Win32::System::Com::CoCreateGuid(&mut guid) } < 0 {
        return Err("QUICK_ACTION_FAILED".into());
    }
    let tail = guid
        .data4
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let id = format!(
        "tray.{:08x}-{:04x}-{:04x}-{}-{}",
        guid.data1,
        guid.data2,
        guid.data3,
        &tail[..4],
        &tail[4..]
    );
    set(
        &store,
        session,
        &json!({"schemaVersion":1,"requestId":id,"expectedRevision":current["revision"],"mode":action}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_focus_is_session_bound_idempotent_and_global() {
        let root = std::env::temp_dir().join(format!(
            "atmos-native-focus-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        let store = Store::new(&root).unwrap();
        let body =
            json!({"schemaVersion":1,"requestId":"once","expectedRevision":0,"mode":"focus"});
        assert_eq!(set(&store, "host.1", &body).unwrap()["revision"], 1);
        assert_eq!(set(&store, "host.1", &body).unwrap()["replayed"], true);
        assert_eq!(status(&store, "host.2").unwrap()["mode"], "normal");
        let mut conflict = body.clone();
        conflict["mode"] = json!("showcase");
        assert_eq!(
            set(&store, "host.1", &conflict).unwrap_err(),
            "REQUEST_CONFLICT"
        );
        quick_action("showcase", &root, "host.1").unwrap();
        assert_eq!(status(&store, "host.1").unwrap()["mode"], "showcase");
        assert!(!root.join("live.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
