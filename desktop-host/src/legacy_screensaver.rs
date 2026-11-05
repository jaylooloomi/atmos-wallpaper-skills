//! Retirement-only compatibility: stop an old owned saver; never create windows.
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Threading::OpenMutexW;
const ACTIVE: &str = "Local\\AtmosWallpaper.ScreensaverActive";
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
pub fn active() -> bool {
    unsafe {
        let h = OpenMutexW(0x00100000, 0, wide(ACTIVE).as_ptr());
        if h.is_null() {
            windows_sys::Win32::Foundation::GetLastError()
                != windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND
        } else {
            windows_sys::Win32::Foundation::CloseHandle(h);
            true
        }
    }
}

fn event_name() -> Result<Vec<u16>, String> {
    use sha2::{Digest, Sha256};
    let folder = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("SCREENSAVER_LOCATION_INVALID")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    Ok(wide(&format!(
        "Local\\AtmosWallpaper.ScreensaverQuit.{:x}",
        Sha256::digest(folder.to_string_lossy().to_lowercase().as_bytes())
    )))
}
pub fn stop() -> Result<i32, String> {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::OpenEventW(
            windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE,
            0,
            event_name()?.as_ptr(),
        );
        if handle.is_null() {
            if windows_sys::Win32::Foundation::GetLastError()
                == windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND
            {
                return Ok(0);
            }
            return Err("SCREENSAVER_EVENT_FAILED".into());
        }
        let signaled = windows_sys::Win32::System::Threading::SetEvent(handle) != 0;
        windows_sys::Win32::Foundation::CloseHandle(handle);
        if !signaled {
            return Err("SCREENSAVER_EVENT_FAILED".into());
        }
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        unsafe {
            let handle = windows_sys::Win32::System::Threading::OpenEventW(
                windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE,
                0,
                event_name()?.as_ptr(),
            );
            if handle.is_null() {
                return Ok(0);
            }
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("SCREENSAVER_STOP_TIMEOUT".into())
}

fn retired_paths(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    [
        "Atmos.scr",
        "payload/app/implementation-p4_2_5/dist/screensaver-client.js",
        "payload/app/implementation-p4_2_5/dist/screensaver-client.css",
        "payload/app/implementation-p4_2_5/src/screensaver-client.js",
        "payload/app/implementation-p4_2_5/src/screensaver-client.css",
        "payload/app/implementation-p4_2_5/src/screensaver-service.mjs",
    ]
    .iter()
    .map(|p| root.join(p))
    .collect()
}
pub fn retire() -> Result<i32, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let paths = retired_paths(exe.parent().ok_or("SCREENSAVER_LOCATION_INVALID")?);
    for file in &paths {
        crate::policy_store::safe_path(file)?;
        if file.exists() && !file.is_file() {
            return Err("LEGACY_SCREENSAVER_PATH_INVALID".into());
        }
    }
    stop()?;
    crate::legacy_screensaver_registration::unregister()?;
    for file in paths {
        if file.is_file() {
            std::fs::remove_file(&file).map_err(|_| "LEGACY_SCREENSAVER_REMOVE_FAILED")?;
        }
    }
    Ok(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_paths_are_fixed_and_confined() {
        let root = std::path::Path::new(r"C:\Apps\Atmos");
        let files = retired_paths(root);
        assert_eq!(files.len(), 6);
        for p in files {
            assert!(p.starts_with(root));
            assert!(!p
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir)));
            assert!(!p.to_string_lossy().contains("store"));
        }
    }
}
