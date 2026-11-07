//! Appearance and display routing for the native HTTP candidate.
use crate::{
    appearance_core::{self, Core},
    asset_store::{hash_bytes, read_bytes},
    displays::Display,
    host_settings, js_json, layout,
    native_control_http::Control,
    native_http::{ApiPolicy, Assets, Failure, Request, Response, Role, Security, BODY_LIMIT},
    portable_store::Store,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::Mutex,
};

const ARTWORK: [&str; 7] = [
    "nature-twilight-clean.png",
    "warm-illustration-clean.png",
    "modern-city.png",
    "future-neon-clean.png",
    "coastal-bay.png",
    "alpine-dawn.png",
    "desert-milky-way.png",
];

struct View<'a> {
    target: Option<&'a str>,
    scope: &'a str,
}

impl View<'_> {
    fn routing(&self) -> Value {
        self.target.map_or(
            Value::Null,
            |id| json!({"targetId":id,"scopeId":self.scope}),
        )
    }

    fn wallpaper_record(&self) -> String {
        match self.target {
            Some(id) if self.scope == "shared" => format!("observations/wallpaper-{id}.json"),
            _ => "observations/wallpaper.json".to_owned(),
        }
    }
}

type Reports = HashMap<String, HashMap<String, Value>>;

pub(crate) struct Bridge {
    owner: Core,
    artwork: BTreeMap<&'static str, (String, Vec<u8>)>,
    session: String,
    reports: Mutex<Reports>,
    decoder: crate::backup_image::Decoder,
    prototype: PathBuf,
    control: Control,
    weather: crate::native_weather::Weather,
    feeds: crate::native_feeds::Feeds,
    calendar: crate::native_calendar::Calendar,
    external_sources: bool,
}

struct Context {
    layout: layout::Settings,
    host: host_settings::Settings,
    native: Option<Value>,
}

fn reply_error(status: u16, code: &str) -> Response {
    Response::json(status, json!({"ok":false,"error":{"code":code}}))
}

fn core_error(code: String) -> Response {
    let status = match code.as_str() {
        "STORE_LOCKED"
        | "REVISION_CONFLICT"
        | "REQUEST_CONFLICT"
        | "PREVIEW_STALE"
        | "PREVIEW_CONFLICT"
        | "RECOVERED_OUTCOME_UNKNOWN" => 409,
        "INVALID_JSON"
        | "PATH_REJECTED"
        | "COMMAND_INVALID"
        | "INVALID_REQUEST_ID"
        | "INVALID_REVISION"
        | "INVALID_STATE"
        | "UNKNOWN_FIELD"
        | "EFFECT_REMOVED"
        | "FIXED_SETTING"
        | "INVALID_PREVIEW"
        | "PREVIEW_EXPIRED"
        | "PREVIEW_CANCELLED"
        | "PREVIEW_MISSING"
        | "HISTORY_MISSING"
        | "INVALID_BACKUP"
        | "BACKUP_CORRUPT"
        | "BACKUP_VERSION_UNSUPPORTED"
        | "ASSET_ID_DENIED"
        | "IMAGE_REJECTED"
        | "INVALID_OBSERVATION" => 400,
        "OBSERVATION_STALE" => 409,
        "ASSET_CORRUPT" => 422,
        _ => 503,
    };
    // Only protocol identifiers may leave the boundary.
    let public = if code.len() <= 64
        && !code.is_empty()
        && code.bytes().all(|c| c.is_ascii_uppercase() || c == b'_')
    {
        &code
    } else {
        "LOCAL_OPERATION_FAILED"
    };
    reply_error(status, public)
}

fn wire(value: js_json::Value) -> Response {
    Response::bytes(
        200,
        "application/json; charset=utf-8",
        value.canonical().into_bytes(),
    )
}

fn bounded_record(store: &Store, name: &str, limit: usize) -> Option<Value> {
    serde_json::from_slice(&read_bytes(&store.path(name).ok()?, limit, "INVALID_RECORD").ok()?).ok()
}

fn fresh(now: u64, at: Option<u64>, window: u64) -> bool {
    at.and_then(|at| now.checked_sub(at))
        .is_some_and(|age| age < window)
}

fn select_display<'a>(displays: &'a [Display], id: &str) -> Option<&'a Display> {
    displays.iter().find(|d| {
        d.selectable
            && if id == "primary" {
                d.primary
            } else {
                d.id == id
            }
    })
}

