//! Runtime-only animation policy: never changes persisted appearance or global Windows settings.
use crate::{host_settings, policy_store, windows};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub fps: u32,
    pub pause_fullscreen: bool,
    pub pause_battery: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub performance: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            last_request_id: None,
            performance: Policy {
                fps: 30,
                pause_fullscreen: false,
                pause_battery: false,
            },
        }
    }
}
fn valid(s: &Settings) -> Result<(), String> {
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || ![5, 15, 30].contains(&s.performance.fps)
        || s.last_request_id
            .as_ref()
            .is_some_and(|id| !policy_store::valid_id(id))
    {
        return Err("PERFORMANCE_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let value = policy_store::read_optional(&root.join("performance-settings.json"), 16384)
        .map_err(|_| "PERFORMANCE_INVALID")?;
    let s = match value {
        Some(v) => serde_json::from_value(v).map_err(|_| "PERFORMANCE_INVALID")?,
        None => Settings::default(),
    };
    valid(&s)?;
    Ok(s)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Update {
    schema_version: u32,
    request_id: String,
    expected_revision: u64,
    performance: Policy,
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len()) || !matches!(args[0].as_str(), "get" | "update") {
        return Err("PERFORMANCE_INVALID".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("PERFORMANCE_INVALID".into());
        }
        p
    } else {
        host_settings::directory()?
    };
    let result = if args[0] == "update" {
        let mut b = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        if b.len() > 16384 {
            return Err("PERFORMANCE_INVALID".into());
        }
        let u: Update = serde_json::from_slice(&b).map_err(|_| "PERFORMANCE_INVALID")?;
        if u.schema_version != 1 {
            return Err("PERFORMANCE_INVALID".into());
        }
        let input = serde_json::to_value(&u).unwrap();
        Some(policy_store::commit(
            &root,
            "PERFORMANCE",
            "performance-settings.json",
            serde_json::to_value(Settings::default()).unwrap(),
            &u.request_id,
            u.expected_revision,
            &input,
            |record| {
                let s: Settings =
                    serde_json::from_value(record.clone()).map_err(|_| "PERFORMANCE_INVALID")?;
                valid(&s)
            },
            |old, revision| {
                let old: Settings =
                    serde_json::from_value(old).map_err(|_| "PERFORMANCE_INVALID")?;
                valid(&old)?;
                let next = Settings {
                    schema_version: 1,
                    revision,
                    last_request_id: Some(u.request_id.clone()),
                    performance: u.performance,
                };
                valid(&next)?;
                Ok(serde_json::to_value(next).unwrap())
            },
        )?)
    } else {
        None
    };
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":read(&root)?,"result":result})
    );
    Ok(0)
}
#[repr(C)]
struct Power {
    ac: u8,
    battery_flag: u8,
    battery_percent: u8,
    reserved: u8,
    life: u32,
    full: u32,
}
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(status: *mut Power) -> i32;
}
pub fn battery() -> Option<bool> {
    let mut p = Power {
        ac: 255,
        battery_flag: 255,
        battery_percent: 255,
        reserved: 0,
        life: 0,
        full: 0,
    };
    if unsafe { GetSystemPowerStatus(&mut p) } == 0 {
        None
    } else {
        match p.ac {
            0 => Some(true),
            1 => Some(false),
            _ => None,
        }
    }
}
pub fn covers(foreground: windows::Bounds, target: windows::Bounds) -> bool {
    foreground.x <= target.x
        && foreground.y <= target.y
        && (foreground.x as i64 + foreground.width as i64)
            >= (target.x as i64 + target.width as i64)
        && (foreground.y as i64 + foreground.height as i64)
            >= (target.y as i64 + target.height as i64)
}
fn application_foreground(desktop_focused: bool, class: &str) -> bool {
    !desktop_focused && !matches!(class, "Progman" | "WorkerW")
}
pub fn state(policy: &Policy, target: windows::Bounds) -> Value {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let on_battery = battery();
    let foreground = unsafe { GetForegroundWindow() };
    let mut rect = windows_sys::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut class = [0u16; 64];
    let ok = unsafe { GetWindowRect(foreground, &mut rect) } != 0;
    unsafe { GetClassNameW(foreground, class.as_mut_ptr(), 64) };
    let name = String::from_utf16_lossy(&class[..class.iter().position(|x| *x == 0).unwrap_or(64)]);
    let fullscreen = ok
        && application_foreground(windows::audio_context().0, name.as_str())
        && covers(
            windows::Bounds {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            },
            target,
        );
    let paused = (policy.pause_battery && on_battery == Some(true))
        || (policy.pause_fullscreen && fullscreen);
    json!({"paused":paused,"fps":policy.fps,"onBattery":on_battery,"fullscreenCovered":fullscreen,"reason":if policy.pause_battery&&on_battery==Some(true){"battery"}else if policy.pause_fullscreen&&fullscreen{"fullscreen"}else{"active"}})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fullscreen_bounds_negative_portrait_and_partial() {
        let b = windows::Bounds {
            x: -1080,
            y: -1920,
            width: 1080,
            height: 1920,
        };
        assert!(covers(b, b));
        assert!(!covers(windows::Bounds { width: 1079, ..b }, b));
        assert!(!covers(windows::Bounds { x: 0, ..b }, b));
        assert!(covers(
            windows::Bounds {
                x: -2000,
                y: -2000,
                width: 4000,
                height: 4000
            },
            b
        ))
    }
    #[test]
    fn desktop_and_icon_focus_are_never_treated_as_fullscreen_apps() {
        for class in ["Progman", "WorkerW", "SHELLDLL_DefView", "SysListView32"] {
            assert!(!application_foreground(true, class));
        }
        assert!(!application_foreground(false, "Progman"));
        assert!(application_foreground(false, "Chrome_WidgetWin_1"));
        assert!(application_foreground(false, "Tauri Window"));
    }
    #[test]
    fn malformed_fps_rejected() {
        let mut s = Settings::default();
        assert!(valid(&s).is_ok());
        s.performance.fps = 144;
        assert!(valid(&s).is_err())
    }
}
