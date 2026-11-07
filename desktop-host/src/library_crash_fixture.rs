//! Deterministic crash points for disposable fixture stores only.
use std::{io::Write, path::Path, sync::OnceLock, time::Duration};

static PHASE: OnceLock<(std::path::PathBuf, String)> = OnceLock::new();

pub(crate) fn command(args: &[String]) -> Result<i32, String> {
    let [root, phase, action] = args else {
        return Err("FIXTURE_ARGUMENTS_INVALID".into());
    };
    if !matches!(
        (phase.as_str(), action.as_str()),
        (
            "move-after-copy" | "move-before-switch" | "move-after-switch",
            "reset"
        ) | ("archive-after-remove", "archive")
    ) {
        return Err("FIXTURE_ARGUMENTS_INVALID".into());
    }
    let root = Path::new(root);
    crate::portable_store::validate_root(root)?;
    let root = root.canonicalize().map_err(|_| "FIXTURE_STORE_INVALID")?;
    let temp = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "FIXTURE_TEMP_INVALID")?;
    if !root.starts_with(&temp) || root.parent().is_none_or(|parent| parent == temp) {
        return Err("FIXTURE_REQUIRES_TEMP_STORE".into());
    }
    let marker = crate::core_http_fixture::STORE_MARKER;
    if crate::asset_store::read_bytes(
        &root.join("core-http-fixture.marker"),
        marker.len(),
        "FIXTURE_STORE_INVALID",
    )? != marker
    {
        return Err("FIXTURE_STORE_INVALID".into());
    }
    let path = crate::portable_store::external_path(&root);
    PHASE
        .set((root, phase.clone()))
        .map_err(|_| "FIXTURE_ALREADY_CONFIGURED")?;
    Ok(crate::native_library_cli::command(&[
        "--library-command".into(),
        action.clone(),
        "--store".into(),
        path,
    ]))
}

pub(crate) fn pause(root: &Path, phase: &str) {
    if !PHASE
        .get()
        .is_some_and(|(expected, selected)| root == expected && phase == selected)
    {
        return;
    }
    println!(
        "{}",
        serde_json::json!({"checkpoint":phase,"pid":std::process::id()})
    );
    let _ = std::io::stdout().flush();
    std::thread::sleep(Duration::from_secs(30));
    std::process::exit(99);
}