fn span(displays: &[Display], targets: &[String], target: &str) -> Value {
    let selected: Vec<_> = targets
        .iter()
        .filter_map(|id| select_display(displays, id))
        .collect();
    let Some(display) = selected.iter().find(|d| d.id == target) else {
        return Value::Null;
    };
    let x = selected
        .iter()
        .map(|d| i64::from(d.bounds.x))
        .min()
        .unwrap();
    let y = selected
        .iter()
        .map(|d| i64::from(d.bounds.y))
        .min()
        .unwrap();
    let right = selected
        .iter()
        .map(|d| i64::from(d.bounds.x) + i64::from(d.bounds.width))
        .max()
        .unwrap();
    let bottom = selected
        .iter()
        .map(|d| i64::from(d.bounds.y) + i64::from(d.bounds.height))
        .max()
        .unwrap();
    let bounds = display.bounds;
    json!({
        "schemaVersion":1,"world":{"x":x,"y":y,"width":right-x,"height":bottom-y},
        "view":{"x":i64::from(bounds.x)-x,"y":i64::from(bounds.y)-y,"width":bounds.width,"height":bounds.height},
        "anchorId":targets[0],"anchor":target==targets[0],
        "offlineIds":targets.iter().filter(|id| !selected.iter().any(|d| &d.id==*id)).collect::<Vec<_>>(),
        "units":"physical-pixels","fit":"contain-world"
    })
}

impl Bridge {
    pub(crate) fn open(
        root: &Path,
        prototype: &Path,
        session: String,
        decoder: crate::backup_image::Decoder,
        fixture: bool,
    ) -> Result<Self, String> {
        if session.is_empty() {
            return Err("HOST_SESSION_REQUIRED".into());
        }
        let manifest: Value = serde_json::from_slice(&read_bytes(
            &prototype.join("manifest.json"),
            262144,
            "BASELINE_CHANGED",
        )?)
        .map_err(|_| "BASELINE_CHANGED")?;
        let entries = manifest["files"].as_array().ok_or("BASELINE_CHANGED")?;
        let mut artwork = BTreeMap::new();
        for name in ARTWORK {
            let relative = format!("assets/{name}");
            let entry = entries
                .iter()
                .find(|entry| entry["path"] == relative)
                .ok_or("BASELINE_CHANGED")?;
            let bytes = read_bytes(&prototype.join(relative), 16 * 1048576, "BASELINE_CHANGED")?;
            let hash = hash_bytes(&bytes);
            if entry["sha256"] != hash {
                return Err("BASELINE_CHANGED".into());
            }
            artwork.insert(name, (hash, bytes));
        }
        let owner = Core::open(root)?;
        if let Err(error) = owner.init() {
            let recoverable = matches!(
                error.as_str(),
                "CORRUPT_STORE"
                    | "RECOVERY_REQUIRED"
                    | "ENOENT"
                    | "INVALID_STATE"
                    | "UNKNOWN_FIELD"
            );
            if !recoverable || !owner.path("live.backup.json")?.is_file() {
                return Err(error);
            }
            // Leave the damaged snapshot intact for an explicit manager recovery.
        }
        Ok(Self {
            owner,
            artwork,
            session,
            reports: Mutex::new(HashMap::new()),
            decoder,
            prototype: prototype.into(),
            control: if fixture { Control::fixture() } else { Control::default() },
            weather: Default::default(),
            feeds: Default::default(),
            calendar: Default::default(),
            external_sources: !fixture,
        })
    }

    fn context(&self) -> Result<Context, String> {
        let root = &self.owner.store.root;
        // The host settings reader is shared with desktop placement.
        self.owner.store.path("host-settings.json")?;
        let host = host_settings::read(root)?;
        let layout = layout::read(root)?;
        let native = bounded_record(&self.owner.store, "host-status.json", 131072).filter(|v| {
            v["schemaVersion"] == 1
                && v["hostSessionId"] == self.session
                && fresh(crate::journal::now_ms(), v["capturedAtMs"].as_u64(), 5000)
                && v["targets"]
                    .as_array()
                    .is_some_and(|targets| targets.len() <= 16)
        });
        Ok(Context {
            layout,
            host,
            native,
        })
    }

    fn view<'a>(&self, context: &'a Context, target: Option<&'a str>) -> Result<View<'a>, Failure> {
        let policy = &context.layout.layout;
        if policy.mode == "single" {
            return if target.is_none() {
                Ok(View {
                    target: None,
                    scope: "shared",
                })
            } else {
                Err(Failure {
                    status: 401,
                    code: "SCOPE_CHANGED",
                })
            };
        }
        let id = target.ok_or(Failure {
            status: 409,
            code: "TARGET_REQUIRED",
        })?;
        if !policy.targets.iter().any(|entry| entry == id) {
            return Err(Failure {
                status: 409,
                code: "TARGET_INACTIVE",
            });
        }
        Ok(View {
            target: Some(id),
            scope: if policy.mode == "independent" {
                id
            } else {
                "shared"
            },
        })
    }

    fn scoped_core(&self, view: &View) -> Result<Option<Core>, String> {
        if view.scope == "shared" {
            Ok(None)
        } else {
            self.owner.ensure_scope(view.scope).map(Some)
        }
    }

