//! Shared durable policy commit mechanics. Namespace-specific schemas stay typed.
use crate::{host_settings, windows};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
// Reject reparse points and invalid record types before any creation/read/write.
pub fn safe_path(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    for p in path.ancestors() {
        match fs::symlink_metadata(p) {
            Ok(m) => {
                if m.file_attributes() & 0x400 != 0 {
                    return Err("POLICY_PATH_INVALID".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("POLICY_PATH_INVALID".into()),
        }
    }
    Ok(())
}
pub fn read_optional(path: &Path, max: u64) -> Result<Option<Value>, String> {
    safe_path(path)?;
    let m = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("POLICY_RECORD_INVALID".into()),
    };
    if !m.is_file() || m.len() > max {
        return Err("POLICY_RECORD_INVALID".into());
    }
    let b = fs::read(path).map_err(|_| "POLICY_RECORD_INVALID")?;
    if b.len() as u64 > max {
        return Err("POLICY_RECORD_INVALID".into());
    }
    serde_json::from_slice(&b)
        .map(Some)
        .map_err(|_| "POLICY_RECORD_INVALID".into())
}
fn receipt(v: &Value) -> bool {
    v.as_object().is_some_and(|o| o.len() == 3)
        && v["fingerprint"]
            .as_str()
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        && v["pending"].is_boolean()
        && v["result"]["committed"] == true
        && v["result"]["revision"]
            .as_u64()
            .is_some_and(|r| r > 0 && r <= 9_007_199_254_740_991)
}
pub fn commit(
    root: &Path,
    namespace: &str,
    file: &str,
    default: Value,
    id: &str,
    expected: u64,
    input: &Value,
    validate: impl Fn(&Value) -> Result<(), String>,
    make: impl FnOnce(Value, u64) -> Result<Value, String>,
) -> Result<Value, String> {
    if !valid_id(id) {
        return Err(format!("{namespace}_INVALID"));
    }
    safe_path(root)?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let _lock = windows::named_singleton(&format!(
        "Local\\AtmosWallpaper.{namespace}.{:x}",
        Sha256::digest(root.to_string_lossy().to_lowercase().as_bytes())
    ))
    .map_err(|_| format!("{namespace}_BUSY"))?;
    let record = root.join(file);
    let old = read_optional(&record, 16384)?.unwrap_or(default);
    if old["schemaVersion"] != 1
        || old["revision"]
            .as_u64()
            .is_none_or(|r| r > 9_007_199_254_740_991)
    {
        return Err(format!("{namespace}_INVALID"));
    }
    validate(&old)?;
    let receipts = root.join(format!("{}-requests", namespace.to_lowercase()));
    safe_path(&receipts)?;
    let name = |id: &str| format!("{:x}.json", Sha256::digest(id.as_bytes()));
    if let Some(previous) = old["lastRequestId"].as_str() {
        if !valid_id(previous) {
            return Err(format!("{namespace}_INVALID"));
        }
        if let Some(mut r) = read_optional(&receipts.join(name(previous)), 16384)? {
            if !receipt(&r) {
                return Err(format!("{namespace}_INVALID"));
            }
            if r["pending"] == true && r["result"]["revision"] == old["revision"] {
                r["pending"] = json!(false);
                host_settings::atomic(&receipts, &name(previous), &r)?;
            }
        }
    }
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(input).unwrap()));
    if let Some(r) = read_optional(&receipts.join(name(id)), 16384)? {
        if !receipt(&r) {
            return Err(format!("{namespace}_INVALID"));
        }
        if r["fingerprint"] != fingerprint {
            return Err(format!("{namespace}_REQUEST_CONFLICT"));
        }
        if r["pending"] == false {
            let mut result = r["result"].clone();
            result["replayed"] = json!(true);
            return Ok(result);
        }
    }
    let revision = old["revision"].as_u64().unwrap();
    if revision != expected {
        return Err(format!("{namespace}_REVISION_CONFLICT"));
    }
    let new_revision = revision
        .checked_add(1)
        .filter(|r| *r <= 9_007_199_254_740_991)
        .ok_or(format!("{namespace}_INVALID"))?;
    let next = make(old.clone(), new_revision)?;
    validate(&next)?;
    if next["revision"] != new_revision || next["lastRequestId"] != id {
        return Err(format!("{namespace}_INVALID"));
    }
    let result = json!({"committed":true,"revision":next["revision"]});
    host_settings::atomic(
        &receipts,
        &name(id),
        &json!({"fingerprint":fingerprint,"pending":true,"result":result}),
    )?;
    safe_path(&root.join(file.replace(".json", ".previous.json")))?;
    host_settings::atomic(&root, &file.replace(".json", ".previous.json"), &old)?;
    host_settings::atomic(&root, file, &next)?;
    host_settings::atomic(
        &receipts,
        &name(id),
        &json!({"fingerprint":fingerprint,"pending":false,"result":result}),
    )?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_record_cannot_be_replayed_even_with_matching_old_receipt() {
        let root = std::env::temp_dir().join(format!(
            "atmos-policy-invalid-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let record =
            json!({"schemaVersion":1,"revision":1,"lastRequestId":"same","unexpected":true});
        host_settings::atomic(&root, "test.json", &record).unwrap();
        let receipt_name = format!("{:x}.json", Sha256::digest(b"same"));
        host_settings::atomic(&root.join("test-requests"),&receipt_name,&json!({"fingerprint":format!("{:x}",Sha256::digest(serde_json::to_vec(&json!({})).unwrap())),"pending":false,"result":{"committed":true,"revision":1}})).unwrap();
        let r = commit(
            &root,
            "TEST",
            "test.json",
            json!({}),
            "same",
            1,
            &json!({}),
            |_| Err("TYPED_INVALID".into()),
            |_, _| panic!("must validate before replay"),
        );
        assert_eq!(r.unwrap_err(), "TYPED_INVALID");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn bounded_record_and_directory_rejection() {
        let root = std::env::temp_dir().join(format!(
            "atmos-policy-bound-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        assert!(read_optional(&root, 16384).is_err());
        fs::write(root.join("large.json"), vec![b' '; 16385]).unwrap();
        assert!(read_optional(&root.join("large.json"), 16384).is_err());
        assert!(read_optional(&root.join("missing.json"), 16384)
            .unwrap()
            .is_none());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn pointer_bound_pending_receipt_recovers_without_second_revision() {
        let root = std::env::temp_dir().join(format!(
            "atmos-policy-recover-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let input = json!({"intent":"enable"});
        let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&input).unwrap()));
        let name = format!("{:x}.json", Sha256::digest(b"once"));
        host_settings::atomic(
            &root,
            "test.json",
            &json!({"schemaVersion":1,"revision":1,"lastRequestId":"once","enabled":true}),
        )
        .unwrap();
        host_settings::atomic(&root.join("test-requests"),&name,&json!({"fingerprint":fingerprint,"pending":true,"result":{"committed":true,"revision":1}})).unwrap();
        let result = commit(
            &root,
            "TEST",
            "test.json",
            json!({}),
            "once",
            0,
            &input,
            |v| {
                if v["enabled"].is_boolean() {
                    Ok(())
                } else {
                    Err("BAD_POLICY".into())
                }
            },
            |_, _| panic!("must replay already committed pointer"),
        )
        .unwrap();
        assert_eq!(result["revision"], 1);
        assert_eq!(result["replayed"], true);
        assert_eq!(
            read_optional(&root.join("test-requests").join(name), 16384)
                .unwrap()
                .unwrap()["pending"],
            false
        );
        fs::remove_dir_all(root).unwrap();
    }
}
