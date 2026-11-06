//! Existing content-addressed library resolution, with no offline recreation or
//! fallback. Image bytes continue to be normalized by the bundled Python codec.
use crate::{
    desired_state,
    js_json::{self, Value},
    portable_store::{self, Store},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}
pub fn read_bytes(file: &Path, max: usize, error: &str) -> Result<Vec<u8>, String> {
    portable_store::no_links(file).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    let mut f = fs::File::open(file).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "ENOENT".into()
        } else {
            error.to_string()
        }
    })?;
    let st = f.metadata().map_err(|_| error)?;
    if !st.is_file() || st.len() > max as u64 {
        return Err(error.into());
    }
    let mut bytes = Vec::new();
    f.by_ref()
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error)?;
    if bytes.len() > max {
        return Err(error.into());
    }
    Ok(bytes)
}
pub fn read_json(file: &Path, max: usize) -> Result<Value, String> {
    js_json::parse(&read_bytes(file, max, "LIBRARY_CORRUPT")?).map_err(|_| "LIBRARY_CORRUPT".into())
}
pub fn path(owner: &Store, name: &str) -> Result<PathBuf, String> {
    let tail = if name == "assets" {
        ""
    } else {
        name.strip_prefix("assets/").ok_or("LIBRARY_PATH_UNSAFE")?
    };
    if !tail.is_empty() {
        let mut parts = tail.split('.');
        let id = parts.next().unwrap();
        let ext = parts.next().unwrap_or("");
        if !desired_state::hex_id(&Value::from(id))
            || !["png", "json"].contains(&ext)
            || parts.next().is_some()
        {
            return Err("LIBRARY_PATH_UNSAFE".into());
        }
    }
    let config_file = owner
        .path("library-config.json")
        .map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    let mut legacy = true;
    let mut root = owner.root.join("assets");
    let mut id = String::new();
    if config_file.exists() {
        let c = read_json(&config_file, 16384)?;
        if !c.as_object().is_some_and(|o| o.len() == 4)
            || !c.has("schemaVersion")
            || !c.has("revision")
            || !c.has("root")
            || !c.has("id")
            || c["schemaVersion"].as_num() != Some(1.0)
            || !c["revision"].safe_uint().is_some_and(|r| r >= 1)
            || !c["root"]
                .as_str()
                .is_some_and(portable_store::node_absolute)
            || !c["id"].as_str().is_some_and(|s| {
                s.len() == 36
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || b == b'-')
            })
        {
            return Err("LIBRARY_CONFIG_INVALID".into());
        }
        root = PathBuf::from(c["root"].as_str().unwrap());
        id = c["id"].as_str().unwrap().into();
        legacy = false;
    }
    let intent = owner
        .path("library-archive-intent.json")
        .map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    if intent.exists() && read_json(&intent, 4 * 1048576)?["state"].as_str() == Some("pending") {
        return Err("LIBRARY_RECOVERY_REQUIRED".into());
    }
    portable_store::no_links(&root).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    if !root.exists() {
        return Err("LIBRARY_OFFLINE".into());
    }
    if !root.is_dir() {
        return Err("LIBRARY_PATH_UNSAFE".into());
    }
    if !legacy {
        let marker = read_json(&root.join(".atmos-library.json"), 16384)?;
        let owner_key = hash_text(&owner.external_root().to_lowercase());
        if marker["schemaVersion"].as_num() != Some(1.0)
            || marker["id"].as_str() != Some(&id)
            || marker["storeKey"].as_str() != Some(&owner_key)
        {
            return Err("LIBRARY_OWNER_MISMATCH".into());
        }
        root = root.join("assets");
        portable_store::no_links(&root).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
        if !root.is_dir() {
            return Err("LIBRARY_OFFLINE".into());
        }
    }
    let file = if tail.is_empty() {
        root
    } else {
        root.join(tail)
    };
    portable_store::no_links(&file).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    Ok(file)
}
