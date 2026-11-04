//! Windows .scr presentation: ordinary own windows, never a lock/authentication surface.
use crate::{displays, host_settings, policy_store, runtime::Runtime, windows};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    System::Threading::OpenMutexW,
    UI::WindowsAndMessaging::*,
};
const ACTIVE: &str = "Local\\AtmosWallpaper.ScreensaverActive";
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
pub fn active() -> bool {
    unsafe {
        let h = OpenMutexW(0x00100000, 0, wide(ACTIVE).as_ptr());
        if h.is_null() {
            windows_sys::Win32::Foundation::GetLastError()
                != windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND
        } else {
            windows_sys::Win32::Foundation::CloseHandle(h);
            true
        }
    }
}

fn event_name() -> Result<Vec<u16>, String> {
    use sha2::{Digest, Sha256};
    let folder = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("SCREENSAVER_LOCATION_INVALID")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    Ok(wide(&format!(
        "Local\\AtmosWallpaper.ScreensaverQuit.{:x}",
        Sha256::digest(folder.to_string_lossy().to_lowercase().as_bytes())
    )))
}
fn quit_event() -> Result<windows::OwnedHandle, String> {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::CreateEventW(
            std::ptr::null(),
            1,
            0,
            event_name()?.as_ptr(),
        );
        if handle.is_null() {
            Err("SCREENSAVER_EVENT_FAILED".into())
        } else {
            Ok(windows::OwnedHandle(handle as isize))
        }
    }
}
fn quit_requested(handle: isize) -> bool {
    unsafe {
        windows_sys::Win32::System::Threading::WaitForSingleObject(handle as _, 0)
            == windows_sys::Win32::Foundation::WAIT_OBJECT_0
    }
}
pub fn stop() -> Result<i32, String> {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::OpenEventW(
            windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE,
            0,
            event_name()?.as_ptr(),
        );
        if handle.is_null() {
            if windows_sys::Win32::Foundation::GetLastError()
                == windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND
            {
                return Ok(0);
            }
            return Err("SCREENSAVER_EVENT_FAILED".into());
        }
        let signaled = windows_sys::Win32::System::Threading::SetEvent(handle) != 0;
        windows_sys::Win32::Foundation::CloseHandle(handle);
        if !signaled {
            return Err("SCREENSAVER_EVENT_FAILED".into());
        }
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        unsafe {
            let handle = windows_sys::Win32::System::Threading::OpenEventW(
                windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE,
                0,
                event_name()?.as_ptr(),
            );
            if handle.is_null() {
                return Ok(0);
            }
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("SCREENSAVER_STOP_TIMEOUT".into())
}
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
}
pub fn open_settings() -> Result<i32, String> {
    unsafe {
        let mut buffer = [0u16; 32768];
        let count = GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) as usize;
        if count == 0 || count >= buffer.len() {
            return Err("SCREENSAVER_SETTINGS_FAILED".into());
        }
        let control = PathBuf::from(String::from_utf16_lossy(&buffer[..count])).join("control.exe");
        let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide(&control.to_string_lossy()).as_ptr(),
            wide("desk.cpl,screensaver,@screensaver").as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        if result as isize <= 32 {
            return Err("SCREENSAVER_SETTINGS_FAILED".into());
        }
    }
    println!(
        "{}",
        json!({"schemaVersion":1,"opened":true,"changesWindowsActivation":false})
    );
    Ok(0)
}

