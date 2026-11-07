//! Offline bundle generation using the same renderer scripts as the JS exporter.
use crate::{
    appearance_core::Core,
    asset_store::{hash_bytes, read_bytes},
    js_json::{object, Value},
    native_http::Assets,
    portable_store,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

const TEMPLATE: &[u8] =
    include_bytes!("../../implementation-p4_2_5/src/offline-renderer-template.json");
const VALIDATOR: &str = include_str!("../../implementation-p4_2_5/src/state.mjs");
const MARKER: &str = "<!--BRIDGE_SESSION--><script src=\"bridge-client.js\"></script>";
const SCRIPTS: &str = concat!(
    "<script src=\"renderer-state.js\"></script><script src=\"verified-artwork.js\"></script>",
    "<script src=\"state-validator.js\"></script><script src=\"p1-bootstrap.js\"></script>",
    "<script src=\"p1-loader.js\"></script>"
);
const APP_GUARD: &str =
    "if(!window.atmosP1Validated)throw Error('No validated offline snapshot');\n";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Template {
    prepared_files: Vec<String>,
    files: BTreeMap<String, String>,
}

fn text_file(files: &mut BTreeMap<String, Vec<u8>>, name: &str) -> Result<String, String> {
    String::from_utf8(files.remove(name).ok_or("RENDERER_NOT_PREPARED")?)
        .map_err(|_| "RENDERER_NOT_PREPARED".into())
}

fn safe_relative(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with('.')
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-@".contains(&c))
        })
}

fn asset_files(prototype: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let manifest = read_bytes(&prototype.join("manifest.json"), 262144, "BASELINE_CHANGED")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest).map_err(|_| "BASELINE_CHANGED")?;
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for entry in manifest["files"].as_array().ok_or("BASELINE_CHANGED")? {
        let Some(name) = entry["path"].as_str().filter(|p| p.starts_with("assets/")) else {
            continue;
        };
        if !safe_relative(name) || files.len() >= 128 {
            return Err("BASELINE_CHANGED".into());
        }
        let bytes = read_bytes(&prototype.join(name), 16 * 1048576, "BASELINE_CHANGED")?;
        total += bytes.len();
        if total > 128 * 1048576
            || entry["sha256"] != hash_bytes(&bytes)
            || files.insert(name.into(), bytes).is_some()
        {
            return Err("BASELINE_CHANGED".into());
        }
    }
    Ok(files)
}

