#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(windows)]
mod agent;
#[cfg(windows)]
mod app;
#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod audio_diagnostics;
#[cfg(windows)]
mod audio_routing;
#[cfg(windows)]
mod control;
#[cfg(windows)]
mod displays;
#[cfg(windows)]
mod host_settings;
#[cfg(windows)]
mod installed;
#[cfg(windows)]
mod journal;
#[cfg(windows)]
mod layout;
#[cfg(windows)]
mod library;
#[cfg(windows)]
mod multi;
mod options;
#[cfg(windows)]
mod performance;
#[cfg(windows)]
mod policy_store;
#[cfg(windows)]
mod runtime;
#[cfg(windows)]
mod renderer_memory;
#[cfg(windows)]
mod screensaver;
#[cfg(windows)]
mod screensaver_registration;
#[cfg(windows)]
mod startup;
#[cfg(windows)]
mod telemetry;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
fn main() {
    eprintln!("The macOS desktop adapter is not implemented.");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("Atmos: {error}");
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                use std::io::Write;
                let dir = std::path::PathBuf::from(local).join("io.atmos.wallpaper");
                let _ = std::fs::create_dir_all(&dir);
                if let Ok(mut log) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("startup-errors.log"))
                {
                    let _ = writeln!(log, "{} {error}", journal::now_ms());
                }
            }
            // Only direct user launches get a dialog; CLI checks never do.
            if std::env::args().len() == 1 {
                windows::show_error(&error);
            }
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn run() -> Result<i32, String> {
    use serde_json::json;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--screensaver-unregister"] {
        return screensaver_registration::unregister();
    }
    if args == ["--screensaver-install"] {
        return screensaver::install();
    }
    if args == ["--screensaver-settings"] {
        return screensaver::open_settings();
    }
    if args == ["--screensaver-stop"] {
        return screensaver::stop();
    }
    if args.first().is_some_and(|a| a == "--screensaver-command") {
        return match screensaver::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(code) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
                );
                Ok(2)
            }
        };
    }
    let scr = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.extension()
                .map(|s| s.to_string_lossy().eq_ignore_ascii_case("scr"))
        })
        .unwrap_or(false);
    if (scr && args.is_empty())
        || args.first().is_some_and(|a| {
            let a = a.to_ascii_lowercase();
            a == "/s" || a == "/c" || a == "/p" || a.starts_with("/p:") || a.starts_with("/c:")
        })
    {
        return screensaver::run(args);
    }
    if args == ["--system-info"] {
        return telemetry::command();
    }
    if args.first().is_some_and(|a| a == "--performance-command") {
        return match performance::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(code) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
                );
                Ok(2)
            }
        };
    }
    if args.first().is_some_and(|a| a == "--audio-routing-command") {
        return match audio_routing::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(code) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
                );
                Ok(2)
            }
        };
    }
    if args.first().is_some_and(|a| a == "--layout-command") {
        return match layout::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(code) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
                );
                Ok(2)
            }
        };
    }
    if args.first().is_some_and(|a| a == "--audio-command") {
        return match audio::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(code) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":code}})
                );
                Ok(2)
            }
        };
    }
    if args == ["--audio-help"] {
        println!(
            "{}",
            json!({"schemaVersion":1,"commands":["--audio-command get [--store DIR]","--audio-command update [--store DIR] (JSON stdin)","--audio-command probe (silent selected-device stream test)","--audio-command diagnose (read-only default-endpoint and own-session level)","--audio-command test (3 seconds; JSON schemaVersion and expectedRevision)"],"example":{"schemaVersion":1,"requestId":"audio.update.001","expectedRevision":0,"audio":{"enabled":false,"muted":true,"volume":50,"focusOnly":false,"source":"weather","output":"system-default"}},"rules":["Separate audio revision/CAS and requestId; read current state first. Replay identical intent only; reconcile conflicts.","All policy fields are required; source is weather, rain or thunder; output is system-default or an exact listed device ID. Missing chosen devices never fall back.","Audio is generated by the native host. No microphone or global volume changes. Preview stays silent.","Weather source is silent for clear and snow. Rain/thunder sources explicitly select sound independent of visual weather.","Only desktop/desktop-icons count as desktop focus, never manager or a folder window.","Missing/unknown input desktop, hidden wallpaper, invalid configuration or device failure mutes/stops sound.","Stream/policy status is not human audibility proof. probe sends only digital silence."]})
        );
        return Ok(0);
    }
    if args.first().is_some_and(|a| a == "--library-folder") {
        return library::folder(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--library-command") {
        return library::command(args);
    }
    if args == ["--host-help"] {
        println!(
            "{}",
            json!({"schemaVersion":1,"commands":{"--list-displays":"Read physical bounds, DPI, rotation and connection identity; no wallpaper change.","--host-command get [--store ABSOLUTE_LOCAL_DIRECTORY]":"Read host settings, display topology and runtime status.","--host-command update [--store ABSOLUTE_LOCAL_DIRECTORY]":"Read one bounded JSON update from stdin. Strict CAS; no arbitrary paths or shell."},"example":{"schemaVersion":1,"expectedRevision":0,"placement":{"enabled":true,"target":"primary"}},"rules":["Read current settings revision immediately before updating.","Use primary to follow primary; otherwise use an exact selectable ID from --list-displays. IDs are connection-specific; never guess or fuzzy-match a missing display.","A conflict requires rereading and reconciliation, not blind retry.","A successful update proves saved configuration only. Poll get until runtimeFresh and nativeAttached with matching settingsRevision; renderer ack and actual visibility are separate.","enabled=false removes only the Atmos visible layer and retains its target.","Do not edit host-settings.json or live.json directly. Appearance Agent API v1 is unchanged.","Host settings use a separate revision from appearance. No automatic cross-screen fallback."],"limits":{"inputBytes":16384},"exitCodes":{"0":"completed","2":"invalid, conflict, busy or I/O error; inspect error.code"}})
        );
        return Ok(0);
    }
    if args == ["--list-displays"] {
        println!(
            "{}",
            serde_json::to_string(&displays::enumerate()?).unwrap()
        );
        return Ok(0);
    }
    if args.first().is_some_and(|a| a == "--host-command") {
        return match host_settings::command(&args[1..]) {
            Ok(code) => Ok(code),
            Err(error) => {
                println!(
                    "{}",
                    json!({"schemaVersion":1,"ok":false,"error":{"code":error}})
                );
                Ok(2)
            }
        };
    }
    if args.first().is_some_and(|v| agent::is_action(v)) {
        return agent::run(args);
    }
    if args == ["--help"] {
        println!("Atmos Wallpaper\nLibrary: --library-command help|get|move|reset|audit|archive|result [--store DIR]; move/reset/result read JSON on stdin.\nHost configuration: --host-help, --list-displays, --host-command get|update [--store DIR] (update JSON on stdin).\nAgent JSON API: --agent-help, --agent-schema, --get-state, --validate-json FILE, --apply-json FILE [--wait-ms 0..30000], --get-result REQUEST_ID [--wait-ms 0..30000].\nMachine schemas: --host-schema. Provider configuration: --providers-command get|update (global owner; no --target).\nAdditional APIs: --layout-command get|update, --audio-command get|update|probe|diagnose|test, --audio-routing-command get|update, --performance-command get|update, --screensaver-command get|update, --system-info.\nAppearance: --appearance-help, --preset-command get|save|apply|delete, --schedule-command get|update, --focus-command get|update.\nAgent watcher: --watch-json FILE [--target DISPLAY_ID] [--duration-ms N]; explicit opt-in, never watches live.json.\nScreensaver: Atmos.scr /s | /c | /p HWND; --screensaver-settings and --screensaver-install only on explicit user action.\nNo arguments: verify bundled resources, launch embedded runtimes and apply to the primary monitor until tray Exit.\n--autostart status|enable|disable: manage sign-in registration.\n--autostart-launch: background logon entry (waits for Explorer).\n--quit: close only this executable's instance (upgrade/uninstall).\n--manager: open the running manager, or start and open it.\n--check-install: verify payload and embedded runtimes without applying wallpaper.\n--inspect-desktop: read native desktop metadata.\nDeveloper mode: --runtime-root DIR --node EXE --python EXE --store DIR --status FILE [--run-seconds 5..3600].\nInstalled mode also accepts --store, --status and --run-seconds for diagnostics.");
        return Ok(0);
    }
    if args == ["--quit"] {
        control::quit()?;
        return Ok(0);
    }
    if args.len() == 2 && args[0] == "--autostart" {
        let result = match args[1].as_str() {
            "status" => startup::status()?,
            "enable" => startup::set(true)?,
            "disable" => startup::set(false)?,
            "initialize" => startup::initialize()?,
            "remove" => {
                startup::remove()?;
                startup::status()?
            }
            _ => return Err("Unknown startup action".into()),
        };
        println!("{}", serde_json::to_string(&result).unwrap());
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
    if args == ["--check-install"] {
        let root = installed::payload_root()?;
        let versions = installed::verify(&root)?;
        runtime::check_embedded(&root)?;
        println!(
            "{}",
            json!({"payloadVerified":true,"embeddedRuntimesVerified":true,"versions":versions,"externalNodePythonRequired":false})
        );
        return Ok(0);
    }
    let manager = args == ["--manager"];
    let autostart = args == ["--autostart-launch"];
    let args = if manager || autostart {
        Vec::new()
    } else {
        args
    };
    let isolated = args.iter().any(|a| a == "--runtime-root");
    let singleton = if isolated {
        use sha2::{Digest, Sha256};
        let index = args
            .iter()
            .position(|a| a == "--store")
            .ok_or("Explicit isolated store required")?;
        let root = std::path::PathBuf::from(args.get(index + 1).ok_or("Missing store")?);
        if !root.is_absolute() {
            return Err("Store must be absolute".into());
        }
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        windows::named_singleton(&format!(
            "Local\\AtmosWallpaperDesktop.Prototype.{:x}",
            Sha256::digest(root.to_string_lossy().to_lowercase().as_bytes())
        ))
    } else {
        windows::singleton()
    };
    let _singleton = match singleton {
        Ok(handle) => handle,
        Err(error) => {
            if autostart && error == "Atmos is already running" {
                return Ok(0);
            }
            if args.is_empty() && control::signal("manager")? {
                return Ok(0);
            }
            return Err(error);
        }
    };
    let quit_event = control::create("quit")?;
    let manager_event = control::create("manager")?;
    let options = installed::configuration(args)?;
    // Upgrade/uninstall can arrive while payload verification is in progress.
    // Publish the control event first, and honor it before starting a renderer.
    if control::requested(quit_event.0) {
        return Ok(0);
    }
    if autostart {
        let started = std::time::Instant::now();
        while !windows::shell_ready() {
            if control::requested(quit_event.0) {
                return Ok(0);
            }
            if started.elapsed() > std::time::Duration::from_secs(60) {
                return Err("Explorer desktop not ready after logon".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    app::launch(options, quit_event.0, manager_event.0, manager)
}
