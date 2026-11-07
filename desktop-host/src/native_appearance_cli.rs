use crate::{
    appearance_core::Core,
    host_settings,
    js_json::{self, object as obj, Value},
    layout, portable_store,
};
use std::{io::Read, path::PathBuf};

pub(crate) fn is_action(arg: &str) -> bool {
    matches!(arg, "--preset-command" | "--schedule-command")
}

fn run(args: &[String], input: impl Read) -> Result<Value, String> {
    let preset = args[0] == "--preset-command";
    let action = args.get(1).map(String::as_str).ok_or("ARGUMENT_INVALID")?;
    let allowed: &[&str] = if preset {
        &["get", "save", "apply", "delete"]
    } else {
        &["get", "update"]
    };
    if !allowed.contains(&action) {
        return Err("ARGUMENT_INVALID".into());
    }
    let mut root = None;
    let mut target = None;
    let mut options = args[2..].chunks_exact(2);
    for pair in &mut options {
        let (key, v) = (pair[0].as_str(), pair[1].as_str());
        match key {
            "--store" if root.is_none() => {
                let path = PathBuf::from(v);
                if !path.is_absolute() {
                    return Err("ARGUMENT_INVALID".into());
                }
                portable_store::validate_root(&path)?;
                root = Some(path);
            }
            "--target" if target.is_none() => {
                if v != "shared" && !crate::appearance_core::display_id(v) {
                    return Err("TARGET_INVALID".into());
                }
                target = Some(v);
            }
            _ => return Err("ARGUMENT_INVALID".into()),
        }
    }
    if !options.remainder().is_empty() {
        return Err("ARGUMENT_INVALID".into());
    }
    let root = root.map(Ok).unwrap_or_else(host_settings::directory)?;
    let owner = Core::open(&root)?;
    owner.init()?;
    let policy = layout::read(&root)?.layout;
    let child = if policy.mode == "independent" {
        let id = target
            .filter(|s| crate::appearance_core::display_id(s))
            .ok_or("TARGET_REQUIRED")?;
        if !policy.targets.iter().any(|v| v == id) {
            return Err("TARGET_INACTIVE".into());
        }
        Some(owner.ensure_scope(id)?)
    } else {
        if target.is_some_and(|s| s != "shared") {
            return Err("TARGET_SCOPE_SHARED".into());
        }
        None
    };
    let core = child.as_ref().unwrap_or(&owner);
    let mut result = obj([
        ("apiVersion", 1.into()),
        ("ok", true.into()),
        (
            "namespace",
            if preset { "preset" } else { "schedule" }.into(),
        ),
        ("rendererAcknowledged", false.into()),
        ("hostVisibility", "unknown".into()),
    ]);
    if action == "get" {
        let status = if preset {
            crate::presets::status(core)?
        } else {
            crate::schedule::status(core)?
        };
        if let (Value::Object(out), Value::Object(status)) = (&mut result, status) {
            out.extend(status);
        }
    } else {
        let mut bytes = Vec::new();
        input
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "LOCAL_OPERATION_FAILED")?;
        if bytes.len() > 16384 {
            return Err("BODY_TOO_LARGE".into());
        }
        let body = js_json::parse(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
            .map_err(|_| "INVALID_JSON")?;
        if preset && body["action"].as_str() != Some(action) {
            return Err("PRESET_COMMAND_INVALID".into());
        }
        let outcome = if preset {
            crate::presets::execute(core, &body)?
        } else {
            crate::schedule::set(core, &body)?
        };
        result.set("result", outcome);
    }
    Ok(result)
}

pub(crate) fn command(args: &[String]) -> i32 {
    match run(args, std::io::stdin().lock()) {
        Ok(v) => {
            println!("{}", v.canonical());
            0
        }
        Err(code) => {
            let (exit, v) = crate::providers::error(&code);
            println!("{v}");
            exit
        }
    }
}
