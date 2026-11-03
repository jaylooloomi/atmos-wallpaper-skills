//! Windows-only desktop adapter. Never changes Explorer's styles, visibility,
//! wallpaper registry values, or another application's windows.
use serde::Serialize;
use std::{
    mem::size_of,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, SetLastError, ERROR_ALREADY_EXISTS, HWND, POINT, RECT,
    },
    Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, ScreenToClient, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    },
    System::{JobObjects::*, Threading::CreateMutexW},
    UI::WindowsAndMessaging::*,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
fn err(label: &str) -> String {
    format!("{label}: Win32 error {}", unsafe { GetLastError() })
}
fn hwnd(h: isize) -> HWND {
    h as HWND
}

pub struct OwnedHandle(pub isize);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0 as _);
        }
    }
}

pub fn singleton() -> Result<OwnedHandle, String> {
    named_singleton("Local\\AtmosWallpaperDesktop.SingleInstance")
}

fn named_singleton(name: &str) -> Result<OwnedHandle, String> {
    unsafe {
        let h = CreateMutexW(null(), 0, wide(name).as_ptr());
        if h.is_null() {
            return Err(err("CreateMutex"));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(h);
            return Err("Atmos is already running".into());
        }
        Ok(OwnedHandle(h as isize))
    }
}

pub fn child_job(process: isize) -> Result<OwnedHandle, String> {
    unsafe {
        let job = CreateJobObjectW(null(), null());
        if job.is_null() {
            return Err(err("CreateJobObject"));
        }
        let owned = OwnedHandle(job as isize);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
            || AssignProcessToJobObject(job, process as _) == 0
        {
            return Err(err("Own runtime job"));
        }
        Ok(owned)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
fn bounds(r: RECT) -> Bounds {
    Bounds {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}
pub fn primary_bounds() -> Result<Bounds, String> {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err(err("GetMonitorInfo"));
        }
        Ok(bounds(info.rcMonitor))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WindowRecord {
    pub handle: isize,
    pub parent: isize,
    pub class: String,
    pub visible: bool,
    pub bounds: Bounds,
}
pub fn record(h: isize) -> WindowRecord {
    unsafe {
        let mut name = [0u16; 256];
        let n = GetClassNameW(hwnd(h), name.as_mut_ptr(), name.len() as i32).max(0) as usize;
        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd(h), &mut rect);
        WindowRecord {
            handle: h,
            parent: GetParent(hwnd(h)) as isize,
            class: String::from_utf16_lossy(&name[..n]),
            visible: IsWindowVisible(hwnd(h)) != 0,
            bounds: bounds(rect),
        }
    }
}
unsafe extern "system" fn collect(h: HWND, data: isize) -> i32 {
    let list = &mut *(data as *mut Vec<isize>);
    list.push(h as isize);
    1
}
pub fn desktop_windows() -> Vec<WindowRecord> {
    unsafe {
        let mut list = Vec::new();
        EnumWindows(Some(collect), &mut list as *mut Vec<isize> as isize);
        let mut out = Vec::new();
        for h in list {
            let r = record(h);
            if ["Progman", "WorkerW"].contains(&r.class.as_str()) {
                out.push(r);
                let mut child = GetWindow(hwnd(h), GW_CHILD);
                while !child.is_null() {
                    out.push(record(child as isize));
                    child = GetWindow(child, GW_HWNDNEXT);
                }
            }
        }
        out
    }
}

pub fn desktop_parent() -> Result<isize, String> {
    // Reuse an existing valid desktop. Re-requesting Shell's background layer
    // while another wallpaper is active is unnecessary and may disturb it.
    if let Some(parent) = existing_desktop_parent() {
        return Ok(parent);
    }
    unsafe {
        let progman = FindWindowW(wide("Progman").as_ptr(), null());
        if progman.is_null() {
            return Err("Explorer desktop is unavailable".into());
        }
        // Shell's undocumented WorkerW request: bounded and fail-closed. No
        // desktop window is hidden/deleted and no arbitrary top-level fallback.
        let mut result = 0;
        SendMessageTimeoutW(progman, 0x052c, 0, 0, SMTO_ABORTIFHUNG, 1000, &mut result);
    }
    existing_desktop_parent()
        .ok_or("No supported background WorkerW found; desktop was not replaced".into())
}

pub fn shell_ready() -> bool {
    desktop_windows()
        .iter()
        .any(|w| w.class == "SHELLDLL_DefView")
}

fn existing_desktop_parent() -> Option<isize> {
    unsafe {
        let windows = desktop_windows();
        for root in windows.iter().filter(|r| r.parent == 0) {
            let icons = FindWindowExW(
                hwnd(root.handle),
                null_mut(),
                wide("SHELLDLL_DefView").as_ptr(),
                null(),
            );
            if icons.is_null() {
                continue;
            }
            // Newer Explorer can put the background WorkerW under Progman.
            let raised = FindWindowExW(
                hwnd(root.handle),
                null_mut(),
                wide("WorkerW").as_ptr(),
                null(),
            );
            if !raised.is_null() && IsWindowVisible(raised) != 0 {
                return Some(raised as isize);
            }
            let sibling = FindWindowExW(
                null_mut(),
                hwnd(root.handle),
                wide("WorkerW").as_ptr(),
                null(),
            );
            if !sibling.is_null() && IsWindowVisible(sibling) != 0 {
                return Some(sibling as isize);
            }
        }
        None
    }
}

pub fn attach(window: isize, parent: isize, target: Bounds) -> Result<WindowRecord, String> {
    unsafe {
        if IsWindow(hwnd(parent)) == 0 || record(parent).class != "WorkerW" {
            return Err("Desktop parent expired".into());
        }
        ShowWindow(hwnd(window), SW_HIDE);
        let style = GetWindowLongPtrW(hwnd(window), GWL_STYLE);
        SetLastError(0);
        SetWindowLongPtrW(
            hwnd(window),
            GWL_STYLE,
            (style & !(WS_POPUP as isize | WS_CAPTION as isize | WS_THICKFRAME as isize))
                | WS_CHILD as isize,
        );
        if GetLastError() != 0 {
            return Err(err("Set child style"));
        }
        let ex = GetWindowLongPtrW(hwnd(window), GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd(window),
            GWL_EXSTYLE,
            (ex & !(WS_EX_APPWINDOW as isize))
                | WS_EX_TOOLWINDOW as isize
                | WS_EX_NOACTIVATE as isize,
        );
        SetLastError(0);
        let old = SetParent(hwnd(window), hwnd(parent));
        if old.is_null() && GetLastError() != 0 {
            return Err(err("SetParent"));
        }
        let mut point = POINT {
            x: target.x,
            y: target.y,
        };
        if ScreenToClient(hwnd(parent), &mut point) == 0 {
            return Err(err("ScreenToClient"));
        }
        if SetWindowPos(
            hwnd(window),
            HWND_TOP,
            point.x,
            point.y,
            target.width,
            target.height,
            SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        ) == 0
        {
            return Err(err("SetWindowPos"));
        }
        let actual = record(window);
        if actual.parent != parent || actual.bounds != target || !actual.visible {
            return Err(format!(
                "Desktop attachment verification failed: {actual:?}"
            ));
        }
        Ok(actual)
    }
}
pub fn still_attached(window: isize, parent: isize, target: Bounds) -> bool {
    unsafe {
        IsWindow(hwnd(parent)) != 0
            && record(window).parent == parent
            && record(window).bounds == target
            && record(window).visible
    }
}
pub fn exists(window: isize) -> bool {
    unsafe { IsWindow(hwnd(window)) != 0 }
}
pub fn static_wallpaper() -> Result<String, String> {
    unsafe {
        let mut text = vec![0u16; 32768];
        if SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            text.len() as u32,
            text.as_mut_ptr() as _,
            0,
        ) == 0
        {
            return Err(err("Read original wallpaper"));
        }
        let n = text.iter().position(|v| *v == 0).unwrap_or(text.len());
        Ok(String::from_utf16_lossy(&text[..n]))
    }
}

pub fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(&format!(
                "Atmos 無法啟動。請重新安裝或查看本機診斷紀錄。\n\n{message}"
            ))
            .as_ptr(),
            wide("Atmos Wallpaper").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_desktop_metadata_is_readable() {
        let b = primary_bounds().unwrap();
        assert!(b.width > 0 && b.height > 0);
        assert!(!desktop_windows().is_empty());
    }
    #[test]
    fn second_instance_is_rejected() {
        let name = format!("Local\\AtmosDesktopPrototype.Test.{}", std::process::id());
        let _first = named_singleton(&name).unwrap();
        assert!(named_singleton(&name).is_err());
    }
    #[test]
    fn static_wallpaper_can_be_read_without_modification() {
        assert!(static_wallpaper().is_ok());
    }
}
