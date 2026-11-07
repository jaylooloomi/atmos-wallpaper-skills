//! Native appearance rotation with the same policy, event keys and receipts as JS.
use crate::{
    appearance_core::{request_id, Core, Receipt},
    desired_state,
    js_json::{object as obj, Value},
    presets,
};
use std::collections::BTreeSet;
use time::{Date, Month, OffsetDateTime};
use windows_sys::Win32::{
    Foundation::SYSTEMTIME,
    System::Time::{
        GetDynamicTimeZoneInformation, SystemTimeToTzSpecificLocalTimeEx,
        DYNAMIC_TIME_ZONE_INFORMATION,
    },
};

const FILE: &str = "schedule.json";
const STATUS: &str = "schedule-status.json";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::tests::{change, save, TestStore};

    #[test]
    fn interval_ticks_preserve_manual_grace_and_do_not_repeat_commits() {
        let store = TestStore::new();
        let core = store.core();
        let first = save(&core, "forest", 0);
        change(&core, "coast", "coast");
        let second = save(&core, "coast", 1);
        let mut policy = defaults();
        policy.set("enabled", true.into());
        policy.set("intervalMinutes", 1.into());
        policy.set("manualGraceMinutes", 2.into());
        policy.set("presetIds", Value::Array(vec![first, second]));
        let body = obj([
            ("schemaVersion", 1.into()),
            ("expectedRevision", 0.into()),
            ("requestId", "schedule".into()),
            ("policy", policy),
        ]);
        set(&core, &body).unwrap();
        assert_eq!(
            tick(&core, 120000).unwrap()["state"].as_str(),
            Some("applied")
        );
        assert_eq!(
            tick(&core, 122000).unwrap()["state"].as_str(),
            Some("already-applied")
        );
        change(&core, "manual", "desert");
        assert_eq!(
            tick(&core, 125000).unwrap()["snoozeUntil"].safe_uint(),
            Some(245000)
        );
        assert_eq!(
            tick(&core, 180000).unwrap()["state"].as_str(),
            Some("manual-change-grace")
        );
        assert_eq!(
            core.live().unwrap()["desired"]["visual"]["background"].as_str(),
            Some("desert")
        );
        assert_eq!(
            tick(&core, 246000).unwrap()["state"].as_str(),
            Some("applied")
        );
        assert_eq!(core.live().unwrap()["revision"].safe_uint(), Some(4));
    }

    #[test]
    fn local_events_use_previous_calendar_day_and_deduplicate_repeated_minutes() {
        let a: Value = "a".repeat(64).into();
        let b: Value = "b".repeat(64).into();
        let entry = |time: &str, id| obj([("time", time.into()), ("presetId", id)]);
        let mut policy = defaults();
        policy.set("enabled", true.into());
        policy.set("mode", "times".into());
        policy.set(
            "entries",
            Value::Array(vec![entry("20:00", b.clone()), entry("08:00", a.clone())]),
        );
        validate(&policy).unwrap();
        let day = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let before = event(&policy, 0, |_| Ok((day, 479))).unwrap();
        assert_eq!(before["key"].as_str(), Some("2025-12-31.2000"));
        assert_eq!(before["presetId"], b);
        let selected = event(&policy, 0, |_| Ok((day, 480))).unwrap();
        assert_eq!(selected["key"].as_str(), Some("2026-01-01.0800"));
        assert_eq!(selected["presetId"], a);
        assert_eq!(
            event(&policy, 3600000, |_| Ok((day, 480))).unwrap(),
            selected
        );
        assert!(local(crate::journal::now_ms()).is_ok());
    }
}

fn defaults() -> Value {
    obj([
        ("enabled", false.into()),
        ("mode", "interval".into()),
        ("intervalMinutes", 30.into()),
        ("presetIds", Value::Array(vec![])),
        ("entries", Value::Array(vec![])),
        ("manualGraceMinutes", 30.into()),
    ])
}

fn time_of_day(text: &str) -> Option<u16> {
    let &[a, b, b':', c, d] = text.as_bytes() else {
        return None;
    };
    if ![a, b, c, d].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let hour = (a - b'0') * 10 + b - b'0';
    let minute = (c - b'0') * 10 + d - b'0';
    (hour < 24 && minute < 60).then_some(u16::from(hour) * 60 + u16::from(minute))
}