    fn page(
        &self,
        security: &Security,
        mut req: Request,
        assets: &Assets,
        displays: &[Display],
    ) -> Response {
        let (page, id) = if let Some((page, id)) = req.target[1..].split_once('/') {
            let page = match page {
                "manager" => "/manager",
                "wallpaper" => "/wallpaper",
                "screensaver" if appearance_core::display_id(id) => {
                    return reply_error(403, "ROLE_DENIED")
                }
                _ => {
                    return security
                        .static_response(&req, assets)
                        .unwrap_or_else(Response::error)
                }
            };
            if !appearance_core::display_id(id) {
                return security
                    .static_response(&req, assets)
                    .unwrap_or_else(Response::error);
            }
            (page, Some(id))
        } else if ["/", "/manager", "/wallpaper", "/index.html"].contains(&req.target.as_str()) {
            (
                if req.target == "/wallpaper" {
                    "/wallpaper"
                } else {
                    "/manager"
                },
                None,
            )
        } else {
            return security
                .static_response(&req, assets)
                .unwrap_or_else(Response::error);
        };
        let context = match self.context() {
            Ok(c) => c,
            Err(e) => return core_error(e),
        };
        if context.layout.layout.mode == "single" && id.is_some() {
            return reply_error(409, "TARGET_INACTIVE");
        }
        let id = id.or_else(|| {
            let targets = &context.layout.layout.targets;
            targets
                .iter()
                .find(|id| displays.iter().any(|d| d.primary && &d.id == *id))
                .or_else(|| targets.first())
                .map(String::as_str)
        });
        let view = match self.view(&context, id) {
            Ok(v) => v,
            Err(e) => return Response::error(e),
        };
        let routing = view.routing();
        req.target = page.into();
        security
            .scoped_static_response(&req, assets, &routing)
            .unwrap_or_else(Response::error)
    }

    fn builtin(&self, desired: &js_json::Value) -> &(String, Vec<u8>) {
        &self.artwork[artwork_name(desired)]
    }

    fn snapshot(
        &self,
        context: &Context,
        view: &View,
        core: &Core,
        displays: &[Display],
        role: Role,
    ) -> Result<js_json::Value, String> {
        let live = core.live()?;
        let desired = &live["desired"];
        let (asset, fallback) = if desired["assetId"].is_null() {
            (self.builtin(desired).0.as_str(), false)
        } else {
            match core.asset(&desired["assetId"]) {
                Ok(_) => (desired["assetId"].as_str().unwrap(), false),
                Err(e)
                    if role == Role::Manager
                        && matches!(
                            e.as_str(),
                            "LIBRARY_OFFLINE"
                                | "LIBRARY_OWNER_MISMATCH"
                                | "LIBRARY_CORRUPT"
                                | "ASSET_MISSING"
                                | "ASSET_CORRUPT"
                        ) =>
                {
                    (self.builtin(desired).0.as_str(), true)
                }
                Err(e) => return Err(e),
            }
        };
        let selected = view.target.or_else(|| {
            if context.host.placement.target == "primary" {
                select_display(displays, "primary").map(|d| d.id.as_str())
            } else {
                Some(context.host.placement.target.as_str())
            }
        });
        let display = selected.and_then(|id| select_display(displays, id));
        let span = if context.layout.layout.mode == "span" {
            span(
                displays,
                &context.layout.layout.targets,
                selected.unwrap_or(""),
            )
        } else {
            Value::Null
        };
        let projection = json!({
            "layoutRevision":context.layout.revision,"mode":context.layout.layout.mode,"targetId":selected,
            "targetBounds":display.map(|d| d.bounds),"scalePercent":display.and_then(|d| d.scale_percent),"span":span
        });
        let runtime_policy = context.native.as_ref().and_then(|native| {
            let targets = native["targets"].as_array()?;
            let target = if let Some(id) = view.target {
                targets.iter().find(|t| t["targetId"] == id)
            } else {
                targets.first()
            }?;
            target.get("runtimePolicy")
        });
        let projection_id = js_json::parse(projection.to_string().as_bytes())?.hash();
        let monitors = displays.iter().map(monitor_record).collect::<Vec<_>>();
        let monitor = json!({
            "schemaVersion":1,"status":match display { Some(d) if d.scale_percent.is_some()=>"detected",Some(_)=>"partial",None=>"unavailable" },
            "source":"windows-native-readonly","capturedAt":appearance_core::iso_now(),
            "reason":match display {Some(d) if d.scale_percent.is_some()=>Value::Null,Some(_)=>json!("FIELDS_UNAVAILABLE"),None=>json!("INVALID_TARGET_ID")},
            "target":display.map(monitor_record),"monitors":if display.is_some(){monitors}else{vec![]},
            "selection":if display.is_some(){"explicit"}else{"unresolved"},
            "previewFallback":if display.is_none(){json!({"width":1920,"height":1080,"source":"default-aspect","detected":false})}else{Value::Null},
            "span":span
        });
        let mut output = js_json::parse(json!({
            "schemaVersion":1,"projection":projection,"projectionId":projection_id,"runtimePolicy":runtime_policy,
            "artworkIsFallback":fallback,"artwork":{"sha256":asset,"url":format!("/api/artwork/{asset}"),"mimeType":"image/png"},
            "source":"local-bridge","hostVisibility":"unknown","controlMode":if role==Role::Manager{"local-controls"}else{"wallpaper-readonly"},
            "monitor":monitor
        }).to_string().as_bytes())?;
        for field in ["revision", "desiredHash", "desired"] {
            output.set(field, live[field].clone());
        }
        output.set("migrationNotice", core.migration_notice()?);
        Ok(output)
    }

