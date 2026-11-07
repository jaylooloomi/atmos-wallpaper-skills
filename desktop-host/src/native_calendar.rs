use crate::{
    appearance_core::{iso_now, timestamp_ms},
    calendar_parse,
    calendar_storage::{self, Source},
    portable_store::Store,
    public_https,
};
use reqwest::Url;
use serde_json::{json, Value};
use std::sync::{Mutex, TryLockError};

#[derive(Default)]
struct State {
    source: Option<Source>,
    initialized: bool,
    events: Option<Value>,
    parsed_at: Option<u64>,
    fetch_attempt: Option<u64>,
    error: Option<String>,
}

impl State {
    fn load(&mut self, store: &Store) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        match calendar_storage::read(store) {
            Ok(source) => self.source = source,
            Err(e) => self.error = Some(e),
        }
    }

    fn status(&self) -> Value {
        let parsed = self.events.as_ref();
        json!({"configured":self.source.is_some(),
            "type":self.source.as_ref().map(|s|&s.kind),
            "name":self.source.as_ref().map(|s|&s.name),
            "updatedAt":self.source.as_ref().map(|s|&s.updated_at),
            "lastError":self.error,
            "events":parsed.map_or(json!([]),|p|p["events"].clone()),
            "truncated":parsed.is_some_and(|p|p["truncated"]==true),
            "zone":parsed.map(|p|&p["zone"]),
        })
    }

    fn install(
        &mut self,
        store: &Store,
        source: Source,
        now: u64,
        zone: &str,
    ) -> Result<(), String> {
        let events = calendar_parse::parse(&source.text, now, zone)?;
        let _guard = store.lock().map_err(|_| "ICS_BUSY")?;
        calendar_storage::save(store, Some(&source))?;
        self.source = Some(source);
        self.events = Some(events);
        self.error = None;
        self.parsed_at = Some(now);
        self.fetch_attempt = None;
        Ok(())
    }

    fn refresh(
        &mut self,
        store: &Store,
        network: bool,
        now: u64,
        zone: &str,
    ) -> Result<(), String> {
        let old = self.source.as_ref().ok_or("ICS_NOT_CONFIGURED")?;
        let text = match &old.url {
            Some(url) => fetch(subscription_url(url)?, network)?,
            None => old.text.clone(),
        };
        let source = Source {
            kind: old.kind.clone(),
            name: old.name.clone(),
            url: old.url.clone(),
            text,
            updated_at: iso_now(),
        };
        self.install(store, source, now, zone)
    }
}

fn subscription_url(input: &str) -> Result<Url, String> {
    if input.encode_utf16().count() > 4096 {
        return Err("ICS_URL_INVALID".into());
    }
    let raw = if input
        .get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("webcal:"))
    {
        format!("https:{}", &input[7..])
    } else {
        input.into()
    };
    let url = Url::parse(&raw).map_err(|_| "ICS_URL_INVALID")?;
    public_https::validate(&url).map_err(|err| {
        if err == public_https::Error::Blocked {
            "ICS_URL_BLOCKED"
        } else {
            "ICS_URL_INVALID"
        }
    })?;
    Ok(url)
}

fn fetch(url: Url, network: bool) -> Result<String, String> {
    if !network {
        return Err("ICS_FETCH_FAILED".into());
    }
    let body = public_https::get(
        url,
        &public_https::Fetch {
            accept: "text/calendar",
            content_types: &[],
            max_bytes: 1048576,
            redirects: 3,
            seconds: 12,
        },
    )
    .map_err(|e| match e {
        public_https::Error::Blocked => "ICS_URL_BLOCKED",
        public_https::Error::Size => "ICS_TOO_LARGE",
        _ => "ICS_FETCH_FAILED",
    })?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

fn zone() -> Result<String, String> {
    iana_time_zone::get_timezone().map_err(|_| "ICS_TIMEZONE_UNSUPPORTED".into())
}

#[derive(Default)]
pub(crate) struct Calendar(Mutex<State>);

impl Calendar {
    pub(crate) fn events(&self, store: &Store, network: bool) -> Result<Value, String> {
        let mut state = self.0.lock().map_err(|_| "ICS_STORAGE_FAILED")?;
        state.load(store);
        let now = crate::journal::now_ms();
        if let Some(source) = &state.source {
            let parse_due = state
                .parsed_at
                .is_none_or(|at| now.checked_sub(at).is_none_or(|age| age > 60000));
            let fetch_due = source.url.is_some()
                && timestamp_ms(&source.updated_at)
                    .is_some_and(|at| i128::from(now) - at >= 900000)
                && state
                    .fetch_attempt
                    .is_none_or(|at| now.checked_sub(at).is_none_or(|age| age > 60000));
            if parse_due {
                match zone().and_then(|zone| {
                    calendar_parse::parse(&source.text, now, &zone).map_err(Into::into)
                }) {
                    Ok(events) => state.events = Some(events),
                    Err(e) => state.error = Some(e),
                }
                state.parsed_at = Some(now);
            }
            if fetch_due {
                state.fetch_attempt = Some(now);
                if let Err(e) = zone().and_then(|zone| state.refresh(store, network, now, &zone)) {
                    state.error = Some(e);
                }
            }
        }
        Ok(state.status())
    }

    pub(crate) fn change(
        &self,
        store: &Store,
        action: &str,
        body: &Value,
        network: bool,
    ) -> Result<Value, String> {
        let permitted: &[&str] = match action {
            "import" => &["name", "text"],
            "subscribe" => &["name", "url"],
            "refresh" | "disconnect" => &[],
            _ => return Err("API_NOT_FOUND".into()),
        };
        if !body
            .as_object()
            .is_some_and(|o| o.keys().all(|k| permitted.contains(&k.as_str())))
        {
            return Err("INVALID_JSON".into());
        }
        let mut state = match self.0.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => return Err("ICS_BUSY".into()),
            Err(TryLockError::Poisoned(_)) => return Err("ICS_STORAGE_FAILED".into()),
        };
        state.load(store);
        let result = (|| {
            if action == "disconnect" {
                let _guard = store.lock().map_err(|_| "ICS_BUSY")?;
                calendar_storage::save(store, None)?;
                *state = State {
                    initialized: true,
                    ..State::default()
                };
                return Ok(());
            }
            let now = crate::journal::now_ms();
            let zone = zone()?;
            if action == "refresh" {
                return state.refresh(store, network, now, &zone);
            }
            let (url, text) = if action == "subscribe" {
                let url = subscription_url(body["url"].as_str().ok_or("ICS_URL_INVALID")?)?;
                (Some(url.to_string()), fetch(url, network)?)
            } else {
                (
                    None,
                    body["text"].as_str().ok_or("ICS_TOO_LARGE")?.to_owned(),
                )
            };
            let mut units = 0;
            let name = body["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("我的日曆")
                .chars()
                .take_while(|c| {
                    units += c.len_utf16();
                    units <= 80
                })
                .collect();
            let source = Source {
                kind: if url.is_some() {
                    "subscription"
                } else {
                    "file"
                }
                .into(),
                name,
                url,
                text,
                updated_at: iso_now(),
            };
            state.install(store, source, now, &zone)
        })();
        if let Err(ref e) = result {
            state.error = Some(e.clone());
        }
        result.map(|_| state.status())
    }
}
