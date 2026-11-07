//! Compatibility with the existing Windows CurrentUser DPAPI calendar file.
use crate::{asset_store, portable_store::Store};
use serde::{Deserialize, Serialize};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

const FILE: &str = "calendar/connection.bin";
const MAX: usize = 3 * 1048576;
const ERROR: &str = "ICS_STORAGE_FAILED";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub url: Option<String>,
    pub text: String,
    pub updated_at: String,
}

impl Source {
    fn validate(&self) -> Result<(), String> {
        if !matches!(
            (self.kind.as_str(), self.url.is_some()),
            ("file", false) | ("subscription", true)
        ) || self.name.encode_utf16().count() > 80
            || self.text.len() > 1048576
            || self.url.as_ref().is_some_and(|url| url.len() > 4096)
            || crate::appearance_core::timestamp_ms(&self.updated_at).is_none()
        {
            return Err(ERROR.into());
        }
        Ok(())
    }
}

struct DpapiBuffer(CRYPT_INTEGER_BLOB);

impl Drop for DpapiBuffer {
    fn drop(&mut self) {
        if !self.0.pbData.is_null() {
            unsafe {
                LocalFree(self.0.pbData.cast());
            }
        }
    }
}

fn transform(bytes: &[u8], decrypt: bool) -> Result<DpapiBuffer, String> {
    if bytes.is_empty() || bytes.len() > MAX {
        return Err(ERROR.into());
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = DpapiBuffer(CRYPT_INTEGER_BLOB::default());
    // No optional entropy or machine scope, matching ProtectedData.CurrentUser.
    let success = unsafe {
        if decrypt {
            CryptUnprotectData(
                &input,
                null_mut(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        } else {
            CryptProtectData(
                &input,
                null(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        }
    };
    if success == 0
        || output.0.pbData.is_null()
        || output.0.cbData == 0
        || output.0.cbData as usize > MAX
    {
        return Err(ERROR.into());
    }
    Ok(output)
}

impl AsRef<[u8]> for DpapiBuffer {
    fn as_ref(&self) -> &[u8] {
        // Only constructed and returned after a successful call and length check.
        unsafe { std::slice::from_raw_parts(self.0.pbData, self.0.cbData as usize) }
    }
}

pub(crate) fn read(store: &Store) -> Result<Option<Source>, String> {
    let path = store.path(FILE).map_err(|_| ERROR)?;
    let protected = match asset_store::read_bytes(&path, MAX, ERROR) {
        Ok(bytes) => bytes,
        Err(e) if e == "ENOENT" => return Ok(None),
        Err(_) => return Err(ERROR.into()),
    };
    let data = transform(&protected, true)?;
    let parsed: Option<Source> = serde_json::from_slice(data.as_ref()).map_err(|_| ERROR)?;
    if let Some(source) = &parsed {
        source.validate()?;
    }
    Ok(parsed)
}

pub(crate) fn save(store: &Store, source: Option<&Source>) -> Result<(), String> {
    if let Some(source) = source {
        source.validate()?;
    }
    let bytes = serde_json::to_vec(&source).map_err(|_| ERROR)?;
    let protected = transform(&bytes, false)?;
    let dir = store.path("calendar").map_err(|_| ERROR)?;
    std::fs::create_dir_all(dir).map_err(|_| ERROR)?;
    store
        .atomic_bytes(FILE, protected.as_ref())
        .map_err(|_| ERROR.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::tests::TestStore;

    #[test]
    fn dpapi_calendar_round_trip_and_corruption_leave_existing_file_intact() {
        let dir = TestStore::new();
        let core = dir.core();
        assert_eq!(read(&core.store).unwrap(), None);
        let source = Source {
            kind: "file".into(),
            name: "測試行程".into(),
            url: None,
            text: "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".into(),
            updated_at: "2026-11-07T00:00:00.000Z".into(),
        };
        save(&core.store, Some(&source)).unwrap();
        let bytes = std::fs::read(core.store.path(FILE).unwrap()).unwrap();
        assert!(!bytes
            .windows(source.text.len())
            .any(|window| window == source.text.as_bytes()));
        assert_eq!(read(&core.store).unwrap(), Some(source));
        core.store
            .atomic_bytes(FILE, b"invalid protected data")
            .unwrap();
        assert_eq!(read(&core.store).unwrap_err(), ERROR);
        assert_eq!(
            std::fs::read(core.store.path(FILE).unwrap()).unwrap(),
            b"invalid protected data"
        );
        save(&core.store, None).unwrap();
        assert_eq!(read(&core.store).unwrap(), None);
    }
}
