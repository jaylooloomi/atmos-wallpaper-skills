#[cfg(windows)]
mod journal;
mod options;
#[cfg(windows)]
mod runtime;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
fn main() {
    eprintln!("This prototype implements Windows only; macOS adapter is not implemented.");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Atmos: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn run() -> Result<i32, String> {
    use serde_json::json;
    use std::{
        io::{self, BufRead},
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("Atmos single-primary-monitor prototype (Windows only)\nRequired absolute paths: --runtime-root DIR --node EXE --python EXE --store DIR --status FILE\nOptional: --run-seconds 5..3600 (default 600)\nTray: open manager / exit and restore. Console stdin: exit. No Lively, autostart, installer, or macOS adapter.");
        return Ok(0);
    }
    if args == ["--inspect-desktop"] {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"primary":windows::primary_bounds()?,"windows":windows::desktop_windows()})
            )
            .unwrap()
        );
        return Ok(0);
    }
    let options = options::parse(args)?;
    let _singleton = windows::singleton()?;
    let journal = Arc::new(journal::Journal::new(options.status.clone())?);
    let original = windows::static_wallpaper()?;
    let target = windows::primary_bounds()?;
    let runtime = runtime::Runtime::start(&options)?;
    let port = runtime.port;
    journal.event(
        "runtime-ready",
        json!({"pid":runtime.child.id(),"port":port,"target":target,"livelyRequired":false}),
    );
    let runtime = Arc::new(Mutex::new(Some(runtime)));
    let attached = Arc::new(Mutex::new(None::<(isize, isize)>));
    let exiting = Arc::new(AtomicBool::new(false));
    let j = journal.clone();
    let a = attached.clone();
    let rt = runtime.clone();
    let quit = exiting.clone();
    let duration = options.seconds;
    let store = options.store.clone();
    let app=tauri::Builder::default().setup(move |app| {
        let url=format!("http://127.0.0.1:{port}/wallpaper").parse()?;
        let data_dir=store.join("webview2");
        let window=WebviewWindowBuilder::new(app,"wallpaper",WebviewUrl::External(url))
            .title("Atmos Wallpaper Prototype").visible(false).decorations(false).resizable(false).skip_taskbar(true)
            .focused(false).shadow(false).data_directory(data_dir).devtools(false)
            .on_navigation(move |url| url.scheme()=="http" && url.host_str()==Some("127.0.0.1") && url.port()==Some(port) && url.path()=="/wallpaper" && url.query().is_none())
            .build()?;
        let handle=window.hwnd()?.0 as isize;
        let parent=windows::desktop_parent()?;
        let actual=windows::attach(handle,parent,target)?;
        *a.lock().unwrap()=Some((handle,parent));
        j.event("attached",json!({"window":actual,"parent":windows::record(parent),"target":target}));
        let open=MenuItem::with_id(app,"manager","Open Atmos manager",true,None::<&str>)?;
        let exit=MenuItem::with_id(app,"exit","Exit and restore desktop",true,None::<&str>)?;
        let menu=Menu::with_items(app,&[&open,&exit])?;
        let icon=tauri::image::Image::new_owned([32u8,165,185,255].repeat(32*32),32,32);
        TrayIconBuilder::new().icon(icon).tooltip("Atmos Desktop Prototype").menu(&menu).on_menu_event(move |app,event| {
            match event.id().as_ref() {
                "exit"=>app.exit(0),
                "manager"=>{
                    if let Some(w)=app.get_webview_window("manager") { let _=w.show();let _=w.set_focus(); }
                    else { let origin=format!("http://127.0.0.1:{port}");let check=origin.clone();let _=WebviewWindowBuilder::new(app,"manager",WebviewUrl::External(format!("{origin}/manager").parse().unwrap())).title("Atmos manager").inner_size(1100.0,800.0).on_navigation(move |u| u.origin().ascii_serialization()==check && ["/manager","/"].contains(&u.path()) && u.query().is_none()).build(); }
                },_=>{}
            }
        }).build(app)?;
        // Internal application lifecycle diagnostics, not UI automation. An ack
        // is accepted only from the unmodified, real wallpaper renderer.
        let application=app.handle().clone();let status=j.clone();let stop=quit.clone();let owned_runtime=rt.clone();let attachment=a.clone();
        std::thread::spawn(move || {
            let start=Instant::now();let mut first_ack=false;let mut last_ack=0u64;
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
                if start.elapsed().as_secs()>=duration { application.exit(0);break; }
                let dead=owned_runtime.lock().unwrap().as_mut().and_then(|r|r.child.try_wait().ok()).flatten().is_some();
                if dead {status.event("runtime-failed",json!({"action":"exit-and-restore"}));application.exit(2);break;}
                if let Some((h,p))=*attachment.lock().unwrap() {
                    if !windows::still_attached(h,p,target) {
                        let app2=application.clone();let status2=status.clone();let state=attachment.clone();
                        let _=application.run_on_main_thread(move || {
                            let result=windows::desktop_parent().and_then(|new_parent|windows::attach(h,new_parent,target).map(|r|(new_parent,r)));
                            match result { Ok((parent,record))=>{*state.lock().unwrap()=Some((h,parent));status2.event("reattached",json!(record));},Err(e)=>{status2.event("attachment-failed",json!({"error":e}));app2.exit(2);} }
                        });
                    }
                }
                if let Ok(bytes)=std::fs::read(store.join("observations/latest.json")) {
                    if let Ok(v)=serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if v["rendererRole"]=="wallpaper" && v["stateApplied"]==true {
                            if let Some(at)=v["capturedAt"].as_str() {
                                // Freshness is measured by observation file modification
                                // time, not a forged native ack or a manager ack.
                                let fresh=std::fs::metadata(store.join("observations/latest.json")).and_then(|m|m.modified()).ok().and_then(|t|t.elapsed().ok()).is_some_and(|d|d.as_secs()<5);
                                if fresh { last_ack=start.elapsed().as_secs(); if !first_ack {first_ack=true;status.event("renderer-ready",json!({"revision":v["revision"],"capturedAt":at,"trust":"renderer-reported; native attachment checked separately"}));} }
                            }
                        }
                    }
                }
                if (!first_ack && start.elapsed().as_secs()>30) || (first_ack && start.elapsed().as_secs().saturating_sub(last_ack)>30) {
                    status.event("renderer-timeout",json!({"action":"exit-and-restore"}));application.exit(2);break;
                }
            }
        });
        let application=app.handle().clone();
        std::thread::spawn(move || { for line in io::stdin().lock().lines().map_while(Result::ok) { if line.trim()=="exit" {application.exit(0);break;} } });
        Ok(())
    }).build(tauri::generate_context!()).map_err(|e|e.to_string())?;
    let j = journal.clone();
    let rt = runtime.clone();
    let a = attached.clone();
    let quit = exiting.clone();
    let intended_code = Arc::new(AtomicI32::new(0));
    let exit_code = intended_code.clone();
    let actual_code = app.run_return(move |app, event| {
        match event {
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if !quit.swap(true, Ordering::SeqCst) {
                    // Let Tauri/Tao destroy its own windows while its event
                    // loop is alive. Raw DestroyWindow invalidates Tao's state.
                    api.prevent_exit();
                    exit_code.store(code.unwrap_or(0), Ordering::SeqCst);
                    rt.lock().unwrap().take();
                    if let Some(manager) = app.get_webview_window("manager") {
                        let _ = manager.destroy();
                    }
                    if let Some(wallpaper) = app.get_webview_window("wallpaper") {
                        if let Err(e) = wallpaper.destroy() {
                            j.event("close-failed", json!({"error":e.to_string()}));
                            app.exit(2);
                        }
                    } else {
                        app.exit(exit_code.load(Ordering::SeqCst));
                    }
                }
            }
            tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } if label == "wallpaper" && quit.load(Ordering::SeqCst) => {
                app.exit(exit_code.load(Ordering::SeqCst));
            }
            _ => {}
        }
    });
    let pair = a.lock().unwrap().take();
    journal.event(
        "restored",
        json!({
            "ownWindowRemoved":pair.is_none_or(|(h,_)|!windows::exists(h)),
            "staticWallpaperUnchanged":windows::static_wallpaper().ok().as_ref()==Some(&original),
            "method":"remove-owned-layer; original wallpaper was never overwritten"
        }),
    );
    Ok(if intended_code.load(Ordering::SeqCst) != 0 {
        intended_code.load(Ordering::SeqCst)
    } else {
        actual_code
    })
}