fn validate(policy: &Value) -> Result<(), String> {
    let invalid = "SCHEDULE_INVALID";
    if !presets::shape(
        policy,
        &[
            "enabled",
            "mode",
            "intervalMinutes",
            "presetIds",
            "entries",
            "manualGraceMinutes",
        ],
    ) || !["intervalMinutes", "manualGraceMinutes"].iter().all(|k| {
        policy[k]
            .safe_uint()
            .is_some_and(|n| (1..=1440).contains(&n))
    }) {
        return Err(invalid.into());
    }
    let enabled = policy["enabled"].as_bool().ok_or(invalid)?;
    let ids = policy["presetIds"]
        .as_array()
        .filter(|a| a.len() <= 40)
        .ok_or(invalid)?;
    let entries = policy["entries"]
        .as_array()
        .filter(|a| a.len() <= 48)
        .ok_or(invalid)?;
    let active = match policy["mode"].as_str() {
        Some("interval") if entries.is_empty() => ids,
        Some("times") if ids.is_empty() => entries,
        _ => return Err(invalid.into()),
    };
    if enabled && active.is_empty() {
        return Err(invalid.into());
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        if !desired_state::hex_id(id) || !seen.insert(id.as_str().unwrap()) {
            return Err(invalid.into());
        }
    }
    let mut seen = BTreeSet::new();
    for e in entries {
        if !presets::shape(e, &["time", "presetId"]) || !desired_state::hex_id(&e["presetId"]) {
            return Err(invalid.into());
        }
        let minute = e["time"].as_str().and_then(time_of_day).ok_or(invalid)?;
        if !seen.insert(minute) {
            return Err(invalid.into());
        }
    }
    Ok(())
}

fn read(core: &Core) -> Result<Value, String> {
    let s = presets::optional(core, FILE, 1024 * 1024, "SCHEDULE_CORRUPT")?.unwrap_or_else(|| {
        obj([
            ("schemaVersion", 1.into()),
            ("revision", 0.into()),
            ("policy", defaults()),
            ("receipts", obj([])),
        ])
    });
    if !presets::shape(&s, &["schemaVersion", "revision", "policy", "receipts"])
        || s["schemaVersion"].safe_uint() != Some(1)
        || s["revision"].safe_uint().is_none()
        || s["receipts"].as_object().is_none()
    {
        return Err("SCHEDULE_CORRUPT".into());
    }
    validate(&s["policy"])?;
    Ok(s)
}

fn runtime(core: &Core) -> Value {
    presets::optional(core, STATUS, 16384, "SCHEDULE_CORRUPT")
        .ok()
        .flatten()
        .unwrap_or(Value::Null)
}

pub(crate) fn status(core: &Core) -> Result<Value, String> {
    let s = read(core)?;
    Ok(obj([
        ("schemaVersion", 1.into()),
        ("revision", s["revision"].clone()),
        ("policy", s["policy"].clone()),
        ("runtime", runtime(core)),
        ("timezone", "system-local".into()),
        ("scope", "appearance-only".into()),
    ]))
}

