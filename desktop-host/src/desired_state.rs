//! Existing desired formats 1..4 and strict object merge, with no UI/schema
//! relaxation. Native strings retain JS UTF-16 length and lone-surrogate data.
use crate::js_json::{self, Value};
use std::collections::BTreeSet;

fn catalog() -> Value {
    js_json::parse(include_bytes!("cli-catalogs/state-defaults.json")).expect("embedded defaults")
}
pub fn initial() -> Value {
    catalog()["4"].clone()
}
pub fn capabilities() -> Value {
    catalog()["capabilities"].clone()
}
fn keys(v: &Value, base: &Value) -> Result<(), String> {
    let object = v.as_object().ok_or("INVALID_STATE")?;
    let allowed = base.as_object().unwrap();
    if object.keys().any(|k| !allowed.contains_key(k)) {
        return Err("UNKNOWN_FIELD".into());
    }
    Ok(())
}
fn required(v: &Value, base: &Value) -> Result<(), String> {
    keys(v, base)?;
    if base
        .as_object()
        .unwrap()
        .keys()
        .any(|key| !v.as_object().unwrap().contains_key(key))
    {
        return Err("INVALID_STATE".into());
    }
    Ok(())
}
fn number(v: &Value, min: f64, max: f64) -> Result<(), String> {
    if v.as_num()
        .is_some_and(|n| n.is_finite() && n >= min && n <= max)
    {
        Ok(())
    } else {
        Err("INVALID_STATE".into())
    }
}
fn choice(v: &Value, choices: &[&str]) -> Result<(), String> {
    if v.as_str().is_some_and(|s| choices.contains(&s)) {
        Ok(())
    } else {
        Err("INVALID_STATE".into())
    }
}
fn boolean(v: &Value) -> Result<(), String> {
    if v.as_bool().is_some() {
        Ok(())
    } else {
        Err("INVALID_STATE".into())
    }
}
fn appearance(size: &Value, color: &Value, max: f64) -> Result<(), String> {
    if !size.is_null() {
        number(size, 12.0, max)?;
    }
    if !color.is_null()
        && !color.as_str().is_some_and(|s| {
            s.len() == 7
                && s.starts_with('#')
                && s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        })
    {
        return Err("INVALID_STATE".into());
    }
    Ok(())
}
pub fn hex_id(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn version(d: &Value) -> Result<u64, String> {
    if !d.has("formatVersion") {
        return Ok(1);
    }
    match d["formatVersion"].safe_uint() {
        Some(v @ 2..=4) => Ok(v),
        _ => Err("DESIRED_VERSION_UNSUPPORTED".into()),
    }
}
pub fn validate(d: &Value) -> Result<(), String> {
    let version = version(d)?;
    let defaults = catalog();
    let base = &defaults[&version.to_string()];
    required(d, base)?;
    required(&d["visual"], &base["visual"])?;
    let v = &d["visual"];
    if version >= 4 {
        if ![Some(12.0), Some(24.0)].contains(&v["hourCycle"].as_num()) {
            return Err("INVALID_STATE".into());
        }
        boolean(&v["showSeconds"])?;
    }
    if version >= 3 {
        choice(&v["previewMode"], &["auto", "manual"])?;
        for key in ["previewWidth", "previewHeight"] {
            number(&v[key], 320.0, 7680.0)?;
            if v[key].as_num().unwrap().fract() != 0.0 {
                return Err("INVALID_STATE".into());
            }
        }
        if v["lockScene"].as_bool() != Some(true) {
            return Err("FIXED_SETTING".into());
        }
    }
    let fonts = ["theme", "system", "sans", "serif", "mono", "rounded"];
    if version >= 2 {
        appearance(&v["clockFontSize"], &v["clockColor"], 160.0)?;
        appearance(&v["calendarFontSize"], &v["calendarColor"], 160.0)?;
        choice(&v["locale"], &["zh-TW", "zh-CN", "en", "ja", "ko"])?;
        for key in ["clockFont", "calendarFont"] {
            choice(&v[key], &fonts)?;
        }
    }
    for key in [
        "reduced",
        "lightning",
        "clockVisible",
        "calendarVisible",
        "paused",
        "lockScene",
    ] {
        boolean(&v[key])?;
    }
    for (key, min, max) in if version == 1 {
        vec![
            ("clockX", 15.0, 70.0),
            ("clockY", 15.0, 85.0),
            ("clockSize", 60.0, 130.0),
            ("calendarY", 3.0, 75.0),
            ("calendarWidth", 15.0, 37.0),
            ("intensity", 0.0, 100.0),
            ("mobileClockX", 15.0, 70.0),
            ("mobileClockY", 15.0, 85.0),
            ("mobileCalendarY", 3.0, 75.0),
        ]
    } else {
        vec![
            ("calendarX", 0.0, 100.0),
            ("mobileCalendarX", 0.0, 100.0),
            ("clockX", 0.0, 100.0),
            ("clockY", 0.0, 100.0),
            ("clockSize", 60.0, 130.0),
            ("calendarY", 0.0, 100.0),
            ("calendarWidth", 15.0, 37.0),
            ("intensity", 0.0, 100.0),
            ("mobileClockX", 0.0, 100.0),
            ("mobileClockY", 0.0, 100.0),
            ("mobileCalendarY", 0.0, 100.0),
        ]
    } {
        number(&v[key], min, max)?;
    }
    choice(
        &v["background"],
        &["forest", "city", "future", "coast", "alpine", "desert"],
    )?;
    if version >= 2 && v["effect"].as_str() == Some("tornado") {
        return Err("EFFECT_REMOVED".into());
    }
    choice(
        &v["effect"],
        if version == 1 {
            &["clear", "rain", "snow", "typhoon", "tornado", "thunder"]
        } else if version < 4 {
            &["clear", "rain", "snow", "typhoon", "thunder"]
        } else {
            &[
                "clear",
                "rain",
                "snow",
                "typhoon",
                "thunder",
                "meteor",
                "firefly",
                "sakura",
                "fireworks",
                "galaxy",
            ]
        },
    )?;
    if version >= 2
        && (v["paused"].as_bool() != Some(false)
            || v["reduced"].as_bool() != Some(false)
            || v["lightning"].as_bool() != Some(true))
    {
        return Err("FIXED_SETTING".into());
    }
    choice(&v["theme"], &["natural", "neon", "warm", "minimal"])?;
    choice(
        &v["calendarMode"],
        if version < 4 {
            &["off", "demo"]
        } else {
            &["off", "demo", "ics"]
        },
    )?;
    choice(&v["layout"], &["A"])?;
    if !d["assetId"].is_null() && !hex_id(&d["assetId"]) {
        return Err("INVALID_STATE".into());
    }
    let w = &d["widgets"];
    required(w, &base["widgets"])?;
    if w["schema"].as_num() != Some(1.0) {
        return Err("INVALID_STATE".into());
    }
    if !w["desktopZone"].is_null() {
        choice(&w["desktopZone"], &["scene", "tray"])?;
    }
    required(&w["cards"], &base["widgets"]["cards"])?;
    for id in ["weather", "todo", "timer", "system", "media"] {
        let c = &w["cards"][id];
        required(c, &base["widgets"]["cards"][id])?;
        for key in ["enabled", "custom", "mobileCustom"] {
            boolean(&c[key])?;
        }
        if version >= 2 {
            appearance(&c["fontSize"], &c["color"], 48.0)?;
        }
        choice(&c["size"], &["S", "M"])?;
        if version >= 2 {
            choice(&c["font"], &fonts)?;
        }
        for key in ["x", "y", "mx", "my"] {
            number(&c[key], 0.0, 100.0)?;
        }
    }
    let todos = w["todos"]
        .as_array()
        .filter(|a| a.len() <= 50)
        .ok_or("INVALID_STATE")?;
    let mut ids = BTreeSet::new();
    let todo_base = js_json::object([
        ("id", "".into()),
        ("text", "".into()),
        ("done", false.into()),
    ]);
    for todo in todos {
        required(todo, &todo_base)?;
        if !todo["id"].string_len().is_some_and(|n| n > 0 && n <= 80)
            || !todo["text"].string_len().is_some_and(|n| n <= 160)
        {
            return Err("INVALID_STATE".into());
        }
        let Value::String(id) = &todo["id"] else {
            return Err("INVALID_STATE".into());
        };
        if !ids.insert(id.units.clone()) {
            return Err("INVALID_STATE".into());
        }
        boolean(&todo["done"])?;
    }
    if !w["undo"].is_null() {
        return Err("INVALID_STATE".into());
    }
    choice(&w["city"], &["台北", "東京", "倫敦", "紐約"])?;
    choice(&w["weatherState"], &["demo", "denied", "offline", "empty"])?;
    choice(&w["systemState"], &["demo", "unavailable"])?;
    choice(&w["mediaState"], &["demo", "empty"])?;
    required(&w["metrics"], &base["widgets"]["metrics"])?;
    let mut any = false;
    for key in ["cpu", "ram", "up", "down"] {
        boolean(&w["metrics"][key])?;
        any |= w["metrics"][key].as_bool().unwrap();
    }
    if !any {
        return Err("INVALID_STATE".into());
    }
    let t = &w["timer"];
    required(t, &base["widgets"]["timer"])?;
    choice(&t["phase"], &["work", "break"])?;
    number(&t["minutes"], 1.0, 120.0)?;
    number(&t["deadlineUtc"], 0.0, 9_007_199_254_740_991.0)?;
    number(&t["pausedRemaining"], 0.0, 7200000.0)?;
    boolean(&t["running"])?;
    boolean(&t["finished"])?;
    if t["running"].as_bool() == Some(true)
        && (t["deadlineUtc"].as_num() == Some(0.0) || t["finished"].as_bool() == Some(true))
    {
        return Err("INVALID_STATE".into());
    }
    Ok(())
}
pub fn validate_current(d: &Value) -> Result<(), String> {
    validate(d)?;
    if version(d)? != 4 {
        return Err("INVALID_STATE".into());
    }
    Ok(())
}
pub fn normalize(d: &Value) -> Result<Value, String> {
    validate(d)?;
    if version(d)? == 4 {
        return Ok(d.clone());
    }
    let mut output = d.clone();
    let mut visual = initial()["visual"].clone();
    for (key, value) in d["visual"].as_object().unwrap() {
        let Value::Object(o) = &mut visual else {
            unreachable!()
        };
        o.insert(key.clone(), value.clone());
    }
    visual.set("lockScene", true.into());
    visual.set("paused", false.into());
    visual.set("reduced", false.into());
    visual.set("lightning", true.into());
    if visual["effect"].as_str() == Some("tornado") {
        visual.set("effect", "clear".into());
    }
    output.set("formatVersion", 4.into());
    output.set("visual", visual);
    let mut widgets = output["widgets"].clone();
    let mut cards = widgets["cards"].clone();
    for id in ["weather", "todo", "timer", "system", "media"] {
        let mut c = cards[id].clone();
        if !c.has("font") {
            c.set("font", "theme".into());
        }
        if !c.has("fontSize") {
            c.set("fontSize", Value::Null);
        }
        if !c.has("color") {
            c.set("color", Value::Null);
        }
        cards.set(id, c);
    }
    widgets.set("cards", cards);
    output.set("widgets", widgets);
    validate_current(&output)?;
    Ok(output)
}
pub fn merge(current: &Value, patch: &Value) -> Result<Value, String> {
    fn walk(current: &Value, patch: &Value) -> Result<Value, String> {
        let updates = patch.as_object().ok_or("INVALID_STATE")?;
        let mut output = current.clone();
        for (key, value) in updates {
            let name = String::from_utf16(key).map_err(|_| "UNKNOWN_FIELD")?;
            if !current.has(&name)
                || ["__proto__", "prototype", "constructor"].contains(&name.as_str())
            {
                return Err("UNKNOWN_FIELD".into());
            }
            let next = if value.as_object().is_some() {
                if current[&name].as_object().is_none() {
                    return Err("INVALID_STATE".into());
                }
                walk(&current[&name], value)?
            } else {
                value.clone()
            };
            output.set(&name, next);
        }
        Ok(output)
    }
    let value = walk(current, patch)?;
    validate_current(&value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Case {
        input: String,
        normalized: Option<String>,
        error: Option<String>,
    }
    #[test]
    fn validation_and_legacy_normalization_match_original_js() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../tests/fixtures/desired-state-contract.json"
        ))
        .unwrap();
        for (i, case) in cases.iter().enumerate() {
            let d = js_json::parse(case.input.as_bytes()).unwrap();
            match normalize(&d) {
                Ok(v) => assert_eq!(Some(v.canonical()), case.normalized, "case {i}"),
                Err(e) => assert_eq!(Some(e), case.error, "case {i}"),
            }
        }
    }
}
