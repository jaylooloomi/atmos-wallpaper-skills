//! Job-owned native loopback worker shared by the host and isolated fixtures.
#[cfg(feature = "http-fixture")]
use crate::runtime::Runtime;
use crate::{
    backup_image::Decoder,
    displays::{self, Display},
    native_core_http::Bridge,
    native_http::{Assets, Limits, Server},
};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub(crate) fn stop_on_input(timeout: Option<Duration>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let eof = stop.clone();
    std::thread::spawn(move || {
        let _ = std::io::stdin().lock().read(&mut [0; 1]);
        eof.store(true, Ordering::Release);
    });
    if let Some(timeout) = timeout {
        let deadline = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            deadline.store(true, Ordering::Release);
        });
    }
    stop
}

pub(crate) fn serve(
    bridge: Bridge,
    server: Server,
    stop: Arc<AtomicBool>,
    assets: Assets,
    topology: impl Fn() -> Vec<Display> + Send + Sync + 'static,
) -> Result<(), String> {
    let bridge = Arc::new(bridge);
    let scheduler = bridge.clone();
    let (cancel, wake) = std::sync::mpsc::channel::<()>();
    let timer = std::thread::spawn(move || {
        scheduler.tick_feeds();
        let mut ticks = 0;
        while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) =
            wake.recv_timeout(Duration::from_secs(5))
        {
            scheduler.tick_schedule();
            ticks += 1;
            if ticks == 12 {
                scheduler.tick_feeds();
                ticks = 0;
            }
        }
    });
    let result = server
        .run(stop, move |security, req| {
            bridge.handle(security, req, &assets, &topology())
        })
        .map_err(|e| e.code.into());
    drop(cancel);
    timer.join().map_err(|_| "SCHEDULE_THREAD_FAILED")?;
    result
}

pub(crate) fn worker(args: &[OsString]) -> Result<(), String> {
    let [root, assets, prototype, python, decoder, session, rest @ ..] = args else {
        return Err("NATIVE_ARGUMENTS_INVALID".into());
    };
    let topology_file: Option<&Path> = match rest {
        [] => None,
        #[cfg(feature = "http-fixture")]
        [path] => Some(Path::new(path)),
        _ => return Err("NATIVE_ARGUMENTS_INVALID".into()),
    };
    let session = session
        .to_str()
        .filter(|s| {
            s.split_once('-').is_some_and(|(pid, time)| {
                pid.parse::<u32>().is_ok_and(|p| p > 0) && time.parse::<u64>().is_ok()
            })
        })
        .ok_or("HOST_SESSION_REQUIRED")?;
    // Do not read or write the store, bind a port or start any descendants
    // until the parent has assigned this process to its kill-on-close job.
    let mut start = [0; 6];
    if std::io::stdin().lock().read_exact(&mut start).is_err() || &start != b"start\n" {
        return Err("NATIVE_START_GATE_DENIED".into());
    }
    let root = Path::new(root);
    #[cfg(feature = "http-fixture")]
    if topology_file.is_some() {
        crate::core_http_fixture::prepare_store(root)?;
    }
    #[cfg(feature = "http-fixture")]
    let fixed = topology_file
        .map(crate::core_http_fixture::load_topology)
        .transpose()?;
    #[cfg(not(feature = "http-fixture"))]
    let fixed = None::<Vec<Display>>;
    let current = if let Some(fixed) = &fixed {
        fixed.clone()
    } else {
        displays::enumerate()?
    };
    let live = Mutex::new((Instant::now(), current));
    let topology = move || {
        if let Some(fixed) = &fixed {
            return fixed.clone();
        }
        let mut state = live.lock().unwrap_or_else(|e| e.into_inner());
        if state.0.elapsed() >= Duration::from_secs(1) {
            state.0 = Instant::now();
            // Stale topology must not authorize placement on an offline display.
            state.1 = displays::enumerate().unwrap_or_default();
        }
        state.1.clone()
    };
    let bridge = Bridge::open(
        root,
        Path::new(prototype),
        session.into(),
        Decoder {
            python: PathBuf::from(python),
            script: PathBuf::from(decoder),
        },
        topology_file.is_some(),
    )?;
    let assets = Assets::load(Path::new(assets)).map_err(|e| e.code)?;
    let server = Server::bind(Limits::default()).map_err(|e| e.code)?;
    let stop = stop_on_input(None);
    println!(
        "{}",
        serde_json::json!({"origin":server.security.origin,"sessionId":session})
    );
    std::io::stdout()
        .flush()
        .map_err(|_| "NATIVE_READY_FAILED")?;
    serve(bridge, server, stop, assets, topology)
}

#[cfg(feature = "http-fixture")]
pub(crate) fn host_fixture(args: &[OsString]) -> Result<(), String> {
    let [root, assets, prototype, topology, python, decoder] = args else {
        return Err("FIXTURE_ARGUMENTS_INVALID".into());
    };
    let root = Path::new(root);
    crate::core_http_fixture::prepare_store(root)?;
    let mut runtime = Runtime::start_native(
        root,
        Path::new(assets),
        Path::new(prototype),
        &Decoder {
            python: PathBuf::from(python),
            script: PathBuf::from(decoder),
        },
        Some(Path::new(topology)),
    )?;
    let stop = stop_on_input(Some(Duration::from_secs(45)));
    println!(
        "{}",
        serde_json::json!({"origin":format!("http://127.0.0.1:{}",runtime.port),
        "hostPid":std::process::id(),"workerPid":runtime.child.id(),"sessionId":runtime.session_id})
    );
    std::io::stdout()
        .flush()
        .map_err(|_| "NATIVE_READY_FAILED")?;
    while !stop.load(Ordering::Acquire) {
        if runtime
            .child
            .try_wait()
            .map_err(|_| "NATIVE_WORKER_MONITOR_FAILED")?
            .is_some()
        {
            return Err("NATIVE_WORKER_EXITED".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    runtime
        .stop_native()
        .then_some(())
        .ok_or_else(|| "NATIVE_WORKER_DID_NOT_EXIT_CLEANLY".into())
}

#[cfg(feature = "http-fixture")]
pub(crate) fn webview_fixture(args: &[OsString]) -> Result<(), String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let [root, origin] = args else {
        return Err("FIXTURE_ARGUMENTS_INVALID".into());
    };
    let root = PathBuf::from(root);
    crate::core_http_fixture::prepare_store(&root)?;
    let origin = origin
        .to_str()
        .and_then(|s| s.strip_prefix("http://127.0.0.1:"))
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|port| *port > 0)
        .map(|port| format!("http://127.0.0.1:{port}"))
        .ok_or("FIXTURE_ORIGIN_INVALID")?;
    tauri::Builder::default()
        .setup(move |app| {
            for label in ["manager", "wallpaper"] {
                let url = format!("{origin}/{label}").parse::<tauri::Url>()?;
                let expected = url.clone();
                let view = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
                    .visible(false)
                    .focused(false)
                    .inner_size(1024.0, 768.0)
                    .data_directory(root.join(format!("webview-{label}")))
                    .devtools(false)
                    .on_navigation(move |url| *url == expected);
                if label == "manager" {
                    view.initialization_script("window.__ATMOS_NATIVE_MANAGER__ = true;")
                        .build()?;
                } else {
                    view.build()?;
                }
            }
            let handle = app.handle().clone();
            let stop = stop_on_input(Some(Duration::from_secs(35)));
            std::thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(25));
                }
                handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| e.to_string())
}
