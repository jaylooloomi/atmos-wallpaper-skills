# Atmos Tauri 2 / Rust single-monitor prototype

Windows x64 only. The desktop host uses Tauri 2 + WebView2 and its own Windows
adapter, without calling or importing Lively. Existing HTML/CSS/JS and Node /
Python/Pillow control core are retained. macOS desktop integration, installers,
autostart, power-aware playback and production Explorer recovery are not yet
certified by this prototype.

Validated on Windows 10 x64 build 19045, primary 1920×1080: real attachment,
clean/timer/fault/forced termination cleanup, and user-confirmed visual
application/restoration. See [the validation report](docs/WINDOWS-PROTOTYPE-VALIDATION-2026-11-04.md)
for the initial white-desktop failure, repair and final retest boundaries.

## Build and apply

Requires Rust stable MSVC, Microsoft C++ Build Tools + Windows SDK, WebView2,
Node >=22 and Python with Pillow. `Cargo.lock` pins Rust dependencies. Run
`npm ci --ignore-scripts` in `implementation-p4_2_5` first. From repository root:

```powershell
.\desktop-host\scripts\run-prototype.ps1 -RunSeconds 600
```

Override `-Node` and `-Python` with absolute executable paths if needed. This
builds the host, prepares web assets and **applies to the primary monitor**.
The console remains attached. Type `exit` + Enter or use the tray's
**Exit and restore desktop**. The bounded prototype also exits automatically
(default 10 minutes, maximum 1 hour). `-SkipBuild` reuses a previous debug build.

The tray's **Open Atmos manager** opens the prototype's own management view.
Its separate settings store is at `%LOCALAPPDATA%\AtmosWallpaper\native-prototype\store`.
An already running manager, its settings/calendar and services are not adopted
or terminated. This is a source prototype; it still requires local Node and
Python and is not a self-contained installable product.

## Desktop and lifecycle contract

- Starts one frameless, non-activating wallpaper WebView on a supported Explorer
  background `WorkerW`. Verifies its actual native parent and physical monitor
  rectangle; refuses to use an ordinary top-level window as a fallback.
  Reuses a valid existing WorkerW before requesting a new Shell background layer.
- Never writes the Windows wallpaper registry or changes another application's
  window styles/visibility. Exit removes only the host's layer, revealing the
  original wallpaper. Existing dynamic wallpaper applications may remain below
  it; this does not test those applications' own recovery behavior.
- No Lively executable, CLI, IPC or code is used. WorkerW integration is an
  undocumented Shell behavior, not a stable Windows wallpaper API.
- Uses an application-owned loopback runtime on a dynamically selected port.
  A startup gate prevents it from serving before assignment to a kill-on-close
  Windows Job; closing/killing the host terminates that runtime tree.
- No native commands or filesystem/shell capabilities are granted to web pages.
  Top-level navigation is restricted to its own loopback paths. Wallpaper
  retains the existing read-only bridge role and manager remains a separate role.
- Diagnostics distinguish native attachment from genuine renderer-reported
  acknowledgements. Missing renderer acknowledgements or runtime failure cause
  an exit and restoration rather than a permanent blank wallpaper.
- A guard attempts reattachment when its parent/geometry is lost. Explorer
  restart, display hotplug, mixed DPI and actual suspend still require dedicated
  validation; automatic recovery is not a blanket compatibility guarantee.

## CLI and tests

```powershell
cargo test --locked --manifest-path desktop-host/Cargo.toml
cargo build --locked --manifest-path desktop-host/Cargo.toml
.\desktop-host\target\debug\atmos-desktop-prototype.exe --inspect-desktop
node desktop-host/scripts/test-windows-host.mjs --apply-to-primary
```

`--inspect-desktop` only reads native metadata and does not apply a wallpaper.
The integration script **does apply to the primary display**, exercises clean
exit, timer exit, backend failure and forced host termination, and checks real
WebView2 acknowledgements, native parent/bounds, process/port cleanup and
preservation of existing visible desktop windows. It does not operate a UI.
If an existing Atmos wallpaper should be monitored across the test, set
`ATMOS_BASELINE_PORT` to its loopback port. This observes its genuine read-only
health before/after each case; it never controls Lively or fabricates acks.

Native metadata and page acks alone **do not establish visual restoration**.
First confirm that the original wallpaper actually works, then observe the
post-exit desktop. An existing broken/paused third-party renderer cannot be
restored merely by removing Atmos's layer. Its repair belongs to that renderer;
the standalone host must not acquire a dependency on another wallpaper engine.

The host's diagnostic status JSON stays local and contains no bridge tokens.
Do not commit local profile data, calendar contents, runtime logs or absolute
wallpaper paths.