    pub(crate) fn handle(
        &self,
        security: &Security,
        req: Request,
        assets: &Assets,
        displays: &[Display],
    ) -> Response {
        if let Err(e) = security.common(&req) {
            return Response::error(e);
        }
        if !req.target.starts_with("/api/") {
            return self.page(security, req, assets, displays);
        }
        let role = match security.authenticate_api(&req) {
            Ok(r) => r,
            Err(e) => return Response::error(e),
        };
        if matches!(req.target.as_str(), "/api/host/screensaver" | "/api/host/screensaver/settings") {
            return if role == Role::Manager { reply_error(410, "SCREENSAVER_REMOVED") }
                else { reply_error(403, "ROLE_DENIED") };
        }
        if let Some(response) = self
            .control
            .handle(security, &req, &self.owner.store, displays)
        {
            return response;
        }
        if req.target == "/api/host/focus" {
            let reading = req.method == "GET";
            let policy = if reading {
                ApiPolicy::read()
            } else {
                ApiPolicy::manager_json(1024)
            };
            if let Err(e) = security.api(&req, policy) {
                return Response::error(e);
            }
            let result = if reading {
                crate::focus::status(&self.owner.store, &self.session)
            } else {
                let Ok(body) = serde_json::from_slice(&req.body) else {
                    return reply_error(400, "INVALID_JSON");
                };
                crate::focus::set(&self.owner.store, &self.session, &body)
            };
            return match result {
                Ok(v) => Response::json(200, v),
                Err(code) if reading => core_error(code),
                Err(code) if code.contains("CONFLICT") || code == "STORE_LOCKED" => {
                    reply_error(409, &code)
                }
                Err(code) => reply_error(400, &code),
            };
        }
        if let Some(action) = req.target.strip_prefix("/api/calendar/") {
            return self.calendar_request(security, &req, action);
        }
        let policy = match req.target.as_str() {
            "/api/presets" | "/api/schedule" if req.method == "GET" => ApiPolicy {
                manager_only: true,
                ..ApiPolicy::read()
            },
            "/api/presets" => ApiPolicy::manager_json(4096),
            "/api/schedule" => ApiPolicy::manager_json(8192),
            "/api/health"
            | "/api/snapshot"
            | "/api/providers/system"
            | "/api/providers/weather" => ApiPolicy::read(),
            "/api/providers/settings" | "/api/daily-feeds" if req.method == "GET" => {
                ApiPolicy::read()
            }
            "/api/providers/settings" => ApiPolicy::manager_json(2048),
            "/api/daily-feeds" => ApiPolicy::manager_json(8192),
            route if route.starts_with("/api/artwork/") => ApiPolicy::read(),
            "/api/ack" => ApiPolicy {
                manager_only: false,
                ..ApiPolicy::manager_json(4096)
            },
            "/api/commands" | "/api/core/preview" | "/api/core/apply" | "/api/core/cancel"
            | "/api/core/rollback" | "/api/core/recover" | "/api/core/backup"
            | "/api/core/restore" | "/api/core/import" | "/api/core/export"
            | "/api/core/observe" | "/api/core/stage" => ApiPolicy::manager_json(BODY_LIMIT),
            _ => return reply_error(404, "API_NOT_FOUND"),
        };
        if let Err(e) = security.api(&req, policy) {
            return Response::error(e);
        }
        let context = match self.context() {
            Ok(c) => c,
            Err(e) => return core_error(e),
        };
        if req.target == "/api/health" {
            return self.health(&context, displays, role);
        }
        let view = match self.view(&context, req.header("x-atmos-target")) {
            Ok(v) => v,
            Err(e) => return Response::error(e),
        };
        if view.target.is_some() && req.header("x-atmos-scope") != Some(view.scope) {
            return reply_error(401, "SCOPE_CHANGED");
        }
        if req.target == "/api/providers/system" {
            return self.control.system();
        }
        if matches!(
            req.target.as_str(),
            "/api/providers/settings" | "/api/providers/weather" | "/api/daily-feeds"
        ) {
            return self.information_sources(&req);
        }
        let scoped = if req.target == "/api/core/recover" && view.scope != "shared" {
            self.owner.open_existing_scope(view.scope).map(Some)
        } else {
            self.scoped_core(&view)
        };
        let scoped = match scoped {
            Ok(c) => c,
            Err(e) => return core_error(e),
        };
        let core = scoped.as_ref().unwrap_or(&self.owner);
        if matches!(req.target.as_str(), "/api/presets" | "/api/schedule") {
            return self.appearance_automation(&req, core);
        }
        match self.dispatch(&req, &context, &view, core, displays, role, assets) {
            Ok(response) => response,
            Err(code) => core_error(code),
        }
    }