fn install_arguments(path: &Path) -> Result<String, String> {
    let original = path.to_string_lossy();
    let p = original.strip_prefix(r"\\?\").unwrap_or(&original);
    if !path.is_absolute()
        || p.starts_with(r"\\")
        || p.contains(['"', '\0', '\n', '\r'])
        || path.file_name().and_then(|s| s.to_str()) != Some("Atmos.scr")
    {
        return Err("SCREENSAVER_INSTALL_INVALID".into());
    }
    Ok(format!("desk.cpl,InstallScreenSaver \"{}\"", p))
}
pub fn install() -> Result<i32, String> {
    use sha2::{Digest, Sha256};
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let folder = exe.parent().ok_or("SCREENSAVER_INSTALL_INVALID")?;
    let source = folder.join("atmos-wallpaper.exe");
    let scr = folder.join("Atmos.scr");
    policy_store::safe_path(&scr)?;
    policy_store::safe_path(&source)?;
    let a = std::fs::metadata(&source).map_err(|_| "SCREENSAVER_INSTALL_MISSING")?;
    let b = std::fs::metadata(&scr).map_err(|_| "SCREENSAVER_INSTALL_MISSING")?;
    if !a.is_file() || !b.is_file() || a.len() > 128 * 1024 * 1024 || a.len() != b.len() {
        return Err("SCREENSAVER_INSTALL_INVALID".into());
    }
    if Sha256::digest(std::fs::read(&source).map_err(|_| "SCREENSAVER_INSTALL_INVALID")?)
        != Sha256::digest(std::fs::read(&scr).map_err(|_| "SCREENSAVER_INSTALL_INVALID")?)
    {
        return Err("SCREENSAVER_INSTALL_INVALID".into());
    }
    let args = install_arguments(&scr)?;
    crate::screensaver_registration::snapshot(&scr)?;
    unsafe {
        let mut buffer = [0u16; 32768];
        let count = GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) as usize;
        if count == 0 || count >= buffer.len() {
            return Err("SCREENSAVER_SETTINGS_FAILED".into());
        }
        let exe = PathBuf::from(String::from_utf16_lossy(&buffer[..count])).join("rundll32.exe");
        let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide(&exe.to_string_lossy()).as_ptr(),
            wide(&args).as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        if result as isize <= 32 {
            return Err("SCREENSAVER_SETTINGS_FAILED".into());
        }
    }
    println!(
        "{}",
        json!({"schemaVersion":1,"selectionRequested":true,"changesWindowsActivation":true,"windowsOwnsSelection":true})
    );
    Ok(0)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub mode: String,
    pub targets: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub screensaver: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            last_request_id: None,
            screensaver: Policy {
                mode: "duplicate".into(),
                targets: Vec::new(),
            },
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Update {
    schema_version: u32,
    request_id: String,
    expected_revision: u64,
    screensaver: Policy,
}
fn valid(s: &Settings) -> Result<(), String> {
    let p = &s.screensaver;
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || s.last_request_id
            .as_ref()
            .is_some_and(|id| !policy_store::valid_id(id))
        || !matches!(p.mode.as_str(), "duplicate" | "independent" | "span")
        || p.targets.len() > 16
        || p.targets
            .iter()
            .any(|id| !crate::audio_routing::valid_id(id))
        || p.targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != p.targets.len()
    {
        return Err("SCREENSAVER_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let value = policy_store::read_optional(&root.join("screensaver-settings.json"), 16384)
        .map_err(|_| "SCREENSAVER_INVALID")?;
    let s = match value {
        Some(v) => serde_json::from_value(v).map_err(|_| "SCREENSAVER_INVALID")?,
        None => Settings::default(),
    };
    valid(&s)?;
    Ok(s)
}
fn update(root: &Path, u: Update) -> Result<Value, String> {
    if u.schema_version != 1 {
        return Err("SCREENSAVER_INVALID".into());
    }
    let input = serde_json::to_value(&u).unwrap();
    let id = u.request_id.clone();
    policy_store::commit(
        root,
        "SCREENSAVER",
        "screensaver-settings.json",
        serde_json::to_value(Settings::default()).unwrap(),
        &id,
        u.expected_revision,
        &input,
        |record| {
            let s: Settings =
                serde_json::from_value(record.clone()).map_err(|_| "SCREENSAVER_INVALID")?;
            valid(&s)
        },
        |old, revision| {
            let old: Settings = serde_json::from_value(old).map_err(|_| "SCREENSAVER_INVALID")?;
            valid(&old)?;
            let next = Settings {
                schema_version: 1,
                revision,
                last_request_id: Some(id.clone()),
                screensaver: u.screensaver,
            };
            valid(&next)?;
            Ok(serde_json::to_value(next).unwrap())
        },
    )
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len()) || !matches!(args[0].as_str(), "get" | "update") {
        return Err("SCREENSAVER_INVALID".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("SCREENSAVER_INVALID".into());
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
            .map_err(|_| "SCREENSAVER_INVALID")?;
        if bytes.len() > 16384 {
            return Err("SCREENSAVER_INVALID".into());
        }
        Some(update(
            &root,
            serde_json::from_slice(&bytes).map_err(|_| "SCREENSAVER_INVALID")?,
        )?)
    } else {
        None
    };
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":read(&root)?,"displays":displays::enumerate().unwrap_or_default(),"audio":"silent","changesWindowsActivation":false,"result":result})
    );
    Ok(0)
}
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Full,
    Configure,
    Preview(isize),
}
pub fn parse(args: &[String]) -> Result<(Mode, Vec<String>), String> {
    let Some(first) = args.first() else {
        return Ok((Mode::Configure, Vec::new()));
    };
    let lower = first.to_ascii_lowercase();
    let mut used = 1;
    let handle = |s: &str| -> Result<isize, String> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err("SCREENSAVER_ARGUMENT_INVALID".into());
        }
        let id = s
            .parse::<isize>()
            .map_err(|_| "SCREENSAVER_ARGUMENT_INVALID")?;
        if id <= 0 {
            return Err("SCREENSAVER_ARGUMENT_INVALID".into());
        }
        Ok(id)
    };
    let mode = if lower == "/s" {
        Mode::Full
    } else if lower == "/c" {
        if args.get(1).is_some_and(|x| !x.starts_with("--")) {
            handle(&args[1])?;
            used = 2;
        }
        Mode::Configure
    } else if let Some(v) = lower.strip_prefix("/c:") {
        handle(v)?;
        Mode::Configure
    } else if lower == "/p" {
        used = 2;
        Mode::Preview(handle(args.get(1).ok_or("SCREENSAVER_ARGUMENT_INVALID")?)?)
    } else if let Some(v) = lower.strip_prefix("/p:") {
        Mode::Preview(handle(v)?)
    } else {
        return Err("SCREENSAVER_ARGUMENT_INVALID".into());
    };
    let rest = args[used..].to_vec();
    if matches!(mode, Mode::Configure) && !rest.is_empty() {
        return Err("SCREENSAVER_ARGUMENT_INVALID".into());
    }
    Ok((mode, rest))
}
#[repr(C)]
struct LastInput {
    size: u32,
    tick: u32,
}
#[link(name = "user32")]
extern "system" {
    fn GetLastInputInfo(info: *mut LastInput) -> i32;
}
fn last_input() -> Option<u32> {
    let mut v = LastInput {
        size: std::mem::size_of::<LastInput>() as u32,
        tick: 0,
    };
    (unsafe { GetLastInputInfo(&mut v) } != 0).then_some(v.tick)
}
fn parent_record(parent: isize) -> Result<(u32, windows::Bounds), String> {
    unsafe {
        let h = parent as HWND;
        if IsWindow(h) == 0 {
            return Err("SCREENSAVER_PREVIEW_PARENT_INVALID".into());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        let mut rect: RECT = std::mem::zeroed();
        if pid == 0
            || GetClientRect(h, &mut rect) == 0
            || rect.right <= 0
            || rect.bottom <= 0
            || rect.right > 16384
            || rect.bottom > 16384
        {
            return Err("SCREENSAVER_PREVIEW_PARENT_INVALID".into());
        }
        Ok((
            pid,
            windows::Bounds {
                x: 0,
                y: 0,
                width: rect.right,
                height: rect.bottom,
            },
        ))
    }
}
#[derive(Clone)]
struct Plan {
    id: String,
    bounds: windows::Bounds,
}
fn plans(policy: &Policy, ds: &[displays::Display]) -> Vec<Plan> {
    let mut selected: Vec<_> = if policy.targets.is_empty() {
        ds.iter().filter(|d| d.selectable).collect()
    } else {
        policy
            .targets
            .iter()
            .filter_map(|id| ds.iter().find(|d| &d.id == id && d.selectable))
            .collect()
    };
    if policy.targets.is_empty() {
        selected.sort_by_key(|d| !d.primary);
    }
    selected
        .into_iter()
        .map(|d| Plan {
            id: d.id.clone(),
            bounds: d.bounds,
        })
        .collect()
}

fn show(
    app: &tauri::AppHandle,
    root: &Path,
    port: u16,
    plan: &Plan,
    preview: Option<isize>,
) -> Result<(), String> {
    let label = format!("screensaver-{}", plan.id);
    let expected = format!("http://127.0.0.1:{port}");
    let route = format!("/screensaver/{}", plan.id);
    let u = format!("{expected}{route}");
    let window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(u.parse().map_err(|_| "SCREENSAVER_URL")?),
    )
    .title("Atmos Screensaver")
    .visible(false)
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(preview.is_none())
    .focused(false)
    .devtools(false)
    .data_directory(root.join("screensaver-webview2").join(&plan.id))
    .on_navigation(move |u| {
        u.origin().ascii_serialization() == expected && u.path() == route && u.query().is_none()
    })
    .build()
    .map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
    let result = (|| -> Result<(), String> {
        window
            .set_position(tauri::PhysicalPosition::new(plan.bounds.x, plan.bounds.y))
            .map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
        window
            .set_size(tauri::PhysicalSize::new(
                plan.bounds.width as u32,
                plan.bounds.height as u32,
            ))
            .map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
        if let Some(parent) = preview {
            parent_record(parent)?;
            unsafe {
                let h = window.hwnd().map_err(|_| "SCREENSAVER_WINDOW_FAILED")?.0 as HWND;
                let style = GetWindowLongPtrW(h, GWL_STYLE);
                SetWindowLongPtrW(
                    h,
                    GWL_STYLE,
                    (style & !(WS_POPUP as isize)) | WS_CHILD as isize,
                );
                windows_sys::Win32::Foundation::SetLastError(0);
                let prior = SetParent(h, parent as HWND);
                if prior.is_null() && windows_sys::Win32::Foundation::GetLastError() != 0 {
                    return Err("SCREENSAVER_PREVIEW_ATTACH_FAILED".into());
                }
                SetWindowPos(
                    h,
                    std::ptr::null_mut(),
                    0,
                    0,
                    plan.bounds.width,
                    plan.bounds.height,
                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
                );
            }
        }
        window.show().map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = window.destroy();
    }
    result
}
struct State {
    runtime: Mutex<Option<Runtime>>,
    stopping: AtomicBool,
    views: Mutex<BTreeMap<String, Plan>>,
    owned: Mutex<Vec<isize>>,
}

fn renderer_ack(root: &Path, policy: &Settings, p: &Plan, session: &str, preview: bool) -> bool {
    if session.is_empty() {
        return false;
    }
    let scope = if policy.screensaver.mode == "independent" {
        root.join("targets").join(&p.id)
    } else {
        root.to_path_buf()
    };
    let file = scope
        .join("observations")
        .join(format!("screensaver-{}.json", p.id));
    let Some(v) = policy_store::read_optional(&file, 16384).ok().flatten() else {
        return false;
    };
    let Some(live) = policy_store::read_optional(&scope.join("live.json"), 16 * 1048576)
        .ok()
        .flatten()
    else {
        return false;
    };
    let fresh = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_millis() < 4000);
    fresh
        && live["revision"].as_u64().is_some()
        && live["desiredHash"].as_str().is_some_and(|h| h.len() == 64)
        && v["rendererRole"] == "screensaver"
        && v["hostSessionId"] == session
        && v["targetId"] == p.id
        && v["source"] == "local-bridge"
        && v["stateApplied"] == true
        && v["assetLoaded"] == true
        && v["revision"] == live["revision"]
        && v["desiredHash"] == live["desiredHash"]
        && v["projection"]["layoutRevision"] == policy.revision
        && (preview || v["projection"]["targetBounds"] == json!(p.bounds))
}
fn synchronize(
    app: &tauri::AppHandle,
    state: &State,
    root: &Path,
    port: u16,
    preview: Option<isize>,
) -> Result<(), String> {
    let ds = displays::enumerate()?;
    let policy = read(root)?;
    let mut desired = plans(&policy.screensaver, &ds);
    if let Some(parent) = preview {
        let first = desired.first().ok_or("SCREENSAVER_NO_TARGET")?;
        desired = vec![Plan {
            id: first.id.clone(),
            bounds: parent_record(parent)?.1,
        }];
    }
    if desired.is_empty() {
        return Err("SCREENSAVER_NO_TARGET".into());
    }
    let mut views = state.views.lock().map_err(|_| "SCREENSAVER_STATE_ERROR")?;
    let removed = views
        .keys()
        .filter(|id| !desired.iter().any(|p| &p.id == *id))
        .cloned()
        .collect::<Vec<_>>();
    // Create replacements before removing old views so a topology change cannot close the app implicitly.
    for p in &desired {
        let label = format!("screensaver-{}", p.id);
        if let Some(window) = app.get_webview_window(&label) {
            if views.get(&p.id).is_some_and(|v| v.bounds != p.bounds) {
                window
                    .set_position(tauri::PhysicalPosition::new(p.bounds.x, p.bounds.y))
                    .map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
                window
                    .set_size(tauri::PhysicalSize::new(
                        p.bounds.width as u32,
                        p.bounds.height as u32,
                    ))
                    .map_err(|_| "SCREENSAVER_WINDOW_FAILED")?;
            }
        } else {
            show(app, root, port, p, preview)?;
        }
        views.insert(p.id.clone(), p.clone());
        if let Some(window) = app.get_webview_window(&label) {
            if let Ok(h) = window.hwnd() {
                let h = h.0 as isize;
                let mut owned = state.owned.lock().unwrap();
                if !owned.contains(&h) {
                    owned.push(h);
                }
            }
        }
    }
    for id in removed {
        if let Some(window) = app.get_webview_window(&format!("screensaver-{id}")) {
            let _ = window.destroy();
        }
        views.remove(&id);
    }
    let session = state
        .runtime
        .lock()
        .unwrap()
        .as_ref()
        .map(|r| r.session_id.clone())
        .unwrap_or_default();
    host_settings::atomic(
        root,
        "screensaver-status.json",
        &json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"mode":policy.screensaver.mode,"revision":policy.revision,"preview":preview.is_some(),"audio":"silent","targets":desired.iter().map(|p|json!({"targetId":p.id,"bounds":p.bounds,"nativeWindowCreated":true,"rendererAcknowledged":renderer_ack(root,&policy,p,&session,preview.is_some()),"presentation":"unknown"})).collect::<Vec<_>>()}),
    )?;
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<i32, String> {
    let (mode, options_args) = parse(&args)?;
    if mode == Mode::Configure {
        let exe = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("atmos-wallpaper.exe");
        if !exe.is_file() {
            return Err("SCREENSAVER_MANAGER_UNAVAILABLE".into());
        }
        unsafe {
            let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                wide("open").as_ptr(),
                wide(&exe.to_string_lossy()).as_ptr(),
                wide("--manager").as_ptr(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
            if result as isize <= 32 {
                return Err("SCREENSAVER_MANAGER_UNAVAILABLE".into());
            }
        }
        return Ok(0);
    }
    let quit = quit_event()?;
    let quit_handle = quit.0;
    let options = crate::installed::configuration(options_args)?;
    if quit_requested(quit_handle) {
        return Ok(0);
    }
    let preview = if let Mode::Preview(h) = mode {
        Some((h, parent_record(h)?.0))
    } else {
        None
    };
    // Preview is silent itself but does not pause the user's actual wallpaper audio.
    let _active = if preview.is_none() {
        Some(windows::named_singleton(ACTIVE)?)
    } else {
        None
    };
    let _preview_singleton = if let Some((h, _)) = preview {
        Some(windows::named_singleton(&format!(
            "Local\\AtmosWallpaper.ScreensaverPreview.{h}"
        ))?)
    } else {
        None
    };
    let root = options.store.clone();
    let status_root = root.clone();
    let runtime = Runtime::start_with_mode(&options, "screensaver")?;
    let port = runtime.port;
    let state = Arc::new(State {
        runtime: Mutex::new(Some(runtime)),
        stopping: AtomicBool::new(false),
        views: Mutex::new(BTreeMap::new()),
        owned: Mutex::new(Vec::new()),
    });
    let shared = state.clone();
    let input = last_input();
    let started = Instant::now();
    let application = tauri::Builder::default()
        .setup(move |app| {
            synchronize(app.handle(), &shared, &root, port, preview.map(|p| p.0))
                .map_err(std::io::Error::other)?;
            let app_handle = app.handle().clone();
            let state = shared.clone();
            let store = root.clone();
            std::thread::spawn(move || {
                let mut last_topology = Instant::now();
                while !state.stopping.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(150));
                    if state.stopping.load(Ordering::SeqCst) {
                        break;
                    }
                    let parent_gone = preview.is_some_and(|(h, pid)| {
                        parent_record(h).map(|p| p.0 != pid).unwrap_or(true)
                    });
                    let current_input = last_input();
                    let changed = preview.is_none()
                        && started.elapsed() > Duration::from_millis(1200)
                        && (current_input.is_none() || current_input != input);
                    let timed = options
                        .seconds
                        .is_some_and(|s| started.elapsed() >= Duration::from_secs(s));
                    let failed = state
                        .runtime
                        .lock()
                        .unwrap()
                        .as_mut()
                        .is_none_or(|r| r.child.try_wait().ok().flatten().is_some());
                    if parent_gone || changed || timed || failed || quit_requested(quit_handle) {
                        app_handle.exit(if failed { 1 } else { 0 });
                        break;
                    }
                    if last_topology.elapsed() >= Duration::from_secs(1) {
                        last_topology = Instant::now();
                        let state = state.clone();
                        let store = store.clone();
                        let main = app_handle.clone();
                        let _ = app_handle.run_on_main_thread(move || {
                            if !state.stopping.load(Ordering::SeqCst)
                                && synchronize(&main, &state, &store, port, preview.map(|p| p.0))
                                    .is_err()
                            {
                                main.exit(1);
                            }
                        });
                    }
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let exit_state = state.clone();
    let exit_code = Arc::new(AtomicI32::new(0));
    let intended = exit_code.clone();
    let actual = application.run_return(move |app, event| match event {
        tauri::RunEvent::ExitRequested { code, api, .. } => {
            if !exit_state.stopping.swap(true, Ordering::SeqCst) {
                api.prevent_exit();
                intended.store(code.unwrap_or(0), Ordering::SeqCst);
                drop(exit_state.runtime.lock().unwrap().take());
                let windows = app.webview_windows();
                for window in windows.values() {
                    let _ = window.destroy();
                }
                if windows.is_empty() {
                    app.exit(intended.load(Ordering::SeqCst));
                }
            }
        }
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::Destroyed,
            ..
        } if exit_state.stopping.load(Ordering::SeqCst) => {
            if app.webview_windows().is_empty() {
                app.exit(intended.load(Ordering::SeqCst));
            }
        }
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { .. },
            ..
        } => app.exit(0),
        _ => {}
    });
    state.stopping.store(true, Ordering::SeqCst);
    drop(state.runtime.lock().unwrap().take());
    let code = exit_code.load(Ordering::SeqCst);
    let _ = host_settings::atomic(
        &status_root,
        "screensaver-status.json",
        &json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"state":"stopped","ownWindowRemoved":state.owned.lock().unwrap().iter().all(|h|!windows::exists(*h)),"audio":"silent"}),
    );
    Ok(if code != 0 { code } else { actual })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|v| v.to_string()).collect()
    }
    #[test]
    fn install_command_quotes_exact_own_scr_only() {
        let args = install_arguments(Path::new(r"C:\Apps\Atmos 中文\Atmos.scr")).unwrap();
        assert_eq!(
            args,
            r#"desk.cpl,InstallScreenSaver "C:\Apps\Atmos 中文\Atmos.scr""#
        );
        for p in [
            "relative/Atmos.scr",
            r"C:\Apps\evil.scr",
            r#"C:\Apps\x"bad\Atmos.scr"#,
        ] {
            assert!(install_arguments(Path::new(p)).is_err());
        }
    }
    #[test]
    fn strict_windows_scr_arguments() {
        assert_eq!(parse(&args(&["/S"])).unwrap().0, Mode::Full);
        assert_eq!(parse(&args(&["/p", "123"])).unwrap().0, Mode::Preview(123));
        assert_eq!(parse(&args(&["/p:123"])).unwrap().0, Mode::Preview(123));
        assert_eq!(parse(&args(&["/c:123"])).unwrap().0, Mode::Configure);
        for input in [
            args(&["/p"]),
            args(&["/p", "0"]),
            args(&["/p", "-1"]),
            args(&["/p", "1;shutdown"]),
            args(&["/c", "123", "junk"]),
            args(&["/exec"]),
        ] {
            assert!(parse(&input).is_err());
        }
        assert_eq!(
            parse(&args(&["/s", "--run-seconds", "5"])).unwrap().1,
            args(&["--run-seconds", "5"])
        );
    }
    #[test]
    fn screensaver_namespace_is_strict_and_has_no_audio_or_activation_side_effect() {
        let mut s = Settings::default();
        assert!(valid(&s).is_ok());
        s.screensaver.mode = "lockscreen".into();
        assert!(valid(&s).is_err());
        s = Settings::default();
        s.screensaver.targets = vec!["primary".into()];
        assert!(valid(&s).is_err());
        assert!(serde_json::from_value::<Policy>(
            json!({"mode":"duplicate","targets":[],"audio":true})
        )
        .is_err());
    }
    #[test]
    fn invalid_preview_parent_is_rejected_without_creating_windows() {
        assert!(parent_record(0).is_err());
        assert!(parent_record(-1).is_err());
    }
}