pub(crate) fn set(core: &Core, body: &Value) -> Result<Value, String> {
    if !presets::shape(
        body,
        &["schemaVersion", "expectedRevision", "requestId", "policy"],
    ) || body["schemaVersion"].safe_uint() != Some(1)
    {
        return Err("SCHEDULE_INVALID".into());
    }
    let id = request_id(&body["requestId"], 96)?;
    let expected = body["expectedRevision"]
        .safe_uint()
        .ok_or("INVALID_REVISION")?;
    let policy = &body["policy"];
    validate(policy)?;
    let _guard = core.lock()?;
    let mut s = read(core)?;
    let mut receipts = s["receipts"].clone();
    let fp = body.hash();
    if let Some(old) = receipts.get(id) {
        if old["fingerprint"].as_str() != Some(&fp) {
            return Err("REQUEST_CONFLICT".into());
        }
        let mut result = old["result"].clone();
        if result.as_object().is_none() {
            return Err("SCHEDULE_CORRUPT".into());
        }
        result.set("replayed", true.into());
        return Ok(result);
    }
    // The legacy lookup also sees inherited Object.prototype members.
    if [
        "constructor",
        "toString",
        "valueOf",
        "toLocaleString",
        "hasOwnProperty",
        "isPrototypeOf",
        "propertyIsEnumerable",
    ]
    .contains(&id)
    {
        return Err("REQUEST_CONFLICT".into());
    }
    if s["revision"].safe_uint() != Some(expected) {
        return Err("SCHEDULE_REVISION_CONFLICT".into());
    }
    let ids = policy["presetIds"].as_array().unwrap().iter().chain(
        policy["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| &e["presetId"]),
    );
    if ids.clone().next().is_some() {
        let catalog = presets::catalog(core)?;
        if !ids
            .into_iter()
            .all(|id| presets::find(&catalog, id).is_some())
        {
            return Err("SCHEDULE_PRESET_MISSING".into());
        }
    }
    if receipts.as_object().unwrap().len() >= 1000
        || expected >= crate::portable_store::MAX_SAFE_INTEGER
    {
        return Err("SCHEDULE_RECEIPT_LIMIT".into());
    }
    let result = obj([("saved", true.into()), ("revision", (expected + 1).into())]);
    receipts.set(
        id,
        obj([("fingerprint", fp.into()), ("result", result.clone())]),
    );
    s.set("receipts", receipts);
    s.set("revision", (expected + 1).into());
    s.set("policy", policy.clone());
    core.atomic(FILE, &s)?;
    Ok(result)
}

fn utc(now: u64) -> Result<OffsetDateTime, String> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(now) * 1_000_000)
        .map_err(|_| "SCHEDULE_CLOCK_UNAVAILABLE".into())
}

fn local(now: u64) -> Result<(Date, u16), String> {
    let utc = utc(now)?;
    let source = SYSTEMTIME {
        wYear: utc
            .year()
            .try_into()
            .map_err(|_| "SCHEDULE_CLOCK_UNAVAILABLE")?,
        wMonth: utc.month() as u16,
        wDay: u16::from(utc.day()),
        wHour: u16::from(utc.hour()),
        wMinute: u16::from(utc.minute()),
        wSecond: u16::from(utc.second()),
        wMilliseconds: utc.millisecond(),
        ..SYSTEMTIME::default()
    };
    let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
    let mut dest = SYSTEMTIME::default();
    if unsafe { GetDynamicTimeZoneInformation(&mut zone) } == u32::MAX
        || unsafe { SystemTimeToTzSpecificLocalTimeEx(&zone, &source, &mut dest) } == 0
    {
        return Err("SCHEDULE_CLOCK_UNAVAILABLE".into());
    }
    let month = Month::try_from(dest.wMonth as u8).map_err(|_| "SCHEDULE_CLOCK_UNAVAILABLE")?;
    let day = Date::from_calendar_date(i32::from(dest.wYear), month, dest.wDay as u8)
        .map_err(|_| "SCHEDULE_CLOCK_UNAVAILABLE")?;
    Ok((day, dest.wHour * 60 + dest.wMinute))
}

fn event(
    policy: &Value,
    now: u64,
    local_time: impl FnOnce(u64) -> Result<(Date, u16), String>,
) -> Result<Value, String> {
    let (key, preset) = if policy["mode"].as_str() == Some("interval") {
        let slot = now / (policy["intervalMinutes"].safe_uint().unwrap() * 60000);
        let ids = policy["presetIds"].as_array().unwrap();
        (
            format!("interval.{slot}"),
            &ids[(slot % ids.len() as u64) as usize],
        )
    } else {
        let (day, minute) = local_time(now)?;
        let entries = policy["entries"].as_array().unwrap();
        let time = |e: &Value| e["time"].as_str().and_then(time_of_day).unwrap();
        let selected = entries
            .iter()
            .filter(|e| time(e) <= minute)
            .max_by_key(|e| time(e));
        let (day, entry) = match selected {
            Some(e) => (day, e),
            None => (
                day.previous_day().ok_or("SCHEDULE_CLOCK_UNAVAILABLE")?,
                entries.iter().max_by_key(|e| time(e)).unwrap(),
            ),
        };
        let t = time(entry);
        (
            format!("{day}.{:02}{:02}", t / 60, t % 60),
            &entry["presetId"],
        )
    };
    Ok(obj([("key", key.into()), ("presetId", preset.clone())]))
}

fn applied_state(state: &str, key: &Value, revision: &Value) -> Value {
    obj([
        ("state", state.into()),
        ("eventKey", key.clone()),
        ("lastAppliedRevision", revision.clone()),
        ("observedRevision", revision.clone()),
        ("snoozeUntil", 0.into()),
    ])
}

