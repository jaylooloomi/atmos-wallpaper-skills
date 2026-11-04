//! Host settings are separate from desired appearance/live.json and use strict CAS.
use crate::{displays, windows};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Placement {
    pub enabled: bool,
    pub target: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub placement: Placement,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            placement: Placement {
                enabled: true,
                target: "primary".into(),
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Update {
    schema_version: u32,
    expected_revision: u64,
    placement: Placement,
}
pub fn directory() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA unavailable")?)
            .join("io.atmos.wallpaper/store"),
    )
}
fn validate(s: &Settings) -> Result<(), String> {
    let t = &s.placement.target;
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || !(t == "primary"
            || (t.len() == 72
                && t.starts_with("display-")
                && t[8..]
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())))
    {
        return Err("HOST_SETTINGS_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let file = root.join("host-settings.json");
    if fs::metadata(&file).is_ok_and(|m| m.len() > 16384) {
        return Err("HOST_SETTINGS_INVALID".into());
    }
    let data = match fs::read(&file) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(_) => return Err("HOST_SETTINGS_READ_FAILED".into()),
    };
    if data.len() > 16384 {
        return Err("HOST_SETTINGS_INVALID".into());
    }
    let s: Settings = serde_json::from_slice(&data).map_err(|_| "HOST_SETTINGS_INVALID")?;
    validate(&s)?;
    Ok(s)
}
pub fn atomic(root: &Path, name: &str, value: &impl Serialize) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let file = root.join(name);
    let temp = root.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = fs::File::create(&temp).map_err(|e| e.to_string())?;
        f.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        let wide = |p: &Path| {
            p.as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain([0])
                .collect::<Vec<_>>()
        };
        if unsafe {
            MoveFileExW(
                wide(&temp).as_ptr(),
                wide(&file).as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err("HOST_SETTINGS_WRITE_FAILED".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn update(root: &Path, u: Update) -> Result<Settings, String> {
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    let lock = format!(
        "Local\\AtmosWallpaper.HostSettings.{:x}",
        Sha256::digest(canonical.to_string_lossy().to_lowercase().as_bytes())
    );
    let _guard = windows::named_singleton(&lock).map_err(|_| "HOST_SETTINGS_BUSY")?;
    let old = read(root)?;
    if u.schema_version != 1 {
        return Err("HOST_SETTINGS_INVALID".into());
    }
    if old.revision != u.expected_revision {
        return Err("HOST_REVISION_CONFLICT".into());
    }
    let next = Settings {
        schema_version: 1,
        revision: old.revision.checked_add(1).ok_or("HOST_SETTINGS_INVALID")?,
        placement: u.placement,
    };
    validate(&next)?;
    if crate::layout::read(root)?.layout.mode != "single"
        && next.placement.target != old.placement.target
    {
        return Err("HOST_LAYOUT_ACTIVE".into());
    }
    if next.placement.enabled {
        displays::select(&displays::enumerate()?, &next.placement.target)?;
    }
    // Keep the previous good host document; do not overwrite or migrate appearance transactions.
    atomic(root, "host-settings.previous.json", &old)?;
    atomic(root, "host-settings.json", &next)?;
    Ok(next)
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len()) || !matches!(args[0].as_str(), "get" | "update") {
        return Err("Expected --host-command get|update (update JSON on stdin)".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("HOST_STORE_INVALID".into());
        }
        p
    } else {
        directory()?
    };
    if args[0] == "update" {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    }
    let settings = if args[0] == "update" {
        let mut data = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() > 16384 {
            return Err("HOST_SETTINGS_INVALID".into());
        }
        update(
            &root,
            serde_json::from_slice(&data).map_err(|_| "HOST_SETTINGS_INVALID")?,
        )?
    } else {
        read(&root)?
    };
    let status = fs::read(root.join("host-status.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    let fresh = status
        .as_ref()
        .and_then(|v| v["capturedAtMs"].as_u64())
        .is_some_and(|t| (crate::journal::now_ms() as u64).abs_diff(t) < 5000);
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":settings,"displays":displays::enumerate()?,"runtime":if fresh{status}else{None},"runtimeFresh":fresh})
    );
    Ok(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multi_layout_rejects_ignored_target_changes_but_allows_global_clear() {
        let root = std::env::temp_dir().join(format!(
            "atmos-host-multi-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        std::fs::create_dir_all(&root).unwrap();
        atomic(&root,"display-layout.json",&serde_json::json!({"schemaVersion":1,"revision":1,"lastRequestId":null,"layout":{"mode":"independent","targets":[format!("display-{}","a".repeat(64))],"askOnApply":false}})).unwrap();
        let result = update(
            &root,
            Update {
                schema_version: 1,
                expected_revision: 0,
                placement: Placement {
                    enabled: true,
                    target: format!("display-{}", "b".repeat(64)),
                },
            },
        );
        assert_eq!(result.unwrap_err(), "HOST_LAYOUT_ACTIVE");
        assert!(!root.join("host-settings.json").exists());
        assert!(
            !update(
                &root,
                Update {
                    schema_version: 1,
                    expected_revision: 0,
                    placement: Placement {
                        enabled: false,
                        target: "primary".into()
                    }
                }
            )
            .unwrap()
            .placement
            .enabled
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_store_defaults_without_mutation() {
        let root = std::env::temp_dir().join(format!("atmos-host-no-file-{}", std::process::id()));
        assert_eq!(read(&root).unwrap(), Settings::default());
        assert!(!root.join("host-settings.json").exists())
    }
    #[test]
    fn strict_schema_rejects_paths_and_unknown_fields() {
        assert!(serde_json::from_str::<Update>(r#"{"schemaVersion":1,"expectedRevision":0,"placement":{"enabled":true,"target":"primary"},"shell":"x"}"#).is_err());
        let mut s = Settings::default();
        s.placement.target = "C:/x".into();
        assert!(validate(&s).is_err());
    }
    #[test]
    fn atomic_cas_preserves_backup_and_rejects_conflict() {
        let root = std::env::temp_dir().join(format!("atmos-host-cas-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let _ = fs::remove_file(root.join("host-settings.json"));
        let u = || Update {
            schema_version: 1,
            expected_revision: 0,
            placement: Placement {
                enabled: false,
                target: "primary".into(),
            },
        };
        let next = update(&root, u()).unwrap();
        assert_eq!(next.revision, 1);
        assert_eq!(update(&root, u()).unwrap_err(), "HOST_REVISION_CONFLICT");
        let prev: Settings =
            serde_json::from_slice(&fs::read(root.join("host-settings.previous.json")).unwrap())
                .unwrap();
        assert_eq!(prev.revision, 0);
        for name in ["host-settings.json", "host-settings.previous.json"] {
            fs::remove_file(root.join(name)).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
}
