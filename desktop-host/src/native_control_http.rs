//! Native host-control endpoints. The installed service is still selected elsewhere.
use crate::{
    asset_store, audio, audio_routing, disks,
    displays::Display,
    host_settings, layout, library,
    native_http::{ApiPolicy, Request, Response, Security},
    performance, policy_store,
    portable_store::Store,
    startup, telemetry, windows,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::{fs, os::windows::fs::MetadataExt, sync::Mutex};

const MAX_SAFE: u64 = 9_007_199_254_740_991;

fn error(status: u16, code: &str) -> Response {
    Response::json(status, json!({"ok":false,"error":{"code":code}}))
}

fn input<T: DeserializeOwned>(value: Value, code: &str) -> Result<T, String> {
    serde_json::from_value(value).map_err(|_| code.into())
}

fn normalize_integers(v: &mut Value) {
    match v {
        Value::Number(n) if n.is_f64() => {
            let f = n.as_f64().unwrap();
            if (0.0..=MAX_SAFE as f64).contains(&f) && f.fract() == 0.0 {
                *n = (f as u64).into();
            }
        }
        Value::Object(o) => o.values_mut().for_each(normalize_integers),
        Value::Array(a) => a.iter_mut().for_each(normalize_integers),
        _ => {}
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartupInput {
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Revision {
    schema_version: u32,
    expected_revision: u64,
}

impl Revision {
    fn validate(&self, code: &str) -> Result<(), String> {
        if self.schema_version == 1 && self.expected_revision <= MAX_SAFE {
            Ok(())
        } else {
            Err(code.into())
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
enum LibraryAction {
    Open(LibraryInput),
    Choose(LibraryInput),
    Reset(LibraryInput),
    Audit(LibraryInput),
    Archive(ArchiveInput),
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LibraryInput {
    schema_version: u32,
    expected_revision: u64,
    request_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveInput {
    schema_version: u32,
    expected_revision: u64,
    request_id: String,
    audit_hash: String,
}

fn recent(now: u64, at: &Value, age: u64) -> Option<u64> {
    now.checked_sub(at.as_u64()?)
        .filter(|elapsed| *elapsed < age)
}

#[derive(Default)]
struct Sample {
    attempt: Option<u64>,
    value: Option<Value>,
}

impl Sample {
    fn due(&self, now: u64, interval: u64) -> bool {
        self.attempt
            .is_none_or(|last| now.checked_sub(last).is_none_or(|age| age >= interval))
    }
}

#[derive(Default)]
struct SystemCache {
    system: Sample,
    disk: Sample,
}

fn valid_system(v: &Value) -> bool {
    let memory = &v["memory"];
    let network = &v["network"];
    v["cpuPercent"]
        .as_f64()
        .is_some_and(|n| (0.0..=100.0).contains(&n))
        && memory["totalBytes"]
            .as_u64()
            .zip(memory["usedBytes"].as_u64())
            .is_some_and(|(total, used)| total > 0 && total <= MAX_SAFE && used <= total)
        && (network.is_null()
            || (network["available"] == true
                && matches!(
                    network["scope"].as_str(),
                    Some("active-physical-interfaces" | "active-nonloopback-interfaces")
                )
                && network["adapterCount"]
                    .as_u64()
                    .is_some_and(|n| (1..=4096).contains(&n))
                && ["receiveBytesPerSecond", "sendBytesPerSecond"]
                    .iter()
                    .all(|k| network[k].as_f64().is_some_and(|n| n >= 0.0))
                && network["sampleWindowMs"]
                    .as_f64()
                    .is_some_and(|n| (1.0..=10000.0).contains(&n))))
}

impl SystemCache {
    fn get(
        &mut self,
        clock: impl Fn() -> u64,
        system: impl FnOnce() -> Result<Value, String>,
        disk: impl FnOnce() -> Value,
    ) -> Value {
        let now = clock();
        if self.system.due(now, 2000) {
            self.system.attempt = Some(now);
            self.system.value = system().ok().filter(valid_system);
        }
        let now = clock();
        if self.disk.due(now, 30000) {
            self.disk.attempt = Some(now);
            let v = disk();
            self.disk.value = recent(clock(), &v["sampledAtMs"], 15001).map(|_| v);
        }
        let now = clock();
        if let Some(mut v) = self.system.value.clone() {
            if let Some(age) = recent(now, &v["sampledAtMs"], 15000) {
                let disk = self
                    .disk
                    .value
                    .as_ref()
                    .filter(|d| recent(now, &d["sampledAtMs"], 45000).is_some());
                v["ageMs"] = age.into();
                v["stale"] = false.into();
                v["disks"] = disk.map_or(Value::Null, |d| d["disks"].clone());
                v["disksSampledAtMs"] = disk.map_or(Value::Null, |d| d["sampledAtMs"].clone());
                return v;
            }
        }
        json!({"schemaVersion":1,"available":false,"source":"windows-native-readonly","error":{"code":"SYSTEM_UNAVAILABLE"},"network":null,"gpu":null,"power":null})
    }
}

#[derive(Default)]
pub(crate) struct Control {
    // Some for the temporary fixture. Registry and shell calls are disabled there.
    fixture_startup: Mutex<Option<bool>>,
    system: Mutex<SystemCache>,
}

impl Control {
    pub(crate) fn fixture() -> Self {
        Self {
            fixture_startup: Mutex::new(Some(false)),
            ..Self::default()
        }
    }

    pub(crate) fn system(&self) -> Response {
        Response::json(
            200,
            self.system.lock().unwrap_or_else(|e| e.into_inner()).get(
                crate::journal::now_ms,
                telemetry::sample,
                disks::sample,
            ),
        )
    }

    pub(crate) fn handle(
        &self,
        security: &Security,
        req: &Request,
        store: &Store,
        displays: &[Display],
    ) -> Option<Response> {
        let limit = match req.target.as_str() {
            "/api/host/startup" | "/api/host/audio/test" => 1024,
            "/api/host/layout" => 4096,
            "/api/host/displays"
            | "/api/host/audio"
            | "/api/host/audio/routing"
            | "/api/host/performance"
            | "/api/host/library" => 2048,
            _ => return None,
        };
        let reading = req.method == "GET" && req.target != "/api/host/audio/test";
        let policy = if reading {
            ApiPolicy {
                manager_only: true,
                ..ApiPolicy::read()
            }
        } else {
            ApiPolicy::manager_json(limit)
        };
        if let Err(e) = security.api(req, policy) {
            return Some(Response::error(e));
        }
        let body = if reading {
            None
        } else {
            match serde_json::from_slice::<Value>(&req.body) {
                Ok(mut v) => {
                    normalize_integers(&mut v);
                    Some(v)
                }
                Err(_) => return Some(error(400, "INVALID_JSON")),
            }
        };
        let route = req.target.as_str();
        let response = match self.dispatch(route, body, store, displays) {
            Ok(v) => Response::json(
                if route == "/api/host/library" && !reading {
                    202
                } else {
                    200
                },
                v,
            ),
            Err(code) => control_error(route, reading, &code),
        };
        Some(response)
    }

    fn dispatch(
        &self,
        route: &str,
        body: Option<Value>,
        store: &Store,
        displays: &[Display],
    ) -> Result<Value, String> {
        let root = &store.root;
        match route {
            "/api/host/displays" => {
                store.path("host-settings.json")?;
                let settings = match body {
                    Some(v) => host_settings::update_with_displays(
                        root,
                        input(v, "HOST_SETTINGS_INVALID")?,
                        || Ok(displays.to_vec()),
                    )?,
                    None => host_settings::read(root)?,
                };
                let runtime = policy_store::read_optional(&store.path("host-status.json")?, 131072)
                    .ok()
                    .flatten()
                    .filter(|v| {
                        recent(crate::journal::now_ms(), &v["capturedAtMs"], 5000).is_some()
                    });
                Ok(
                    json!({"schemaVersion":1,"available":true,"settings":settings,"displays":displays,"runtimeFresh":runtime.is_some(),"runtime":runtime}),
                )
            }
            "/api/host/layout" => {
                let result = body
                    .map(|v| {
                        layout::update_with_displays(root, input(v, "LAYOUT_INVALID")?, || {
                            Ok(displays.to_vec())
                        })
                    })
                    .transpose()?;
                Ok(policy(layout::read(root)?, result))
            }
            "/api/host/startup" => {
                let requested = body
                    .map(|v| input::<StartupInput>(v, "STARTUP_INVALID").map(|v| v.enabled))
                    .transpose()?;
                let mut fixture = self
                    .fixture_startup
                    .lock()
                    .map_err(|_| "STARTUP_OPERATION_FAILED")?;
                let (available, enabled) = if let Some(enabled) = fixture.as_mut() {
                    if let Some(v) = requested {
                        *enabled = v;
                    }
                    (true, *enabled)
                } else {
                    let status = requested.map_or_else(startup::status, startup::set)?;
                    (status.available, status.enabled)
                };
                Ok(
                    json!({"available":available,"enabled":enabled,"scope":"current-user","trigger":"logon"}),
                )
            }
            "/api/host/audio" => {
                let result = body
                    .map(|v| audio::update(root, input(v, "AUDIO_SETTINGS_INVALID")?))
                    .transpose()?;
                audio::status(root, result)
            }
            "/api/host/audio/routing" => {
                let result = body
                    .map(|v| {
                        if !v["routing"]
                            .as_object()
                            .is_some_and(|o| o.contains_key("target"))
                        {
                            return Err("AUDIO_ROUTING_INVALID".into());
                        }
                        audio_routing::update(root, input(v, "AUDIO_ROUTING_INVALID")?)
                    })
                    .transpose()?;
                let mut value = policy(audio_routing::read(root)?, result);
                value["displays"] = json!(displays);
                Ok(value)
            }
            "/api/host/audio/test" => {
                let v: Revision = input(body.unwrap_or_default(), "AUDIO_SETTINGS_INVALID")?;
                v.validate("AUDIO_SETTINGS_INVALID")?;
                let result = audio::request_test(root, v.schema_version, v.expected_revision)?;
                audio::status(root, Some(result))
            }
            "/api/host/performance" => {
                let result = body.map(|v| performance::update(root, v)).transpose()?;
                Ok(policy(performance::read(root)?, result))
            }
            "/api/host/library" => match body {
                None => library_status(store),
                Some(v) => self.library_action(store, input(v, "LIBRARY_COMMAND_INVALID")?),
            },
            _ => unreachable!(),
        }
    }

    fn library_action(&self, store: &Store, action: LibraryAction) -> Result<Value, String> {
        let audit_hash = match &action {
            LibraryAction::Archive(v) => v.audit_hash.clone(),
            _ => String::new(),
        };
        let (revision, request_id, operation) = match action {
            LibraryAction::Open(v) => (
                Revision {
                    schema_version: v.schema_version,
                    expected_revision: v.expected_revision,
                },
                v.request_id,
                "open",
            ),
            LibraryAction::Audit(v) => (
                Revision {
                    schema_version: v.schema_version,
                    expected_revision: v.expected_revision,
                },
                v.request_id,
                "audit",
            ),
            LibraryAction::Choose(v) => (
                Revision {
                    schema_version: v.schema_version,
                    expected_revision: v.expected_revision,
                },
                v.request_id,
                "choose",
            ),
            LibraryAction::Reset(v) => (
                Revision {
                    schema_version: v.schema_version,
                    expected_revision: v.expected_revision,
                },
                v.request_id,
                "reset",
            ),
            LibraryAction::Archive(v) => {
                if !hex(&v.audit_hash) {
                    return Err("LIBRARY_COMMAND_INVALID".into());
                }
                (
                    Revision {
                        schema_version: v.schema_version,
                        expected_revision: v.expected_revision,
                    },
                    v.request_id,
                    "archive",
                )
            }
        };
        revision.validate("LIBRARY_COMMAND_INVALID")?;
        if !policy_store::valid_id(&request_id) {
            return Err("LIBRARY_COMMAND_INVALID".into());
        }
        let guard = windows::named_singleton(&format!(
            "Local\\AtmosWallpaper.LibraryOpen.{}",
            asset_store::hash_text(&store.external_root().to_lowercase())
        ))
        .map_err(|_| "LIBRARY_BUSY")?;
        let job_path = store.path("library-service-job.json")?;
        let active = job_record(store, "library-service-job.json")?;
        if matches!(
            active["state"].as_str(),
            Some("queued" | "copying" | "choosing")
        ) {
            return Err("LIBRARY_BUSY".into());
        }
        let (root, _, current, _) = asset_store::library_config(store)?;
        if revision.expected_revision != current {
            return Err("LIBRARY_REVISION_CONFLICT".into());
        }
        let fixture = self
            .fixture_startup
            .lock()
            .map_err(|_| "LIBRARY_OPERATION_FAILED")?
            .is_some();
        let store = Store {
            root: store.root.clone(),
        };
        let id = crate::native_http::random_hex(16).map_err(|e| e.code)?;
        let job = json!({"schemaVersion":1,"id":id,"requestId":request_id,"ownerPid":std::process::id(),"state":"queued","completed":0,"total":0});
        write_job(&job_path, &job)?;
        let mut done = job.clone();
        let path = job_path.clone();
        if std::thread::Builder::new()
            .spawn(move || {
                let _guard = guard;
                let result = if matches!(operation, "choose" | "reset") {
                    (|| {
                        let destination = if operation == "choose" {
                            done["state"] = "choosing".into();
                            write_job(&path, &done)?;
                            let selected = if fixture {
                                None
                            } else {
                                library::choose_folder()?
                            };
                            let Some(selected) = selected else {
                                return Ok(None);
                            };
                            selected.join("Atmos Wallpaper Library")
                        } else {
                            crate::library_move::default_path(&store)
                        };
                        crate::library_move::relocate(
                            &store,
                            &destination,
                            current,
                            &request_id,
                            |completed, total| {
                                done["state"] = "copying".into();
                                done["completed"] = completed.into();
                                done["total"] = total.into();
                                let _ = write_job(&path, &done);
                            },
                        )
                        .map(Some)
                    })()
                } else if operation == "archive" {
                    crate::library_archive::archive(&store, current, &request_id, &audit_hash)
                        .map(Some)
                } else if operation == "audit" {
                    crate::library_audit::audit(&store, current)
                        .map(|audit| Some(json!({"committed":false,"audit":audit})))
                } else {
                    let path = crate::portable_store::external_path(&root);
                    library::local_path(&path)
                        .and_then(|_| asset_store::path(&store, "assets"))
                        .and_then(|_| {
                            if fixture {
                                Ok(())
                            } else {
                                library::open_folder(&path)
                            }
                        })
                        .map(|_| Some(json!({"opened":true,"committed":false})))
                };
                match result {
                    Ok(Some(result)) => {
                        done["state"] = "completed".into();
                        done["result"] = result;
                    }
                    Ok(None) => {
                        done["state"] = "cancelled".into();
                        done["result"] = json!({"committed":false});
                    }
                    Err(code) => {
                        done["state"] = "failed".into();
                        done["error"] = json!({"code":code});
                    }
                }
                let _ = write_job(&path, &done);
            })
            .is_err()
        {
            let mut failed = job;
            failed["state"] = "failed".into();
            failed["error"] = json!({"code":"LIBRARY_OPERATION_FAILED"});
            write_job(&job_path, &failed)?;
            return Err("LIBRARY_OPERATION_FAILED".into());
        }
        Ok(job)
    }
}

fn write_job(path: &std::path::Path, job: &Value) -> Result<(), String> {
    crate::portable_store::atomic_file(path, &serde_json::to_vec(job).unwrap())
}

fn policy(settings: impl serde::Serialize, result: Option<Value>) -> Value {
    json!({"schemaVersion":1,"available":true,"settings":settings,"result":result})
}

fn control_error(route: &str, read: bool, code: &str) -> Response {
    let (invalid, failed, conflicts): (&str, &str, &[&str]) = match route {
        "/api/host/displays" => (
            "HOST_SETTINGS_INVALID",
            "HOST_OPERATION_FAILED",
            &[
                "HOST_LAYOUT_ACTIVE",
                "HOST_REVISION_CONFLICT",
                "HOST_SETTINGS_BUSY",
            ],
        ),
        "/api/host/layout" => (
            "LAYOUT_INVALID",
            "LAYOUT_OPERATION_FAILED",
            &[
                "LAYOUT_REVISION_CONFLICT",
                "LAYOUT_REQUEST_CONFLICT",
                "LAYOUT_BUSY",
                "LAYOUT_STORE_BUSY",
            ],
        ),
        "/api/host/startup" => ("STARTUP_INVALID", "STARTUP_OPERATION_FAILED", &[]),
        "/api/host/audio" => (
            "AUDIO_SETTINGS_INVALID",
            "AUDIO_OPERATION_FAILED",
            &[
                "AUDIO_REVISION_CONFLICT",
                "AUDIO_REQUEST_CONFLICT",
                "AUDIO_BUSY",
            ],
        ),
        "/api/host/audio/routing" => (
            "AUDIO_ROUTING_INVALID",
            "AUDIO_ROUTING_OPERATION_FAILED",
            &[
                "AUDIO_ROUTING_REVISION_CONFLICT",
                "AUDIO_ROUTING_REQUEST_CONFLICT",
                "AUDIO_ROUTING_BUSY",
            ],
        ),
        "/api/host/audio/test" => (
            "AUDIO_SETTINGS_INVALID",
            "AUDIO_TEST_BLOCKED",
            &["AUDIO_REVISION_CONFLICT"],
        ),
        "/api/host/performance" => (
            "PERFORMANCE_INVALID",
            "PERFORMANCE_FAILED",
            &[
                "PERFORMANCE_REVISION_CONFLICT",
                "PERFORMANCE_REQUEST_CONFLICT",
                "PERFORMANCE_BUSY",
            ],
        ),
        "/api/host/library" => (
            "LIBRARY_COMMAND_INVALID",
            "LIBRARY_OPERATION_FAILED",
            &["LIBRARY_REVISION_CONFLICT", "LIBRARY_BUSY"],
        ),
        _ => unreachable!(),
    };
    if read {
        let code = match route {
            "/api/host/layout" | "/api/host/performance" if code == invalid => invalid,
            _ => failed,
        };
        return error(503, code);
    }
    if conflicts.contains(&code) {
        return error(409, code);
    }
    if code == invalid
        || (matches!(route, "/api/host/displays" | "/api/host/layout")
            && matches!(
                code,
                "TARGET_OFFLINE_OR_AMBIGUOUS" | "TARGET_IDENTITY_UNRESOLVED"
            ))
    {
        return error(400, code);
    }
    if code == "LIBRARY_UNAVAILABLE" {
        return error(503, code);
    }
    error(
        if matches!(
            route,
            "/api/host/audio/test" | "/api/host/layout" | "/api/host/performance"
        ) {
            400
        } else {
            503
        },
        failed,
    )
}

fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn job_record(store: &Store, name: &str) -> Result<Value, String> {
    let mut v = policy_store::read_optional(&store.path(name)?, 65536)
        .map_err(|_| "LIBRARY_JOB_INVALID")?
        .unwrap_or_default();
    if matches!(v["state"].as_str(), Some("queued" | "copying" | "choosing"))
        && !v["ownerPid"]
            .as_u64()
            .and_then(|p| u32::try_from(p).ok())
            .is_some_and(windows::process_alive)
    {
        v["state"] = "interrupted".into();
        v["error"] = json!({"code":"LIBRARY_JOB_INTERRUPTED"});
    }
    Ok(v)
}

pub(crate) fn library_status(store: &Store) -> Result<Value, String> {
    let (root, _, revision, legacy) = asset_store::library_config(store)?;
    let job = job_record(store, "library-service-job.json").unwrap_or_default();
    let last = job_record(store, "library-job.json")
        .unwrap_or_else(|_| json!({"state":"unknown","error":{"code":"LIBRARY_JOB_INVALID"}}));
    let display_path = crate::portable_store::external_path(&root);
    let mut v = json!({"schemaVersion":1,"available":true,"revision":revision,"path":display_path,"isDefault":false,"legacy":legacy,"online":false,"job":job,"lastJob":last});
    let inventory = library::local_path(&display_path)
        .and_then(|_| asset_store::path(store, "assets"))
        .and_then(|dir| library_inventory(&dir, &root));
    match inventory {
        Ok((count, bytes, free)) => {
            let default = store.root.with_file_name(format!(
                "{}-library",
                store.root.file_name().unwrap_or_default().to_string_lossy()
            ));
            v["isDefault"] = (!legacy
                && display_path
                    .eq_ignore_ascii_case(&crate::portable_store::external_path(&default)))
            .into();
            v["online"] = true.into();
            v["assetCount"] = count.into();
            v["bytes"] = bytes.into();
            v["freeBytes"] = free.into();
            v["retention"] =
                "retain-all-including-history-previews-and-backups; no automatic deletion".into();
            v["cleanup"] = json!({"policy":"explicit verified archive; reference audit required; no permanent deletion","auditRequired":true});
        }
        Err(code) => {
            v["error"] = json!({"code": if code.starts_with("LIBRARY_") { &code } else { "LIBRARY_UNAVAILABLE" }});
            v["retention"] = "retain-all; no fallback or recreation".into();
        }
    }
    Ok(v)
}

fn library_inventory(
    dir: &std::path::Path,
    root: &std::path::Path,
) -> Result<(usize, u64, u64), String> {
    use std::{collections::BTreeMap, os::windows::ffi::OsStrExt};
    let mut files = BTreeMap::new();
    let mut bytes = 0;
    for entry in fs::read_dir(dir).map_err(|_| "LIBRARY_UNAVAILABLE")? {
        if files.len() >= 20000 {
            return Err("LIBRARY_LIMIT".into());
        }
        let entry = entry.map_err(|_| "LIBRARY_CONTENT_INVALID")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "LIBRARY_CONTENT_INVALID")?;
        let (id, ext) = name.split_once('.').ok_or("LIBRARY_CONTENT_INVALID")?;
        let limit = match ext {
            "png" => 16 * 1048576,
            "json" => 16384,
            _ => 0,
        };
        let meta = fs::symlink_metadata(entry.path()).map_err(|_| "LIBRARY_CONTENT_INVALID")?;
        if limit == 0
            || !hex(id)
            || !meta.is_file()
            || meta.file_attributes() & 0x400 != 0
            || meta.len() > limit
        {
            return Err("LIBRARY_CONTENT_INVALID".into());
        }
        bytes += meta.len();
        files.insert(name, meta.len());
    }
    for (name, size) in &files {
        let id = &name[..64];
        if name.ends_with(".png") {
            let metadata = format!("{id}.json");
            if !files.contains_key(&metadata) {
                return Err("LIBRARY_ASSET_CORRUPT".into());
            }
            let v = asset_store::read_json(&dir.join(metadata), 16384)?;
            if v["assetId"].as_str() != Some(id)
                || v["format"].as_str() != Some("PNG")
                || v["bytes"].safe_uint() != Some(*size)
                || !["width", "height"]
                    .iter()
                    .all(|k| v[k].safe_uint().is_some_and(|n| n > 0))
            {
                return Err("LIBRARY_ASSET_CORRUPT".into());
            }
        } else if !files.contains_key(&format!("{id}.png")) {
            return Err("LIBRARY_ASSET_CORRUPT".into());
        }
    }
    let path: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
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
    Ok((files.len() / 2, bytes, free))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(at: u64) -> Value {
        json!({"sampledAtMs":at,"available":true,"cpuPercent":5,"memory":{"usedBytes":5,"totalBytes":10}})
    }

    #[test]
    fn disk_cache_is_independent_of_system_polling_and_expires_on_failure() {
        let mut cache = SystemCache::default();
        let disk = |at| json!({"sampledAtMs":at,"disks":[{"drive":"C:","available":false}]});
        let first = cache.get(|| 1000, || Ok(sample(1000)), || disk(1000));
        assert_eq!(first["disksSampledAtMs"], 1000);
        let soon = cache.get(
            || 1500,
            || panic!("system sampled too soon"),
            || panic!("disk sampled too soon"),
        );
        assert_eq!(soon["ageMs"], 500);
        let later = cache.get(
            || 30999,
            || Ok(sample(30999)),
            || panic!("disk sampled too soon"),
        );
        assert_eq!(later["disksSampledAtMs"], 1000);
        let refresh = cache.get(
            || 31000,
            || panic!("system sampled too soon"),
            || disk(31000),
        );
        assert_eq!(refresh["disksSampledAtMs"], 31000);
        let future = cache.get(|| 61000, || Ok(sample(61000)), || disk(61001));
        assert!(future["disks"].is_null());
        let rollback = cache.get(|| 60000, || Ok(sample(60000)), || disk(60000));
        assert_eq!(rollback["disksSampledAtMs"], 60000);
    }

    #[test]
    fn stale_and_failed_telemetry_never_reports_available() {
        let mut cache = SystemCache::default();
        let missing = || Value::Null;
        for (now, sampled) in [(15000, 0), (17000, 17001)] {
            assert_eq!(
                cache.get(|| now, || Ok(sample(sampled)), missing)["error"]["code"],
                "SYSTEM_UNAVAILABLE"
            );
        }
        assert_eq!(
            cache.get(|| 19000, || Err("system".into()), missing)["available"],
            false
        );
    }
}
