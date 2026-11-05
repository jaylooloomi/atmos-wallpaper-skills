//! Read-only, fixed CDP counters from our own WebViews. No debugger port, UI, JS evaluation or GC.
use ::windows::core::w;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tauri::{Manager, WebviewWindow};
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2,
};

const INTERVAL: u64 = 60_000;
#[derive(Clone, Copy)]
enum Kind {
    Heap,
    Dom,
}
#[derive(Default)]
struct Entry {
    role: String,
    identity: isize,
    generation: u64,
    waiting: [bool; 2],
    heap: Option<Value>,
    dom: Option<Value>,
    errors: [Option<&'static str>; 2],
}
#[derive(Clone, Default)]
pub struct Sampler {
    entries: Arc<Mutex<BTreeMap<String, Entry>>>,
}
fn now() -> u64 {
    crate::journal::now_ms() as u64
}
fn count(value: &Value, name: &str, maximum: f64) -> Result<u64, &'static str> {
    let n = value[name].as_f64().ok_or("COUNTER_INVALID")?;
    if !n.is_finite() || n < 0.0 || n > maximum || n.fract() != 0.0 {
        return Err("COUNTER_INVALID");
    }
    Ok(n as u64)
}
fn parse(kind: Kind, body: &str, at: u64) -> Result<Value, &'static str> {
    if body.len() > 4096 {
        return Err("COUNTER_TOO_LARGE");
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "COUNTER_INVALID")?;
    if !v.is_object() {
        return Err("COUNTER_INVALID");
    }
    match kind {
        Kind::Heap => {
            let used = count(&v, "usedSize", 68_719_476_736.0)?;
            let total = count(&v, "totalSize", 68_719_476_736.0)?;
            if used > total {
                return Err("COUNTER_INVALID");
            }
            let mut out = json!({"capturedAtMs":at,"usedBytes":used,"totalBytes":total});
            for (input, output) in [
                ("embedderHeapUsedSize", "embedderUsedBytes"),
                ("backingStorageSize", "backingStorageBytes"),
            ] {
                if v.get(input).is_some() {
                    out[output] = json!(count(&v, input, 68_719_476_736.0)?);
                }
            }
            Ok(out)
        }
        Kind::Dom => Ok(
            json!({"capturedAtMs":at,"documents":count(&v,"documents",100_000_000.0)?,"nodes":count(&v,"nodes",100_000_000.0)?,"jsEventListeners":count(&v,"jsEventListeners",100_000_000.0)?}),
        ),
    }
}
impl Sampler {
    fn complete(&self, label: &str, generation: u64, kind: Kind, body: Result<&str, &'static str>) {
        let result = body.and_then(|body| parse(kind, body, now()));
        let mut map = self.entries.lock().unwrap();
        let Some(entry) = map.get_mut(label) else {
            return;
        };
        let index = if matches!(kind, Kind::Heap) { 0 } else { 1 };
        if entry.generation != generation || !entry.waiting[index] {
            return;
        }
        entry.waiting[index] = false;
        match result {
            Ok(value) => {
                if index == 0 {
                    entry.heap = Some(value)
                } else {
                    entry.dom = Some(value)
                };
                entry.errors[index] = None;
            }
            Err(code) => entry.errors[index] = Some(code),
        }
    }
    fn request(&self, window: WebviewWindow, label: String, generation: u64) {
        let sampler = self.clone();
        let failed = self.clone();
        let failed_label = label.clone();
        let result = window.with_webview(move |view| {
            let core = unsafe { view.controller().CoreWebView2() };
            match core {
                Ok(core) => {
                    sampler.ask(&core, &label, generation, Kind::Heap);
                    sampler.ask(&core, &label, generation, Kind::Dom);
                }
                Err(_) => {
                    for kind in [Kind::Heap, Kind::Dom] {
                        sampler.complete(&label, generation, kind, Err("WEBVIEW_UNAVAILABLE"));
                    }
                }
            }
        });
        if result.is_err() {
            for kind in [Kind::Heap, Kind::Dom] {
                failed.complete(&failed_label, generation, kind, Err("WEBVIEW_UNAVAILABLE"));
            }
        }
    }
    fn ask(&self, core: &ICoreWebView2, label: &str, generation: u64, kind: Kind) {
        let sampler = self.clone();
        let callback_label = label.to_string();
        let callback =
            CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, body| {
                sampler.complete(
                    &callback_label,
                    generation,
                    kind,
                    if result.is_ok() {
                        Ok(&body)
                    } else {
                        Err("CDP_FAILED")
                    },
                );
                Ok(())
            }));
        let method = match kind {
            Kind::Heap => w!("Runtime.getHeapUsage"),
            Kind::Dom => w!("Memory.getDOMCounters"),
        };
        if unsafe { core.CallDevToolsProtocolMethod(method, w!("{}"), &callback) }.is_err() {
            self.complete(label, generation, kind, Err("CDP_UNAVAILABLE"));
        }
    }
    pub fn poll(&self, app: &tauri::AppHandle, labels: Vec<(String, String)>) {
        let live = labels
            .into_iter()
            .take(17)
            .filter_map(|(label, role)| app.get_webview_window(&label).map(|w| (label, role, w)))
            .collect::<Vec<_>>();
        self.entries
            .lock()
            .unwrap()
            .retain(|label, _| live.iter().any(|(l, _, _)| l == label));
        for (label, role, window) in live {
            let Ok(handle) = window.hwnd() else { continue };
            let identity = handle.0 as isize;
            if identity == 0 {
                continue;
            }
            let at = now();
            {
                let mut map = self.entries.lock().unwrap();
                let entry = map.entry(label.clone()).or_default();
                if entry.identity != identity {
                    *entry = Entry {
                        identity,
                        role: role.clone(),
                        ..Default::default()
                    };
                }
                // Never accumulate requests for a non-responsive view. Late callbacks cannot replace a newer generation.
                if entry.waiting.iter().any(|v| *v)
                    || at.saturating_sub(entry.generation) < INTERVAL
                {
                    continue;
                }
                entry.generation = at;
                entry.waiting = [true, true];
                entry.errors = [None, None];
            }
            self.request(window, label, at);
        }
    }
    pub fn snapshot(&self) -> Value {
        let at = now();
        let map = self.entries.lock().unwrap();
        json!({"schemaVersion":1,"method":"read-only-cdp-no-gc","intervalMs":INTERVAL,"views":map.iter().map(|(label,e)|{let fresh=[&e.heap,&e.dom].iter().all(|v|v.as_ref().and_then(|v|v["capturedAtMs"].as_u64()).is_some_and(|t|at>=t&&at-t<=90_000));json!({"windowLabel":label,"role":e.role,"requestedAtMs":e.generation,"available":fresh&&e.errors.iter().all(Option::is_none),"pending":e.waiting.iter().any(|v|*v),"timedOut":e.waiting.iter().any(|v|*v)&&at.saturating_sub(e.generation)>30_000,"heap":e.heap,"dom":e.dom,"errors":{"heap":e.errors[0],"dom":e.errors[1]}})}).collect::<Vec<_>>()})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_only_heap_and_dom_counts() {
        let heap = parse(
            Kind::Heap,
            r#"{"usedSize":10,"totalSize":20,"embedderHeapUsedSize":5,"backingStorageSize":2}"#,
            7,
        )
        .unwrap();
        assert_eq!(heap["usedBytes"], 10);
        assert_eq!(heap["capturedAtMs"], 7);
        assert_eq!(
            parse(
                Kind::Dom,
                r#"{"documents":1,"nodes":40,"jsEventListeners":3}"#,
                7
            )
            .unwrap()["nodes"],
            40
        );
    }
    #[test]
    fn rejects_missing_negative_fractional_and_oversized_counters() {
        for body in [
            r#"{"usedSize":21,"totalSize":20}"#,
            r#"{"usedSize":-1,"totalSize":20}"#,
            r#"{"usedSize":1.5,"totalSize":20}"#,
            r#"{"usedSize":1}"#,
            "null",
        ] {
            assert!(parse(Kind::Heap, body, 0).is_err());
        }
        assert!(parse(
            Kind::Dom,
            r#"{"documents":1,"nodes":-1,"jsEventListeners":0}"#,
            0
        )
        .is_err());
        assert!(parse(Kind::Heap, &"x".repeat(4097), 0).is_err());
    }
    #[test]
    fn failed_calls_do_not_become_zero_usage() {
        let s = Sampler::default();
        s.entries.lock().unwrap().insert(
            "manager".into(),
            Entry {
                generation: 1,
                waiting: [true, true],
                ..Default::default()
            },
        );
        s.complete("manager", 1, Kind::Heap, Err("CDP_FAILED"));
        let v = s.snapshot();
        assert_eq!(v["views"][0]["available"], false);
        assert!(v["views"][0]["heap"].is_null());
        assert_eq!(v["views"][0]["errors"]["heap"], "CDP_FAILED");
    }
    #[test]
    fn pending_probes_keep_fresh_counters_but_errors_and_stale_counters_are_unavailable() {
        let s = Sampler::default();
        let at = now();
        s.entries.lock().unwrap().insert(
            "manager".into(),
            Entry {
                generation: at,
                waiting: [true, true],
                heap: Some(json!({"capturedAtMs":at-1000,"usedBytes":3,"totalBytes":4})),
                dom: Some(json!({"capturedAtMs":at-1000,"nodes":10})),
                ..Default::default()
            },
        );
        assert_eq!(s.snapshot()["views"][0]["available"], true);
        s.complete("manager", at, Kind::Heap, Err("CDP_FAILED"));
        assert_eq!(s.snapshot()["views"][0]["available"], false);
        let mut map = s.entries.lock().unwrap();
        let e = map.get_mut("manager").unwrap();
        e.errors = [None, None];
        e.heap.as_mut().unwrap()["capturedAtMs"] = json!(at - 100_000);
        drop(map);
        assert_eq!(s.snapshot()["views"][0]["available"], false);
    }
    #[test]
    fn stale_and_duplicate_callbacks_cannot_replace_current_counters() {
        let s = Sampler::default();
        s.entries.lock().unwrap().insert(
            "manager".into(),
            Entry {
                generation: 2,
                waiting: [true, true],
                ..Default::default()
            },
        );
        s.complete(
            "manager",
            1,
            Kind::Heap,
            Ok(r#"{"usedSize":1,"totalSize":2}"#),
        );
        assert!(s.snapshot()["views"][0]["heap"].is_null());
        s.complete(
            "manager",
            2,
            Kind::Heap,
            Ok(r#"{"usedSize":3,"totalSize":4}"#),
        );
        s.complete(
            "manager",
            2,
            Kind::Heap,
            Ok(r#"{"usedSize":1,"totalSize":2}"#),
        );
        assert_eq!(s.snapshot()["views"][0]["heap"]["usedBytes"], 3);
        s.entries.lock().unwrap().clear();
        s.complete(
            "manager",
            2,
            Kind::Dom,
            Ok(r#"{"documents":1,"nodes":2,"jsEventListeners":3}"#),
        );
        assert!(s.snapshot()["views"].as_array().unwrap().is_empty());
    }
}