fn tick(core: &Core, now: u64) -> Result<Value, String> {
    let _guard = core.lock()?;
    let s = read(core)?;
    let policy = &s["policy"];
    if policy["enabled"].as_bool() != Some(true) {
        return Ok(obj([("state", "disabled".into())]));
    }
    let e = event(policy, now, local)?;
    let key = &e["key"];
    let preset = &e["presetId"];
    let version = &s["revision"];
    let live = core.live()?;
    let current = &live["revision"];
    let id = format!(
        "schedule.{}.{}.{}",
        version.safe_uint().unwrap(),
        key.as_str().unwrap(),
        &preset.as_str().unwrap()[..12]
    );
    let receipt = Receipt {
        id,
        fingerprint: obj([("scheduleRevision", version.clone()), ("event", e.clone())]).hash(),
    };
    let record = |mut value: Value| -> Result<Value, String> {
        let t = utc(now)?;
        let at = format!(
            "{}T{:02}:{:02}:{:02}.{:03}Z",
            t.date(),
            t.hour(),
            t.minute(),
            t.second(),
            t.millisecond()
        );
        value.set("schemaVersion", 1.into());
        value.set("scheduleRevision", version.clone());
        value.set("at", at.into());
        core.atomic(STATUS, &value)?;
        Ok(value)
    };
    let state = runtime(core);
    let seen = core
        .receipt(&receipt.id, &receipt.fingerprint, &live)
        .map_err(|e| {
            if e == "REQUEST_CONFLICT" {
                "SCHEDULE_RECEIPT_CONFLICT".into()
            } else {
                e
            }
        })?;
    let already = |r: &Value| obj([("state", "already-applied".into()), ("revision", r.clone())]);
    if let Some(r) = seen.as_ref().filter(|r| r["revision"] == *current) {
        if state["eventKey"] != *key
            || state["scheduleRevision"] != *version
            || state["state"].as_str() == Some("error")
            || state["lastAppliedRevision"] != r["revision"]
        {
            record(applied_state("already-applied", key, &r["revision"]))?;
        }
        return Ok(already(&r["revision"]));
    }
    if state["scheduleRevision"] == *version {
        if state["observedRevision"] != *current && state["lastAppliedRevision"] != *current {
            let mut next = state;
            next.set("state", "manual-change-grace".into());
            next.set("observedRevision", current.clone());
            next.set(
                "snoozeUntil",
                now.saturating_add(policy["manualGraceMinutes"].safe_uint().unwrap() * 60000)
                    .into(),
            );
            return record(next);
        }
        if state["snoozeUntil"]
            .as_num()
            .is_some_and(|until| (now as f64) < until)
        {
            return Ok(obj([
                ("state", "manual-change-grace".into()),
                ("snoozeUntil", state["snoozeUntil"].clone()),
            ]));
        }
    }
    if let Some(r) = seen {
        return Ok(already(&r["revision"]));
    }
    let apply = || {
        let catalog = presets::catalog(core)?;
        let p = presets::find(&catalog, preset).ok_or("SCHEDULE_PRESET_MISSING")?;
        presets::apply(core, &live, p, &receipt)
    };
    match apply() {
        Ok(r) => record(applied_state("applied", key, &r["revision"])),
        Err(code) => record(obj([
            ("state", "error".into()),
            ("code", code.into()),
            ("eventKey", key.clone()),
            ("lastAppliedRevision", state["lastAppliedRevision"].clone()),
            ("observedRevision", current.clone()),
            (
                "snoozeUntil",
                state["snoozeUntil"].safe_uint().unwrap_or(0).into(),
            ),
        ])),
    }
}

pub(crate) fn tick_existing(owner: &Core) {
    let Ok(layout) = crate::layout::read(&owner.store.root) else {
        return;
    };
    let now = crate::journal::now_ms();
    let tick_if_configured = |core: &Core| {
        if core.path(FILE).is_ok_and(|p| p.is_file()) {
            let _ = tick(core, now);
        }
    };
    if layout.layout.mode == "independent" {
        for target in &layout.layout.targets {
            if let Ok(core) = owner.open_existing_scope(target) {
                tick_if_configured(&core);
            }
        }
    } else {
        tick_if_configured(owner);
    }
}