pub(crate) fn export(
    core: &Core,
    preview_id: &Value,
    assets: &Assets,
    prototype: &Path,
) -> Result<Value, String> {
    let _guard = core.lock()?;
    let live = core.live()?;
    let preview = if preview_id.is_null() {
        None
    } else {
        Some(core.read_preview(preview_id)?)
    };
    if let Some(preview) = &preview {
        if preview["status"].as_str() != Some("ready")
            || preview["expiresAt"]
                .safe_uint()
                .is_none_or(|at| crate::journal::now_ms() > at)
        {
            return Err("INVALID_PREVIEW".into());
        }
        if preview["baseRevision"] != live["revision"] || preview["baseHash"] != live["desiredHash"]
        {
            return Err("PREVIEW_STALE".into());
        }
    }
    let desired = &preview.as_ref().unwrap_or(&live)["desired"];
    let asset_id = &desired["assetId"];
    let kind = if preview.is_some() {
        "candidate-preview"
    } else {
        "live-snapshot"
    };
    let template: Template =
        serde_json::from_slice(TEMPLATE).map_err(|_| "RENDERER_NOT_PREPARED")?;
    let mut files = asset_files(prototype)?;
    let custom_path = asset_id.as_str().map(|id| format!("assets/p1-{id}.png"));
    let art_path = custom_path
        .clone()
        .unwrap_or_else(|| format!("assets/{}", crate::native_core_http::artwork_name(desired)));
    if custom_path.is_some() {
        files.insert(art_path.clone(), core.asset(asset_id)?.1);
    }
    let artwork = files.get(&art_path).ok_or("BASELINE_CHANGED")?;
    let art_hash = hash_bytes(artwork);
    let encoded = STANDARD.encode(artwork);
    for name in template.prepared_files {
        let bytes = assets
            .bytes(&format!("/{name}"))
            .ok_or("RENDERER_NOT_PREPARED")?;
        if !safe_relative(&name) || files.insert(name, bytes.to_vec()).is_some() {
            return Err("RENDERER_NOT_PREPARED".into());
        }
    }
    for (name, text) in template.files {
        if !safe_relative(&name) || files.insert(name, text.into_bytes()).is_some() {
            return Err("RENDERER_NOT_PREPARED".into());
        }
    }
    let input_hashes: Vec<_> = files
        .iter()
        .map(|(name, bytes)| Value::Array(vec![name.as_str().into(), hash_bytes(bytes).into()]))
        .collect();
    let build = object([
        (
            "generator",
            hash_bytes(include_bytes!("native_offline.rs")).into(),
        ),
        ("validator", hash_bytes(VALIDATOR.as_bytes()).into()),
        ("template", hash_bytes(TEMPLATE).into()),
        ("files", Value::Array(input_hashes)),
    ])
    .hash();
    let desired_hash = desired.hash();
    let id = object([
        ("kind", kind.into()),
        ("revision", live["revision"].clone()),
        ("desiredHash", desired_hash.clone().into()),
        ("rendererBuild", build.into()),
        ("artHash", art_hash.clone().into()),
    ])
    .hash();
    let snapshot = object([
        ("schemaVersion", 1.into()),
        ("bundleId", id.clone().into()),
        ("kind", kind.into()),
        ("revision", live["revision"].clone()),
        ("desiredHash", desired_hash.into()),
        ("desired", desired.clone()),
        ("source", "preview".into()),
        ("hostVisibility", "unknown".into()),
        (
            "artwork",
            object([("path", art_path.into()), ("sha256", art_hash.into())]),
        ),
        (
            "customAssetPath",
            custom_path.map_or(Value::Null, Into::into),
        ),
    ]);
    let html = text_file(&mut files, "index.html")?;
    if html.matches(MARKER).count() != 1 {
        return Err("RENDERER_NOT_PREPARED".into());
    }
    files.insert(
        "index.html".into(),
        html.replace(MARKER, SCRIPTS).into_bytes(),
    );
    let app = text_file(&mut files, "app.js")?;
    let widgets = text_file(&mut files, "widgets.js")?;
    if !app.contains("calendarFont")
        || !app.contains("AtmosAppearance")
        || !widgets.contains("fontSize")
    {
        return Err("RENDERER_NOT_PREPARED".into());
    }
    files.insert(
        "app.js".into(),
        format!(
            "{APP_GUARD}{}",
            app.replace(
                "return window.atmosBridgeCurrent.url;",
                "return window.atmosP1VerifiedArtworkURL;"
            )
        )
        .into_bytes(),
    );
    files.insert(
        "widgets.js".into(),
        format!("{APP_GUARD}{widgets}").into_bytes(),
    );
    files.get_mut("hifi.css").ok_or("RENDERER_NOT_PREPARED")?
        .extend_from_slice(b"\n/* Offline display: preserve manager DOM for existing handlers, do not expose dead settings. */\nbody[data-render-mode=wallpaper] #widgetManager{display:none!important}\n");
    let validator=VALIDATOR.replace(
        "import {ControlError} from '../../implementation-p1/src/state.mjs';export {ControlError};",
        "class ControlError extends Error{constructor(code,message){super(message);this.code=code}}")
        .replace("export ","");
    files.insert("state-validator.js".into(),format!(
        "(()=>{{\n{validator}\nwindow.atmosP1Validate=validateDesired;window.atmosP1Canonical=canonical;}})();\n").into_bytes());
    let literal = snapshot
        .canonical()
        .replace('<', "\\u003c")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    files.insert(
        "renderer-state.js".into(),
        format!("window.atmosP1Snapshot={literal};\n").into_bytes(),
    );
    files.insert(
        "verified-artwork.js".into(),
        format!("window.atmosP1ArtworkBytes=\"{encoded}\";\n").into_bytes(),
    );
    let hashes = files
        .iter()
        .map(|(name, bytes)| {
            object([
                ("path", name.as_str().into()),
                ("sha256", hash_bytes(bytes).into()),
            ])
        })
        .collect();
    let manifest = object([
        ("schemaVersion", 1.into()),
        ("snapshot", snapshot.clone()),
        ("files", Value::Array(hashes)),
    ]);
    files.insert("p1-manifest.json".into(), manifest.canonical().into_bytes());
    let directory = core.store.path(&format!("exports/{id}"))?;
    let existing = directory.exists();
    if existing {
        for (name, expected) in &files {
            let actual = read_bytes(&directory.join(name), expected.len(), "EXPORT_CORRUPT")?;
            if actual != *expected {
                return Err("EXPORT_CORRUPT".into());
            }
        }
    } else {
        publish(&directory, &files)?;
    }
    Ok(object([
        ("bundleId", id.into()),
        (
            "directory",
            portable_store::external_path(&directory).into(),
        ),
        (
            "entry",
            portable_store::external_path(&directory.join("index.html")).into(),
        ),
        ("snapshot", snapshot),
        ("existing", existing.into()),
    ]))
}

