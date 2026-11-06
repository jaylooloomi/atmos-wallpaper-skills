use crate::{
    control, displays, host_settings, journal::Journal, options::Options, runtime::Runtime, windows,
};
use serde_json::json;
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
    views: Mutex<std::collections::BTreeMap<String, crate::multi::View>>,
    memory: crate::renderer_memory::Sampler,
    owned_handles: Mutex<Vec<isize>>,
    renderer_ready: AtomicBool,
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
    .title("Atmos — 個人資訊桌面")
    // Cosmetic host marker only; it grants no native capabilities or permissions.
    .initialization_script("window.__ATMOS_NATIVE_MANAGER__ = true;")
    .inner_size(1100.0, 800.0)
    .data_directory(store.join("manager-webview2"))
    .devtools(false)
    .on_navigation(move |u| {
        u.origin().ascii_serialization() == expected
            && (["/manager", "/"].contains(&u.path())
                || u.path().strip_prefix("/manager/").is_some_and(|id| {
                    id.len() == 72
                        && id.starts_with("display-")
                        && id[8..]
                            .bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                }))
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
    let next = MenuItem::with_id(
        app,
        "next-preset",
        "主螢幕下一個外觀 / Next primary preset",
        true,
        None::<&str>,
    )?;
    let normal = MenuItem::with_id(app, "focus-normal", "正常顯示 / Normal", true, None::<&str>)?;
    let focus = MenuItem::with_id(
        app,
        "focus-private",
        "專注：隱藏私人資訊 / Focus",
        true,
        None::<&str>,
    )?;
    let showcase = MenuItem::with_id(
        app,
        "focus-showcase",
        "展示：只顯示背景 / Showcase",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&open, &next, &normal, &focus, &showcase, &exit])?;
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
            "next-preset" | "focus-normal" | "focus-private" | "focus-showcase" => {
                let action = match event.id().as_ref() {
                    "next-preset" => "next-preset",
                    "focus-private" => "focus",
                    "focus-showcase" => "showcase",
                    _ => "normal",
                };
                let target = (|| -> Result<String, String> {
                    if action != "next-preset" {
                        return Ok("shared".into());
                    }
                    let layout = crate::layout::read(&store)?;
                    if layout.layout.mode != "independent" {
                        return Ok("shared".into());
                    }
                    let displays = crate::displays::enumerate()?;
                    let primary = displays
                        .iter()
                        .find(|d| d.primary)
                        .ok_or("TARGET_PRIMARY_UNAVAILABLE")?;
                    let views = state.views.lock().map_err(|_| "VIEWS_BUSY")?;
                    views
                        .values()
                        .find(|v| v.plan.id == primary.id)
                        .map(|v| v.plan.scope.clone())
                        .ok_or("TARGET_PRIMARY_NOT_SELECTED".into())
                })();
                match target {
                    Ok(target) => {
                        let session = state
                            .runtime
                            .lock()
                            .ok()
                            .and_then(|r| r.as_ref().map(|r| r.session_id.clone()));
                        if let Some(session) = session {
                            let store = store.clone();
                            let state = state.clone();
                            std::thread::spawn(move || {
                                match tray_quick_action(action, &store, &target, &session) {
                                    Ok(()) => state.journal.event(
                                        "quick-action-committed",
                                        json!({"action":action,"rendererAcknowledged":false}),
                                    ),
                                    Err(e) => state.journal.event(
                                        "quick-action-failed",
                                        json!({"action":action,"error":e}),
                                    ),
                                }
                            });
                        }
                    }
                    Err(e) => state
                        .journal
                        .event("quick-action-failed", json!({"action":action,"error":e})),
                }
            }
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

fn tray_quick_action(
    action: &str,
    store: &Path,
    target: &str,
    session: &str,
) -> Result<(), String> {
    if action != "next-preset" {
        return crate::focus::quick_action(action, store, session);
    }
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    let payload = crate::installed::payload_root()?;
    crate::installed::verify(&payload)?;
    let out = Command::new(payload.join("runtime/node/node.exe"))
        .arg(payload.join("app/implementation-p4_2_5/src/quick-actions.mjs"))
        .args([
            action,
            "--store",
            &store.to_string_lossy(),
            "--target",
            target,
            "--session",
            session,
        ])
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    if out.stdout.len() > 65536 {
        return Err("QUICK_ACTION_RESPONSE_INVALID".into());
    }
    let response: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|_| "QUICK_ACTION_RESPONSE_INVALID")?;
    if !out.status.success() || response["ok"] != true {
        return Err(response["error"]["code"]
            .as_str()
            .unwrap_or("QUICK_ACTION_FAILED")
            .into());
    }
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
            let app2 = app.clone();
            let s = state.clone();
            let root = options.store.clone();
            let elapsed = start.elapsed().as_secs();
            let _=app.run_on_main_thread(move||{
                let host=host_settings::read(&root);let layout=crate::layout::read(&root);let topology=displays::enumerate();
                let desired=match (&host,&layout,&topology){(Ok(h),Ok(l),Ok(ds))=>crate::multi::plans(h,l,ds),_=>vec![]};
                let session=s.runtime.lock().unwrap().as_ref().map(|r|r.session_id.clone()).unwrap_or_default();
                let mut views=s.views.lock().unwrap();crate::multi::synchronize(&app2,&root,port,&desired,&mut views);
                let mut all=s.owned_handles.lock().unwrap();for v in views.values(){if !all.contains(&v.handle){all.push(v.handle)}}drop(all);
                if let Some(audio)=s.audio.lock().unwrap().as_ref(){audio.set_targets(views.values().filter(|v|v.error.is_none()).map(|v|crate::audio::AudioTarget{id:v.plan.id.clone(),primary:topology.as_ref().is_ok_and(|ds|ds.iter().any(|d|d.id==v.plan.id&&d.primary)),handle:v.handle,source_root:if v.plan.scope=="shared"{root.clone()}else{root.join("targets").join(&v.plan.id)}}).collect());}
                let performance=crate::performance::read(&root).ok();
                let targets=desired.iter().map(|p|{let view=views.get(&p.label);let native=view.is_some_and(|v|v.error.is_none()&&windows::still_attached(v.handle,v.parent,p.bounds));json!({"targetId":p.id,"scopeId":p.scope,"bounds":p.bounds,"runtimePolicy":performance.as_ref().map(|s|crate::performance::state(&s.performance,p.bounds)),"nativeAttached":native,"rendererAcknowledged":crate::multi::observation(&root,p,&session),"error":view.and_then(|v|v.error.clone()),"presentation":"unknown"})}).collect::<Vec<_>>();
                if targets.iter().any(|t|t["rendererAcknowledged"]==true)&&!s.renderer_ready.swap(true,Ordering::SeqCst){s.journal.event("renderer-ready",json!({"targets":targets.len(),"trust":"renderer-reported; native attachment separate"}));}
                if !desired.is_empty()&&!s.renderer_ready.load(Ordering::SeqCst)&&elapsed>45{s.journal.event("renderer-timeout",json!({"action":"exit-and-restore"}));app2.exit(2);}
                let configured=layout.as_ref().ok().map(|l|if l.layout.mode=="single"{desired.len()}else{l.layout.targets.len()}).unwrap_or(0);
                let attached=!targets.is_empty()&&targets.len()==configured&&targets.iter().all(|v|v["nativeAttached"]==true);
                let mut memory_views=desired.iter().map(|p|(p.label.clone(),"wallpaper".to_string())).collect::<Vec<_>>();memory_views.push(("manager".into(),"manager".into()));s.memory.poll(&app2,memory_views);
                let _=host_settings::atomic(&root,"host-status.json",&json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"hostSessionId":session,"settingsRevision":host.as_ref().ok().map(|h|h.revision),"layoutRevision":layout.as_ref().ok().map(|l|l.revision),"targetId":desired.first().map(|p|&p.id),"bounds":desired.first().map(|p|p.bounds),"nativeAttached":attached,"targets":targets,"rendererMemory":s.memory.snapshot(),"configuredTargetCount":configured,"presentation":"unknown","state":if attached{"attached"}else if host.as_ref().is_ok_and(|h|!h.placement.enabled){"cleared"}else{"pending-or-offline"}}));
            });
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
    let layout = crate::layout::read(&options.store)?;
    let initial_plans = crate::multi::plans(&configuration, &layout, &topology);
    let target = initial_plans.first().map(|p| p.bounds);
    let runtime = Runtime::start(&options)?;
    let port = runtime.port;
    journal.event("runtime-ready",json!({"pid":runtime.child.id(),"port":port,"target":target,"livelyRequired":false,"boundedSeconds":options.seconds,"persistent":options.seconds.is_none()}));
    let state = Arc::new(State {
        runtime: Mutex::new(Some(runtime)),
        audio: Mutex::new(None),
        views: Mutex::new(std::collections::BTreeMap::new()),
        memory: crate::renderer_memory::Sampler::default(),
        owned_handles: Mutex::new(Vec::new()),
        renderer_ready: AtomicBool::new(false),
        exiting: AtomicBool::new(false),
        journal: journal.clone(),
    });
    let status_store = options.store.clone();
    let setup_state = state.clone();
    let store = options.store.clone();
    let app = tauri::Builder::default()
        .setup(move |app| {
            {let mut views=setup_state.views.lock().unwrap();crate::multi::synchronize(app.handle(),&store,port,&initial_plans,&mut views);
            *setup_state.owned_handles.lock().unwrap()=views.values().map(|v|v.handle).collect();
            let audio=crate::audio::AudioEngine::start(store.clone(),0);
            audio.set_targets(views.values().filter(|v|v.error.is_none()).map(|v|crate::audio::AudioTarget{id:v.plan.id.clone(),primary:topology.iter().any(|d|d.id==v.plan.id&&d.primary),handle:v.handle,source_root:if v.plan.scope=="shared"{store.clone()}else{store.join("targets").join(&v.plan.id)}}).collect());
            *setup_state.audio.lock().unwrap()=Some(audio);
            setup_state.journal.event("attached",json!({"target":target,"window":views.values().next().map(|v|windows::record(v.handle)),"parent":views.values().next().map(|v|windows::record(v.parent)),"views":views.len()}));}
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
            // Closing the last manager or changing the last wallpaper must not quit the tray host.
            if code.is_none() && !s.exiting.load(Ordering::SeqCst) {
                api.prevent_exit();
                return;
            }
            if !s.exiting.swap(true, Ordering::SeqCst) {
                api.prevent_exit();
                intended.store(code.unwrap_or(0), Ordering::SeqCst);
                s.audio.lock().unwrap().take();
                s.runtime.lock().unwrap().take();
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
        } if s.exiting.load(Ordering::SeqCst) => {
            if app.webview_windows().is_empty() {
                app.exit(intended.load(Ordering::SeqCst));
            }
        }
        _ => {}
    });
    let handles = state.owned_handles.lock().unwrap();
    let _ = host_settings::atomic(
        &status_store,
        "host-status.json",
        &json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"hostSessionId":null,"nativeAttached":false,"state":"stopped","presentation":"unknown"}),
    );
    journal.event("restored",json!({"ownWindowRemoved":handles.iter().all(|h|!windows::exists(*h)),"staticWallpaperUnchanged":windows::static_wallpaper().ok().as_ref()==Some(&original),"method":"remove-owned-layer; original wallpaper was never overwritten"}));
    let requested = code.load(Ordering::SeqCst);
    Ok(if requested != 0 { requested } else { actual })
}
