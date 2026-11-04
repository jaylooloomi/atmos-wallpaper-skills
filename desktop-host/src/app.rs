use crate::{
    control, displays, host_settings, journal::Journal, options::Options, runtime::Runtime, windows,
};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WebviewUrl, WebviewWindowBuilder,
};

struct State {
    runtime: Mutex<Option<Runtime>>,
    audio: Mutex<Option<crate::audio::AudioEngine>>,
    attached: Mutex<Option<(isize, isize)>>,
    exiting: AtomicBool,
    journal: Arc<Journal>,
}

fn open_manager(app: &tauri::AppHandle, port: u16, store: &Path) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("manager") {
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }
    let origin = format!("http://127.0.0.1:{port}");
    let expected = origin.clone();
    WebviewWindowBuilder::new(
        app,
        "manager",
        WebviewUrl::External(format!("{origin}/manager").parse().unwrap()),
    )
    .title("Atmos Wallpaper 管理")
    // Cosmetic host marker only; it grants no native capabilities or permissions.
    .initialization_script("window.__ATMOS_NATIVE_MANAGER__ = true;")
    .inner_size(1100.0, 800.0)
    .data_directory(store.join("manager-webview2"))
    .devtools(false)
    .on_navigation(move |u| {
        u.origin().ascii_serialization() == expected
            && ["/manager", "/"].contains(&u.path())
            && u.query().is_none()
    })
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn setup_tray(app: &tauri::App, port: u16, store: PathBuf, state: Arc<State>) -> tauri::Result<()> {
    let open = MenuItem::with_id(
        app,
        "manager",
        "開啟管理頁 / Open manager",
        true,
        None::<&str>,
    )?;
    let exit = MenuItem::with_id(app, "exit", "退出並還原桌布 / Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &exit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("application icon".into()))?;
    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Atmos Wallpaper")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "exit" => app.exit(0),
            "manager" => {
                if let Err(e) = open_manager(app, port, &store) {
                    state.journal.event("manager-failed", json!({"error":e}));
                }
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn watch(
    app: tauri::AppHandle,
    state: Arc<State>,
    options: Options,
    port: u16,
    quit_event: isize,
    manager_event: isize,
) {
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut first_ack = false;
        let mut stale = false;
        let mut last_ack = 0u64;
        while !state.exiting.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(1));
            if state.exiting.load(Ordering::SeqCst) {
                break;
            }
            if control::requested(quit_event)
                || options
                    .seconds
                    .is_some_and(|s| start.elapsed().as_secs() >= s)
            {
                app.exit(0);
                break;
            }
            if control::requested(manager_event) {
                let app2 = app.clone();
                let store = options.store.clone();
                let s = state.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Err(e) = open_manager(&app2, port, &store) {
                        s.journal.event("manager-failed", json!({"error":e}));
                    }
                });
            }
            let dead = state
                .runtime
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|r| r.child.try_wait().ok())
                .flatten()
                .is_some();
            if dead {
                state
                    .journal
                    .event("runtime-failed", json!({"action":"exit-and-restore"}));
                app.exit(2);
                break;
            }
            let configuration = host_settings::read(&options.store);
            let topology = displays::enumerate();
            let target = configuration
                .as_ref()
                .ok()
                .filter(|s| s.placement.enabled)
                .and_then(|s| {
                    topology
                        .as_ref()
                        .ok()
                        .and_then(|ds| displays::select(ds, &s.placement.target).ok())
                })
                .cloned();
            let pair = *state.attached.lock().unwrap();
            let mut native_attached = false;
            if let Some((h, p)) = pair {
                if let Some(ref display) = target {
                    native_attached = windows::still_attached(h, p, display.bounds);
                    if !native_attached {
                        let s = state.clone();
                        let bounds = display.bounds;
                        let _ = app.run_on_main_thread(move || {
                            let result = windows::desktop_parent().and_then(|parent| {
                                windows::attach(h, parent, bounds).map(|record| (parent, record))
                            });
                            match result {
                                Ok((parent, record)) => {
                                    *s.attached.lock().unwrap() = Some((h, parent));
                                    s.journal.event("reattached", json!(record));
                                }
                                Err(e) => {
                                    windows::hide(h);
                                    s.journal.event("attachment-pending", json!({"error":e}));
                                }
                            }
                        });
                    }
                } else if windows::record(h).visible {
                    let _ = app.run_on_main_thread(move || windows::hide(h));
                    state.journal.event("target-paused",json!({"reason":if configuration.as_ref().is_ok_and(|s|!s.placement.enabled){"cleared"}else{"offline-or-unresolved"}}));
                }
            }
            let _ = host_settings::atomic(
                &options.store,
                "host-status.json",
                &json!({
                    "schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),
                    "settingsRevision":configuration.as_ref().ok().map(|s|s.revision),
                    "targetId":target.as_ref().map(|d|&d.id),"nativeAttached":native_attached,
                    "bounds":target.as_ref().map(|d|d.bounds),"presentation":"unknown",
                    "state":if native_attached{"attached"}else if configuration.as_ref().is_ok_and(|s|!s.placement.enabled){"cleared"}else if target.is_none(){"target-unavailable"}else{"pending"}
                }),
            );
            let file = options.store.join("observations/wallpaper.json");
            let observation = std::fs::read(&file)
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
            let fresh = std::fs::metadata(&file)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|d| d.as_secs() < 5);
            if let Some(v) = observation
                .filter(|v| v["rendererRole"] == "wallpaper" && v["stateApplied"] == true && fresh)
            {
                last_ack = start.elapsed().as_secs();
                if !first_ack {
                    first_ack = true;
                    state.journal.event("renderer-ready",json!({"revision":v["revision"],"capturedAt":v["capturedAt"],"trust":"renderer-reported; native attachment checked separately"}));
                }
                if stale {
                    stale = false;
                    state
                        .journal
                        .event("renderer-recovered", json!({"revision":v["revision"]}));
                }
            }
            if target.is_some() && !first_ack && start.elapsed().as_secs() > 45 {
                state
                    .journal
                    .event("renderer-timeout", json!({"action":"exit-and-restore"}));
                app.exit(2);
                break;
            }
            // Locked/occluded WebViews can throttle; stale is not a startup failure.
            if first_ack && !stale && start.elapsed().as_secs().saturating_sub(last_ack) > 30 {
                stale = true;
                state.journal.event(
                    "renderer-stale",
                    json!({"hostAttached":true,"presentation":"unknown"}),
                );
            }
        }
    });
}