    pub(crate) fn tick_schedule(&self) {
        crate::schedule::tick_existing(&self.owner);
    }

    pub(crate) fn tick_feeds(&self) {
        if self.external_sources {
            let _ = self.feeds.status(&self.owner.store, true);
        }
    }

    fn information_sources(&self, req: &Request) -> Response {
        let store = &self.owner.store;
        let reading = req.method == "GET";
        let result = match (req.target.as_str(), reading) {
            ("/api/providers/weather", _) => self.weather.read(store, self.external_sources),
            ("/api/providers/settings", true) => crate::providers::settings(store).map(|settings|
                json!({"schemaVersion":1,"available":true,"settings":settings,"systemAvailable":true,
                       "weatherProvider":"Open-Meteo; free API for non-commercial use only"})),
            ("/api/daily-feeds", true) => self.feeds.status(store, self.external_sources),
            (route, false) => {
                let Ok(body) = serde_json::from_slice(&req.body) else {
                    return reply_error(400, if route == "/api/daily-feeds" { "FEED_SETTINGS_INVALID" } else { "INVALID_JSON" });
                };
                if route == "/api/daily-feeds" { crate::native_feeds::update(store, &body) }
                else { crate::providers::update(store, &body) }
            }
            _ => unreachable!(),
        };
        match result {
            Ok(v) => Response::json(200, v),
            Err(code) => match (req.target.as_str(), reading, code.as_str()) {
                ("/api/daily-feeds", false, "FEED_REVISION_CONFLICT" | "STORE_LOCKED") => {
                    reply_error(409, &code)
                }
                ("/api/daily-feeds", false, "FEED_SETTINGS_INVALID") => reply_error(400, &code),
                ("/api/daily-feeds", false, _) => reply_error(503, "FEED_OPERATION_FAILED"),
                (
                    _,
                    false,
                    "PROVIDERS_REVISION_CONFLICT" | "PROVIDERS_REQUEST_CONFLICT" | "STORE_LOCKED",
                ) => reply_error(409, &code),
                (_, false, "PROVIDERS_SETTINGS_INVALID") => reply_error(400, &code),
                (_, false, _) => reply_error(503, "PROVIDERS_OPERATION_FAILED"),
                _ => core_error(code),
            },
        }
    }

    fn calendar_request(&self, security: &Security, req: &Request, action: &str) -> Response {
        let reading = action == "events";
        let policy = match action {
            "events" => ApiPolicy::read(),
            "import" => ApiPolicy::manager_json(2 * 1048576),
            "subscribe" | "refresh" | "disconnect" => ApiPolicy::manager_json(8192),
            _ => return reply_error(404, "API_NOT_FOUND"),
        };
        if let Err(e) = security.api(req, policy) {
            return Response::error(e);
        }
        let result = if reading {
            self.calendar
                .events(&self.owner.store, self.external_sources)
        } else {
            let Ok(body) = serde_json::from_slice(&req.body) else {
                return reply_error(400, "INVALID_JSON");
            };
            self.calendar
                .change(&self.owner.store, action, &body, self.external_sources)
        };
        match result {
            Ok(value) => Response::json(200, value),
            Err(code) if reading => core_error(code),
            Err(code) => reply_error(if code == "ICS_BUSY" { 409 } else { 400 }, &code),
        }
    }

    fn appearance_automation(&self, req: &Request, core: &Core) -> Response {
        let preset = req.target == "/api/presets";
        let reading = req.method == "GET";
        let result = if reading {
            if preset {
                crate::presets::status(core)
            } else {
                crate::schedule::status(core)
            }
        } else {
            let Ok(body) = js_json::parse(&req.body) else {
                return reply_error(400, "INVALID_JSON");
            };
            if preset {
                crate::presets::execute(core, &body)
            } else {
                crate::schedule::set(core, &body)
            }
        };
        match result {
            Ok(value) => wire(value),
            Err(code) => {
                let status = if code == "STORE_LOCKED" || !reading && code.contains("CONFLICT") {
                    409
                } else if !reading
                    && ["INVALID", "NOT_FOUND", "LIMIT"]
                        .iter()
                        .any(|k| code.contains(k))
                    || !reading && !preset && code.contains("MISSING")
                {
                    400
                } else {
                    503
                };
                reply_error(status, &code)
            }
        }
    }

