//! Same-user, same-session, executable-specific control. No TCP write endpoint
//! and no generic process-name termination during upgrade/uninstall.
use crate::windows::OwnedHandle;
use sha2::{Digest, Sha256};
use std::{
    ptr::null,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_FILE_NOT_FOUND, WAIT_OBJECT_0},
    System::Threading::*,
};

fn name(action: &str) -> Result<Vec<u16>, String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let id = format!(
        "{:x}",
        Sha256::digest(exe.to_string_lossy().to_lowercase().as_bytes())
    );
    Ok(format!("Local\\AtmosWallpaper.{id}.{action}")
        .encode_utf16()
        .chain([0])
        .collect())
}
pub fn create(action: &str) -> Result<OwnedHandle, String> {
    unsafe {
        let h = CreateEventW(null(), 0, 0, name(action)?.as_ptr());
        if h.is_null() {
            return Err(format!("Create control event: {}", GetLastError()));
        }
        Ok(OwnedHandle(h as isize))
    }
}
pub fn requested(handle: isize) -> bool {
    unsafe { WaitForSingleObject(handle as _, 0) == WAIT_OBJECT_0 }
}
pub fn signal(action: &str) -> Result<bool, String> {
    unsafe {
        let h = OpenEventW(EVENT_MODIFY_STATE, 0, name(action)?.as_ptr());
        if h.is_null() {
            return if GetLastError() == ERROR_FILE_NOT_FOUND {
                Ok(false)
            } else {
                Err(format!("Open control event: {}", GetLastError()))
            };
        }
        let ok = SetEvent(h) != 0;
        CloseHandle(h);
        if !ok {
            return Err("Signal control event failed".into());
        }
        Ok(true)
    }
}
pub fn quit() -> Result<(), String> {
    if !signal("quit")? {
        return Ok(());
    }
    let start = Instant::now();
    loop {
        unsafe {
            let h = OpenEventW(EVENT_MODIFY_STATE, 0, name("quit")?.as_ptr());
            if h.is_null() {
                if GetLastError() == ERROR_FILE_NOT_FOUND {
                    return Ok(());
                }
                return Err("Could not verify app closure".into());
            }
            CloseHandle(h);
        }
        if start.elapsed() > Duration::from_secs(20) {
            return Err("Atmos did not close within 20 seconds".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
