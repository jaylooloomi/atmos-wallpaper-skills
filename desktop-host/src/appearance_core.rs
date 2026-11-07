//! Native implementation of the existing preview/apply store protocol. File
//! names, fingerprints, watermark ordering, retained history and receipts remain
//! compatible with portable-core.mjs and the shared owner lock.
use crate::{
    asset_store::{self, hash_text},
    desired_state,
    js_json::{self, object as obj, Value},
    portable_store::{self, Lock, Store},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::{Path, PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn timestamp_ms(text: &str) -> Option<i128> {
    OffsetDateTime::parse(text, &Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp_nanos() / 1_000_000)
}
pub fn iso_now() -> String {
    let t = OffsetDateTime::from_unix_timestamp_nanos(crate::journal::now_ms() as i128 * 1_000_000)
        .unwrap();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.year(),
        t.month() as u8,
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.millisecond()
    )
}
pub fn request_id(v: &Value, max: usize) -> Result<&str, String> {
    v.as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= max
                && s.as_bytes()[0].is_ascii_alphanumeric()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
        .ok_or("INVALID_REQUEST_ID".into())
}
pub fn display_id(s: &str) -> bool {
    s.len() == 72
        && s.starts_with("display-")
        && s.as_bytes()[8..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}
pub fn observed_none() -> Value {
    obj([
        ("source", "none".into()),
        ("rendererRevision", Value::Null),
        ("hostVisibility", "unknown".into()),
    ])
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
        Value::String(s) => !s.units.is_empty(),
        _ => true,
    }
}
pub(crate) struct Receipt {
    pub(crate) id: String,
    pub(crate) fingerprint: String,
}
pub struct Core {
    pub store: Store,
    owner: Option<Store>,
}
impl Core {
    pub fn open(root: &Path) -> Result<Self, String> {
        let core = Self {
            store: Store::new(root)?,
            owner: None,
        };
        core.migrate()?;
        Ok(core)
    }
    pub fn owner(&self) -> &Store {
        self.owner.as_ref().unwrap_or(&self.store)
    }
    pub fn lock(&self) -> Result<Lock, String> {
        self.owner().lock()
    }
    pub fn path(&self, name: &str) -> Result<PathBuf, String> {
        if name == "assets" || name.starts_with("assets/") {
            return asset_store::path(self.owner(), name);
        }
        if self.owner.is_some() && (name == "imports" || name.starts_with("imports/")) {
            return self.owner().path(name);
        }
        self.store.path(name)
    }
    pub fn read(&self, name: &str) -> Result<Value, String> {
        let bytes = asset_store::read_bytes(&self.path(name)?, 16 * 1048576, "CORRUPT_STORE")?;
        js_json::parse(&bytes).map_err(|_| "CORRUPT_STORE".into())
    }
    pub fn atomic(&self, name: &str, value: &Value) -> Result<(), String> {
        let mut bytes = value.canonical().into_bytes();
        bytes.push(b'\n');
        portable_store::atomic_file(&self.path(name)?, &bytes)
    }
    fn validate_live(&self, s: &Value, current: bool) -> Result<(), String> {
        if s.as_object().is_none() {
            return Err("CORRUPT_STORE".into());
        }
        if s["schemaVersion"].as_num() != Some(1.0) {
            return Err("STORE_VERSION_UNSUPPORTED".into());
        }
        if s["revision"].safe_uint().is_none()
            || s["history"].as_array().is_none()
            || s["receipts"].as_object().is_none()
        {
            return Err("CORRUPT_STORE".into());
        }
        if current {
            desired_state::validate_current(&s["desired"])?;
        } else {
            desired_state::validate(&s["desired"])?;
        }
        if s["desiredHash"].as_str() != Some(&s["desired"].hash()) {
            return Err("CORRUPT_STORE".into());
        }
        Ok(())
    }
    pub fn live(&self) -> Result<Value, String> {
        let s = self.read("live.json")?;
        self.validate_live(&s, true)?;
        Ok(s)
    }
    pub fn init(&self) -> Result<(), String> {
        let _guard = self.lock()?;
        if self.path("live.json")?.exists() {
            self.live()?;
            return Ok(());
        }
        if self.path("revision.json")?.exists() {
            return Err("RECOVERY_REQUIRED".into());
        }
        let desired = desired_state::initial();
        self.atomic("revision.json", &obj([("last", 0.into())]))?;
        self.atomic(
            "live.json",
            &obj([
                ("schemaVersion", 1.into()),
                ("revision", 0.into()),
                ("desiredHash", desired.hash().into()),
                ("desired", desired),
                ("history", Value::Array(vec![])),
                ("receipts", obj([])),
            ]),
        )
    }
    pub fn migration_notice(&self) -> Result<Value, String> {
        if self.path("compatibility-notice.json")?.exists() {
            self.read("compatibility-notice.json")
        } else {
            Ok(Value::Null)
        }
    }
    fn record_migration(&self, kind: &str, old: &Value, raw: Option<&[u8]>) -> Result<(), String> {
        let name = format!(
            "backups/migration-before-tornado-{kind}-{}.json",
            &old.hash()[..24]
        );
        if !self.path(&name)?.exists() {
            if let Some(bytes) = raw {
                portable_store::atomic_file(&self.path(&name)?, bytes)?;
            } else {
                self.atomic(&name, old)?;
            }
        }
        let tornado = if old.has("desired") {
            &old["desired"]
        } else {
            old
        }["visual"]["effect"]
            .as_str()
            == Some("tornado");
        let defaults = js_json::parse(include_bytes!("cli-catalogs/state-defaults.json")).unwrap();
        self.atomic(
            "compatibility-notice.json",
            &obj([
                (
                    "reason",
                    if tornado {
                        "TORNADO_REMOVED"
                    } else {
                        "DESIRED_FORMAT_UPDATED"
                    }
                    .into(),
                ),
                ("kind", kind.into()),
                ("backup", name.into()),
                (
                    "text",
                    defaults["migrationTexts"][if tornado { "tornado" } else { "other" }].clone(),
                ),
            ]),
        )
    }
    fn migrate(&self) -> Result<(), String> {
        if !self.path("live.json")?.exists() {
            return Ok(());
        }
        let first = match self.read("live.json") {
            Ok(v) => v,
            Err(e) if ["CORRUPT_STORE", "ENOENT"].contains(&e.as_str()) => return Ok(()),
            Err(e) => return Err(e),
        };
        if first["schemaVersion"].as_num() != Some(1.0)
            || first["desired"]["formatVersion"].as_num() == Some(4.0)
        {
            return Ok(());
        }
        let _guard = self.lock()?;
        let old = self.read("live.json")?;
        if old["desired"]["formatVersion"].as_num() == Some(4.0) {
            return Ok(());
        }
        if let Err(e) = self.validate_live(&old, false) {
            if [
                "CORRUPT_STORE",
                "INVALID_STATE",
                "UNKNOWN_FIELD",
                "STORE_VERSION_UNSUPPORTED",
            ]
            .contains(&e.as_str())
            {
                return Ok(());
            }
            return Err(e);
        }
        let desired = desired_state::normalize(&old["desired"])?;
        let raw = asset_store::read_bytes(&self.path("live.json")?, 16 * 1048576, "CORRUPT_STORE")?;
        self.record_migration("live", &old, Some(&raw))?;
        let id = format!(
            "migrate-config-{}-{}",
            old["revision"].safe_uint().unwrap(),
            &old["desiredHash"].as_str().unwrap()[..32]
        );
        let fp = obj([
            ("migration", "p4.2.3-desired-format4".into()),
            ("revision", old["revision"].clone()),
            ("desiredHash", old["desiredHash"].clone()),
        ])
        .hash();
        self.commit(
            &old,
            &desired,
            &Receipt {
                id,
                fingerprint: fp,
            },
        )?;
        Ok(())
    }
    pub(crate) fn receipt(
        &self,
        id: &str,
        fingerprint: &str,
        s: &Value,
    ) -> Result<Option<Value>, String> {
        let archived = format!("requests/{}.json", hash_text(id));
        let receipt = if let Some(r) = s["receipts"].get(id) {
            Some(r.clone())
        } else if self.path(&archived)?.exists() {
            Some(self.read(&archived)?)
        } else {
            None
        };
        if let Some(r) = receipt {
            if r["fingerprint"].as_str() != Some(fingerprint) {
                return Err("REQUEST_CONFLICT".into());
            }
            if truthy(&r["uncertain"]) {
                return Err("RECOVERED_OUTCOME_UNKNOWN".into());
            }
            let mut result = r["result"].clone();
            if result.as_object().is_none() {
                return Err("CORRUPT_STORE".into());
            }
            result.set("replayed", true.into());
            return Ok(Some(result));
        }
        Ok(None)
    }
    fn mutate(
        &self,
        command: &str,
        args: &Value,
        f: impl FnOnce(&Value, &Receipt) -> Result<Value, String>,
    ) -> Result<Value, String> {
        let id = request_id(&args["requestId"], 96)?.to_string();
        let fp = obj([("command", command.into()), ("args", args.clone())]).hash();
        let _guard = self.lock()?;
        let live = self.live()?;
        if let Some(r) = self.receipt(&id, &fp, &live)? {
            return Ok(r);
        }
        f(
            &live,
            &Receipt {
                id,
                fingerprint: fp,
            },
        )
    }
    fn save_receipt(&self, r: &Receipt, result: Value) -> Result<Value, String> {
        self.atomic(
            &format!("requests/{}.json", hash_text(&r.id)),
            &obj([
                ("fingerprint", r.fingerprint.clone().into()),
                ("result", result.clone()),
            ]),
        )?;
        Ok(result)
    }
    pub(crate) fn assert_revision(&self, s: &Value, expected: &Value) -> Result<(), String> {
        let r = expected.safe_uint().ok_or("INVALID_REVISION")?;
        if s["revision"].safe_uint() != Some(r) {
            return Err("REVISION_CONFLICT".into());
        }
        Ok(())
    }
    pub(crate) fn commit(
        &self,
        s: &Value,
        desired: &Value,
        receipt: &Receipt,
    ) -> Result<Value, String> {
        let counter = self.read("revision.json")?;
        let last = counter["last"]
            .safe_uint()
            .filter(|r| *r >= s["revision"].safe_uint().unwrap())
            .ok_or("CORRUPT_STORE")?;
        let next_revision = last
            .checked_add(1)
            .filter(|r| *r <= portable_store::MAX_SAFE_INTEGER)
            .ok_or("REVISION_LIMIT")?;
        self.atomic("revision.json", &obj([("last", next_revision.into())]))?;
        let hash = desired.hash();
        let result = obj([
            ("revision", next_revision.into()),
            ("desiredHash", hash.clone().into()),
            ("observed", observed_none()),
        ]);
        let mut history = s["history"].as_array().unwrap().clone();
        history.push(obj([
            ("revision", s["revision"].clone()),
            ("desired", s["desired"].clone()),
            ("desiredHash", s["desiredHash"].clone()),
        ]));
        if history.len() > 20 {
            history.drain(..history.len() - 20);
        }
        let mut receipts = s["receipts"].clone();
        receipts.set(
            &receipt.id,
            obj([
                ("fingerprint", receipt.fingerprint.clone().into()),
                ("result", result.clone()),
            ]),
        );
        if receipts.as_object().unwrap().len() > 10000 {
            return Err("RECEIPT_LIMIT".into());
        }
        let next = obj([
            ("schemaVersion", 1.into()),
            ("revision", next_revision.into()),
            ("desired", desired.clone()),
            ("desiredHash", hash.into()),
            ("history", Value::Array(history)),
            ("receipts", receipts),
        ]);
        self.atomic(
            &format!("commit-intents/{}.json", hash_text(&receipt.id)),
            &obj([
                ("id", receipt.id.clone().into()),
                ("fingerprint", receipt.fingerprint.clone().into()),
                ("targetRevision", next_revision.into()),
                ("result", result.clone()),
            ]),
        )?;
        self.atomic("live.backup.json", s)?;
        self.atomic("live.json", &next)?;
        Ok(result)
    }
    pub fn asset(&self, id: &Value) -> Result<(Value, Vec<u8>), String> {
        if !desired_state::hex_id(id) {
            return Err("INVALID_ASSET".into());
        }
        let id = id.as_str().unwrap();
        let result = (|| {
            let meta = self.read(&format!("assets/{id}.json"))?;
            let bytes = asset_store::read_bytes(
                &self.path(&format!("assets/{id}.png"))?,
                16 * 1048576,
                "ASSET_CORRUPT",
            )?;
            if meta["assetId"].as_str() != Some(id)
                || asset_store::hash_bytes(&bytes) != id
                || meta["bytes"].as_num() != Some(bytes.len() as f64)
            {
                return Err("ASSET_CORRUPT".into());
            }
            Ok((meta, bytes))
        })();
        result.map_err(|e: String| {
            if e == "ENOENT" {
                "ASSET_MISSING".into()
            } else {
                e
            }
        })
    }
    #[cfg(feature = "native-core")]
    pub(crate) fn stage(
        &self,
        args: &Value,
        decoder: &crate::backup_image::Decoder,
    ) -> Result<Value, String> {
        request_id(&args["requestId"], 96)?;
        let name = args["file"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or("PATH_REJECTED")?;
        let input = self.path(&format!("imports/{name}"))?;
        let bytes =
            asset_store::read_bytes(&input, 8 * 1048576, "IMAGE_REJECTED").map_err(|e| {
                if e == "ENOENT" || e == "LIBRARY_PATH_UNSAFE" {
                    "PATH_REJECTED".into()
                } else {
                    e
                }
            })?;
        let source_hash = asset_store::hash_bytes(&bytes);
        let intent = obj([
            ("requestId", args["requestId"].clone()),
            ("file", name.into()),
            ("sourceHash", source_hash.clone().into()),
        ]);
        self.mutate("stage", &intent, |live, receipt| {
            let (normalized, width, height) = decoder.decode(&bytes, false)?;
            let id = asset_store::hash_bytes(&normalized);
            let metadata = obj([
                ("assetId", id.clone().into()),
                ("sourceHash", source_hash.into()),
                ("bytes", (normalized.len() as u64).into()),
                ("width", width.into()),
                ("height", height.into()),
                ("format", "PNG".into()),
            ]);
            let image_path = self.path(&format!("assets/{id}.png"))?;
            if !image_path.exists() {
                portable_store::atomic_new_file(&image_path, &normalized)?;
            }
            let metadata_path = format!("assets/{id}.json");
            if !self.path(&metadata_path)?.exists() {
                self.atomic(&metadata_path, &metadata)?;
            }
            self.asset(&id.into())?;
            self.save_receipt(
                receipt,
                obj([
                    ("asset", metadata),
                    ("liveRevision", live["revision"].clone()),
                    ("liveUnchanged", true.into()),
                ]),
            )
        })
    }
    pub fn preview(&self, args: &Value) -> Result<Value, String> {
        self.mutate("preview", args, |s, r| {
            self.assert_revision(s, &args["expectedRevision"])?;
            let desired = desired_state::merge(&s["desired"], &args["patch"])?;
            if !desired["assetId"].is_null() {
                self.asset(&desired["assetId"])?;
            }
            let id = obj([
                ("id", r.id.clone().into()),
                ("fingerprint", r.fingerprint.clone().into()),
            ])
            .hash();
            let candidate = desired.hash();
            let name = format!("previews/{id}.json");
            if self.path(&name)?.exists() {
                let old = self.read(&name)?;
                if old["candidateHash"].as_str() != Some(&candidate)
                    || old["baseRevision"] != s["revision"]
                {
                    return Err("PREVIEW_CONFLICT".into());
                }
            } else {
                self.atomic(
                    &name,
                    &obj([
                        ("schemaVersion", 1.into()),
                        ("previewId", id.clone().into()),
                        ("baseRevision", s["revision"].clone()),
                        ("baseHash", s["desiredHash"].clone()),
                        ("desired", desired),
                        ("candidateHash", candidate.clone().into()),
                        ("createdAt", iso_now().into()),
                        ("expiresAt", (crate::journal::now_ms() + 1800000).into()),
                        ("status", "ready".into()),
                    ]),
                )?;
            }
            self.save_receipt(
                r,
                obj([
                    ("previewId", id.into()),
                    ("baseRevision", s["revision"].clone()),
                    ("candidateHash", candidate.into()),
                    ("liveUnchanged", true.into()),
                    ("observed", observed_none()),
                ]),
            )
        })
    }
    pub fn read_preview(&self, id: &Value) -> Result<Value, String> {
        if !desired_state::hex_id(id) {
            return Err("INVALID_PREVIEW".into());
        }
        let id = id.as_str().unwrap();
        let p = self.read(&format!("previews/{id}.json")).map_err(|e| {
            if e == "ENOENT" {
                "PREVIEW_MISSING".into()
            } else {
                e
            }
        })?;
        desired_state::validate_current(&p["desired"])?;
        if p["baseRevision"].safe_uint().is_none()
            || !desired_state::hex_id(&p["baseHash"])
            || p["expiresAt"].safe_uint().is_none()
            || !p["status"]
                .as_str()
                .is_some_and(|s| ["ready", "cancelled"].contains(&s))
            || !p["createdAt"]
                .as_str()
                .is_some_and(|s| timestamp_ms(s).is_some())
            || p["schemaVersion"].as_num() != Some(1.0)
            || p["previewId"].as_str() != Some(id)
            || p["candidateHash"].as_str() != Some(&p["desired"].hash())
        {
            return Err("PREVIEW_CORRUPT".into());
        }
        Ok(p)
    }
    pub fn apply(&self, args: &Value) -> Result<Value, String> {
        self.mutate("apply", args, |s, r| {
            self.assert_revision(s, &args["expectedRevision"])?;
            let p = self.read_preview(&args["previewId"])?;
            if p["status"].as_str() != Some("ready") {
                return Err("PREVIEW_CANCELLED".into());
            }
            if crate::journal::now_ms() > p["expiresAt"].safe_uint().unwrap() {
                return Err("PREVIEW_EXPIRED".into());
            }
            if p["baseRevision"] != s["revision"] || p["baseHash"] != s["desiredHash"] {
                return Err("PREVIEW_STALE".into());
            }
            if !p["desired"]["assetId"].is_null() {
                self.asset(&p["desired"]["assetId"])?;
            }
            self.commit(s, &p["desired"], r)
        })
    }
    pub fn cancel(&self, args: &Value) -> Result<Value, String> {
        self.mutate("cancel", args, |live, receipt| {
            let mut preview = self.read_preview(&args["previewId"])?;
            let id = preview["previewId"].as_str().unwrap().to_owned();
            preview.set("status", "cancelled".into());
            self.atomic(&format!("previews/{id}.json"), &preview)?;
            self.save_receipt(
                receipt,
                obj([
                    ("previewId", id.into()),
                    ("cancelled", true.into()),
                    ("liveRevision", live["revision"].clone()),
                    ("liveUnchanged", true.into()),
                ]),
            )
        })
    }
    pub fn rollback(&self, args: &Value) -> Result<Value, String> {
        self.mutate("rollback", args, |live, receipt| {
            self.assert_revision(live, &args["expectedRevision"])?;
            let revision = args["targetRevision"]
                .safe_uint()
                .ok_or("INVALID_REVISION")?;
            let old = live["history"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["revision"].safe_uint() == Some(revision))
                .ok_or("HISTORY_MISSING")?;
            if old["desired"].hash() != old["desiredHash"].as_str().unwrap_or("") {
                return Err("CORRUPT_STORE".into());
            }
            let desired = desired_state::normalize(&old["desired"])?;
            if !desired["assetId"].is_null() {
                self.asset(&desired["assetId"])?;
            }
            if desired != old["desired"] {
                self.record_migration("rollback", old, None)?;
            }
            self.commit(live, &desired, receipt)
        })
    }
    pub fn recover(&self, args: &Value) -> Result<Value, String> {
        let id = request_id(&args["requestId"], 96)?.to_owned();
        let fingerprint = obj([
            ("command", "recover".into()),
            ("requestId", id.clone().into()),
        ])
        .hash();
        let _guard = self.lock()?;
        match self.live() {
            Ok(live) => {
                if let Some(mut previous) = self.receipt(&id, &fingerprint, &live)? {
                    previous.set("recovered", true.into());
                    return Ok(previous);
                }
                return Ok(obj([
                    ("recovered", false.into()),
                    ("schemaVersion", 1.into()),
                    ("revision", live["revision"].clone()),
                    ("desiredHash", live["desiredHash"].clone()),
                    ("desired", live["desired"].clone()),
                    ("observed", observed_none()),
                ]));
            }
            Err(e)
                if matches!(
                    e.as_str(),
                    "CORRUPT_STORE" | "ENOENT" | "INVALID_STATE" | "UNKNOWN_FIELD"
                ) => {}
            Err(e) => return Err(e),
        }
        let mut backup = self.read("live.backup.json")?;
        self.validate_live(&backup, false)?;
        let desired = desired_state::normalize(&backup["desired"])?;
        if !desired["assetId"].is_null() {
            self.asset(&desired["assetId"])?;
        }
        let recovered_from = backup["revision"].safe_uint().unwrap();
        let last = self.read("revision.json")?["last"]
            .safe_uint()
            .filter(|last| *last >= recovered_from)
            .ok_or("CORRUPT_STORE")?;
        let mut receipts = backup["receipts"].clone();
        for item in std::fs::read_dir(self.path("commit-intents")?).map_err(|_| "CORRUPT_STORE")? {
            let name = item.map_err(|_| "CORRUPT_STORE")?.file_name();
            let name = name.to_str().ok_or("CORRUPT_STORE")?;
            let intent = self.read(&format!("commit-intents/{name}"))?;
            let target = intent["targetRevision"]
                .safe_uint()
                .ok_or("CORRUPT_STORE")?;
            if target > recovered_from && target <= last {
                let intent_id = request_id(&intent["id"], 96)?;
                if !desired_state::hex_id(&intent["fingerprint"]) {
                    return Err("CORRUPT_STORE".into());
                }
                receipts.set(
                    intent_id,
                    obj([
                        ("fingerprint", intent["fingerprint"].clone()),
                        ("uncertain", true.into()),
                    ]),
                );
            }
        }
        backup.set("receipts", receipts);
        if self.receipt(&id, &fingerprint, &backup)?.is_some() {
            return Err("REQUEST_CONFLICT".into());
        }
        if let Some(raw) = self.store.read_bytes("live.json")? {
            self.store.atomic_bytes(
                &format!("recovery/corrupt-{}.bin", crate::journal::now_ms()),
                &raw,
            )?;
        }
        if desired != backup["desired"] {
            self.record_migration("recovery", &backup, None)?;
        }
        let mut result = self.commit(&backup, &desired, &Receipt { id, fingerprint })?;
        result.set("recovered", true.into());
        result.set("recoveredFromRevision", recovered_from.into());
        Ok(result)
    }

    pub fn backup(&self) -> Result<Value, String> {
        let _guard = self.lock()?;
        let live = self.live()?;
        let id = &live["desired"]["assetId"];
        let assets = if id.is_null() {
            vec![]
        } else {
            let (meta, bytes) = self.asset(id)?;
            vec![obj([
                ("assetId", id.clone()),
                ("sha256", id.clone()),
                ("format", "PNG".into()),
                ("width", meta["width"].clone()),
                ("height", meta["height"].clone()),
                ("byteLength", (bytes.len() as u64).into()),
                ("base64", STANDARD.encode(bytes).into()),
            ])]
        };
        let count = assets.len();
        let mut body = obj([
            ("schemaVersion", 1.into()),
            ("kind", "atmos-backup".into()),
            ("sourceRevision", live["revision"].clone()),
            ("desired", live["desired"].clone()),
            ("desiredHash", live["desiredHash"].clone()),
            ("assets", Value::Array(assets)),
        ]);
        let hash = body.hash();
        body.set("backupHash", hash.clone().into());
        let name = format!("backups/{hash}.json");
        self.atomic(&name, &body)?;
        Ok(obj([
            ("backupHash", hash.into()),
            (
                "path",
                portable_store::external_path(&self.path(&name)?).into(),
            ),
            ("sourceRevision", live["revision"].clone()),
            ("assetCount", (count as u64).into()),
            ("liveUnchanged", true.into()),
            ("observedIncluded", false.into()),
        ]))
    }

    /// A bounded JSON reference keeps large backup bytes outside the HTTP body.
    /// Imported bundles are staged under imports/<backupHash>.json by the caller.
    pub fn restore_local_backup(
        &self,
        args: &Value,
        imported: bool,
        decoder: &crate::backup_image::Decoder,
    ) -> Result<Value, String> {
        let hash = &args["backupHash"];
        if !desired_state::hex_id(hash) {
            return Err("INVALID_BACKUP".into());
        }
        let dir = if imported { "imports" } else { "backups" };
        let path = self.path(&format!("{dir}/{}.json", hash.as_str().unwrap()))?;
        let bytes = asset_store::read_bytes(&path, 24 * 1048576, "INVALID_BACKUP")?;
        let mut backup = js_json::parse(&bytes).map_err(|_| "INVALID_BACKUP")?;
        let fields = [
            "schemaVersion",
            "kind",
            "sourceRevision",
            "desired",
            "desiredHash",
            "assets",
            "backupHash",
        ];
        if !backup.as_object().is_some_and(|o| o.len() == fields.len())
            || !fields.iter().all(|k| backup.has(k))
            || backup["kind"].as_str() != Some("atmos-backup")
            || backup["backupHash"] != *hash
            || backup["sourceRevision"].safe_uint().is_none()
        {
            return Err("INVALID_BACKUP".into());
        }
        if backup["schemaVersion"].safe_uint() != Some(1) {
            return Err("BACKUP_VERSION_UNSUPPORTED".into());
        }
        backup.remove("backupHash");
        if backup.hash() != hash.as_str().unwrap()
            || backup["desired"].hash() != backup["desiredHash"].as_str().unwrap_or("")
        {
            return Err("BACKUP_CORRUPT".into());
        }
        let desired = desired_state::normalize(&backup["desired"])?;
        let migrated = desired != backup["desired"];
        let asset_id = desired["assetId"].clone();
        let id = &asset_id;
        let assets = backup["assets"].as_array().ok_or("INVALID_BACKUP")?;
        if assets.len() != usize::from(!id.is_null()) {
            return Err("INVALID_BACKUP".into());
        }
        self.mutate("restore", args, |live, receipt| {
            self.assert_revision(live, &args["expectedRevision"])?;
            if let Some(asset) = assets.first() {
                let fields = [
                    "assetId",
                    "sha256",
                    "format",
                    "width",
                    "height",
                    "byteLength",
                    "base64",
                ];
                if !asset.as_object().is_some_and(|o| o.len() == fields.len())
                    || !fields.iter().all(|k| asset.has(k))
                    || asset["assetId"] != *id
                    || asset["sha256"] != *id
                    || asset["format"].as_str() != Some("PNG")
                {
                    return Err("INVALID_BACKUP".into());
                }
                let encoded = asset["base64"].as_str().ok_or("INVALID_BACKUP")?;
                let data = STANDARD.decode(encoded).map_err(|_| "BACKUP_CORRUPT")?;
                if data.len() > 16 * 1048576
                    || asset["byteLength"].safe_uint() != Some(data.len() as u64)
                    || STANDARD.encode(&data) != encoded
                    || asset_store::hash_bytes(&data) != id.as_str().unwrap()
                {
                    return Err("BACKUP_CORRUPT".into());
                }
                let dimension = |key| {
                    asset[key]
                        .safe_uint()
                        .filter(|n| (1..=4096).contains(n))
                        .ok_or("INVALID_BACKUP")
                };
                decoder.verify(&data, dimension("width")?, dimension("height")?)?;
                let id_text = id.as_str().unwrap();
                let image_name = format!("assets/{id_text}.png");
                let image_path = self.path(&image_name)?;
                if !image_path.exists() {
                    portable_store::atomic_file(&image_path, &data)?;
                }
                let meta_name = format!("assets/{id_text}.json");
                if !self.path(&meta_name)?.exists() {
                    self.atomic(
                        &meta_name,
                        &obj([
                            ("assetId", id.clone()),
                            ("sourceHash", id.clone()),
                            ("format", "PNG".into()),
                            ("bytes", asset["byteLength"].clone()),
                            ("width", asset["width"].clone()),
                            ("height", asset["height"].clone()),
                        ]),
                    )?;
                }
                let (meta, existing) = self.asset(id)?;
                if existing != data
                    || meta["width"] != asset["width"]
                    || meta["height"] != asset["height"]
                {
                    return Err("BACKUP_CORRUPT".into());
                }
            }
            if migrated {
                self.record_migration("backup", &backup, None)?;
            }
            let preview_id = obj([
                ("id", receipt.id.clone().into()),
                ("fingerprint", receipt.fingerprint.clone().into()),
            ])
            .hash();
            let candidate = desired.hash();
            self.atomic(
                &format!("previews/{preview_id}.json"),
                &obj([
                    ("schemaVersion", 1.into()),
                    ("previewId", preview_id.clone().into()),
                    ("baseRevision", live["revision"].clone()),
                    ("baseHash", live["desiredHash"].clone()),
                    ("desired", desired),
                    ("candidateHash", candidate.clone().into()),
                    ("createdAt", iso_now().into()),
                    ("expiresAt", (crate::journal::now_ms() + 1800000).into()),
                    ("status", "ready".into()),
                ]),
            )?;
            self.save_receipt(
                receipt,
                obj([
                    ("previewId", preview_id.into()),
                    ("baseRevision", live["revision"].clone()),
                    ("candidateHash", candidate.into()),
                    ("restoredFromRevision", backup["sourceRevision"].clone()),
                    ("restorePreview", true.into()),
                    ("liveUnchanged", true.into()),
                    ("migrated", migrated.into()),
                    ("schemaMigrated", false.into()),
                    ("normalizedLegacy", migrated.into()),
                    (
                        "migrationNotice",
                        if migrated {
                            self.migration_notice()?
                        } else {
                            Value::Null
                        },
                    ),
                    ("schemaVersion", 1.into()),
                    ("observed", observed_none()),
                ]),
            )
        })
    }
    pub fn command_patch(command: &Value) -> Result<Value, String> {
        let fields = [
            "schemaVersion",
            "requestId",
            "expectedRevision",
            "type",
            "changes",
        ];
        if !command.as_object().is_some_and(|o| {
            o.keys().all(|k| {
                String::from_utf16(k)
                    .ok()
                    .is_some_and(|s| fields.contains(&s.as_str()))
            })
        }) || command["schemaVersion"].as_num() != Some(1.0)
            || command["type"].as_str() != Some("settings.update")
        {
            return Err("COMMAND_INVALID".into());
        }
        request_id(&command["requestId"], 80)?;
        command["expectedRevision"]
            .safe_uint()
            .ok_or("INVALID_REVISION")?;
        let changes = command["changes"]
            .as_object()
            .filter(|o| !o.is_empty())
            .ok_or("COMMAND_INVALID")?;
        if changes.iter().any(|(k, v)| {
            !String::from_utf16(k)
                .ok()
                .is_some_and(|s| ["visual", "widgets"].contains(&s.as_str()))
                || !v.as_object().is_some_and(|o| !o.is_empty())
        }) {
            return Err("COMMAND_INVALID".into());
        }
        let mut patch = command["changes"].clone();
        if patch["visual"].has("background") {
            patch.set("assetId", Value::Null);
        }
        Ok(patch)
    }
    pub fn execute_command(&self, command: &Value) -> Result<Value, String> {
        let patch = Self::command_patch(command)?;
        let id = command["requestId"].as_str().unwrap();
        let preview = self.preview(&obj([
            ("requestId", format!("ui.{id}.preview").into()),
            ("expectedRevision", command["expectedRevision"].clone()),
            ("patch", patch),
        ]))?;
        self.apply(&obj([
            ("requestId", format!("ui.{id}.apply").into()),
            ("expectedRevision", command["expectedRevision"].clone()),
            ("previewId", preview["previewId"].clone()),
        ]))
    }
    pub fn validate_command(&self, command: &Value) -> Result<Value, String> {
        let patch = Self::command_patch(command)?;
        let _guard = self.lock()?;
        let live = self.live()?;
        self.assert_revision(&live, &command["expectedRevision"])?;
        desired_state::merge(&live["desired"], &patch)?;
        Ok(obj([("apiVersion",1.into()),("ok",true.into()),("valid",true.into()),("committed",false.into()),("requestId",command["requestId"].clone()),("revision",live["revision"].clone()),("note","Merged-state and revision validation only; not reserved and not a receipt replay check.".into())]))
    }
    pub fn ensure_scope(&self, target: &str) -> Result<Core, String> {
        if !display_id(target) {
            return Err("TARGET_INVALID".into());
        }
        let root = self.store.path(&format!("targets/{target}"))?;
        let key = hash_text(&self.store.external_root());
        let marker = root.join("scope.json");
        if marker.exists() {
            let m = asset_store::read_json(&marker, 16384).map_err(|_| "SCOPE_OWNER_MISMATCH")?;
            if m["schemaVersion"].as_num() != Some(1.0)
                || m["targetId"].as_str() != Some(target)
                || m["ownerKey"].as_str() != Some(&key)
            {
                return Err("SCOPE_OWNER_MISMATCH".into());
            }
        }
        let child = Core {
            store: Store::new(Path::new(&portable_store::external_path(&root)))?,
            owner: Some(Store {
                root: self.store.root.clone(),
            }),
        };
        child.migrate()?;
        child.init()?;
        if marker.exists() {
            return Ok(child);
        }
        {
            let _guard = self.lock()?;
            let intent = if child.path("scope-seed-intent.json")?.exists() {
                let i = child.read("scope-seed-intent.json")?;
                desired_state::validate_current(&i["desired"]).map_err(|_| "SCOPE_SEED_INVALID")?;
                if i["schemaVersion"].as_num() != Some(1.0)
                    || i["targetId"].as_str() != Some(target)
                    || i["ownerKey"].as_str() != Some(&key)
                    || i["desiredHash"].as_str() != Some(&i["desired"].hash())
                {
                    return Err("SCOPE_SEED_INVALID".into());
                }
                i
            } else {
                let base = self.live()?;
                let i = obj([
                    ("schemaVersion", 1.into()),
                    ("targetId", target.into()),
                    ("ownerKey", key.clone().into()),
                    ("baseRevision", base["revision"].clone()),
                    ("desired", base["desired"].clone()),
                    ("desiredHash", base["desiredHash"].clone()),
                    ("createdAt", iso_now().into()),
                ]);
                child.atomic("scope-seed-intent.json", &i)?;
                i
            };
            let live = child.live()?;
            let id = format!("scope.seed.{}", &target[8..]);
            let fp = intent.hash();
            if live["revision"].as_num() == Some(0.0) {
                child.commit(
                    &live,
                    &intent["desired"],
                    &Receipt {
                        id: id.clone(),
                        fingerprint: fp.clone(),
                    },
                )?;
            } else if live["receipts"][&id]["fingerprint"].as_str() != Some(&fp) {
                return Err("SCOPE_SEED_UNCERTAIN".into());
            }
            child.atomic(
                "scope.json",
                &obj([
                    ("schemaVersion", 1.into()),
                    ("targetId", target.into()),
                    ("ownerKey", key.into()),
                    ("baseRevision", intent["baseRevision"].clone()),
                    ("desiredHash", intent["desiredHash"].clone()),
                    ("createdAt", intent["createdAt"].clone()),
                ]),
            )?;
        }
        Ok(child)
    }

    pub fn open_existing_scope(&self, target: &str) -> Result<Core, String> {
        if !display_id(target) {
            return Err("TARGET_INVALID".into());
        }
        let root = self.store.path(&format!("targets/{target}"))?;
        let marker = asset_store::read_json(&root.join("scope.json"), 16384)
            .map_err(|_| "SCOPE_OWNER_MISMATCH")?;
        if marker["schemaVersion"].safe_uint() != Some(1)
            || marker["targetId"].as_str() != Some(target)
            || marker["ownerKey"].as_str() != Some(&hash_text(&self.store.external_root()))
        {
            return Err("SCOPE_OWNER_MISMATCH".into());
        }
        Ok(Core {
            store: Store { root },
            owner: Some(Store {
                root: self.store.root.clone(),
            }),
        })
    }
}
