//! Compatible appearance-only catalog and tray cycle. The owner lock covers
//! both the catalog and appearance commits, including independent displays.
use crate::{
    appearance_core::{iso_now, request_id, Core, Receipt},
    asset_store::{hash_text, read_bytes},
    desired_state,
    js_json::{self, object as obj, JsString, Value},
};
use std::{collections::BTreeSet, path::Path};

const FILE: &str = "presets.json";
const LIMIT: usize = 524288;
const EXCLUDED: [&str; 5] = [
    "previewMode",
    "previewWidth",
    "previewHeight",
    "locale",
    "calendarMode",
];

pub(crate) fn shape(v: &Value, keys: &[&str]) -> bool {
    v.as_object().is_some_and(|m| m.len() == keys.len()) && keys.iter().all(|k| v.has(k))
}

pub(crate) fn optional(
    core: &Core,
    file: &str,
    max: usize,
    invalid: &str,
) -> Result<Option<Value>, String> {
    match read_bytes(&core.path(file)?, max, invalid) {
        Ok(bytes) => js_json::parse(&bytes).map(Some).map_err(|_| invalid.into()),
        Err(e) if e == "ENOENT" => Ok(None),
        Err(e) => Err(e),
    }
}

fn name(v: &Value) -> Option<Value> {
    let Value::String(s) = v else { return None };
    if s.units.len() > 80 {
        return None;
    }
    // ECMAScript trim operates on UTF-16 and includes the byte-order mark.
    let space = |u: u16| matches!(u, 0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff);
    let start = s.units.iter().position(|u| !space(*u))?;
    let end = s.units.iter().rposition(|u| !space(*u)).unwrap() + 1;
    Some(Value::String(JsString::new(s.units[start..end].to_vec())))
}

fn appearance(desired: &Value) -> Value {
    let mut visual = desired["visual"].clone();
    for key in EXCLUDED {
        visual.remove(key);
    }
    obj([
        ("visual", visual),
        ("assetId", desired["assetId"].clone()),
        ("cards", desired["widgets"]["cards"].clone()),
        ("desktopZone", desired["widgets"]["desktopZone"].clone()),
    ])
}

fn patch(v: &Value) -> Result<Value, String> {
    if !shape(v, &["assetId", "cards", "desktopZone", "visual"])
        || v["visual"].as_object().is_none()
        || EXCLUDED.iter().any(|k| v["visual"].has(k))
    {
        return Err("PRESET_INVALID".into());
    }
    let p = obj([
        ("visual", v["visual"].clone()),
        ("assetId", v["assetId"].clone()),
        (
            "widgets",
            obj([
                ("cards", v["cards"].clone()),
                ("desktopZone", v["desktopZone"].clone()),
            ]),
        ),
    ]);
    let normalized = desired_state::merge(&desired_state::initial(), &p)?;
    if appearance(&normalized) != *v {
        return Err("PRESET_INVALID".into());
    }
    Ok(p)
}

pub(crate) fn catalog(core: &Core) -> Result<Value, String> {
    let c = optional(core, FILE, LIMIT, "PRESET_CATALOG_CORRUPT")?.unwrap_or_else(|| {
        obj([
            ("schemaVersion", 1.into()),
            ("revision", 0.into()),
            ("presets", Value::Array(vec![])),
            ("receipts", obj([])),
        ])
    });
    let presets = c["presets"]
        .as_array()
        .filter(|v| v.len() <= 40)
        .ok_or("PRESET_CATALOG_CORRUPT")?;
    if !shape(&c, &["schemaVersion", "revision", "presets", "receipts"])
        || c["schemaVersion"].safe_uint() != Some(1)
        || c["revision"].safe_uint().is_none()
        || c["receipts"].as_object().is_none()
    {
        return Err("PRESET_CATALOG_CORRUPT".into());
    }
    let mut ids = BTreeSet::new();
    for p in presets {
        if !shape(p, &["appearance", "createdAt", "id", "name"])
            || !desired_state::hex_id(&p["id"])
            || !ids.insert(p["id"].as_str().unwrap())
            || name(&p["name"]).is_none()
            || p["createdAt"]
                .as_str()
                .and_then(crate::appearance_core::timestamp_ms)
                .is_none()
        {
            return Err("PRESET_CATALOG_CORRUPT".into());
        }
        patch(&p["appearance"])?;
    }
    Ok(c)
}

