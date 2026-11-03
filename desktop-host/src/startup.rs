//! Documented per-user Run registration. No administrator, task credentials,
//! Windows service, or modification of StartupApproved / Windows policy.
use serde::Serialize;
use std::{
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::*,
};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const PREFS: &str = r"Software\AtmosWallpaper";
const VALUE: &str = "AtmosWallpaper";
const CHOICE: &str = "StartupEnabled";
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
fn failure(code: u32) -> String {
    format!("Windows startup registry operation failed ({code})")
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn writable(path: &str) -> Result<Key, String> {
    unsafe {
        let mut key = null_mut();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        );
        if result != ERROR_SUCCESS {
            return Err(failure(result));
        }
        Ok(Key(key))
    }
}
fn read_string(path: &str, value: &str) -> Result<Option<String>, String> {
    unsafe {
        let mut size = 0;
        let code = RegGetValueW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            wide(value).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut size,
        );
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(failure(code));
        }
        if size > 32768 {
            return Err("Startup entry is too large".into());
        }
        let mut data = vec![0u16; size as usize / 2 + 1];
        let code = RegGetValueW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            wide(value).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            data.as_mut_ptr() as _,
            &mut size,
        );
        if code != ERROR_SUCCESS {
            return Err(failure(code));
        }
        let end = data.iter().position(|c| *c == 0).unwrap_or(data.len());
        Ok(Some(
            String::from_utf16(&data[..end]).map_err(|_| "Invalid startup entry encoding")?,
        ))
    }
}
fn preference() -> Result<Option<bool>, String> {
    unsafe {
        let mut value = 0u32;
        let mut size = 4;
        let code = RegGetValueW(
            HKEY_CURRENT_USER,
            wide(PREFS).as_ptr(),
            wide(CHOICE).as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut value as *mut _ as _,
            &mut size,
        );
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(failure(code));
        }
        if value > 1 {
            return Err("Invalid startup preference".into());
        }
        Ok(Some(value == 1))
    }
}
fn save_preference(enabled: bool) -> Result<(), String> {
    let key = writable(PREFS)?;
    let value = if enabled { 1u32 } else { 0 };
    let code = unsafe {
        RegSetValueExW(
            key.0,
            wide(CHOICE).as_ptr(),
            0,
            REG_DWORD,
            &value as *const _ as _,
            4,
        )
    };
    if code != ERROR_SUCCESS {
        return Err(failure(code));
    }
    Ok(())
}

pub fn command_for(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or("Startup path is not Unicode")?
        .strip_prefix(r"\\?\")
        .unwrap_or(path.to_str().unwrap());
    if !path.is_absolute() || text.contains(['"', '\r', '\n']) {
        return Err("Unsafe startup executable path".into());
    }
    let command = format!("\"{text}\" --autostart-launch");
    if command.encode_utf16().count() > 260 {
        return Err("Windows Run entry exceeds 260 characters; install in a shorter path".into());
    }
    Ok(command)
}
fn command() -> Result<String, String> {
    command_for(&std::env::current_exe().map_err(|e| e.to_string())?)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub available: bool,
    pub enabled: bool,
    pub scope: &'static str,
    pub trigger: &'static str,
    pub command: String,
}
pub fn status() -> Result<Status, String> {
    let command = command()?;
    let actual = read_string(RUN, VALUE)?;
    Ok(Status {
        available: crate::installed::payload_root()?
            .join("payload-manifest.json")
            .is_file(),
        enabled: actual.is_some_and(|v| v.eq_ignore_ascii_case(&command)),
        scope: "current-user",
        trigger: "logon",
        command,
    })
}
pub fn remove() -> Result<(), String> {
    if read_string(RUN, VALUE)?
        .is_some_and(|v| v.eq_ignore_ascii_case(&command().unwrap_or_default()))
    {
        let key = writable(RUN)?;
        let code = unsafe { RegDeleteValueW(key.0, wide(VALUE).as_ptr()) };
        if code != ERROR_SUCCESS && code != ERROR_FILE_NOT_FOUND {
            return Err(failure(code));
        }
    }
    Ok(())
}
pub fn set(enabled: bool) -> Result<Status, String> {
    if enabled {
        if !status()?.available {
            return Err("Startup is available in the installed Windows app only".into());
        }
        let key = writable(RUN)?;
        let data = wide(&command()?);
        let code = unsafe {
            RegSetValueExW(
                key.0,
                wide(VALUE).as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as _,
                (data.len() * 2) as u32,
            )
        };
        if code != ERROR_SUCCESS {
            return Err(failure(code));
        }
    } else {
        remove()?;
    }
    save_preference(enabled)?;
    status()
}
pub fn initialize() -> Result<Status, String> {
    set(preference()?.unwrap_or(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_spaces_and_unicode_without_shell() {
        assert_eq!(
            command_for(Path::new(r"C:\測試 App\atmos-wallpaper.exe")).unwrap(),
            "\"C:\\測試 App\\atmos-wallpaper.exe\" --autostart-launch"
        );
    }
    #[test]
    fn rejects_relative_quotes_and_overlong_commands() {
        for s in [
            "relative.exe".to_string(),
            "C:\\bad\"name.exe".into(),
            format!("C:\\{}\\app.exe", "a".repeat(260)),
        ] {
            assert!(command_for(Path::new(&s)).is_err());
        }
    }
}