pub fn launch(
    options: Options,
    quit_event: isize,
    manager_event: isize,
    show_manager: bool,
) -> Result<i32, String> {
    let journal = Arc::new(Journal::new(options.status.clone())?);
    let original = windows::static_wallpaper()?;
    let configuration = host_settings::read(&options.store)?;
    let topology = displays::enumerate()?;
    let selected = configuration
        .placement
        .enabled
        .then(|| displays::select(&topology, &configuration.placement.target).ok())
        .flatten()
        .cloned();
    let target = selected.as_ref().map(|d| d.bounds);
    let runtime = Runtime::start(&options)?;
    let port = runtime.port;
    journal.event("runtime-ready",json!({"pid":runtime.child.id(),"port":port,"target":target,"livelyRequired":false,"boundedSeconds":options.seconds,"persistent":options.seconds.is_none()}));
    let state = Arc::new(State {
        runtime: Mutex::new(Some(runtime)),
        audio: Mutex::new(None),
        attached: Mutex::new(None),
        exiting: AtomicBool::new(false),
        journal: journal.clone(),
    });
    let status_store = options.store.clone();
    let setup_state = state.clone();
    let store = options.store.clone();
    let app = tauri::Builder::default()
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(
                app,
                "wallpaper",
                WebviewUrl::External(format!("http://127.0.0.1:{port}/wallpaper").parse()?),
            )
            .title("Atmos Wallpaper")
            .visible(false)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .data_directory(store.join("webview2"))
            .devtools(false)
            .on_navigation(move |u| {
                u.scheme() == "http"
                    && u.host_str() == Some("127.0.0.1")
                    && u.port() == Some(port)
                    && u.path() == "/wallpaper"
                    && u.query().is_none()
            })
            .build()?;
            let handle = window.hwnd()?.0 as isize;
            let parent = windows::desktop_parent()?;
            let actual = if let Some(bounds) = target {
                Some(windows::attach(handle, parent, bounds)?)
            } else {
                None
            };
            *setup_state.attached.lock().unwrap() = Some((handle, parent));
            setup_state.journal.event(
                "attached",
                json!({"window":actual,"parent":windows::record(parent),"target":target}),
            );
            *setup_state.audio.lock().unwrap() =
                Some(crate::audio::AudioEngine::start(store.clone(), handle));
            setup_tray(app, port, store.clone(), setup_state.clone())?;
            if show_manager {
                open_manager(app.handle(), port, &store)?;
            }
            watch(
                app.handle().clone(),
                setup_state.clone(),
                options,
                port,
                quit_event,
                manager_event,
            );
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for line in io::stdin().lock().lines().map_while(Result::ok) {
                    if line.trim() == "exit" {
                        handle.exit(0);
                        break;
                    }
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let code = Arc::new(AtomicI32::new(0));
    let intended = code.clone();
    let s = state.clone();
    let actual = app.run_return(move |app, event| match event {
        tauri::RunEvent::ExitRequested { code, api, .. } => {
            if !s.exiting.swap(true, Ordering::SeqCst) {
                api.prevent_exit();
                intended.store(code.unwrap_or(0), Ordering::SeqCst);
                s.audio.lock().unwrap().take();
                s.runtime.lock().unwrap().take();
                if let Some(manager) = app.get_webview_window("manager") {
                    let _ = manager.destroy();
                }
                if let Some(wallpaper) = app.get_webview_window("wallpaper") {
                    if let Err(e) = wallpaper.destroy() {
                        s.journal
                            .event("close-failed", json!({"error":e.to_string()}));
                        app.exit(2);
                    }
                } else {
                    app.exit(intended.load(Ordering::SeqCst));
                }
            }
        }
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Destroyed,
            ..
        } if label == "wallpaper" && s.exiting.load(Ordering::SeqCst) => {
            app.exit(intended.load(Ordering::SeqCst))
        }
        _ => {}
    });
    let pair = state.attached.lock().unwrap().take();
    let _ = host_settings::atomic(
        &status_store,
        "host-status.json",
        &json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"nativeAttached":false,"state":"stopped","presentation":"unknown"}),
    );
    journal.event("restored",json!({"ownWindowRemoved":pair.is_none_or(|(h,_)|!windows::exists(h)),"staticWallpaperUnchanged":windows::static_wallpaper().ok().as_ref()==Some(&original),"method":"remove-owned-layer; original wallpaper was never overwritten"}));
    let requested = code.load(Ordering::SeqCst);
    Ok(if requested != 0 { requested } else { actual })
}
