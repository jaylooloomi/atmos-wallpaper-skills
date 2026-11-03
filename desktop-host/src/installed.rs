use crate::options::{self, Options};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    architecture: String,
    versions: Versions,
    files: Vec<Entry>,
}
#[derive(Deserialize, Serialize)]
pub struct Versions {
    pub node: String,
    pub python: String,
    pub pillow: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}

pub fn payload_root() -> Result<PathBuf, String> {
    Ok(std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Missing executable directory")?
        .join("payload"))
}

pub fn verify(root: &Path) -> Result<Versions, String> {
    let manifest_path = root.join("payload-manifest.json");
    if fs::metadata(&manifest_path)
        .map_err(|_| "Bundled payload is missing; reinstall Atmos")?
        .len()
        > 4 * 1024 * 1024
    {
        return Err("Payload manifest too large".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(manifest_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if manifest.schema_version != 1
        || manifest.architecture != "x86_64"
        || manifest.files.is_empty()
        || manifest.files.len() > 10000
    {
        return Err("Invalid payload manifest".into());
    }
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    let mut names = std::collections::HashSet::new();
    for entry in manifest.files {
        safe_relative(&entry.path)?;
        if !names.insert(entry.path.clone())
            || entry.bytes > 256 * 1024 * 1024
            || entry.sha256.len() != 64
        {
            return Err("Invalid payload file entry".into());
        }
        let file = root.join(&entry.path);
        if !file
            .canonicalize()
            .map_err(|_| format!("Bundled file missing: {}", entry.path))?
            .starts_with(&canonical)
        {
            return Err("Payload path escaped its directory".into());
        }
        let mut reader = fs::File::open(file).map_err(|e| e.to_string())?;
        if reader.metadata().map_err(|e| e.to_string())?.len() != entry.bytes {
            return Err(format!("Bundled file size mismatch: {}", entry.path));
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if format!("{:x}", hash.finalize()) != entry.sha256 {
            return Err(format!("Bundled file checksum mismatch: {}", entry.path));
        }
    }
    for required in [
        "runtime/node/node.exe",
        "runtime/python/python.exe",
        "app/implementation-p4_2_5/src/windows-runtime.mjs",
        "app/implementation-p4_2_5/dist/index.html",
        "app/implementation-p1/src/decode-image.py",
    ] {
        if !names.contains(required) {
            return Err(format!("Required bundled file missing: {required}"));
        }
    }
    Ok(manifest.versions)
}

fn safe_relative(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || value
            .split('/')
            .any(|v| v.is_empty() || v == "." || v == "..")
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Unsafe payload path".into());
    }
    Ok(())
}

pub fn configuration(args: Vec<String>) -> Result<Options, String> {
    // Explicit developer/test mode retains the original isolated prototype CLI.
    if args
        .iter()
        .any(|a| ["--runtime-root", "--node", "--python"].contains(&a.as_str()))
    {
        return options::parse(args);
    }
    let root = payload_root()?;
    verify(&root)?;
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA unavailable")?)
        .join("io.atmos.wallpaper");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let defaults = [
        ("--runtime-root", root.join("app/implementation-p4_2_5")),
        ("--node", root.join("runtime/node/node.exe")),
        ("--python", root.join("runtime/python/python.exe")),
        ("--store", local.join("store")),
        (
            "--status",
            local.join(format!("runs/{stamp}-{}/status.json", std::process::id())),
        ),
    ];
    let bounded = args.iter().any(|v| v == "--run-seconds");
    let mut merged = args.clone();
    for (key, path) in defaults {
        if !args.iter().any(|v| v == key) {
            merged.push(key.into());
            merged.push(path.to_string_lossy().into_owned());
        }
    }
    let mut options = options::parse(merged)?;
    if !bounded {
        options.seconds = None;
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_escape_and_windows_prefixes() {
        for p in [
            "../node.exe",
            "C:/node.exe",
            "/node.exe",
            "a\\node.exe",
            "a//b",
            "./a",
        ] {
            assert!(safe_relative(p).is_err(), "{p}");
        }
        assert!(safe_relative("runtime/node/node.exe").is_ok());
    }
    #[test]
    fn corrupt_or_missing_payload_fails_closed() {
        let p = std::env::temp_dir().join(format!("atmos-missing-payload-{}", std::process::id()));
        assert!(verify(&p).is_err());
    }
}
