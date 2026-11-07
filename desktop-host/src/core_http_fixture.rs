//! Feature-gated listener using a fresh temporary store and supplied topology.
use crate::{
    asset_store::read_bytes,
    displays::Display,
    native_core_http::Bridge,
    native_http::{Assets, Limits, Server},
    native_service, portable_store,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) const STORE_MARKER: &[u8] = b"atmos-core-http-fixture-v1";

pub(crate) fn prepare_store(root: &Path) -> Result<(), String> {
    portable_store::validate_root(&root)?;
    let temporary = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "FIXTURE_TEMP_INVALID")?;
    let parent = root
        .parent()
        .ok_or("FIXTURE_STORE_INVALID")?
        .canonicalize()
        .map_err(|_| "FIXTURE_STORE_INVALID")?;
    if parent == temporary || !parent.starts_with(&temporary) {
        return Err("FIXTURE_REQUIRES_TEMP_STORE".into());
    }
    let marker = root.join("core-http-fixture.marker");
    let marker_bytes = STORE_MARKER;
    if root.exists() {
        if read_bytes(&marker, marker_bytes.len(), "FIXTURE_STORE_INVALID")? != marker_bytes {
            return Err("FIXTURE_STORE_INVALID".into());
        }
    } else {
        std::fs::create_dir(root).map_err(|_| "FIXTURE_REQUIRES_TEMP_STORE")?;
        portable_store::atomic_file(&marker, marker_bytes)?;
    }
    Ok(())
}

pub(crate) fn load_topology(topology: &Path) -> Result<Vec<Display>, String> {
    let displays: Vec<Display> =
        serde_json::from_slice(&read_bytes(topology, 131072, "FIXTURE_TOPOLOGY_INVALID")?)
            .map_err(|_| "FIXTURE_TOPOLOGY_INVALID")?;
    if displays.len() > 64
        || displays.iter().any(|d| {
            !crate::appearance_core::display_id(&d.id)
                || d.bounds.width <= 0
                || d.bounds.height <= 0
        })
    {
        return Err("FIXTURE_TOPOLOGY_INVALID".into());
    }
    Ok(displays)
}

pub fn run(args: &[OsString]) -> Result<(), String> {
    let [root, assets, prototype, topology, python, decoder] = args else {
        return Err("FIXTURE_ARGUMENTS_INVALID".into());
    };
    let root = PathBuf::from(root);
    prepare_store(&root)?;
    let displays = load_topology(Path::new(topology))?;
    let assets = Assets::load(Path::new(assets)).map_err(|e| e.code)?;
    let bridge = Bridge::open(
        &root,
        Path::new(prototype),
        format!(
            "fixture-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ),
        crate::backup_image::Decoder {
            python: PathBuf::from(python),
            script: PathBuf::from(decoder),
        },
        true,
    )?;
    let server = Server::bind(Limits::default()).map_err(|e| e.code)?;
    println!(
        "{}",
        serde_json::json!({"origin":server.security.origin,"syntheticOnly":true})
    );
    native_service::serve(
        bridge,
        server,
        native_service::stop_on_input(Some(Duration::from_secs(120))),
        assets,
        move || displays.clone(),
    )
}
