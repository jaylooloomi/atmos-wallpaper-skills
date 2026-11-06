//! Native Agent state/validate/apply/result entry points. No process launch or
//! desktop startup. The JSON watcher is migrated separately from this CLI.
use crate::{
    appearance_core::{self, Core},
    asset_store,
    js_json::{self, object as obj, Value},
    portable_store,
};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct Options {
    pub(crate) action: String,
    pub(crate) value: Option<String>,
    pub(crate) root: PathBuf,
    pub(crate) target: Option<String>,
    pub(crate) wait: u64,
    pub(crate) poll: u64,
    pub(crate) settle: u64,
    pub(crate) duration: u64,
}
pub(crate) struct Routing {
    mode: String,
    scope: String,
    target: Option<String>,
    expected: Vec<Option<String>>,
}
pub fn is_action(s: &str) -> bool {
    [
        "--get-state",
        "--validate-json",
        "--apply-json",
        "--get-result",
        "--watch-json",
    ]
    .contains(&s)
}
pub(crate) fn parse(args: &[String]) -> Result<Options, String> {
    let action = args
        .first()
        .filter(|s| is_action(s))
        .ok_or("ARGUMENT_INVALID")?
        .clone();
    let mut at = 1;
    let mut value = None;
    if action != "--get-state" {
        let v = args
            .get(at)
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .ok_or("ARGUMENT_INVALID")?;
        value = Some(v.clone());
        at += 1;
    }
    let mut poll = 250;
    let mut settle = 750;
    let mut duration = 0;
    let mut root = None;
    let mut target = None;
    let mut wait = if action == "--apply-json" { 3000 } else { 0 };
    let mut seen = BTreeSet::new();
    while at < args.len() {
        let key = &args[at];
        let v = args.get(at + 1).ok_or("ARGUMENT_INVALID")?;
        at += 2;
        if !seen.insert(key) {
            return Err("ARGUMENT_INVALID".into());
        }
        match key.as_str() {
            "--store" => {
                if !portable_store::node_absolute(v) {
                    return Err("ARGUMENT_INVALID".into());
                }
                root = Some(PathBuf::from(v));
            }
            "--target" => {
                if !appearance_core::display_id(v) {
                    return Err("TARGET_INVALID".into());
                }
                target = Some(v.clone());
            }
            "--wait-ms" if ["--apply-json", "--get-result"].contains(&action.as_str()) => {
                if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("ARGUMENT_INVALID".into());
                }
                wait = v
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n <= 30000)
                    .ok_or("ARGUMENT_INVALID")?;
            }
            "--poll-ms" | "--settle-ms" | "--duration-ms" if action == "--watch-json" => {
                if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("ARGUMENT_INVALID".into());
                }
                let n = v.parse::<u64>().map_err(|_| "ARGUMENT_INVALID")?;
                match key.as_str() {
                    "--poll-ms" => {
                        if !(100..=5000).contains(&n) {
                            return Err("ARGUMENT_INVALID".into());
                        }
                        poll = n;
                    }
                    "--settle-ms" => {
                        if !(500..=10000).contains(&n) {
                            return Err("ARGUMENT_INVALID".into());
                        }
                        settle = n;
                    }
                    _ => {
                        if n > 86400000 {
                            return Err("ARGUMENT_INVALID".into());
                        }
                        duration = n;
                    }
                }
            }
            _ => return Err("ARGUMENT_INVALID".into()),
        }
    }
    let root = root
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join("io.atmos.wallpaper/store"))
        })
        .ok_or("STORE_UNAVAILABLE")?;
    if action == "--watch-json" && settle < 2 * poll {
        return Err("ARGUMENT_INVALID".into());
    }
    Ok(Options {
        action,
        value,
        root,
        target,
        wait,
        poll,
        settle,
        duration,
    })
}
pub fn layout(core: &Core) -> Result<Value, String> {
    let file = core.owner().path("display-layout.json")?;
    if !file.exists() {
        return Ok(obj([
            ("mode", "single".into()),
            ("targets", Value::Array(vec![])),
            ("askOnApply", false.into()),
        ]));
    }
    let v = asset_store::read_json(&file, 16384).map_err(|_| "LAYOUT_INVALID")?;
    let l = &v["layout"];
    let valid_targets = l["targets"].as_array().is_some_and(|a| {
        a.len() <= 16
            && a.iter()
                .all(|v| v.as_str().is_some_and(appearance_core::display_id))
            && a.iter()
                .filter_map(Value::as_str)
                .collect::<BTreeSet<_>>()
                .len()
                == a.len()
    });
    if !v.as_object().is_some_and(|o| o.len() == 4)
        || !v.has("lastRequestId")
        || v["schemaVersion"].as_num() != Some(1.0)
        || v["revision"].safe_uint().is_none()
        || (!v["lastRequestId"].is_null()
            && appearance_core::request_id(&v["lastRequestId"], 80).is_err())
        || !l.as_object().is_some_and(|o| o.len() == 3)
        || !l["mode"]
            .as_str()
            .is_some_and(|s| ["single", "independent", "duplicate", "span"].contains(&s))
        || l["askOnApply"].as_bool().is_none()
        || !valid_targets
        || (l["mode"].as_str() == Some("single")) != l["targets"].as_array().unwrap().is_empty()
    {
        return Err("LAYOUT_INVALID".into());
    }
    Ok(l.clone())
}
pub fn read_command(file: &Path, store: &Path) -> Result<Value, String> {
    let text = file.to_str().ok_or("INPUT_PATH_INVALID")?;
    let b = text.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || !b"\\/".contains(&b[2]) {
        return Err("INPUT_PATH_INVALID".into());
    }
    let meta = fs::symlink_metadata(file).map_err(|_| "ENOENT")?;
    if !meta.is_file() || meta.file_attributes() & 0x400 != 0 {
        return Err("INPUT_NOT_REGULAR".into());
    }
    let real = file.canonicalize().map_err(|_| "ENOENT")?;
    let owner = store.canonicalize().map_err(|_| "ENOENT")?;
    let real = portable_store::external_path(&real).to_lowercase();
    let owner = portable_store::external_path(&owner).to_lowercase();
    if real == owner || real.starts_with(&(owner.trim_end_matches('\\').to_string() + "\\")) {
        return Err("INTERNAL_STORE_INPUT_DENIED".into());
    }
    let mut f = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x00200000)
        .open(file)
        .map_err(|_| "LOCAL_OPERATION_FAILED")?;
    let m = f.metadata().map_err(|_| "INPUT_NOT_REGULAR")?;
    if !m.is_file() || m.file_attributes() & 0x400 != 0 {
        return Err("INPUT_NOT_REGULAR".into());
    }
    if m.len() > 32768 {
        return Err("BODY_TOO_LARGE".into());
    }
    let mut bytes = Vec::new();
    f.by_ref()
        .take(32769)
        .read_to_end(&mut bytes)
        .map_err(|_| "LOCAL_OPERATION_FAILED")?;
    if bytes.len() > 32768 {
        return Err("BODY_TOO_LARGE".into());
    }
    js_json::parse(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
}
pub(crate) fn status(core: &Core, id: Option<&str>, routing: &Routing) -> Result<Value, String> {
    let _guard = core.lock()?;
    let live = core.live()?;
    let mut receipt = None;
    if let Some(id) = id {
        appearance_core::request_id(&Value::from(id), 80)?;
        let key = format!("ui.{id}.apply");
        let archived = format!("requests/{}.json", asset_store::hash_text(&key));
        receipt = if let Some(r) = live["receipts"].get(&key) {
            Some(r.clone())
        } else if core.path(&archived)?.exists() {
            Some(core.read(&archived)?)
        } else {
            None
        };
        if receipt
            .as_ref()
            .is_some_and(|r| r["uncertain"].as_bool() == Some(true))
        {
            return Err("RECOVERED_OUTCOME_UNKNOWN".into());
        }
    }
    let target = if id.is_some() {
        receipt.as_ref().map(|r| &r["result"])
    } else {
        Some(&live)
    };
    let revision = target.map(|t| t["revision"].clone()).unwrap_or(Value::Null);
    let desired_hash = target
        .map(|t| t["desiredHash"].clone())
        .unwrap_or(Value::Null);
    let superseded =
        target.is_some() && (revision != live["revision"] || desired_hash != live["desiredHash"]);
    let now = crate::journal::now_ms() as i128;
    let mut reports = Vec::new();
    let mut captured = Vec::new();
    for expected in &routing.expected {
        let name = expected
            .as_ref()
            .map(|id| format!("observations/wallpaper-{id}.json"))
            .unwrap_or("observations/wallpaper.json".into());
        let observation = core.read(&name).ok();
        let expected_id = expected.as_ref().or(routing.target.as_ref());
        let matched = observation.as_ref().is_some_and(|o| {
            let age = o["capturedAt"]
                .as_str()
                .and_then(appearance_core::timestamp_ms)
                .map(|stamp| now - stamp);
            target.is_some()
                && !superseded
                && o["rendererRole"].as_str() == Some("wallpaper")
                && o["source"].as_str() == Some("local-bridge")
                && o["stateApplied"].as_bool() == Some(true)
                && o["revision"] == revision
                && o["desiredHash"] == desired_hash
                && age.is_some_and(|a| (0..4000).contains(&a))
                && expected_id.is_none_or(|id| {
                    o["targetId"].as_str() == Some(id)
                        && o["hostSessionId"].as_str().is_some_and(|s| !s.is_empty())
                })
        });
        let captured_at = if matched {
            let v = observation.unwrap()["capturedAt"].clone();
            captured.push(v.as_str().unwrap().to_string());
            v
        } else {
            Value::Null
        };
        reports.push(obj([
            (
                "targetId",
                expected_id
                    .map(|s| Value::from(s.as_str()))
                    .unwrap_or(Value::Null),
            ),
            ("acknowledged", matched.into()),
            ("capturedAt", captured_at),
        ]));
    }
    let acknowledged = !reports.is_empty()
        && reports
            .iter()
            .all(|r| r["acknowledged"].as_bool() == Some(true));
    captured.sort();
    let mut output = obj([
        ("apiVersion", 1.into()),
        ("ok", true.into()),
        ("revision", revision),
        ("desiredHash", desired_hash),
        ("currentRevision", live["revision"].clone()),
        ("superseded", superseded.into()),
        ("rendererAcknowledged", acknowledged.into()),
        (
            "acknowledgement",
            obj([
                (
                    "source",
                    if acknowledged {
                        "wallpaper-renderer"
                    } else {
                        "none"
                    }
                    .into(),
                ),
                (
                    "capturedAt",
                    if acknowledged {
                        captured[0].clone().into()
                    } else {
                        Value::Null
                    },
                ),
                ("freshForMs", 4000.into()),
                (
                    "trust",
                    "renderer-reported; not OS or visual verification".into(),
                ),
            ]),
        ),
        ("hostVisibility", "unknown".into()),
        ("targetMode", routing.mode.clone().into()),
        ("targetScope", routing.scope.clone().into()),
        (
            "selectedTarget",
            routing
                .target
                .clone()
                .map(Value::from)
                .unwrap_or(Value::Null),
        ),
        ("targetAcknowledgements", Value::Array(reports)),
    ]);
    if let Some(id) = id {
        output.set("requestId", id.into());
        output.set("found", receipt.is_some().into());
        output.set("committed", receipt.is_some().into());
    } else {
        output.set("desired", live["desired"].clone());
    }
    Ok(output)
}
pub(crate) fn prepare(options: &Options) -> Result<(Core, Routing, Value), String> {
    let owner = Core::open(&options.root)?;
    owner.init()?;
    let layout = layout(&owner)?;
    let mode = layout["mode"].as_str().unwrap().to_string();
    let ids = layout["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let mut routing = Routing {
        mode: mode.clone(),
        scope: "shared".into(),
        target: None,
        expected: vec![None],
    };
    let core = if mode == "independent" {
        let id = options.target.as_ref().ok_or("TARGET_REQUIRED")?;
        if !ids.contains(id) {
            return Err("TARGET_INACTIVE".into());
        }
        routing.scope = id.clone();
        routing.target = Some(id.clone());
        owner.ensure_scope(id)?
    } else {
        if ["duplicate", "span"].contains(&mode.as_str()) {
            if options.target.as_ref().is_some_and(|id| !ids.contains(id)) {
                return Err("TARGET_INACTIVE".into());
            }
            routing.expected = ids.into_iter().map(Some).collect();
        } else if options.target.is_some() {
            return Err("TARGET_INACTIVE".into());
        }
        owner
    };
    Ok((core, routing, layout))
}
pub fn execute(args: &[String]) -> Result<Value, String> {
    let options = parse(args)?;
    let (core, routing, layout) = prepare(&options)?;
    if options.action == "--watch-json" {
        return crate::appearance_watch::run(&options, &core, &routing, &layout);
    }
    if options.action == "--get-state" {
        return status(&core, None, &routing);
    }
    let mut id = options.value.unwrap();
    let mut replayed = false;
    if ["--apply-json", "--validate-json"].contains(&options.action.as_str()) {
        let command = read_command(Path::new(&id), &options.root)?;
        Core::command_patch(&command)?;
        id = command["requestId"].as_str().unwrap().into();
        if options.action == "--validate-json" {
            return core.validate_command(&command);
        }
        replayed = core.execute_command(&command)?["replayed"].as_bool() == Some(true);
    }
    let start = crate::journal::now_ms();
    let mut result = status(&core, Some(&id), &routing)?;
    while result["committed"].as_bool() == Some(true)
        && result["rendererAcknowledged"].as_bool() != Some(true)
        && result["superseded"].as_bool() != Some(true)
        && crate::journal::now_ms().saturating_sub(start) < options.wait
    {
        std::thread::sleep(Duration::from_millis(
            200.min(
                options
                    .wait
                    .saturating_sub(crate::journal::now_ms().saturating_sub(start)),
            ),
        ));
        result = status(&core, Some(&id), &routing)?;
    }
    if options.action == "--apply-json" {
        result.set("replayed", replayed.into());
    }
    result.set(
        "waitedMs",
        crate::journal::now_ms().saturating_sub(start).into(),
    );
    Ok(result)
}
pub fn command(args: &[String]) -> i32 {
    let (exit, result) = match execute(args) {
        Ok(v) => (0, v),
        Err(code) => {
            let conflicts = [
                "TARGET_SCOPE_CHANGED",
                "REVISION_CONFLICT",
                "REQUEST_CONFLICT",
                "RECOVERED_OUTCOME_UNKNOWN",
                "PREVIEW_STALE",
                "PREVIEW_EXPIRED",
                "PREVIEW_CANCELLED",
            ];
            let invalid = [
                "TARGET_REQUIRED",
                "TARGET_INVALID",
                "TARGET_INACTIVE",
                "ARGUMENT_INVALID",
                "INPUT_PATH_INVALID",
                "INPUT_NOT_REGULAR",
                "INTERNAL_STORE_INPUT_DENIED",
                "BODY_TOO_LARGE",
                "INVALID_JSON",
                "COMMAND_INVALID",
                "INVALID_REQUEST_ID",
                "INVALID_REVISION",
                "UNKNOWN_FIELD",
                "INVALID_STATE",
                "EFFECT_REMOVED",
                "FIXED_SETTING",
            ];
            let exit = if code == "STORE_LOCKED" {
                4
            } else if conflicts.contains(&code.as_str()) {
                3
            } else if invalid.contains(&code.as_str()) {
                2
            } else {
                1
            };
            (exit,obj([("apiVersion",1.into()),("ok",false.into()),("error",obj([("code",if exit==1{"LOCAL_OPERATION_FAILED"}else{&code}.into()),("retryable",(code=="STORE_LOCKED").into())])),("committed",Value::Null),("note","No success claimed. Query --get-result before retrying an unknown commit outcome.".into())]))
        }
    };
    println!("{}", result.canonical());
    exit
}