    fn dispatch(
        &self,
        req: &Request,
        context: &Context,
        view: &View,
        core: &Core,
        displays: &[Display],
        role: Role,
        assets: &Assets,
    ) -> Result<Response, String> {
        if req.target == "/api/snapshot" {
            let snapshot = self.snapshot(context, view, core, displays, role)?;
            // Captured-at changes do not invalidate an otherwise unchanged snapshot.
            let mut identity = snapshot.clone();
            let mut monitor = identity["monitor"].clone();
            monitor.remove("capturedAt");
            identity.set("monitor", monitor);
            let etag = format!("\"{}\"", identity.hash());
            let mut response = if req.header("if-none-match") == Some(&etag) {
                Response::bytes(304, "application/json; charset=utf-8", vec![])
            } else {
                wire(snapshot)
            };
            response.headers.insert("ETag".into(), etag);
            return Ok(response);
        }
        if let Some(id) = req.target.strip_prefix("/api/artwork/") {
            if !crate::desired_state::hex_id(&id.into()) {
                return Err("ASSET_ID_DENIED".into());
            }
            let bytes = match self.artwork.values().find(|(hash, _)| hash == id) {
                Some((_, bytes)) => bytes.clone(),
                None => core.asset(&id.into())?.1,
            };
            return Ok(Response::bytes(200, "image/png", bytes));
        }
        let args = js_json::parse(&req.body).map_err(|_| "INVALID_JSON")?;
        if req.target == "/api/ack" {
            return self.ack(context, view, core, displays, role, &args);
        }
        let result = match req.target.as_str() {
            "/api/commands" => {
                let mut result = core.execute_command(&args)?;
                result.set("committed", true.into());
                result.set("requestId", args["requestId"].clone());
                result.set("rendererAcknowledged", false.into());
                result.set("hostVisibility", "unknown".into());
                result
            }
            "/api/core/preview" => {
                fields(&args, &["requestId", "expectedRevision", "patch"])?;
                core.preview(&args)?
            }
            "/api/core/apply" => {
                fields(&args, &["requestId", "expectedRevision", "previewId"])?;
                core.apply(&args)?
            }
            "/api/core/cancel" => {
                fields(&args, &["requestId", "previewId"])?;
                core.cancel(&args)?
            }
            "/api/core/rollback" => {
                fields(&args, &["requestId", "expectedRevision", "targetRevision"])?;
                core.rollback(&args)?
            }
            "/api/core/recover" => {
                fields(&args, &["requestId"])?;
                core.recover(&args)?
            }
            "/api/core/backup" => {
                fields(&args, &[])?;
                core.backup()?
            }
            "/api/core/restore" | "/api/core/import" => {
                fields(&args, &["requestId", "expectedRevision", "backupHash"])?;
                core.restore_local_backup(&args, req.target == "/api/core/import", &self.decoder)?
            }
            "/api/core/export" => {
                if args.has("previewId") {
                    fields(&args, &["previewId"])?;
                } else {
                    fields(&args, &[])?;
                }
                crate::native_offline::export(core, &args["previewId"], assets, &self.prototype)?
            }
            "/api/core/observe" => crate::native_offline::observe(core, &args)?,
            "/api/core/stage" => {
                fields(&args, &["requestId", "file"])?;
                core.stage(&args, &self.decoder)?
            }
            _ => return Ok(reply_error(404, "API_NOT_FOUND")),
        };
        Ok(wire(js_json::object([
            ("ok", true.into()),
            ("result", result),
        ])))
    }

