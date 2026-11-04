//! Fixed current-user screensaver selection snapshot. Never touches auth/timeout.
use crate::{host_settings, policy_store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::*,
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Value {
    kind: u32,
    data: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Backup {
    schema_version: u32,
    owner_key: String,
    previous: Option<Value>,
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn open(write: bool) -> Result<Key, String> {
    let mut key = null_mut();
    let code = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(r"Control Panel\Desktop").as_ptr(),
            0,
            KEY_QUERY_VALUE | if write { KEY_SET_VALUE } else { 0 },
            &mut key,
        )
    };
    if code != ERROR_SUCCESS {
        return Err("SCREENSAVER_REGISTRY_UNAVAILABLE".into());
    }
    Ok(Key(key))
}
fn current(key: &Key) -> Result<Option<Value>, String> {
    let mut kind = 0;
    let mut size = 0;
    let first = unsafe {
        RegQueryValueExW(
            key.0,
            wide("SCRNSAVE.EXE").as_ptr(),
            null(),
            &mut kind,
            null_mut(),
            &mut size,
        )
    };
    if first == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if first != ERROR_SUCCESS || size > 8192 {
        return Err("SCREENSAVER_REGISTRY_INVALID".into());
    }
    let mut data = vec![0u8; size as usize];
    let second = unsafe {
        RegQueryValueExW(
            key.0,
            wide("SCRNSAVE.EXE").as_ptr(),
            null(),
            &mut kind,
            data.as_mut_ptr(),
            &mut size,
        )
    };
    if second != ERROR_SUCCESS || size as usize > data.len() {
        return Err("SCREENSAVER_REGISTRY_INVALID".into());
    }
    data.truncate(size as usize);
    let value = Value { kind, data };
    valid(&value)?;
    Ok(Some(value))
}
fn valid(v: &Value) -> Result<(), String> {
    if ![REG_SZ, REG_EXPAND_SZ].contains(&v.kind)
        || v.data.len() > 8192
        || v.data.len() < 2
        || v.data.len() % 2 != 0
        || v.data[v.data.len() - 2..] != [0, 0]
    {
        return Err("SCREENSAVER_REGISTRY_INVALID".into());
    }
    let words = v
        .data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    if words[..words.len() - 1].contains(&0)
        || String::from_utf16(&words[..words.len() - 1]).is_err()
    {
        return Err("SCREENSAVER_REGISTRY_INVALID".into());
    }
    Ok(())
}
fn text(v: &Value) -> Option<String> {
    valid(v).ok()?;
    let w = v
        .data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&w[..w.len() - 1]).ok()
}
fn normalized(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s);
    s.strip_prefix(r"\\?\")
        .unwrap_or(s)
        .replace('/', "\\")
        .to_lowercase()
}
fn owner(own: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(normalized(&own.to_string_lossy()).as_bytes())
    )
}
fn file() -> Result<PathBuf, String> {
    Ok(host_settings::directory()?.join("screensaver-registration-backup.json"))
}
fn read_backup(own: &Path) -> Result<Option<Backup>, String> {
    let Some(value) = policy_store::read_optional(&file()?, 65536)? else {
        return Ok(None);
    };
    let b: Backup =
        serde_json::from_value(value).map_err(|_| "SCREENSAVER_REGISTRY_BACKUP_INVALID")?;
    if b.schema_version != 1 || b.owner_key != owner(own) {
        return Err("SCREENSAVER_REGISTRY_BACKUP_INVALID".into());
    }
    if let Some(value) = &b.previous {
        valid(value)?;
    }
    Ok(Some(b))
}
pub fn snapshot(own: &Path) -> Result<(), String> {
    let key = open(false)?;
    let old = current(&key)?;
    if old
        .as_ref()
        .and_then(text)
        .is_some_and(|p| normalized(&p) == normalized(&own.to_string_lossy()))
    {
        let _ = read_backup(own)?;
        return Ok(());
    }
    let next = Backup {
        schema_version: 1,
        owner_key: owner(own),
        previous: old,
    };
    let file = file()?;
    host_settings::atomic(
        file.parent().unwrap(),
        "screensaver-registration-backup.json",
        &next,
    )
}
fn restoration(
    current: Option<&Value>,
    own: &Path,
    previous: Option<&Value>,
) -> Result<Option<Option<Value>>, String> {
    let Some(now) = current else { return Ok(None) };
    valid(now)?;
    if !text(now).is_some_and(|s| normalized(&s) == normalized(&own.to_string_lossy())) {
        return Ok(None);
    }
    if let Some(p) = previous {
        valid(p)?;
        if !text(p).is_some_and(|s| normalized(&s) == normalized(&own.to_string_lossy())) {
            return Ok(Some(Some(p.clone())));
        }
    }
    Ok(Some(None))
}
pub fn unregister() -> Result<i32, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let own = exe
        .parent()
        .ok_or("SCREENSAVER_LOCATION_INVALID")?
        .join("Atmos.scr");
    let key = open(true)?;
    let now = current(&key)?;
    if !now
        .as_ref()
        .and_then(text)
        .is_some_and(|p| normalized(&p) == normalized(&own.to_string_lossy()))
    {
        println!(
            "{}",
            serde_json::json!({"schemaVersion":1,"ownedSelectionRemoved":false,"restoredPreviousSelection":false,"authenticationAndTimeoutUnchanged":true})
        );
        return Ok(0);
    }
    let backup = read_backup(&own)?;
    let restore = restoration(
        now.as_ref(),
        &own,
        backup.as_ref().and_then(|b| b.previous.as_ref()),
    )?;
    let mut restored = false;
    if let Some(previous) = &restore {
        let code = unsafe {
            if let Some(value) = previous {
                restored = true;
                RegSetValueExW(
                    key.0,
                    wide("SCRNSAVE.EXE").as_ptr(),
                    0,
                    value.kind,
                    value.data.as_ptr(),
                    value.data.len() as u32,
                )
            } else {
                RegDeleteValueW(key.0, wide("SCRNSAVE.EXE").as_ptr())
            }
        };
        if code != ERROR_SUCCESS && code != ERROR_FILE_NOT_FOUND {
            return Err("SCREENSAVER_REGISTRY_UPDATE_FAILED".into());
        }
        let file = file()?;
        policy_store::safe_path(&file)?;
        if file.is_file() {
            std::fs::remove_file(file).map_err(|_| "SCREENSAVER_REGISTRY_BACKUP_INVALID")?;
        }
    }
    println!(
        "{}",
        serde_json::json!({"schemaVersion":1,"ownedSelectionRemoved":restore.is_some(),"restoredPreviousSelection":restored,"authenticationAndTimeoutUnchanged":true})
    );
    Ok(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn value(text: &str) -> Value {
        Value {
            kind: REG_SZ,
            data: wide(text)
                .into_iter()
                .flat_map(|c| c.to_le_bytes())
                .collect(),
        }
    }
    #[test]
    fn uninstall_restores_only_exact_owned_selection() {
        let own = Path::new(r"C:\Apps\Atmos\Atmos.scr");
        let old = value(r"C:\Windows\System32\Mystify.scr");
        let current = value(r#""C:\Apps\Atmos\Atmos.scr""#);
        assert_eq!(
            restoration(Some(&current), own, Some(&old)).unwrap(),
            Some(Some(old.clone()))
        );
        assert_eq!(restoration(Some(&old), own, None).unwrap(), None);
        assert_eq!(restoration(None, own, None).unwrap(), None);
        assert_eq!(restoration(Some(&current), own, None).unwrap(), Some(None));
        assert_eq!(
            restoration(Some(&current), own, Some(&current)).unwrap(),
            Some(None)
        );
    }
    #[test]
    fn registry_blob_and_paths_are_bounded_without_expanding_unknown_env() {
        assert!(valid(&Value {
            kind: REG_BINARY,
            data: vec![0, 0]
        })
        .is_err());
        assert!(valid(&Value {
            kind: REG_SZ,
            data: vec![65, 0]
        })
        .is_err());
        assert_ne!(
            normalized(r"%APPDATA%\Atmos.scr"),
            normalized(r"C:\Apps\Atmos.scr")
        );
        assert_eq!(
            normalized(r"\\?\C:\Apps\Atmos.scr"),
            normalized(r"C:\Apps\Atmos.scr")
        );
    }
}
