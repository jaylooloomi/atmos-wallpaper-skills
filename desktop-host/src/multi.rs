//! Desired native views. Pure planning is separately testable from Explorer attachment.
use crate::{
    displays::Display,
    host_settings, layout,
    windows::{self, Bounds},
};
use std::{collections::BTreeMap, path::Path};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
#[derive(Clone, Debug)]
pub struct Plan {
    pub id: String,
    pub label: String,
    pub scope: String,
    pub bounds: Bounds,
    pub projection_revision: u64,
}
#[derive(Clone)]
pub struct View {
    pub plan: Plan,
    pub handle: isize,
    pub parent: isize,
    pub error: Option<String>,
}
pub fn plans(host: &host_settings::Settings, l: &layout::Settings, ds: &[Display]) -> Vec<Plan> {
    if !host.placement.enabled {
        return vec![];
    }
    if l.layout.mode == "single" {
        return crate::displays::select(ds, &host.placement.target)
            .ok()
            .map(|d| {
                vec![Plan {
                    id: d.id.clone(),
                    label: "wallpaper".into(),
                    scope: "shared".into(),
                    bounds: d.bounds,
                    projection_revision: l.revision,
                }]
            })
            .unwrap_or_default();
    }
    l.layout
        .targets
        .iter()
        .filter_map(|id| crate::displays::select(ds, id).ok())
        .map(|d| Plan {
            id: d.id.clone(),
            label: format!(
                "wp-{}-{}",
                if l.layout.mode == "independent" {
                    "own"
                } else {
                    "shared"
                },
                &d.id[8..]
            ),
            scope: if l.layout.mode == "independent" {
                d.id.clone()
            } else {
                "shared".into()
            },
            bounds: d.bounds,
            projection_revision: l.revision,
        })
        .collect()
}
pub fn synchronize(
    app: &tauri::AppHandle,
    root: &Path,
    port: u16,
    desired: &[Plan],
    views: &mut BTreeMap<String, View>,
) {
    let removed = views
        .keys()
        .filter(|key| !desired.iter().any(|p| &p.label == *key))
        .cloned()
        .collect::<Vec<_>>();
    for key in removed {
        if let Some(window) = app.get_webview_window(&key) {
            let _ = window.destroy();
        }
        views.remove(&key);
    }
    for p in desired {
        if let Some(view) = views.get_mut(&p.label) {
            view.plan = p.clone();
            if windows::still_attached(view.handle, view.parent, p.bounds) {
                view.error = None;
                continue;
            }
            match windows::desktop_parent()
                .and_then(|parent| windows::attach(view.handle, parent, p.bounds).map(|_| parent))
            {
                Ok(parent) => {
                    view.parent = parent;
                    view.error = None
                }
                Err(e) => {
                    windows::hide(view.handle);
                    view.error = Some(e)
                }
            }
            continue;
        }
        let route = if p.label == "wallpaper" {
            "/wallpaper".into()
        } else {
            format!("/wallpaper/{}", p.id)
        };
        let allowed = route.clone();
        let created = (|| -> Result<View, String> {
            let w = WebviewWindowBuilder::new(
                app,
                &p.label,
                WebviewUrl::External(
                    format!("http://127.0.0.1:{port}{route}")
                        .parse()
                        .map_err(|_| "VIEW_URL")?,
                ),
            )
            .title("Atmos Wallpaper")
            .visible(false)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .data_directory(root.join("webview2"))
            .devtools(false)
            .on_navigation(move |u| {
                u.scheme() == "http"
                    && u.host_str() == Some("127.0.0.1")
                    && u.port() == Some(port)
                    && u.path() == allowed
                    && u.query().is_none()
            })
            .build()
            .map_err(|e| e.to_string())?;
            let h = w.hwnd().map_err(|e| e.to_string())?.0 as isize;
            let attached = windows::desktop_parent()
                .and_then(|parent| windows::attach(h, parent, p.bounds).map(|_| parent));
            Ok(match attached {
                Ok(parent) => View {
                    plan: p.clone(),
                    handle: h,
                    parent,
                    error: None,
                },
                Err(e) => View {
                    plan: p.clone(),
                    handle: h,
                    parent: 0,
                    error: Some(e),
                },
            })
        })();
        if let Ok(view) = created {
            views.insert(p.label.clone(), view);
        }
    }
}
pub fn observation(root: &Path, p: &Plan, session: &str) -> bool {
    if session.is_empty() {
        return false;
    }
    let dir = if p.scope == "shared" {
        root.to_path_buf()
    } else {
        root.join("targets").join(&p.id)
    };
    let name = if p.label == "wallpaper" || p.scope != "shared" {
        "wallpaper.json".into()
    } else {
        format!("wallpaper-{}.json", p.id)
    };
    let file = dir.join("observations").join(name);
    let Some(v) = crate::policy_store::read_optional(&file, 16384)
        .ok()
        .flatten()
    else {
        return false;
    };
    let Some(live) = crate::policy_store::read_optional(&dir.join("live.json"), 16 * 1048576)
        .ok()
        .flatten()
    else {
        return false;
    };
    let target_matches = if p.label == "wallpaper" {
        v["targetId"].is_null()
    } else {
        v["targetId"] == p.id
    };
    v["hostSessionId"] == session
        && v["projection"]["layoutRevision"] == p.projection_revision
        && v["projection"]["targetId"] == p.id
        && v["projection"]["targetBounds"] == serde_json::json!(p.bounds)
        && target_matches
        && v["source"] == "local-bridge"
        && v["rendererRole"] == "wallpaper"
        && v["stateApplied"] == true
        && v["assetLoaded"] == true
        && v["revision"].as_u64().is_some()
        && v["desiredHash"].as_str().is_some()
        && v["revision"] == live["revision"]
        && v["desiredHash"] == live["desiredHash"]
        && v["capturedAt"].as_str().is_some_and(|_| {
            // File age is host measured; renderer-provided dates are never accepted as host evidence.
            std::fs::metadata(file)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|d| d.as_millis() < 4000)
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn display(c: char, x: i32, primary: bool) -> Display {
        Display {
            id: format!("display-{}", c.to_string().repeat(64)),
            name: "fixture".into(),
            device_name: "fixture".into(),
            number: 1,
            primary,
            bounds: Bounds {
                x,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale_percent: Some(100),
            rotation_degrees: Some(0),
            identity_confidence: "device-path".into(),
            selectable: true,
        }
    }
    #[test]
    fn planning_preserves_offline_ids_and_never_falls_back() {
        let ds = vec![display('a', -1920, false), display('b', 0, true)];
        let h = host_settings::Settings::default();
        let mut l = layout::Settings::default();
        l.layout.mode = "independent".into();
        l.layout.targets = vec![ds[0].id.clone(), format!("display-{}", "c".repeat(64))];
        let p = plans(&h, &l, &ds);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].bounds.x, -1920);
        assert_eq!(p[0].scope, ds[0].id);
        assert_eq!(l.layout.targets.len(), 2);
        l.layout.mode = "duplicate".into();
        assert_eq!(plans(&h, &l, &ds)[0].scope, "shared");
        let mut cleared = h;
        cleared.placement.enabled = false;
        assert!(plans(&cleared, &l, &ds).is_empty())
    }
}
#[cfg(test)]
mod observation_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn native_ack_rejects_prior_session_target_projection_and_unmatched_live() {
        let root = std::env::temp_dir().join(format!(
            "atmos-observation-{}-{}",
            std::process::id(),
            crate::journal::now_ms()
        ));
        let id = format!("display-{}", "a".repeat(64));
        let plan = Plan {
            id: id.clone(),
            label: "wp-shared-fixture".into(),
            scope: "shared".into(),
            bounds: Bounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            projection_revision: 3,
        };
        let file = format!("wallpaper-{}.json", id);
        crate::host_settings::atomic(
            &root,
            "live.json",
            &json!({"revision":2,"desiredHash":"fixture-hash"}),
        )
        .unwrap();
        let good = json!({"hostSessionId":"active-session","rendererRole":"wallpaper","source":"local-bridge","stateApplied":true,"assetLoaded":true,"targetId":id,"revision":2,"desiredHash":"fixture-hash","capturedAt":"host timestamp is not trusted here","projection":{"layoutRevision":3,"targetId":id,"targetBounds":plan.bounds}});
        crate::host_settings::atomic(&root.join("observations"), &file, &good).unwrap();
        assert!(observation(&root, &plan, "active-session"));
        for key in ["hostSessionId", "targetId", "desiredHash"] {
            let mut wrong = good.clone();
            wrong[key] = json!("wrong");
            crate::host_settings::atomic(&root.join("observations"), &file, &wrong).unwrap();
            assert!(!observation(&root, &plan, "active-session"));
        }
        let mut wrong = good.clone();
        wrong["projection"]["layoutRevision"] = json!(2);
        crate::host_settings::atomic(&root.join("observations"), &file, &wrong).unwrap();
        assert!(!observation(&root, &plan, "active-session"));
        let mut wrong = good;
        wrong["projection"]["targetBounds"]["width"] = json!(2560);
        crate::host_settings::atomic(&root.join("observations"), &file, &wrong).unwrap();
        assert!(!observation(&root, &plan, "active-session"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
