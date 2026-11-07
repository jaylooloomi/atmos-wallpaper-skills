//! JSON.parse / canonical() compatibility for legacy desired-state hashes.
//! JS strings are UTF-16 (including lone surrogates), keys sort by UTF-16 units,
//! and every JSON number is an IEEE-754 double. serde_json::Value alone cannot
//! preserve all three properties. This AST is data only; it evaluates no JS.
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ops::Index};

#[derive(Clone, Debug, PartialEq)]
pub struct JsString {
    pub units: Vec<u16>,
    text: Option<String>,
}
impl JsString {
    pub(crate) fn new(units: Vec<u16>) -> Self {
        let text = String::from_utf16(&units).ok();
        Self { units, text }
    }
    pub fn as_str(&self) -> Option<&str> {
        self.text.as_deref()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(JsString),
    Array(Vec<Value>),
    Object(BTreeMap<Vec<u16>, Value>),
}
static NULL: Value = Value::Null;
impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&NULL)
    }
}
impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(o) => o.get(&key.encode_utf16().collect::<Vec<_>>()),
            _ => None,
        }
    }
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    pub fn set(&mut self, key: &str, value: Value) {
        if let Self::Object(o) = self {
            o.insert(key.encode_utf16().collect(), value);
        } else {
            panic!("set on non-object");
        }
    }
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        match self {
            Self::Object(o) => o.remove(&key.encode_utf16().collect::<Vec<_>>()),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => s.as_str(),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn safe_uint(&self) -> Option<u64> {
        self.as_num()
            .filter(|n| {
                n.is_finite() && *n >= 0.0 && *n <= 9_007_199_254_740_991.0 && n.fract() == 0.0
            })
            .map(|n| n as u64)
    }
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&BTreeMap<Vec<u16>, Value>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn string_len(&self) -> Option<usize> {
        match self {
            Self::String(s) => Some(s.units.len()),
            _ => None,
        }
    }
    pub fn canonical(&self) -> String {
        let mut output = String::new();
        self.write(&mut output);
        output
    }
    pub fn hash(&self) -> String {
        format!("{:x}", Sha256::digest(self.canonical().as_bytes()))
    }
    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(n) => {
                if !n.is_finite() {
                    out.push_str("null");
                } else if *n == 0.0 {
                    out.push('0');
                } else {
                    out.push_str(ryu_js::Buffer::new().format(*n));
                }
            }
            Self::String(s) => write_string(&s.units, out),
            Self::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Self::Object(o) => {
                out.push('{');
                for (i, (key, v)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(JsString::new(s.encode_utf16().collect()))
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}
impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Self::Number(n as f64)
    }
}
impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Self::Number(n as f64)
    }
}
impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Number(n as f64)
    }
}
impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}
pub fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, v)| (key.encode_utf16().collect(), v))
            .collect(),
    )
}
fn write_string(units: &[u16], out: &mut String) {
    out.push('"');
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        i += 1;
        match u {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            8 => out.push_str("\\b"),
            12 => out.push_str("\\f"),
            10 => out.push_str("\\n"),
            13 => out.push_str("\\r"),
            9 => out.push_str("\\t"),
            0..=31 => out.push_str(&format!("\\u{u:04x}")),
            0xd800..=0xdbff if i < units.len() && (0xdc00..=0xdfff).contains(&units[i]) => {
                let low = units[i];
                i += 1;
                out.push(
                    char::from_u32(0x10000 + (((u as u32) - 0xd800) << 10) + (low as u32) - 0xdc00)
                        .unwrap(),
                );
            }
            0xd800..=0xdfff => out.push_str(&format!("\\u{u:04x}")),
            _ => out.push(char::from_u32(u as u32).unwrap()),
        }
    }
    out.push('"');
}
pub fn parse(bytes: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "INVALID_JSON")?;
    let mut parser = Parser { text, at: 0 };
    let value = parser.value(0)?;
    parser.space();
    if parser.at != text.len() {
        return Err("INVALID_JSON".into());
    }
    Ok(value)
}
struct Parser<'a> {
    text: &'a str,
    at: usize,
}
impl Parser<'_> {
    fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }
    fn space(&mut self) {
        while self
            .bytes()
            .get(self.at)
            .is_some_and(|b| b" \r\n\t".contains(b))
        {
            self.at += 1;
        }
    }
    fn take(&mut self, byte: u8) -> bool {
        if self.bytes().get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > 256 {
            return Err("INVALID_JSON".into());
        }
        self.space();
        match self.bytes().get(self.at).copied() {
            Some(b'"') => Ok(Value::String(JsString::new(self.string()?))),
            Some(b'[') => {
                self.at += 1;
                let mut values = Vec::new();
                self.space();
                if self.take(b']') {
                    return Ok(Value::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.space();
                    if self.take(b']') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err("INVALID_JSON".into());
                    }
                }
                Ok(Value::Array(values))
            }
            Some(b'{') => {
                self.at += 1;
                let mut values = BTreeMap::new();
                self.space();
                if self.take(b'}') {
                    return Ok(Value::Object(values));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    if !self.take(b':') {
                        return Err("INVALID_JSON".into());
                    }
                    values.insert(key, self.value(depth + 1)?);
                    self.space();
                    if self.take(b'}') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err("INVALID_JSON".into());
                    }
                }
                Ok(Value::Object(values))
            }
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err("INVALID_JSON".into()),
        }
    }
    fn literal(&mut self, text: &str, value: Value) -> Result<Value, String> {
        if self.text[self.at..].starts_with(text) {
            self.at += text.len();
            Ok(value)
        } else {
            Err("INVALID_JSON".into())
        }
    }
    fn string(&mut self) -> Result<Vec<u16>, String> {
        if !self.take(b'"') {
            return Err("INVALID_JSON".into());
        }
        let mut units = Vec::new();
        loop {
            let b = *self.bytes().get(self.at).ok_or("INVALID_JSON")?;
            self.at += 1;
            match b {
                b'"' => return Ok(units),
                0..=31 => return Err("INVALID_JSON".into()),
                b'\\' => {
                    let escape = *self.bytes().get(self.at).ok_or("INVALID_JSON")?;
                    self.at += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => units.push(escape as u16),
                        b'b' => units.push(8),
                        b'f' => units.push(12),
                        b'n' => units.push(10),
                        b'r' => units.push(13),
                        b't' => units.push(9),
                        b'u' => {
                            let end = self.at.checked_add(4).ok_or("INVALID_JSON")?;
                            let hex = self.bytes().get(self.at..end).ok_or("INVALID_JSON")?;
                            if !hex.iter().all(|b| b.is_ascii_hexdigit()) {
                                return Err("INVALID_JSON".into());
                            }
                            units.push(
                                u16::from_str_radix(std::str::from_utf8(hex).unwrap(), 16)
                                    .map_err(|_| "INVALID_JSON")?,
                            );
                            self.at = end;
                        }
                        _ => return Err("INVALID_JSON".into()),
                    }
                }
                32..=127 => units.push(b as u16),
                _ => {
                    self.at -= 1;
                    let ch = self.text[self.at..].chars().next().unwrap();
                    let mut buf = [0u16; 2];
                    units.extend_from_slice(ch.encode_utf16(&mut buf));
                    self.at += ch.len_utf8();
                }
            }
        }
    }
    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        self.take(b'-');
        if !self.take(b'0') {
            if !self
                .bytes()
                .get(self.at)
                .is_some_and(|b| (b'1'..=b'9').contains(b))
            {
                return Err("INVALID_JSON".into());
            }
            while self.bytes().get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self.take(b'.') {
            let first = self.at;
            while self.bytes().get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
            if first == self.at {
                return Err("INVALID_JSON".into());
            }
        }
        if self.take(b'e') || self.take(b'E') {
            if !self.take(b'+') {
                self.take(b'-');
            }
            let first = self.at;
            while self.bytes().get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
            if first == self.at {
                return Err("INVALID_JSON".into());
            }
        }
        self.text[start..self.at]
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| "INVALID_JSON".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Case {
        input: String,
        canonical: String,
        sha256: String,
    }
    #[test]
    fn canonical_bytes_match_original_js_fixtures() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../tests/fixtures/js-json-contract.json")).unwrap();
        for (i, case) in cases.iter().enumerate() {
            let value = parse(case.input.as_bytes()).unwrap();
            assert_eq!(value.canonical(), case.canonical, "case {i}");
            assert_eq!(value.hash(), case.sha256, "case {i}");
        }
    }
    #[test]
    fn invalid_json_remains_invalid() {
        for text in [
            "",
            "undefined",
            "01",
            "-01",
            "+1",
            "1.",
            "1e",
            "[1,]",
            "{\"x\":1,}",
            "\"\\uZZZZ\"",
            "true false",
            "\"unterminated",
        ] {
            assert!(parse(text.as_bytes()).is_err(), "{text}");
        }
        assert!(parse(&[0xff]).is_err());
    }
}
