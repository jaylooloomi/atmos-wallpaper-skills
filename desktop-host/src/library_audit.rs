//! Read-only reference audit for the native library worker.
use crate::{
    asset_store, desired_state, js_json, library,
    portable_store::{self, Store},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const UNCERTAIN: &str = "LIBRARY_REFERENCES_UNCERTAIN";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::tests::TestStore;

    #[test]
    fn read_only_audit_protects_offline_references_and_fails_closed_on_corruption() {
        let dir = TestStore::new();
        let core = dir.core();
        let owner = &core.store;
        let assets = asset_store::path(owner, "assets").unwrap();
        let make = |bytes: &[u8]| {
            let id = asset_store::hash_bytes(bytes);
            fs::write(assets.join(format!("{id}.png")), bytes).unwrap();
            fs::write(
                assets.join(format!("{id}.json")),
                json!({"assetId":id,"bytes":bytes.len(),"format":"PNG","width":1,"height":1})
                    .to_string(),
            )
            .unwrap();
            id
        };
        let protected = make(b"protected");
        let free = make(b"unreferenced");
        let offline = core
            .ensure_scope(&format!("display-{}", "e".repeat(64)))
            .unwrap();
        offline
            .atomic(
                "presets.json",
                &js_json::object([("fixture", protected.clone().into())]),
            )
            .unwrap();
        let result = audit(owner, 0).unwrap();
        assert_eq!(result["candidateCount"], 1);
        assert_eq!(result["protectedCount"], 1);
        assert!(assets.join(format!("{free}.png")).exists());
        assert!(assets.join(format!("{protected}.png")).exists());
        assert_eq!(audit(owner, 1).unwrap_err(), "LIBRARY_REVISION_CONFLICT");
        fs::write(offline.path("live.json").unwrap(), b"broken").unwrap();
        assert_eq!(audit(owner, 0).unwrap_err(), UNCERTAIN);
        assert!(assets.join(format!("{free}.png")).exists());
    }
}
const DIRS: [&str; 7] = [
    "observations",
    "previews",
    "requests",
    "exports",
    "backups",
    "commit-intents",
    "recovery",
];

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct File {
    pub name: String,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Serialize)]
struct Reference {
    file: String,
    hash: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Pointer {
    pub root: String,
    pub id: Option<String>,
    pub revision: u64,
}

#[derive(Serialize)]
struct AuditInput<'a> {
    library: Pointer,
    files: &'a [File],
    references: String,
}

fn regular(path: &Path) -> Result<bool, String> {
    portable_store::no_links(path).map_err(|_| UNCERTAIN)?;
    match fs::metadata(path) {
        Ok(m) if m.is_file() => Ok(true),
        Ok(_) => Err(UNCERTAIN.into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(UNCERTAIN.into()),
    }
}

pub(crate) fn inventory(dir: &Path, allow_partial: bool) -> Result<Vec<File>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| "LIBRARY_UNAVAILABLE")? {
        if files.len() >= 20000 {
            return Err("LIBRARY_LIMIT".into());
        }
        let entry = entry.map_err(|_| "LIBRARY_CONTENT_INVALID")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "LIBRARY_CONTENT_INVALID")?;
        let (id, ext) = name.split_once('.').ok_or("LIBRARY_CONTENT_INVALID")?;
        if !desired_state::hex_id(&id.into()) {
            return Err("LIBRARY_CONTENT_INVALID".into());
        }
        let limit = match ext {
            "png" => 16 * 1048576,
            "json" => 16384,
            _ => return Err("LIBRARY_CONTENT_INVALID".into()),
        };
        let data = asset_store::read_bytes(&entry.path(), limit, "LIBRARY_CONTENT_INVALID")?;
        let sha256 = asset_store::hash_bytes(&data);
        if ext == "png" && sha256 != id {
            return Err("LIBRARY_ASSET_CORRUPT".into());
        }
        files.push(File {
            name,
            bytes: data.len(),
            sha256,
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    for file in &files {
        let id = &file.name[..64];
        let is_metadata = file.name.ends_with(".json");
        let other = format!("{id}.{}", if is_metadata { "png" } else { "json" });
        let Ok(index) = files.binary_search_by(|f| f.name.cmp(&other)) else {
            if allow_partial {
                continue;
            }
            return Err("LIBRARY_ASSET_CORRUPT".into());
        };
        if !is_metadata {
            continue;
        }
        let image = &files[index];
        let v = asset_store::read_json(&dir.join(&file.name), 16384)?;
        if v["assetId"].as_str() != Some(id)
            || v["bytes"].safe_uint() != Some(image.bytes as u64)
            || v["format"].as_str() != Some("PNG")
            || !["width", "height"]
                .iter()
                .all(|k| v[k].safe_uint().is_some_and(|x| x > 0))
        {
            return Err("LIBRARY_ASSET_CORRUPT".into());
        }
    }
    Ok(files)
}

struct References<'a> {
    root: &'a Path,
    ids: BTreeSet<&'a str>,
    used: BTreeSet<String>,
    records: Vec<Reference>,
    bytes: usize,
    directories: usize,
}