fn publish(directory: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let mut entropy = [0u8; 16];
    getrandom::fill(&mut entropy).map_err(|_| "ENTROPY_UNAVAILABLE")?;
    let suffix = hash_bytes(&entropy);
    let temp = directory.with_extension(format!("tmp-{suffix}"));
    fs::create_dir(&temp).map_err(|_| "EXPORT_WRITE_FAILED")?;
    let result = (|| {
        for (name, bytes) in files {
            let file = temp.join(name);
            fs::create_dir_all(file.parent().unwrap()).map_err(|_| "EXPORT_WRITE_FAILED")?;
            portable_store::atomic_file(&file, bytes)?;
        }
        portable_store::no_links(directory)?;
        fs::rename(&temp, directory).map_err(|_| "EXPORT_WRITE_FAILED".into())
    })();
    if result.is_err()
        && temp.parent() == directory.parent()
        && portable_store::no_links(&temp).is_ok()
    {
        let _ = fs::remove_dir_all(&temp);
    }
    result
}

pub(crate) fn observe(core: &Core, report: &Value) -> Result<Value, String> {
    let _guard = core.lock()?;
    let live = core.live()?;
    let allowed = [
        "schemaVersion",
        "revision",
        "desiredHash",
        "source",
        "stateUnchanged",
        "assetSha256",
        "assetLoaded",
        "hostVisibility",
        "capturedAt",
        "bundleId",
    ];
    if !report.as_object().is_some_and(|o| o.len() == allowed.len())
        || !allowed.iter().all(|k| report.has(k))
        || report["schemaVersion"].safe_uint() != Some(1)
        || report["source"].as_str() != Some("preview")
        || report["stateUnchanged"].as_bool() != Some(true)
        || report["hostVisibility"].as_str() != Some("unknown")
        || report["assetLoaded"].as_bool().is_none()
        || !crate::desired_state::hex_id(&report["bundleId"])
        || !report["capturedAt"]
            .as_str()
            .is_some_and(|at| crate::appearance_core::timestamp_ms(at).is_some())
    {
        return Err("INVALID_OBSERVATION".into());
    }
    if report["revision"] != live["revision"] || report["desiredHash"] != live["desiredHash"] {
        return Err("OBSERVATION_STALE".into());
    }
    let id = report["bundleId"].as_str().unwrap();
    let file = core.path(&format!("exports/{id}/p1-manifest.json"))?;
    let exported =
        crate::asset_store::read_json(&file, 262144).map_err(|_| "INVALID_OBSERVATION")?;
    let snapshot = &exported["snapshot"];
    if snapshot["kind"].as_str() != Some("live-snapshot")
        || snapshot["bundleId"] != report["bundleId"]
        || snapshot["revision"] != report["revision"]
        || snapshot["desiredHash"] != report["desiredHash"]
        || snapshot["artwork"]["sha256"] != report["assetSha256"]
    {
        return Err("INVALID_OBSERVATION".into());
    }
    core.atomic("observations/latest.json", report)?;
    Ok(object([
        ("accepted", true.into()),
        (
            "trust",
            "renderer-reported, not independent visual or OS verification".into(),
        ),
        ("hostVisibility", "unknown".into()),
    ]))
}