pub(crate) fn status(core: &Core) -> Result<Value, String> {
    let c = catalog(core)?;
    let presets = c["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let v = &p["appearance"]["visual"];
            obj([
                ("id", p["id"].clone()),
                ("name", p["name"].clone()),
                ("createdAt", p["createdAt"].clone()),
                ("background", v["background"].clone()),
                ("effect", v["effect"].clone()),
                ("theme", v["theme"].clone()),
            ])
        })
        .collect();
    Ok(obj([
        ("schemaVersion", 1.into()),
        ("catalogRevision", c["revision"].clone()),
        ("currentRevision", core.live()?["revision"].clone()),
        ("presets", Value::Array(presets)),
        ("scope", "appearance-only".into()),
        ("maxPresets", 40.into()),
    ]))
}

pub(crate) fn find<'a>(c: &'a Value, id: &Value) -> Option<&'a Value> {
    c["presets"].as_array()?.iter().find(|p| p["id"] == *id)
}

pub(crate) fn apply(
    core: &Core,
    live: &Value,
    preset: &Value,
    receipt: &Receipt,
) -> Result<Value, String> {
    let desired = desired_state::merge(&live["desired"], &patch(&preset["appearance"])?)?;
    if !desired["assetId"].is_null() {
        core.asset(&desired["assetId"])?;
    }
    core.commit(live, &desired, receipt)
}

fn applied(mut result: Value, c: &Value, id: &Value) -> Value {
    result.set("committed", true.into());
    result.set("appearanceApplied", true.into());
    result.set("catalogRevision", c["revision"].clone());
    result.set("presetId", id.clone());
    result
}

pub(crate) fn execute(core: &Core, body: &Value) -> Result<Value, String> {
    let action = body["action"].as_str().unwrap_or("");
    let save = action == "save";
    if !matches!(action, "save" | "apply" | "delete")
        || body["schemaVersion"].safe_uint() != Some(1)
        || !shape(
            body,
            &[
                "schemaVersion",
                "action",
                "requestId",
                "expectedRevision",
                "expectedCatalogRevision",
                if save { "name" } else { "presetId" },
            ],
        )
    {
        return Err("PRESET_COMMAND_INVALID".into());
    }
    let id = format!("preset.{}", request_id(&body["requestId"], 96)?);
    let expected = body["expectedCatalogRevision"]
        .safe_uint()
        .ok_or("INVALID_REVISION")?;
    body["expectedRevision"]
        .safe_uint()
        .ok_or("INVALID_REVISION")?;
    let trimmed = if save {
        Some(name(&body["name"]).ok_or("PRESET_COMMAND_INVALID")?)
    } else if desired_state::hex_id(&body["presetId"]) {
        None
    } else {
        return Err("PRESET_COMMAND_INVALID".into());
    };
    let receipt = Receipt {
        id,
        fingerprint: body.hash(),
    };
    let _guard = core.lock()?;
    let mut c = catalog(core)?;
    let live = core.live()?;
    let id = &body["presetId"];
    let previous = core.receipt(&receipt.id, &receipt.fingerprint, &live)?;
    let previous = match (previous, c["receipts"].get(&receipt.id)) {
        (Some(r), _) => Some(r),
        (None, Some(r)) => {
            if r["fingerprint"].as_str() != Some(&receipt.fingerprint) {
                return Err("REQUEST_CONFLICT".into());
            }
            let mut result = r["result"].clone();
            if result.as_object().is_none() {
                return Err("PRESET_CATALOG_CORRUPT".into());
            }
            result.set("replayed", true.into());
            Some(result)
        }
        _ => None,
    };
    if let Some(r) = previous {
        return Ok(if action == "apply" {
            applied(r, &c, id)
        } else {
            r
        });
    }
    if c["revision"].safe_uint() != Some(expected) {
        return Err("PRESET_CATALOG_CONFLICT".into());
    }
    core.assert_revision(&live, &body["expectedRevision"])?;
    if action == "apply" {
        let p = find(&c, id).ok_or("PRESET_NOT_FOUND")?;
        return Ok(applied(apply(core, &live, p, &receipt)?, &c, id));
    }
    let mut presets = c["presets"].as_array().unwrap().clone();
    let (kind, id) = if let Some(name) = trimmed {
        if presets.len() >= 40 {
            return Err("PRESET_LIMIT".into());
        }
        let id: Value = hash_text(body["requestId"].as_str().unwrap()).into();
        presets.push(obj([
            ("id", id.clone()),
            ("name", name),
            ("createdAt", iso_now().into()),
            ("appearance", appearance(&live["desired"])),
        ]));
        ("saved", id)
    } else {
        let index = presets
            .iter()
            .position(|p| p["id"] == *id)
            .ok_or("PRESET_NOT_FOUND")?;
        presets.remove(index);
        ("deleted", id.clone())
    };
    let mut receipts = c["receipts"].clone();
    if receipts.as_object().unwrap().len() >= 1000
        || expected >= crate::portable_store::MAX_SAFE_INTEGER
    {
        return Err("PRESET_RECEIPT_LIMIT".into());
    }
    let result = obj([
        (kind, true.into()),
        ("presetId", id),
        ("committed", false.into()),
        ("revision", live["revision"].clone()),
        ("catalogRevision", (expected + 1).into()),
    ]);
    receipts.set(
        &receipt.id,
        obj([
            ("fingerprint", receipt.fingerprint.into()),
            ("result", result.clone()),
        ]),
    );
    c.set("revision", (expected + 1).into());
    c.set("presets", Value::Array(presets));
    c.set("receipts", receipts);
    if c.canonical().len() + 1 > LIMIT {
        return Err("PRESET_LIMIT".into());
    }
    core.atomic(FILE, &c)?;
    Ok(result)
}

