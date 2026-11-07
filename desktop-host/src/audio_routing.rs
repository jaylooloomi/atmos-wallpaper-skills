//! Logical wallpaper audio owners, separate from physical WASAPI output selection.
use crate::{host_settings, policy_store};
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
    pub target: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub routing: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            last_request_id: None,
            routing: Policy {
                mode: "primary".into(),
                target: None,
            },
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Update {
    schema_version: u32,
    request_id: String,
    expected_revision: u64,
    routing: Policy,
}
pub fn valid_id(s: &str) -> bool {
    s.len() == 72
        && s.starts_with("display-")
        && s[8..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b"abcdef".contains(&b))
}
fn validate(s: &Settings) -> Result<(), String> {
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || s.last_request_id
            .as_ref()
            .is_some_and(|id| !policy_store::valid_id(id))
        || !matches!(s.routing.mode.as_str(), "primary" | "display" | "all")
        || (if s.routing.mode == "display" {
            !s.routing.target.as_deref().is_some_and(valid_id)
        } else {
            s.routing.target.is_some()
        })
    {
        return Err("AUDIO_ROUTING_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let value = policy_store::read_optional(&root.join("audio-routing.json"), 16384)
        .map_err(|_| "AUDIO_ROUTING_INVALID")?;
    let s = match value {
        Some(v) => serde_json::from_value(v).map_err(|_| "AUDIO_ROUTING_INVALID")?,
        None => Settings::default(),
    };
    validate(&s)?;
    Ok(s)
}
pub(crate) fn update(root: &Path, u: Update) -> Result<Value, String> {
    validate(&Settings {
        schema_version: u.schema_version,
        revision: u.expected_revision,
        last_request_id: Some(u.request_id.clone()),
        routing: u.routing.clone(),
    })?;
    let input = serde_json::to_value(&u).unwrap();
    let id = u.request_id.clone();
    policy_store::commit(
        root,
        "AUDIO_ROUTING",
        "audio-routing.json",
        serde_json::to_value(Settings::default()).unwrap(),
        &id,
        u.expected_revision,
        &input,
        |record| {
            let s: Settings =
                serde_json::from_value(record.clone()).map_err(|_| "AUDIO_ROUTING_INVALID")?;
            validate(&s)
        },
        |old, revision| {
            let old: Settings = serde_json::from_value(old).map_err(|_| "AUDIO_ROUTING_INVALID")?;
            validate(&old)?;
            let next = Settings {
                schema_version: 1,
                revision,
                last_request_id: Some(id.clone()),
                routing: u.routing,
            };
            validate(&next)?;
            Ok(serde_json::to_value(next).unwrap())
        },
    )
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len()) || !matches!(args[0].as_str(), "get" | "update") {
        return Err("AUDIO_ROUTING_INVALID".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("AUDIO_ROUTING_INVALID".into());
        }
        p
    } else {
        host_settings::directory()?
    };
    let result = if args[0] == "update" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "AUDIO_ROUTING_INVALID")?;
        if bytes.len() > 16384 {
            return Err("AUDIO_ROUTING_INVALID".into());
        }
        Some(update(
            &root,
            serde_json::from_slice(&bytes).map_err(|_| "AUDIO_ROUTING_INVALID")?,
        )?)
    } else {
        None
    };
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":read(&root)?,"displays":crate::displays::enumerate().unwrap_or_default(),"result":result})
    );
    Ok(0)
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: String,
    pub primary: bool,
    pub visible: bool,
    pub kind: u32,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub owners: Vec<String>,
    pub source_kind: u32,
}
pub fn select(policy: &Policy, targets: &[Candidate]) -> Selection {
    let chosen: Vec<_> = targets
        .iter()
        .filter(|t| {
            t.visible
                && match policy.mode.as_str() {
                    "primary" => t.primary,
                    "display" => policy.target.as_ref() == Some(&t.id),
                    "all" => true,
                    _ => false,
                }
        })
        .collect();
    let kinds: std::collections::BTreeSet<_> = chosen
        .iter()
        .map(|t| t.kind)
        .filter(|k| matches!(k, 1..=3))
        .collect();
    let source_kind = if kinds.len() == 1 {
        *kinds.first().unwrap()
    } else {
        kinds.into_iter().fold(0, |mask, k| mask | (1 << (k + 2)))
    };
    Selection {
        owners: chosen.into_iter().map(|t| t.id.clone()).collect(),
        source_kind,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn target(c: char, primary: bool, visible: bool, kind: u32) -> Candidate {
        Candidate {
            id: format!("display-{}", c.to_string().repeat(64)),
            primary,
            visible,
            kind,
        }
    }
    #[test]
    fn strict_routes() {
        let mut s = Settings::default();
        assert!(validate(&s).is_ok());
        s.routing.mode = "display".into();
        assert!(validate(&s).is_err());
        s.routing.target = Some(target('a', false, true, 1).id);
        assert!(validate(&s).is_ok());
        s.routing.mode = "all".into();
        assert!(validate(&s).is_err())
    }
    #[test]
    fn owners_do_not_fall_back_and_duplicate_sounds_do_not_stack() {
        let a = target('a', true, true, 1);
        let b = target('b', false, true, 1);
        let mut p = Settings::default().routing;
        assert_eq!(select(&p, &[b.clone()]).source_kind, 0);
        p.mode = "display".into();
        p.target = Some(a.id.clone());
        assert_eq!(select(&p, &[b.clone()]).source_kind, 0);
        p.mode = "all".into();
        p.target = None;
        let both = select(&p, &[a.clone(), b]);
        assert_eq!(both.owners.len(), 2);
        assert_eq!(both.source_kind, 1);
        assert_eq!(
            select(&p, &[a, target('c', false, true, 2)]).source_kind,
            24
        );
        assert_eq!(select(&p, &[target('d', true, false, 2)]).source_kind, 0)
    }
    #[test]
    fn route_cas_is_durable_and_separate_from_audio_settings() {
        let root = std::env::temp_dir().join(format!(
            "atmos-audio-routing-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let input = || Update {
            schema_version: 1,
            request_id: "route.1".into(),
            expected_revision: 0,
            routing: Policy {
                mode: "all".into(),
                target: None,
            },
        };
        assert_eq!(update(&root, input()).unwrap()["revision"], 1);
        assert_eq!(update(&root, input()).unwrap()["replayed"], true);
        let mut stale = input();
        stale.request_id = "route.2".into();
        assert_eq!(
            update(&root, stale).unwrap_err(),
            "AUDIO_ROUTING_REVISION_CONFLICT"
        );
        assert!(!root.join("audio-settings.json").exists());
        fs::remove_dir_all(&root).unwrap()
    }
}