    fn ack(
        &self,
        context: &Context,
        view: &View,
        core: &Core,
        displays: &[Display],
        role: Role,
        report: &js_json::Value,
    ) -> Result<Response, String> {
        let fields = [
            "schemaVersion",
            "revision",
            "desiredHash",
            "assetSha256",
            "rendererId",
            "renderedAt",
            "stateApplied",
            "projectionId",
            "animation",
        ];
        let now = crate::journal::now_ms();
        let id = report["rendererId"].as_str().unwrap_or("");
        let animation = &report["animation"];
        let valid_animation = !report.has("animation")
            || (animation.as_object().is_some_and(|a| a.len() == 3)
                && animation["informationMode"].as_bool().is_some()
                && ["drawCount", "scheduleCount"]
                    .iter()
                    .all(|key| animation[key].safe_uint().is_some()));
        if !report.as_object().is_some_and(|o| {
            o.keys().all(|k| {
                fields
                    .iter()
                    .any(|f| f.encode_utf16().eq(k.iter().copied()))
            })
        }) || report["schemaVersion"].safe_uint() != Some(1)
            || report["stateApplied"].as_bool() != Some(true)
            || !(8..=64).contains(&id.len())
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || !report["renderedAt"]
                .as_str()
                .and_then(appearance_core::timestamp_ms)
                .is_some_and(|t| (t - i128::from(now)).abs() <= 30000)
            || !valid_animation
        {
            return Ok(reply_error(400, "ACK_INVALID"));
        }
        let _guard = core.lock()?;
        let current = self.context()?;
        if current.layout.revision != context.layout.revision
            || current.host.revision != context.host.revision
        {
            return Ok(reply_error(409, "ACK_STALE"));
        }
        let snapshot = self.snapshot(&current, view, core, displays, Role::Wallpaper)?;
        if ["revision", "desiredHash", "projectionId"]
            .iter()
            .any(|k| report[k] != snapshot[k])
            || report["assetSha256"] != snapshot["artwork"]["sha256"]
        {
            return Ok(reply_error(409, "ACK_STALE"));
        }
        let mut reports = self.reports.lock().map_err(|_| "LOCAL_OPERATION_FAILED")?;
        reports.retain(|_, entries| {
            entries.retain(|_, entry| fresh(now, entry["capturedAtMs"].as_u64(), 30000));
            !entries.is_empty()
        });
        let key = format!("{}:{}", view.target.unwrap_or("single"), view.scope);
        if !reports.contains_key(&key) && reports.len() >= 32 {
            return Ok(reply_error(429, "RENDERER_LIMIT"));
        }
        let entries = reports.entry(key).or_default();
        if !entries.contains_key(id) && entries.len() >= 16 {
            return Ok(reply_error(429, "RENDERER_LIMIT"));
        }
        let role_name = if role == Role::Manager {
            "manager"
        } else {
            "wallpaper"
        };
        let mut observation = js_json::object([
            ("revision", snapshot["revision"].clone()),
            ("desiredHash", snapshot["desiredHash"].clone()),
            ("assetSha256", report["assetSha256"].clone()),
            ("assetLoaded", true.into()),
            ("stateApplied", true.into()),
            ("rendererId", id.into()),
            ("rendererRole", role_name.into()),
            ("source", "local-bridge".into()),
            ("capturedAt", appearance_core::iso_now().into()),
            ("staleAfterMs", 4000.into()),
            ("trust", "renderer-reported, not OS verification".into()),
            ("hostVisibility", "unknown".into()),
            (
                "targetId",
                view.target.map_or(js_json::Value::Null, Into::into),
            ),
            ("hostSessionId", self.session.as_str().into()),
            ("projection", snapshot["projection"].clone()),
            ("projectionId", snapshot["projectionId"].clone()),
        ]);
        if report.has("animation") {
            observation.set("animation", animation.clone());
        }
        core.atomic("observations/latest.json", &observation)?;
        if role == Role::Wallpaper {
            core.atomic(&view.wallpaper_record(), &observation)?;
        }
        let mut detail = json!({
            "rendererId":id,"role":role_name,"capturedAtMs":now,
            "revision":snapshot["revision"].safe_uint(),"desiredHash":snapshot["desiredHash"].as_str(),
            "assetSha256":report["assetSha256"].as_str()
        });
        if report.has("animation") {
            detail["animation"] =
                serde_json::from_str(&animation.canonical()).map_err(|_| "ACK_INVALID")?;
        }
        entries.insert(id.into(), detail);
        Ok(Response::json(
            200,
            json!({"accepted":true,"revision":snapshot["revision"].safe_uint(),
            "source":"local-bridge","hostVisibility":"unknown"}),
        ))
    }

