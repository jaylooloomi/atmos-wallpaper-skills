//! Read-only physical display topology. GDI numbers are never persisted as identity.
use crate::windows::Bounds;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    mem::size_of,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Devices::Display::*,
    Foundation::RECT,
    Graphics::Gdi::*,
    UI::{
        HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        Shell::GetScaleFactorForMonitor,
    },
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub number: u32,
    pub primary: bool,
    pub bounds: Bounds,
    pub scale_percent: Option<u32>,
    pub rotation_degrees: Option<u32>,
    pub identity_confidence: String,
    pub selectable: bool,
}
fn text(v: &[u16]) -> String {
    String::from_utf16_lossy(&v[..v.iter().position(|x| *x == 0).unwrap_or(v.len())])
}
fn identity(path: &str) -> String {
    format!(
        "display-{:x}",
        Sha256::digest(path.to_lowercase().as_bytes())
    )
}
unsafe extern "system" fn collect(h: HMONITOR, _: HDC, _: *mut RECT, data: isize) -> i32 {
    let out = &mut *(data as *mut Vec<Display>);
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(h, &mut info as *mut _ as *mut MONITORINFO) == 0 {
        return 1;
    }
    let name = text(&info.szDevice);
    let r = info.monitorInfo.rcMonitor;
    let mut scale = 0;
    let scale_percent = (GetScaleFactorForMonitor(h, &mut scale) == 0).then_some(scale as u32);
    out.push(Display {
        id: identity(&format!("unresolved:{name}")),
        name: name.clone(),
        number: name.trim_start_matches(r"\\.\DISPLAY").parse().unwrap_or(0),
        device_name: name,
        primary: info.monitorInfo.dwFlags & 1 != 0,
        bounds: Bounds {
            x: r.left,
            y: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
        },
        scale_percent,
        rotation_degrees: None,
        identity_confidence: "unresolved".into(),
        selectable: false,
    });
    1
}
pub fn enumerate() -> Result<Vec<Display>, String> {
    struct DpiGuard(isize);
    impl Drop for DpiGuard {
        fn drop(&mut self) {
            if self.0 != 0 {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0 as _);
                }
            }
        }
    }
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if previous.is_null() {
            return Err("DISPLAY_DPI_CONTEXT_FAILED".into());
        }
        let _dpi = DpiGuard(previous as isize);
        let mut out: Vec<Display> = Vec::new();
        if EnumDisplayMonitors(
            null_mut(),
            null(),
            Some(collect),
            &mut out as *mut Vec<Display> as isize,
        ) == 0
        {
            return Err("DISPLAY_ENUM_FAILED".into());
        }
        // Topology may change between sizing and query. Retry only boundedly.
        for _ in 0..3 {
            let (mut np, mut nm) = (0, 0);
            let status = GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm);
            if status != 0 || np > 128 || nm > 512 {
                break;
            }
            let mut paths = vec![std::mem::zeroed::<DISPLAYCONFIG_PATH_INFO>(); np as usize];
            let mut modes = vec![std::mem::zeroed::<DISPLAYCONFIG_MODE_INFO>(); nm as usize];
            let code = QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                paths.as_mut_ptr(),
                &mut nm,
                modes.as_mut_ptr(),
                null_mut(),
            );
            if code == 122 {
                continue;
            }
            if code != 0 {
                break;
            }
            for p in paths.iter().take(np as usize) {
                let mut source: DISPLAYCONFIG_SOURCE_DEVICE_NAME = std::mem::zeroed();
                source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: p.sourceInfo.adapterId,
                    id: p.sourceInfo.id,
                };
                let mut target: DISPLAYCONFIG_TARGET_DEVICE_NAME = std::mem::zeroed();
                target.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: p.targetInfo.adapterId,
                    id: p.targetInfo.id,
                };
                if DisplayConfigGetDeviceInfo(&mut source.header) != 0
                    || DisplayConfigGetDeviceInfo(&mut target.header) != 0
                {
                    continue;
                }
                let path = text(&target.monitorDevicePath);
                if path.is_empty() {
                    continue;
                }
                if let Some(d) = out.iter_mut().find(|d| {
                    d.device_name
                        .eq_ignore_ascii_case(&text(&source.viewGdiDeviceName))
                }) {
                    // Multiple physical targets sharing a GDI source (hardware clone) are ambiguous.
                    if d.selectable {
                        d.selectable = false;
                        d.identity_confidence = "ambiguous-clone".into();
                        continue;
                    }
                    if d.identity_confidence == "ambiguous-clone" {
                        continue;
                    }
                    d.id = identity(&path);
                    let friendly = text(&target.monitorFriendlyDeviceName);
                    if !friendly.is_empty() {
                        d.name = friendly
                    }
                    d.rotation_degrees = match p.targetInfo.rotation {
                        1 => Some(0),
                        2 => Some(90),
                        3 => Some(180),
                        4 => Some(270),
                        _ => None,
                    };
                    d.identity_confidence = "device-path-current-connection".into();
                    d.selectable = true;
                }
            }
            break;
        }
        let ids: Vec<_> = out.iter().map(|d| d.id.clone()).collect();
        for d in &mut out {
            if ids.iter().filter(|id| **id == d.id).count() != 1 {
                d.selectable = false;
                d.identity_confidence = "ambiguous".into()
            }
        }
        out.sort_by_key(|d| d.number);
        Ok(out)
    }
}
/// Exact persisted connection identity only. No fuzzy replacement on another port.
pub fn select<'a>(displays: &'a [Display], target: &str) -> Result<&'a Display, String> {
    let matches: Vec<_> = displays
        .iter()
        .filter(|d| {
            if target == "primary" {
                d.primary
            } else {
                d.id == target
            }
        })
        .collect();
    if matches.len() != 1 {
        return Err("TARGET_OFFLINE_OR_AMBIGUOUS".into());
    }
    let d = matches[0];
    if target != "primary" && !d.selectable {
        return Err("TARGET_IDENTITY_UNRESOLVED".into());
    }
    Ok(d)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d(id: &str, x: i32, primary: bool) -> Display {
        Display {
            id: id.into(),
            name: "fixture".into(),
            device_name: "fixture".into(),
            number: 1,
            primary,
            bounds: Bounds {
                x,
                y: -1080,
                width: 1080,
                height: 1920,
            },
            scale_percent: Some(150),
            rotation_degrees: Some(90),
            identity_confidence: "fixture".into(),
            selectable: true,
        }
    }
    #[test]
    fn selection_never_falls_back() {
        let list = vec![d("a", -1080, false), d("b", 0, true)];
        assert_eq!(select(&list, "a").unwrap().bounds.x, -1080);
        assert_eq!(select(&list, "primary").unwrap().id, "b");
        assert!(select(&list, "missing").is_err())
    }
    #[test]
    fn duplicate_identity_fails_closed() {
        assert!(select(&[d("a", 0, true), d("a", 1, false)], "a").is_err())
    }
    #[test]
    fn native_topology_readable() {
        let ds = enumerate().unwrap();
        assert!(!ds.is_empty());
        assert_eq!(ds.iter().filter(|d| d.primary).count(), 1);
        assert!(ds.iter().all(|d| d.bounds.width > 0 && d.bounds.height > 0));
    }
}
