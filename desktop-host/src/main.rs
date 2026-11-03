#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(windows)]
mod app;
#[cfg(windows)]
mod control;
#[cfg(windows)]
mod installed;
#[cfg(windows)]
mod journal;
mod options;
#[cfg(windows)]
mod runtime;
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
    if args == ["--help"] {
        println!("Atmos Wallpaper\nNo arguments: verify bundled resources, launch embedded runtimes and apply to the primary monitor until tray Exit.\n--quit: close only this executable's instance (upgrade/uninstall).\n--manager: open the running manager, or start and open it.\n--check-install: verify payload and embedded runtimes without applying wallpaper.\n--inspect-desktop: read native desktop metadata.\nDeveloper mode: --runtime-root DIR --node EXE --python EXE --store DIR --status FILE [--run-seconds 5..3600].\nInstalled mode also accepts --store, --status and --run-seconds for diagnostics.");
        return Ok(0);
    }
    if args == ["--quit"] {
        control::quit()?;
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
    let args = if manager { Vec::new() } else { args };
    let _singleton = match windows::singleton() {
        Ok(handle) => handle,
        Err(error) => {
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
    app::launch(options, quit_event.0, manager_event.0, manager)
}