    fn health(&self, context: &Context, displays: &[Display], role: Role) -> Response {
        let policy = &context.layout.layout;
        let now = crate::journal::now_ms();
        let enabled = context.host.placement.enabled;
        let ids: Vec<Option<&str>> = if !enabled {
            vec![]
        } else if policy.mode == "single" {
            let target = &context.host.placement.target;
            vec![if target != "primary" {
                Some(target.as_str())
            } else {
                select_display(displays, "primary")
                    .map(|d| d.id.as_str())
                    .or_else(|| {
                        context
                            .native
                            .as_ref()
                            .and_then(|v| v["targets"][0]["targetId"].as_str())
                    })
            }]
        } else {
            policy.targets.iter().map(|id| Some(id.as_str())).collect()
        };
        let native_current = context.native.as_ref().filter(|v| {
            v["layoutRevision"] == context.layout.revision
                && v["settingsRevision"] == context.host.revision
        });
        let mut targets = Vec::new();
        let mut renderers = Vec::new();
        for id in ids {
            let view = View {
                target: if policy.mode == "single" { None } else { id },
                scope: if policy.mode == "independent" {
                    id.unwrap()
                } else {
                    "shared"
                },
            };
            let native_target = native_current.and_then(|v| {
                v["targets"]
                    .as_array()?
                    .iter()
                    .find(|t| t["targetId"].as_str() == id)
            });
            let mut target = json!({"targetId":id,"scopeId":view.scope,"revision":null,"desiredHash":null,
                "rendererAcknowledged":false,"nativeAttached":native_target.map(|t|t["nativeAttached"]==true),
                "online":id.map(|id|select_display(displays,id).is_some()),"lastAckAt":null,"renderer":null,
                "error":native_target.and_then(|t|t["error"].as_str()).map(|e|json!({"code":e})),
                "hostVisibility":"unknown"});
            let state = (|| -> Result<_, String> {
                // Health does not seed independent profiles.
                let scoped = if view.scope == "shared" {
                    None
                } else {
                    Some(self.owner.open_existing_scope(view.scope)?)
                };
                let core = scoped.as_ref().unwrap_or(&self.owner);
                let snapshot = self.snapshot(context, &view, core, displays, Role::Wallpaper)?;
                let observation = bounded_record(&core.store, &view.wallpaper_record(), 16384);
                Ok((snapshot, observation))
            })();
            match state {
                Ok((snapshot, observation)) => {
                    target["revision"] = json!(snapshot["revision"].safe_uint());
                    target["desiredHash"] = json!(snapshot["desiredHash"].as_str());
                    if let Some(renderer) = observation.and_then(|o| {
                        wallpaper_report(&o, &snapshot, view.target, &self.session, now)
                    }) {
                        target["rendererAcknowledged"] = json!(true);
                        target["lastAckAt"] = renderer["capturedAtMs"].clone();
                        target["renderer"] = renderer.clone();
                        renderers.push(renderer);
                    }
                }
                Err(_) => target["error"] = json!({"code":"TARGET_STATE_UNAVAILABLE"}),
            }
            targets.push(target);
        }
        if let Ok(reports) = self.reports.lock() {
            for report in reports.values().flat_map(HashMap::values) {
                if report["role"] == "manager" && fresh(now, report["capturedAtMs"].as_u64(), 30000)
                {
                    let mut detail = report.clone();
                    detail["recent"] = json!(fresh(now, report["capturedAtMs"].as_u64(), 4000));
                    renderers.push(detail);
                }
            }
        }
        let all = !targets.is_empty()
            && targets.iter().all(|t| {
                !t["targetId"].is_null()
                    && t["online"] != false
                    && t["rendererAcknowledged"] == true
            });
        let attached = native_current.is_some()
            && !targets.is_empty()
            && targets.iter().all(|t| t["nativeAttached"] == true);
        Response::json(
            200,
            json!({"schemaVersion":1,"source":"local-bridge","hostVisibility":"unknown",
            "mode":policy.mode,"enabled":enabled,"configuredTargetCount":targets.len(),
            "rendererRecentlyAcknowledged":all,"allTargetsAcknowledged":all,
            "nativeRecentlyReported":context.native.is_some(),"nativeAllAttached":attached,
            "lastAckAt":if all {targets.iter().filter_map(|t|t["lastAckAt"].as_u64()).min()} else {None},
            "perTarget":targets,"renderers":renderers,
            "capabilities":{"snapshot":true,"ack":true,"httpDesiredWrites":role==Role::Manager,"hostConnected":false},
            "trust":"renderer acknowledgements and native attachment reports are separate; visibility remains unknown"}),
        )
    }
}

pub(crate) fn artwork_name(desired: &js_json::Value) -> &'static str {
    match desired["visual"]["background"].as_str() {
        Some("forest") if desired["visual"]["theme"].as_str() == Some("warm") => ARTWORK[1],
        Some("city") => ARTWORK[2],
        Some("future") => ARTWORK[3],
        Some("coast") => ARTWORK[4],
        Some("alpine") => ARTWORK[5],
        Some("desert") => ARTWORK[6],
        _ => ARTWORK[0],
    }
}

fn wallpaper_report(
    observation: &Value,
    snapshot: &js_json::Value,
    target: Option<&str>,
    session: &str,
    now: u64,
) -> Option<Value> {
    let at = observation["capturedAt"]
        .as_str()
        .and_then(appearance_core::timestamp_ms)
        .and_then(|at| u64::try_from(at).ok())?;
    if !fresh(now, Some(at), 4000)
        || observation["hostSessionId"] != session
        || observation["targetId"] != json!(target)
        || observation["rendererRole"] != "wallpaper"
        || observation["source"] != "local-bridge"
        || observation["stateApplied"] != true
        || observation["assetLoaded"] != true
        || observation["revision"].as_u64() != snapshot["revision"].safe_uint()
        || observation["desiredHash"].as_str() != snapshot["desiredHash"].as_str()
        || observation["assetSha256"].as_str() != snapshot["artwork"]["sha256"].as_str()
        || observation["projectionId"].as_str() != snapshot["projectionId"].as_str()
    {
        return None;
    }
    let mut renderer = json!({"rendererId":observation["rendererId"].as_str()?,"role":"wallpaper",
        "revision":observation["revision"],"desiredHash":observation["desiredHash"],
        "assetSha256":observation["assetSha256"],"capturedAtMs":at,"recent":true});
    if let Some(animation) = observation.get("animation") {
        renderer["animation"] = animation.clone();
    }
    Some(renderer)
}

fn fields(value: &js_json::Value, names: &[&str]) -> Result<(), String> {
    if value
        .as_object()
        .is_some_and(|object| object.len() == names.len())
        && names.iter().all(|name| value.has(name))
    {
        Ok(())
    } else {
        Err("COMMAND_INVALID".into())
    }
}

fn monitor_record(d: &Display) -> Value {
    json!({"id":d.device_name,"idKind":"windows-gdi-device-name","primary":d.primary,
        "width":d.bounds.width,"height":d.bounds.height,"scalePercent":d.scale_percent,
        "resolutionUnit":"physical-pixels","scaleSource":if d.scale_percent.is_some(){"GetScaleFactorForMonitor"}else{"unknown"}})
}
