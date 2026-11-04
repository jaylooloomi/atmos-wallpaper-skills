use crate::{displays, host_settings, policy_store, windows};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub mode: String,
    pub targets: Vec<String>,
    pub ask_on_apply: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub layout: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            last_request_id: None,
            layout: Policy {
                mode: "single".into(),
                targets: Vec::new(),
                ask_on_apply: false,
            },
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Update {
    schema_version: u32,
    request_id: String,
    expected_revision: u64,
    layout: Policy,
}
pub fn valid(s: &Settings) -> Result<(), String> {
    let p = &s.layout;
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || s.last_request_id
            .as_ref()
            .is_some_and(|s| !policy_store::valid_id(s))
        || !matches!(
            p.mode.as_str(),
            "single" | "independent" | "duplicate" | "span"
        )
        || p.targets.len() > 16
        || p.targets.iter().any(|id| {
            id.len() != 72
                || !id.starts_with("display-")
                || !id[8..]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b"abcdef".contains(&b))
        })
        || p.targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != p.targets.len()
        || (if p.mode == "single" {
            !p.targets.is_empty()
        } else {
            p.targets.is_empty()
        })
    {
        return Err("LAYOUT_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let value = policy_store::read_optional(&root.join("display-layout.json"), 16384)
        .map_err(|_| "LAYOUT_INVALID")?;
    let s = match value {
        Some(v) => serde_json::from_value(v).map_err(|_| "LAYOUT_INVALID")?,
        None => Settings::default(),
    };
    valid(&s)?;
    Ok(s)
}
pub fn update(root: &Path, u: Update) -> Result<Value, String> {
    if u.schema_version != 1 {
        return Err("LAYOUT_INVALID".into());
    }
    let input = serde_json::to_value(&u).unwrap();
    let id = u.request_id.clone();
    policy_store::commit(
        root,
        "LAYOUT",
        "display-layout.json",
        serde_json::to_value(Settings::default()).unwrap(),
        &id,
        u.expected_revision,
        &input,
        |record| {
            let s: Settings =
                serde_json::from_value(record.clone()).map_err(|_| "LAYOUT_INVALID")?;
            valid(&s)
        },
        |old, revision| {
            let old: Settings = serde_json::from_value(old).map_err(|_| "LAYOUT_INVALID")?;
            valid(&old)?;
            if let Some(v) = policy_store::read_optional(&root.join(".lock"), 1024)
                .map_err(|_| "LAYOUT_STORE_BUSY")?
            {
                let pid = v["pid"]
                    .as_u64()
                    .filter(|p| *p > 0 && *p <= u32::MAX as u64)
                    .ok_or("LAYOUT_STORE_BUSY")?;
                if windows::process_alive(pid as u32) {
                    return Err("LAYOUT_STORE_BUSY".into());
                }
            }
            let next = Settings {
                schema_version: 1,
                revision,
                last_request_id: Some(id.clone()),
                layout: u.layout,
            };
            valid(&next)?;
            let ds = displays::enumerate()?;
            for target in &next.layout.targets {
                if !old.layout.targets.contains(target) {
                    displays::select(&ds, target)?;
                }
            }
            host_settings::atomic(
                root,
                "display-layout-validation.json",
                &json!({"revision":revision,"validatedAtMs":crate::journal::now_ms()}),
            )?;
            Ok(serde_json::to_value(next).unwrap())
        },
    )
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len()) || !matches!(args[0].as_str(), "get" | "update") {
        return Err("LAYOUT_INVALID".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("LAYOUT_INVALID".into());
        }
        p
    } else {
        host_settings::directory()?
    };
    let result = if args[0] == "update" {
        let mut b = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        if b.len() > 16384 {
            return Err("LAYOUT_INVALID".into());
        }
        Some(update(
            &root,
            serde_json::from_slice(&b).map_err(|_| "LAYOUT_INVALID")?,
        )?)
    } else {
        None
    };
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":read(&root)?,"result":result})
    );
    Ok(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_modes_and_unique_explicit_ids() {
        let mut s = Settings::default();
        assert!(valid(&s).is_ok());
        s.layout.mode = "independent".into();
        assert!(valid(&s).is_err());
        s.layout.targets = vec![format!("display-{}", "a".repeat(64))];
        assert!(valid(&s).is_ok());
        s.layout.targets.push(s.layout.targets[0].clone());
        assert!(valid(&s).is_err());
    }
}