const CURSOR: &str = "quick-preset.json";

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct TestStore(std::path::PathBuf);

    impl TestStore {
        pub(crate) fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "atmos-native-presets-{}",
                crate::native_http::random_hex(12).unwrap()
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        pub(crate) fn core(&self) -> Core {
            let core = Core::open(&self.0).unwrap();
            core.init().unwrap();
            core
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn save(core: &Core, id: &str, catalog_rev: u64) -> Value {
        execute(
            core,
            &obj([
                ("schemaVersion", 1.into()),
                ("action", "save".into()),
                ("requestId", id.into()),
                ("name", id.into()),
                ("expectedCatalogRevision", catalog_rev.into()),
                ("expectedRevision", core.live().unwrap()["revision"].clone()),
            ]),
        )
        .unwrap()["presetId"]
            .clone()
    }

    pub(crate) fn change(core: &Core, request: &str, background: &str) {
        core.execute_command(&obj([
            ("schemaVersion", 1.into()),
            ("type", "settings.update".into()),
            ("requestId", request.into()),
            ("expectedRevision", core.live().unwrap()["revision"].clone()),
            (
                "changes",
                obj([("visual", obj([("background", background.into())]))]),
            ),
        ]))
        .unwrap();
    }

    #[test]
    fn tray_cycles_legacy_cursor_and_isolates_independent_primary_scope() {
        let store = TestStore::new();
        let owner = store.core();
        let target = format!("display-{}", "a".repeat(64));
        let child = owner.ensure_scope(&target).unwrap();
        let first = save(&child, "forest", 0);
        change(&child, "coast", "coast");
        let second = save(&child, "coast", 1);
        child
            .atomic(
                CURSOR,
                &obj([("schemaVersion", 1.into()), ("presetId", first.clone())]),
            )
            .unwrap();
        std::fs::write(
            owner.path("display-layout.json").unwrap(),
            serde_json::json!({
                "schemaVersion":1,"revision":1,"lastRequestId":null,
                "layout":{"mode":"independent","targets":[target],"askOnApply":false}
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(next(&store.0, "shared").unwrap_err(), "TARGET_REQUIRED");
        next(&store.0, &target).unwrap();
        assert_eq!(child.read(CURSOR).unwrap()["presetId"], second);
        next(&store.0, &target).unwrap();
        assert_eq!(child.read(CURSOR).unwrap()["presetId"], first);
        assert_eq!(
            child.live().unwrap()["desired"]["visual"]["background"].as_str(),
            Some("forest")
        );
        assert_eq!(owner.live().unwrap()["revision"].safe_uint(), Some(0));
        assert!(!owner.path(CURSOR).unwrap().exists());
    }

    #[test]
    fn tray_recovers_a_committed_intent_without_applying_it_again() {
        let store = TestStore::new();
        let core = store.core();
        let id = save(&core, "saved", 0);
        let c = catalog(&core).unwrap();
        let live = core.live().unwrap();
        let receipt = Receipt {
            id: "preset.tray.interrupted".into(),
            fingerprint: "a".repeat(64),
        };
        core.atomic(
            CURSOR,
            &obj([
                ("schemaVersion", 1.into()),
                ("presetId", Value::Null),
                (
                    "pending",
                    obj([
                        ("id", receipt.id.clone().into()),
                        ("fingerprint", receipt.fingerprint.clone().into()),
                        ("presetId", id.clone()),
                    ]),
                ),
            ]),
        )
        .unwrap();
        apply(&core, &live, find(&c, &id).unwrap(), &receipt).unwrap();
        next(&store.0, "shared").unwrap();
        assert_eq!(core.live().unwrap()["revision"].safe_uint(), Some(1));
        assert_eq!(
            core.read(CURSOR).unwrap(),
            obj([("schemaVersion", 1.into()), ("presetId", id)])
        );
    }
}

pub(crate) fn next(store: &Path, target: &str) -> Result<(), String> {
    let owner = Core::open(store)?;
    let layout = crate::layout::read(store)?;
    let child = if layout.layout.mode == "independent" {
        if !layout.layout.targets.iter().any(|id| id == target) {
            return Err("TARGET_REQUIRED".into());
        }
        Some(owner.ensure_scope(target)?)
    } else {
        if target != "shared" && !target.is_empty() {
            return Err("TARGET_INVALID".into());
        }
        None
    };
    next_in(child.as_ref().unwrap_or(&owner))
}

fn next_in(core: &Core) -> Result<(), String> {
    let _guard = core.lock()?;
    let c = catalog(core)?;
    let presets = c["presets"].as_array().unwrap();
    if presets.is_empty() {
        return Err("PRESET_NOT_FOUND".into());
    }
    let live = core.live()?;
    let cursor = optional(core, CURSOR, 16384, "PRESET_CURSOR_INVALID")?.unwrap_or(Value::Null);
    let mut last = cursor["presetId"].clone();
    // Recover a committed appearance if finalizing the cursor was interrupted.
    let pending = &cursor["pending"];
    if !pending.is_null() {
        let id = request_id(&pending["id"], 110)?;
        let fp = pending["fingerprint"]
            .as_str()
            .ok_or("PRESET_CURSOR_INVALID")?;
        if !desired_state::hex_id(&pending["presetId"]) || !id.starts_with("preset.tray.") {
            return Err("PRESET_CURSOR_INVALID".into());
        }
        if core.receipt(id, fp, &live)?.is_some() {
            last = pending["presetId"].clone();
            core.atomic(
                CURSOR,
                &obj([("schemaVersion", 1.into()), ("presetId", last.clone())]),
            )?;
            return Ok(());
        }
    }
    let index = presets
        .iter()
        .position(|p| p["id"] == last)
        .map_or(0, |i| (i + 1) % presets.len());
    let p = &presets[index];
    let request = format!(
        "tray.{}",
        crate::native_http::random_hex(16).map_err(|e| e.code)?
    );
    let body = obj([
        ("schemaVersion", 1.into()),
        ("action", "apply".into()),
        ("requestId", request.clone().into()),
        ("expectedRevision", live["revision"].clone()),
        ("expectedCatalogRevision", c["revision"].clone()),
        ("presetId", p["id"].clone()),
    ]);
    let receipt = Receipt {
        id: format!("preset.{request}"),
        fingerprint: body.hash(),
    };
    core.atomic(
        CURSOR,
        &obj([
            ("schemaVersion", 1.into()),
            ("presetId", last),
            (
                "pending",
                obj([
                    ("id", receipt.id.clone().into()),
                    ("fingerprint", receipt.fingerprint.clone().into()),
                    ("presetId", p["id"].clone()),
                ]),
            ),
        ]),
    )?;
    apply(core, &live, p, &receipt)?;
    core.atomic(
        CURSOR,
        &obj([("schemaVersion", 1.into()), ("presetId", p["id"].clone())]),
    )
}