impl References<'_> {
    fn visit(&mut self, v: &js_json::Value, depth: usize) -> Result<(), String> {
        if depth > 80 {
            return Err(UNCERTAIN.into());
        }
        match v {
            js_json::Value::String(s) => {
                if let Some(id) = s.as_str().filter(|id| self.ids.contains(id)) {
                    self.used.insert(id.into());
                }
            }
            js_json::Value::Object(v) => {
                for x in v.values() {
                    self.visit(x, depth + 1)?;
                }
            }
            js_json::Value::Array(v) => {
                for x in v {
                    self.visit(x, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn file(&mut self, path: &Path, live: bool) -> Result<(), String> {
        if self.records.len() >= 30000 {
            return Err(UNCERTAIN.into());
        }
        let remaining = 256 * 1048576 - self.bytes;
        let bytes = asset_store::read_bytes(path, remaining.min(32 * 1048576), UNCERTAIN)
            .map_err(|_| UNCERTAIN)?;
        self.bytes += bytes.len();
        let v = js_json::parse(&bytes).map_err(|_| UNCERTAIN)?;
        if live {
            let history = v["history"].as_array().ok_or(UNCERTAIN)?;
            if v["schemaVersion"].safe_uint() != Some(1)
                || !std::iter::once(&v).chain(history).all(|p| {
                    desired_state::validate(&p["desired"]).is_ok()
                        && p["desiredHash"].as_str() == Some(&p["desired"].hash())
                })
            {
                return Err(UNCERTAIN.into());
            }
        }
        self.visit(&v, 0)?;
        let file = path
            .strip_prefix(self.root)
            .map_err(|_| UNCERTAIN)?
            .to_str()
            .ok_or(UNCERTAIN)?
            .replace('\\', "/");
        self.records.push(Reference {
            file,
            hash: asset_store::hash_bytes(&bytes),
        });
        Ok(())
    }

    fn directory(&mut self, path: &Path, depth: usize) -> Result<(), String> {
        if depth > 64 || self.directories >= 30000 {
            return Err(UNCERTAIN.into());
        }
        portable_store::no_links(path).map_err(|_| UNCERTAIN)?;
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(UNCERTAIN.into()),
        };
        self.directories += 1;
        for entry in entries {
            let entry = entry.map_err(|_| UNCERTAIN)?;
            let p = entry.path();
            portable_store::no_links(&p).map_err(|_| UNCERTAIN)?;
            let kind = entry.file_type().map_err(|_| UNCERTAIN)?;
            if kind.is_dir() {
                self.directory(&p, depth + 1)?;
            } else if !kind.is_file() || p.extension().is_some_and(|ext| ext == "bin") {
                return Err(UNCERTAIN.into());
            } else if p.extension().is_some_and(|ext| ext == "json") {
                self.file(&p, false)?;
            }
        }
        Ok(())
    }

    fn profile(&mut self, path: &Path) -> Result<(), String> {
        self.file(&path.join("live.json"), true)?;
        for name in [
            "live.backup.json",
            "scope-seed-intent.json",
            "scope.json",
            "presets.json",
        ] {
            let p = path.join(name);
            if regular(&p)? {
                self.file(&p, false)?;
            }
        }
        for name in DIRS {
            self.directory(&path.join(name), 0)?;
        }
        Ok(())
    }

    fn scan(&mut self) -> Result<(), String> {
        self.profile(self.root)?;
        let targets = self.root.join("targets");
        portable_store::no_links(&targets).map_err(|_| UNCERTAIN)?;
        match fs::read_dir(&targets) {
            Ok(entries) => {
                for e in entries {
                    let e = e.map_err(|_| UNCERTAIN)?;
                    if !e.file_type().map_err(|_| UNCERTAIN)?.is_dir()
                        || !e
                            .file_name()
                            .to_str()
                            .is_some_and(crate::appearance_core::display_id)
                    {
                        return Err(UNCERTAIN.into());
                    }
                    portable_store::no_links(&e.path()).map_err(|_| UNCERTAIN)?;
                    self.profile(&e.path())?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(UNCERTAIN.into()),
        }
        self.records.sort_by(|a, b| a.file.cmp(&b.file));
        Ok(())
    }
}

fn digest(v: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(v)
        .map(|b| asset_store::hash_bytes(&b))
        .map_err(|_| "LIBRARY_OPERATION_FAILED".into())
}

pub(crate) fn audit(store: &Store, expected: u64) -> Result<Value, String> {
    crate::library_archive::recover(store)?;
    let _guard = store.lock()?;
    Ok(audit_locked(store, expected)?.0)
}

pub(crate) fn pointer(store: &Store) -> Result<Pointer, String> {
    let (root, id, revision, legacy) = asset_store::library_config(store)?;
    Ok(Pointer {
        root: portable_store::external_path(&root),
        id: (!legacy).then_some(id),
        revision,
    })
}

pub(crate) fn audit_locked(store: &Store, expected: u64) -> Result<(Value, Vec<File>), String> {
    let library = pointer(store)?;
    let revision = library.revision;
    if revision != expected {
        return Err("LIBRARY_REVISION_CONFLICT".into());
    }
    library::local_path(&library.root)?;
    let dir: PathBuf = asset_store::path(store, "assets")?;
    let files = inventory(&dir, false)?;
    let ids = files.iter().map(|file| &file.name[..64]).collect();
    let mut scan = References {
        root: &store.root,
        ids,
        used: BTreeSet::new(),
        records: Vec::new(),
        bytes: 0,
        directories: 0,
    };
    scan.scan()?;
    let candidates: Vec<_> = files
        .iter()
        .filter(|file| !scan.used.contains(&file.name[..64]))
        .collect();
    let audit_hash = digest(&AuditInput {
        library,
        files: &files,
        references: digest(&scan.records)?,
    })?;
    Ok((
        json!({"schemaVersion":1,"available":true,"blocked":false,"revision":revision,"auditHash":audit_hash,
        "candidateCount":candidates.len()/2,"candidateBytes":candidates.iter().map(|f|f.bytes).sum::<usize>(),
        "protectedCount":(files.len()-candidates.len())/2,"referenceFiles":scan.records.len(),
        "policy":"explicit verified archive; history, previews, receipts, exports, backups and offline target profiles protected; archives retained; no permanent deletion"}),
        candidates.into_iter().cloned().collect(),
    ))
}
