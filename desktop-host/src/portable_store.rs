//! File/lock protocol shared with the existing JS Store. Never replace this with
//! a namespace-only mutex: HTTP/CLI/library writers must acquire the same .lock.
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER},
    Storage::FileSystem::{
        FileDispositionInfo, MoveFileExW, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    },
    System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
// node:path.win32.isAbsolute also accepts rooted paths and UNC. A parser may
// accept those before validate_root rejects them for actual store access.
pub fn node_absolute(s: &str) -> bool {
    let b = s.as_bytes();
    b.first().is_some_and(|c| b"\\/".contains(c))
        || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b"\\/".contains(&b[2]))
}
pub fn safe_uint(value: &Value) -> Option<u64> {
    // JSON.parse represents 1, 1.0 and 1e0 identically. Policy fields use only
    // nonnegative safe integers; normalize before hashing or writing them.
    let n = value.as_f64()?;
    (n.is_finite() && n >= 0.0 && n <= MAX_SAFE_INTEGER as f64 && n.fract() == 0.0)
        .then_some(n as u64)
}
fn component(part: &str) -> bool {
    if part.is_empty()
        || part == "."
        || part == ".."
        || part.ends_with(['.', ' '])
        || part
            .chars()
            .any(|c| c <= '\u{1f}' || "<>:\"|?*\\/".contains(c))
    {
        return false;
    }
    let base = part.split('.').next().unwrap().to_uppercase();
    !["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit()
            && base.as_bytes()[3] != b'0')
        && !["COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³"].contains(&base.as_str())
}
pub fn validate_root(root: &Path) -> Result<(), String> {
    let s = root.to_str().ok_or("PATH_REJECTED")?;
    let b = s.as_bytes();
    if b.len() < 3
        || !b[0].is_ascii_alphabetic()
        || b[1] != b':'
        || !b"\\/".contains(&b[2])
        || !root.is_absolute()
    {
        return Err("PATH_REJECTED".into());
    }
    let tail = s[3..].replace('\\', "/");
    let tail = tail.strip_suffix('/').unwrap_or(&tail);
    if !tail.is_empty() && !tail.split('/').all(component) {
        return Err("PATH_REJECTED".into());
    }
    no_links(root)
}
pub fn no_links(path: &Path) -> Result<(), String> {
    for p in path.ancestors() {
        match fs::symlink_metadata(p) {
            Ok(m) if m.file_attributes() & 0x400 != 0 => return Err("PATH_REJECTED".into()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("PATH_REJECTED".into()),
        }
    }
    Ok(())
}
fn io_code(error: std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        _ => "LOCAL_OPERATION_FAILED",
    }
    .into()
}
pub struct Store {
    pub root: PathBuf,
}
pub struct Lock {
    file: PathBuf,
}
fn open_owner_record(file: &Path) -> Result<fs::File, String> {
    // Sharing DELETE is essential: inspecting a live owner must not stop its
    // JS finally/unlink from releasing the lock. Stale removal below targets
    // this exact handle, so a replacement pathname is never removed.
    OpenOptions::new().read(true).access_mode(0x80000000 | 0x00010000)
        .share_mode(1 | 2 | 4).custom_flags(0x00200000).open(file)
        .map_err(|_| "STORE_LOCKED".into())
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

impl Store {
    pub fn new(root: &Path) -> Result<Self, String> {
        validate_root(root)?;
        fs::create_dir_all(root).map_err(io_code)?;
        no_links(root)?;
        let store = Self {
            root: root.canonicalize().map_err(io_code)?,
        };
        for name in [
            "imports",
            "assets",
            "previews",
            "requests",
            "exports",
            "observations",
            "commit-intents",
            "recovery",
            "backups",
        ] {
            let p = store.path(name)?;
            if fs::metadata(&p).is_ok_and(|m| !m.is_dir()) {
                return Err("PATH_REJECTED".into());
            }
            fs::create_dir_all(&p).map_err(io_code)?;
            no_links(&p)?;
        }
        Ok(store)
    }
    pub fn path(&self, name: &str) -> Result<PathBuf, String> {
        if !name.split('/').all(component) {
            return Err("PATH_REJECTED".into());
        }
        let p = self.root.join(name);
        no_links(&p)?;
        Ok(p)
    }
    pub fn read_bytes(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        let p = self.path(name)?;
        let mut f = match fs::File::open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_code(e)),
        };
        let metadata = f.metadata().map_err(io_code)?;
        if !metadata.is_file() || metadata.len() > 16 * 1048576 {
            return Err("CORRUPT_STORE".into());
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut f)
            .take(16 * 1048576 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_code)?;
        if bytes.len() > 16 * 1048576 {
            return Err("CORRUPT_STORE".into());
        }
        Ok(Some(bytes))
    }
    pub fn read_optional(&self, name: &str) -> Result<Option<Value>, String> {
        self.read_bytes(name)?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| "CORRUPT_STORE".into()))
            .transpose()
    }
    pub fn atomic(&self, name: &str, value: &Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| "LOCAL_OPERATION_FAILED")?;
        bytes.push(b'\n');
        self.atomic_bytes(name, &bytes)
    }
    pub fn atomic_bytes(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        atomic_file(&self.path(name)?, bytes)
    }
    pub fn external_root(&self) -> String {
        external_path(&self.root)
    }
    pub fn lock(&self) -> Result<Lock, String> {
        let file = self.path(".lock")?;
        for _ in 0..8 {
            match OpenOptions::new().write(true).create_new(true).open(&file) {
                Ok(mut f) => {
                    let guard = Lock { file: file.clone() };
                    f.write_all(
                        serde_json::to_string(&json!({"pid":std::process::id()}))
                            .unwrap()
                            .as_bytes(),
                    )
                    .map_err(io_code)?;
                    drop(f);
                    return Ok(guard);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(io_code(e)),
            }
            no_links(&file)?;
            let mut old = open_owner_record(&file)?;
            let metadata=old.metadata().map_err(|_| "STORE_LOCKED")?;
            if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
                return Err("STORE_LOCKED".into());
            }
            let mut bytes = Vec::new();
            Read::by_ref(&mut old)
                .take(16385)
                .read_to_end(&mut bytes)
                .map_err(|_| "STORE_LOCKED")?;
            if bytes.len() > 16384 {
                return Err("STORE_LOCKED".into());
            }
            let v: Value = serde_json::from_slice(&bytes).map_err(|_| "STORE_LOCKED")?;
            let pid = safe_uint(&v["pid"])
                .filter(|p| *p > 0 && *p <= u32::MAX as u64)
                .ok_or("STORE_LOCKED")? as u32;
            if process_alive(pid) {
                return Err("STORE_LOCKED".into());
            }
            let info = FILE_DISPOSITION_INFO { DeleteFile: true };
            if unsafe {
                SetFileInformationByHandle(
                    old.as_raw_handle() as _,
                    FileDispositionInfo,
                    &info as *const _ as _,
                    std::mem::size_of_val(&info) as u32,
                )
            } == 0
            {
                return Err("STORE_LOCKED".into());
            }
            drop(old);
        }
        Err("STORE_LOCKED".into())
    }
}
pub fn process_alive(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return GetLastError() != ERROR_INVALID_PARAMETER;
        }
        let mut code = 259;
        let valid = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        !valid || code == 259
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_local_path_escapes_and_reserved_names() {
        for name in [
            "../escape",
            "C:/escape",
            "a\\b",
            "a//b",
            "NUL.json",
            "a.",
            "COM1/file",
        ] {
            assert!(!name.split('/').all(component), "{name}");
        }
        for p in [
            r"\\server\share",
            r"\\?\C:\store",
            r"C:\a\..\b",
            r"C:\CON\store",
            r"C:\store ",
        ] {
            assert!(validate_root(Path::new(p)).is_err(), "{p}");
        }
    }
    #[test]
    fn safe_integer_matches_json_number_semantics() {
        for text in ["1", "1.0", "1e0", "-0"] {
            assert!(safe_uint(&serde_json::from_str(text).unwrap()).is_some());
        }
        for text in ["-1", "1.5", "9007199254740992", "null", "true"] {
            assert!(safe_uint(&serde_json::from_str(text).unwrap()).is_none());
        }
    }
    #[test]
    fn atomic_replace_retries_transient_windows_readers_but_stays_bounded() {
        let root=std::env::temp_dir().join(format!("atmos-atomic-busy-{}-{}",std::process::id(),crate::journal::now_ms()));
        fs::create_dir_all(&root).unwrap();let dest=root.join("settings.json");fs::write(&dest,b"old").unwrap();
        let held=OpenOptions::new().read(true).share_mode(1).open(&dest).unwrap();
        let release=std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_millis(20));drop(held)});
        atomic_file(&dest,b"new").unwrap();release.join().unwrap();assert_eq!(fs::read(&dest).unwrap(),b"new");
        let held=OpenOptions::new().read(true).share_mode(1).open(&dest).unwrap();let start=std::time::Instant::now();assert!(atomic_file(&dest,b"must-not-appear").is_err());assert!(start.elapsed()<std::time::Duration::from_secs(1));assert_eq!(fs::read(&dest).unwrap(),b"new");drop(held);
        assert_eq!(fs::read_dir(&root).unwrap().count(),1);fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn inspecting_owner_does_not_block_legacy_lock_release() {
        let root=std::env::temp_dir().join(format!("atmos-owner-sharing-{}-{}",std::process::id(),crate::journal::now_ms()));
        let store=Store::new(&root).unwrap();let lock=store.path(".lock").unwrap();fs::write(&lock,format!("{{\"pid\":{}}}",std::process::id())).unwrap();
        let inspecting=open_owner_record(&lock).unwrap();fs::remove_file(&lock).expect("a live JS owner must still be able to unlink its own lock");drop(inspecting);
        drop(store.lock().unwrap());fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn shared_lock_atomic_write_and_stale_recovery() {
        let root = std::env::temp_dir().join(format!(
            "atmos-native-store-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        let store = Store::new(&root).unwrap();
        let guard = store.lock().unwrap();
        assert!(store.lock().is_err());
        store
            .atomic("requests/sample.json", &json!({"revision":1}))
            .unwrap();
        assert_eq!(
            store
                .read_optional("requests/sample.json")
                .unwrap()
                .unwrap()["revision"],
            1
        );
        drop(guard);
        assert!(!root.join(".lock").exists());
        // PID 0 is invalid and must never be treated as a recoverable dead owner.
        fs::write(root.join(".lock"), b"{\"pid\":0}").unwrap();
        assert!(store.lock().is_err());
        fs::remove_file(root.join(".lock")).unwrap();
        let mut child = std::process::Command::new(r"C:\Windows\System32\cmd.exe")
            .args(["/c", "exit", "0"])
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        fs::write(root.join(".lock"), format!("{{\"pid\":{pid}}}")).unwrap();
        drop(store.lock().unwrap());
        assert!(!root.join(".lock").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

pub fn external_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}
pub fn atomic_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write(dest, bytes, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
}
#[cfg(feature = "native-core")]
pub(crate) fn atomic_new_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write(dest, bytes, MOVEFILE_WRITE_THROUGH)
}
fn atomic_write(dest: &Path, bytes: &[u8], flags: u32) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let parent = dest.parent().ok_or("PATH_REJECTED")?;
    let temp = parent.join(format!(
        ".tmp-native-{}-{}-{}",
        std::process::id(),
        crate::journal::now_ms(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    no_links(&temp)?;
    let mut owned = false;
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(io_code)?;
        owned = true;
        f.write_all(bytes).map_err(io_code)?;
        f.sync_all().map_err(io_code)?;
        drop(f);
        no_links(&dest)?;
        let wide = |p: &Path| {
            p.as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain([0])
                .collect::<Vec<_>>()
        };
        // Windows readers/virus scanners can briefly deny replacement (5/32/33).
        // Retry only this exact temp -> destination rename while the caller retains
        // its transaction lock. Never restart the transaction or claim success on error.
        let delays = [5, 15, 40];
        for attempt in 0..=delays.len() {
            no_links(dest)?;
            if unsafe { MoveFileExW(wide(&temp).as_ptr(), wide(dest).as_ptr(), flags) } != 0 { return Ok(()); }
            let code = unsafe { GetLastError() };
            if attempt == delays.len() || ![5,32,33].contains(&code) { return Err("LOCAL_OPERATION_FAILED".into()); }
            std::thread::sleep(std::time::Duration::from_millis(delays[attempt]));
        }
        unreachable!("bounded replacement loop returns")
    })();
    if result.is_err() && owned {
        let _ = fs::remove_file(temp);
    }
    result
}
