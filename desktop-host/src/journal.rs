use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
pub struct Journal {
    path: PathBuf,
    events: Mutex<Vec<Value>>,
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl Journal {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(path.parent().ok_or("Status path needs a parent")?)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            events: Mutex::new(Vec::new()),
        })
    }
    pub fn event(&self, kind: &str, detail: Value) {
        let event = json!({"atMs":now_ms(),"kind":kind,"detail":detail});
        println!("{event}");
        let mut events = self.events.lock().unwrap();
        events.push(event);
        if events.len() > 200 {
            events.remove(0);
        }
        // A private per-run diagnostic journal. It contains no session tokens.
        if let Err(e) = fs::write(
            &self.path,
            serde_json::to_vec_pretty(&json!({"schemaVersion":1,"events":*events})).unwrap(),
        ) {
            eprintln!("Could not write diagnostic journal: {e}");
        }
    }
}
