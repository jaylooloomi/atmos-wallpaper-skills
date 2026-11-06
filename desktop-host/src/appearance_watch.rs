//! Native, opt-in JSONL file watcher. Submits only the stable bytes just read;
//! no recursive watch, revision rewriting, desktop launch or Node child process.
use crate::{
    appearance_agent::{self, Options, Routing},
    appearance_core::Core,
    asset_store,
    js_json::{self, object as obj, Value},
    portable_store,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use windows_sys::Win32::{
    Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    System::Console::SetConsoleCtrlHandler,
};
static STOP: AtomicBool = AtomicBool::new(false);
unsafe extern "system" fn control(kind: u32) -> i32 {
    if kind == 0 || kind == 1 {
        STOP.store(true, Ordering::Relaxed);
        1
    } else {
        0
    }
}
struct SignalGuard(bool);
impl SignalGuard {
    fn new() -> Self {
        STOP.store(false, Ordering::Relaxed);
        Self(unsafe { SetConsoleCtrlHandler(Some(control), 1) } != 0)
    }
}
impl Drop for SignalGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                SetConsoleCtrlHandler(Some(control), 0);
            }
        }
    }
}
fn emit(value: Value) -> Result<(), String> {
    let mut out = std::io::stdout().lock();
    out.write_all(value.canonical().as_bytes())
        .and_then(|_| out.write_all(b"\n"))
        .and_then(|_| out.flush())
        .map_err(|_| "LOCAL_OPERATION_FAILED".into())
}
fn path_valid(file: &Path) -> Result<(), String> {
    let s = file.to_str().ok_or("INPUT_PATH_INVALID")?;
    let b = s.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || !b"\\/".contains(&b[2]) {
        return Err("INPUT_PATH_INVALID".into());
    }
    Ok(())
}
fn outside(file: &Path, store: &Path) -> Result<(), String> {
    let real = file.canonicalize().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "ENOENT"
        } else {
            "LOCAL_OPERATION_FAILED"
        }
    })?;
    let root = store.canonicalize().map_err(|_| "LOCAL_OPERATION_FAILED")?;
    let real = portable_store::external_path(&real).to_lowercase();
    let root = portable_store::external_path(&root).to_lowercase();
    if real == root || real.starts_with(&(root.trim_end_matches('\\').to_string() + "\\")) {
        return Err("INTERNAL_STORE_INPUT_DENIED".into());
    }
    Ok(())
}
fn info(f: &File) -> Result<[u64; 6], String> {
    let mut v: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as _, &mut v) } == 0 {
        return Err("INPUT_CHANGING".into());
    }
    Ok([
        v.dwVolumeSerialNumber as u64,
        ((v.nFileIndexHigh as u64) << 32) | v.nFileIndexLow as u64,
        ((v.nFileSizeHigh as u64) << 32) | v.nFileSizeLow as u64,
        ((v.ftLastWriteTime.dwHighDateTime as u64) << 32) | v.ftLastWriteTime.dwLowDateTime as u64,
        ((v.ftCreationTime.dwHighDateTime as u64) << 32) | v.ftCreationTime.dwLowDateTime as u64,
        v.dwFileAttributes as u64,
    ])
}
fn open(file: &Path) -> Result<File, String> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(0x00200000)
        .open(file)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "ENOENT"
            } else {
                "INPUT_CHANGING"
            }
        })?;
    let m = f.metadata().map_err(|_| "INPUT_NOT_REGULAR")?;
    if !m.is_file() || m.file_attributes() & 0x400 != 0 {
        return Err("INPUT_NOT_REGULAR".into());
    }
    Ok(f)
}
struct Snapshot {
    bytes: Vec<u8>,
    key: String,
}
fn snapshot(file: &Path, root: &Path) -> Result<Snapshot, String> {
    path_valid(file)?;
    outside(file, root)?;
    let m = fs::symlink_metadata(file).map_err(|_| "ENOENT")?;
    if !m.is_file() || m.file_attributes() & 0x400 != 0 {
        return Err("INPUT_NOT_REGULAR".into());
    }
    let mut f = open(file)?;
    let before = info(&f)?;
    if before[2] > 32768 {
        return Err("BODY_TOO_LARGE".into());
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut f)
        .take(32769)
        .read_to_end(&mut bytes)
        .map_err(|_| "INPUT_CHANGING")?;
    if bytes.len() > 32768 {
        return Err("BODY_TOO_LARGE".into());
    }
    let after = info(&f)?;
    let current = info(&open(file)?)?;
    if before != after || after != current {
        return Err("INPUT_CHANGING".into());
    }
    Ok(Snapshot {
        key: asset_store::hash_bytes(&bytes),
        bytes,
    })
}
fn input_error(code: &str) -> Value {
    obj([
        ("code", code.into()),
        (
            "retryable",
            ["ENOENT", "INPUT_CHANGING"].contains(&code).into(),
        ),
    ])
}
struct Candidate {
    snapshot: Snapshot,
    since: u64,
}
struct Watcher {
    candidate: Option<Candidate>,
    attempted: Option<String>,
    read_error: Option<String>,
    retry: u32,
    retry_at: u64,
}
impl Watcher {
    fn new() -> Self {
        Self {
            candidate: None,
            attempted: None,
            read_error: None,
            retry: 0,
            retry_at: 0,
        }
    }
    fn poll(
        &mut self,
        options: &Options,
        core: &Core,
        routing: &Routing,
        initial_layout: &Value,
        now: u64,
    ) -> Result<(), String> {
        let file = Path::new(options.value.as_ref().unwrap());
        let snapshot = match snapshot(file, &options.root) {
            Ok(s) => {
                self.read_error = None;
                s
            }
            Err(code) => {
                self.candidate = None;
                if self.read_error.as_deref() != Some(&code) {
                    emit(obj([
                        ("apiVersion", 1.into()),
                        ("event", "input-error".into()),
                        ("ok", false.into()),
                        ("error", input_error(&code)),
                        ("committed", false.into()),
                    ]))?;
                    self.read_error = Some(code);
                }
                return Ok(());
            }
        };
        if self
            .candidate
            .as_ref()
            .is_none_or(|c| c.snapshot.key != snapshot.key)
        {
            self.candidate = Some(Candidate {
                snapshot,
                since: now,
            });
            self.retry = 0;
            self.retry_at = 0;
            return Ok(());
        }
        if now.saturating_sub(self.candidate.as_ref().unwrap().since) < options.settle
            || self.attempted.as_deref() == Some(&snapshot.key)
            || now < self.retry_at
        {
            return Ok(());
        }
        let command = match js_json::parse(
            snapshot
                .bytes
                .strip_prefix(&[0xef, 0xbb, 0xbf])
                .unwrap_or(&snapshot.bytes),
        ) {
            Ok(v) => v,
            Err(_) => {
                self.attempted = Some(snapshot.key.clone());
                emit(obj([
                    ("apiVersion", 1.into()),
                    ("event", "input-error".into()),
                    ("inputHash", snapshot.key.into()),
                    ("ok", false.into()),
                    (
                        "error",
                        obj([("code", "INVALID_JSON".into()), ("retryable", false.into())]),
                    ),
                    ("committed", false.into()),
                ]))?;
                return Ok(());
            }
        };
        let result = (|| {
            if appearance_agent::layout(core)? != *initial_layout {
                return Err("TARGET_SCOPE_CHANGED".into());
            }
            Core::command_patch(&command)?;
            let result = core.execute_command(&command)?;
            let mut state = appearance_agent::status(core, command["requestId"].as_str(), routing)?;
            state.set(
                "replayed",
                (result["replayed"].as_bool() == Some(true)).into(),
            );
            state.set("waitedMs", 0.into());
            Ok(state)
        })();
        match result {
            Ok(mut value) => {
                self.attempted = Some(snapshot.key.clone());
                self.retry = 0;
                value.set("event", "result".into());
                value.set("inputHash", snapshot.key.into());
                emit(value)?;
            }
            Err(code) => {
                let code: String = code;
                if code == "STORE_LOCKED" && self.retry < 3 {
                    self.retry_at = now + 250 * (1 << self.retry);
                    self.retry += 1;
                    return Ok(());
                }
                self.attempted = Some(snapshot.key.clone());
                emit(obj([("apiVersion",1.into()),("event","command-error".into()),("inputHash",snapshot.key.into()),("ok",false.into()),("error",obj([("code",code.clone().into()),("retryable",(code=="STORE_LOCKED").into())])),("committed",Value::Null),("note","Inspect receipt after an unknown outcome. Conflicts require a new reconciled intent; the watcher never rewrites revisions.".into())]))?;
            }
        }
        Ok(())
    }
}
pub(crate) fn run(
    options: &Options,
    core: &Core,
    routing: &Routing,
    initial_layout: &Value,
) -> Result<Value, String> {
    let file = Path::new(options.value.as_ref().unwrap());
    path_valid(file)?;
    outside(file.parent().ok_or("INPUT_PATH_INVALID")?, &options.root)?;
    let _signals = SignalGuard::new();
    let started = crate::journal::now_ms();
    let mut watcher = Watcher::new();
    emit(obj([
        ("apiVersion", 1.into()),
        ("event", "watch-started".into()),
        ("ok", true.into()),
        ("pollMs", options.poll.into()),
        ("settleMs", options.settle.into()),
        ("durationMs", options.duration.into()),
        ("autoStart", false.into()),
    ]))?;
    loop {
        let now = crate::journal::now_ms();
        let elapsed = now.saturating_sub(started);
        if STOP.load(Ordering::Relaxed) || (options.duration > 0 && elapsed >= options.duration) {
            break;
        }
        watcher.poll(options, core, routing, initial_layout, now)?;
        let remaining = if options.duration > 0 {
            options
                .duration
                .saturating_sub(crate::journal::now_ms().saturating_sub(started))
        } else {
            options.poll
        };
        let pause = options.poll.min(remaining);
        let mut slept = 0;
        while slept < pause && !STOP.load(Ordering::Relaxed) {
            let step = 50.min(pause - slept);
            std::thread::sleep(Duration::from_millis(step));
            slept += step;
        }
    }
    Ok(obj([
        ("apiVersion", 1.into()),
        ("event", "watch-stopped".into()),
        ("ok", true.into()),
        (
            "reason",
            if STOP.load(Ordering::Relaxed) {
                "signal"
            } else {
                "duration"
            }
            .into(),
        ),
        (
            "elapsedMs",
            crate::journal::now_ms().saturating_sub(started).into(),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn busy_retry_contract_is_three_backoffs_then_one_terminal_error() {
        let base = std::env::temp_dir().join(format!(
            "atmos-watch-backoff-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        fs::create_dir_all(&base).unwrap();
        let file = base.join("input.json");
        let root = base.join("store");
        fs::write(&file,br#"{"schemaVersion":1,"type":"settings.update","requestId":"backoff","expectedRevision":0,"changes":{"visual":{"effect":"rain"}}}"#).unwrap();
        let args = vec![
            "--watch-json".into(),
            file.to_string_lossy().to_string(),
            "--store".into(),
            root.to_string_lossy().to_string(),
        ];
        let options = appearance_agent::parse(&args).unwrap();
        let (core, routing, layout) = appearance_agent::prepare(&options).unwrap();
        let held = core.lock().unwrap();
        let mut watcher = Watcher::new();
        watcher.poll(&options, &core, &routing, &layout, 0).unwrap();
        for (now, retry, next) in [(750, 1, 1000), (1000, 2, 1500), (1500, 3, 2500)] {
            watcher
                .poll(&options, &core, &routing, &layout, now)
                .unwrap();
            assert_eq!(watcher.retry, retry);
            assert_eq!(watcher.retry_at, next);
            assert!(watcher.attempted.is_none());
        }
        watcher
            .poll(&options, &core, &routing, &layout, 2500)
            .unwrap();
        assert!(watcher.attempted.is_some());
        watcher
            .poll(&options, &core, &routing, &layout, 10000)
            .unwrap();
        assert_eq!(watcher.retry, 3);
        drop(held);
        fs::remove_dir_all(base).unwrap();
    }
}
