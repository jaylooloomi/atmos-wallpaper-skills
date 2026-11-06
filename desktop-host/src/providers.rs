//! Native global provider-policy CLI. No network, telemetry or desktop startup.
//! Uses the legacy Store lock/receipt filenames so the existing HTTP writer and
//! older installations remain interoperable throughout the migration.
use crate::portable_store::{safe_uint, Store, MAX_SAFE_INTEGER};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Read, path::PathBuf};

fn keys(v: &Value, names: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == names.len() && names.iter().all(|n| o.contains_key(*n)))
}
fn valid_weather(v: &Value) -> bool {
    keys(v, &["enabled", "city"])
        && v["enabled"].is_boolean()
        && v["city"]
            .as_str()
            .is_some_and(|s| ["台北", "東京", "倫敦", "紐約"].contains(&s))
}
pub fn normalized_input(v: &Value) -> Result<Value, String> {
    if !keys(
        v,
        &["schemaVersion", "requestId", "expectedRevision", "weather"],
    ) || safe_uint(&v["schemaVersion"]) != Some(1)
        || !v["requestId"]
            .as_str()
            .is_some_and(crate::policy_store::valid_id)
        || !valid_weather(&v["weather"])
    {
        return Err("PROVIDERS_SETTINGS_INVALID".into());
    }
    let revision = safe_uint(&v["expectedRevision"]).ok_or("PROVIDERS_SETTINGS_INVALID")?;
    Ok(
        json!({"schemaVersion":1,"requestId":v["requestId"],"expectedRevision":revision,"weather":v["weather"]}),
    )
}
fn defaults() -> Value {
    json!({"schemaVersion":1,"revision":0,"lastRequestId":null,"weather":{"enabled":false,"city":"台北"}})
}
pub fn settings(store: &Store) -> Result<Value, String> {
    let Some(v) = store.read_optional("provider-settings.json")? else {
        return Ok(defaults());
    };
    if !keys(
        &v,
        &["schemaVersion", "revision", "lastRequestId", "weather"],
    ) || safe_uint(&v["schemaVersion"]) != Some(1)
        || safe_uint(&v["revision"]).is_none()
        || !valid_weather(&v["weather"])
        || !(v["lastRequestId"].is_null()
            || v["lastRequestId"]
                .as_str()
                .is_some_and(|s| s.is_empty() || crate::policy_store::valid_id(s)))
    {
        return Err("PROVIDERS_SETTINGS_INVALID".into());
    }
    Ok(v)
}
// This is intentionally a narrow hash boundary: all keys are ASCII, both number
// fields were normalized to safe integers, and all strings are fixed city names
// or bounded ASCII IDs. serde_json's sorted map encoding equals JS canonical()
// here. Do not reuse this for general appearance JSON/floats/UTF-16 key sorting.
fn fingerprint(input: &Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(input).unwrap()))
}
pub fn update(store: &Store, input: &Value) -> Result<Value, String> {
    let input = normalized_input(input)?;
    let _guard = store.lock()?;
    let current = settings(store)?;
    let id = input["requestId"].as_str().unwrap();
    let name = format!("requests/provider.{:x}.json", Sha256::digest(id.as_bytes()));
    let fp = fingerprint(&input);
    if let Some(receipt) = store.read_optional(&name)? {
        if receipt["fingerprint"] != fp {
            return Err("PROVIDERS_REQUEST_CONFLICT".into());
        }
        if receipt["result"]["committed"] != true
            || safe_uint(&receipt["result"]["revision"]).is_none()
        {
            return Err("CORRUPT_STORE".into());
        }
        if receipt["pending"] != true
            || (current["lastRequestId"] == id
                && safe_uint(&current["revision"]) == safe_uint(&receipt["result"]["revision"]))
        {
            store.atomic(&name, &json!({"fingerprint":fp,"result":receipt["result"]}))?;
            let mut result = receipt["result"].clone();
            result["replayed"] = json!(true);
            return Ok(result);
        }
    }
    let revision = safe_uint(&current["revision"]).unwrap();
    if revision != safe_uint(&input["expectedRevision"]).unwrap() {
        return Err("PROVIDERS_REVISION_CONFLICT".into());
    }
    if revision >= MAX_SAFE_INTEGER {
        return Err("PROVIDERS_SETTINGS_INVALID".into());
    }
    let next = json!({"schemaVersion":1,"revision":revision+1,"lastRequestId":id,"weather":input["weather"]});
    let result = json!({"committed":true,"revision":revision+1});
    store.atomic(
        &name,
        &json!({"fingerprint":fp,"pending":true,"result":result}),
    )?;
    store.atomic("provider-settings.previous.json", &current)?;
    store.atomic("provider-settings.json", &next)?;
    store.atomic(&name, &json!({"fingerprint":fp,"result":result}))?;
    Ok(result)
}
pub fn arguments(args: &[String], command: &str) -> Result<(String, PathBuf), String> {
    if args.len() < 2 || args[0] != command || !["get", "update"].contains(&args[1].as_str()) {
        return Err("ARGUMENT_INVALID".into());
    }
    let mut root = None;
    let mut seen = HashSet::new();
    let mut i = 2;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).ok_or("ARGUMENT_INVALID")?;
        if key != "--store" || !seen.insert(key) || !crate::portable_store::node_absolute(value) {
            return Err("ARGUMENT_INVALID".into());
        }
        root = Some(PathBuf::from(value));
        i += 2;
    }
    let root = match root {
        Some(root) => root,
        None => std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("io.atmos.wallpaper/store"))
            .ok_or("STORE_UNAVAILABLE")?,
    };
    crate::portable_store::validate_root(&root)?;
    Ok((args[1].clone(), root))
}
pub fn execute(args: &[String], input: impl Read) -> Result<Value, String> {
    let (action, root) = arguments(args, "--providers-command")?;
    let store = Store::new(&root)?;
    if action == "get" {
        return Ok(
            json!({"apiVersion":1,"ok":true,"namespace":"providers","schemaVersion":1,"available":true,"settings":settings(&store)?,"weatherProvider":"Open-Meteo; free API for non-commercial use only","nativeAvailability":"not-probed","rendererAcknowledged":false,"hostVisibility":"unknown"}),
        );
    }
    let body = read_input(input)?;
    Ok(
        json!({"apiVersion":1,"ok":true,"namespace":"providers","result":update(&store,&body)?,"rendererAcknowledged":false,"hostVisibility":"unknown"}),
    )
}
pub fn read_input(input: impl Read) -> Result<Value, String> {
    let mut bytes = Vec::new();
    input
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| "LOCAL_OPERATION_FAILED")?;
    if bytes.len() > 16384 {
        return Err("BODY_TOO_LARGE".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|_| "INVALID_JSON".into())
}
pub fn error(code: &str) -> (i32, Value) {
    let exit = if code == "STORE_LOCKED" {
        4
    } else if code.contains("CONFLICT") {
        3
    } else if [
        "INVALID",
        "REQUIRED",
        "INACTIVE",
        "SHARED",
        "BODY_TOO_LARGE",
    ]
    .iter()
    .any(|s| code.contains(s))
    {
        2
    } else {
        1
    };
    (
        exit,
        json!({"apiVersion":1,"ok":false,"error":{"code":code,"retryable":code=="STORE_LOCKED"},"committed":null}),
    )
}
pub fn command(args: &[String]) -> i32 {
    let (exit, value) = match execute(args, std::io::stdin().lock()) {
        Ok(v) => (0, v),
        Err(code) => error(&code),
    };
    println!("{value}");
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(id: &str, revision: u64) -> Value {
        json!({"schemaVersion":1,"requestId":id,"expectedRevision":revision,"weather":{"enabled":true,"city":"東京"}})
    }
    #[test]
    fn strict_policy_input_and_numeric_normalization() {
        let mut valid = input("once", 0);
        valid["expectedRevision"] = serde_json::from_str("0.0").unwrap();
        assert_eq!(normalized_input(&valid).unwrap(), input("once", 0));
        for field in ["url", "target", "latitude"] {
            let mut bad = valid.clone();
            bad[field] = json!("unexpected");
            assert!(normalized_input(&bad).is_err());
        }
        for city in ["invalid", "Taipei", ""] {
            let mut bad = valid.clone();
            bad["weather"]["city"] = json!(city);
            assert!(normalized_input(&bad).is_err());
        }
    }
    #[test]
    fn native_policy_replays_recovers_pointer_and_keeps_backup() {
        let root = std::env::temp_dir().join(format!(
            "atmos-native-provider-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        let store = Store::new(&root).unwrap();
        assert_eq!(settings(&store).unwrap(), defaults());
        let intent = input("once", 0);
        assert_eq!(update(&store, &intent).unwrap()["revision"], 1);
        assert_eq!(update(&store, &intent).unwrap()["replayed"], true);
        assert_eq!(
            update(&store, &input("stale", 0)).unwrap_err(),
            "PROVIDERS_REVISION_CONFLICT"
        );
        let mut conflicting = intent.clone();
        conflicting["weather"]["enabled"] = json!(false);
        assert_eq!(
            update(&store, &conflicting).unwrap_err(),
            "PROVIDERS_REQUEST_CONFLICT"
        );
        let name = format!("requests/provider.{:x}.json", Sha256::digest(b"once"));
        let mut receipt = store.read_optional(&name).unwrap().unwrap();
        receipt["pending"] = json!(true);
        store.atomic(&name, &receipt).unwrap();
        assert_eq!(update(&store, &intent).unwrap()["replayed"], true);
        assert_eq!(settings(&store).unwrap()["revision"], 1);
        assert_eq!(
            store
                .read_optional("provider-settings.previous.json")
                .unwrap()
                .unwrap(),
            defaults()
        );
        assert!(!root.join("live.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
