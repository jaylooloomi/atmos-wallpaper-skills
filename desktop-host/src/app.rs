use crate::{control, journal::Journal, options::Options, runtime::Runtime, windows};
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
    target: windows::Bounds,
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
            let pair = *state.attached.lock().unwrap();
            if let Some((h, p)) = pair {
                if !windows::still_attached(h, p, target) {
                    let app2 = app.clone();
                    let s = state.clone();
                    let _ = app.run_on_main_thread(move || {
                        let result = windows::desktop_parent().and_then(|parent| {
                            windows::attach(h, parent, target).map(|record| (parent, record))
                        });
                        match result {
                            Ok((parent, record)) => {
                                *s.attached.lock().unwrap() = Some((h, parent));
                                s.journal.event("reattached", json!(record));
                            }
                            Err(e) => {
                                s.journal.event("attachment-failed", json!({"error":e}));
                                app2.exit(2);
                            }
                        }
                    });
                }
            }
            let file = options.store.join("observations/latest.json");
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
            if !first_ack && start.elapsed().as_secs() > 45 {
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
    let target = windows::primary_bounds()?;
    let runtime = Runtime::start(&options)?;
    let port = runtime.port;
    journal.event("runtime-ready",json!({"pid":runtime.child.id(),"port":port,"target":target,"livelyRequired":false,"boundedSeconds":options.seconds,"persistent":options.seconds.is_none()}));
    let state = Arc::new(State {
        runtime: Mutex::new(Some(runtime)),
        attached: Mutex::new(None),
        exiting: AtomicBool::new(false),
        journal: journal.clone(),
    });
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
            let actual = windows::attach(handle, parent, target)?;
            *setup_state.attached.lock().unwrap() = Some((handle, parent));
            setup_state.journal.event(
                "attached",
                json!({"window":actual,"parent":windows::record(parent),"target":target}),
            );
            setup_tray(app, port, store.clone(), setup_state.clone())?;
            if show_manager {
                open_manager(app.handle(), port, &store)?;
            }
            watch(
                app.handle().clone(),
                setup_state.clone(),
                options,
                target,
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
    journal.event("restored",json!({"ownWindowRemoved":pair.is_none_or(|(h,_)|!windows::exists(h)),"staticWallpaperUnchanged":windows::static_wallpaper().ok().as_ref()==Some(&original),"method":"remove-owned-layer; original wallpaper was never overwritten"}));
    let requested = code.load(Ordering::SeqCst);
    Ok(if requested != 0 { requested } else { actual })
}
