use crate::{portable_store::Store, providers, public_https};
use reqwest::Url;
use serde_json::{json, Value};
use std::sync::Mutex;

const ATTRIBUTION: &str = "Weather data by Open-Meteo";
const SOURCE: &str = "Open-Meteo model data; not a local observation";

fn coordinates(city: &str) -> (f64, f64) {
    match city {
        "東京" => (35.6762, 139.6503),
        "倫敦" => (51.5072, -0.1276),
        "紐約" => (40.7128, -74.006),
        _ => (25.033, 121.5654),
    }
}

pub(crate) fn parse(value: &Value, now: u64) -> Result<Value, &'static str> {
    let invalid = "WEATHER_RESPONSE_INVALID";
    let current = &value["current"];
    let temperature = current["temperature_2m"]
        .as_f64()
        .filter(|n| (-100.0..=70.0).contains(n))
        .ok_or(invalid)?;
    let humidity = current["relative_humidity_2m"]
        .as_f64()
        .filter(|n| (0.0..=100.0).contains(n))
        .ok_or(invalid)?;
    let code = crate::portable_store::safe_uint(&current["weather_code"])
        .filter(|n| {
            [
                0, 1, 2, 3, 45, 48, 51, 53, 55, 56, 57, 61, 63, 65, 66, 67, 71, 73, 75, 77, 80, 81,
                82, 85, 86, 95, 96, 97, 99,
            ]
            .contains(n)
        })
        .ok_or(invalid)?;
    let model = current["time"]
        .as_f64()
        .map(|t| t * 1000.0)
        .filter(|t| (now as f64 - t).abs() <= 48.0 * 3600000.0)
        .ok_or(invalid)?;
    let is_day = match current.get("is_day").map(crate::portable_store::safe_uint) {
        None => None,
        Some(Some(0)) => Some(false),
        Some(Some(1)) => Some(true),
        _ => return Err(invalid),
    };
    let effect = match code {
        95.. => "thunder",
        71 | 73 | 75 | 77 | 85 | 86 => "snow",
        51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => "rain",
        _ => "clear",
    };
    Ok(
        json!({"isDay":is_day,"temperatureC":temperature,"humidityPercent":humidity,
              "weatherCode":code,"effect":effect,"modelTimeMs":model}),
    )
}

fn fetch(city: &str) -> Result<Value, &'static str> {
    let (lat, lon) = coordinates(city);
    let mut url = Url::parse("https://api.open-meteo.com/v1/forecast").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("latitude", lat.to_string().as_str()),
        ("longitude", lon.to_string().as_str()),
        (
            "current",
            "temperature_2m,relative_humidity_2m,weather_code,is_day",
        ),
        ("timeformat", "unixtime"),
        ("timezone", "UTC"),
        ("forecast_days", "1"),
    ]);
    let bytes = public_https::get(
        url,
        &public_https::Fetch {
            accept: "application/json",
            content_types: &["application/json"],
            max_bytes: 65536,
            redirects: 0,
            seconds: 6,
        },
    )
    .map_err(|e| match e {
        public_https::Error::Size | public_https::Error::Type => "WEATHER_RESPONSE_INVALID",
        _ => "WEATHER_UNAVAILABLE",
    })?;
    serde_json::from_slice(&bytes).map_err(|_| "WEATHER_RESPONSE_INVALID")
}

#[derive(Default)]
struct Cache {
    city: String,
    revision: Option<u64>,
    attempt: Option<u64>,
    observation: Option<(u64, Value)>,
    error: Option<&'static str>,
}

#[derive(Default)]
pub(crate) struct Weather(Mutex<Cache>);

impl Weather {
    pub(crate) fn read(&self, store: &Store, network: bool) -> Result<Value, String> {
        self.current(
            || providers::settings(store),
            crate::journal::now_ms,
            |city| {
                if network {
                    fetch(city)
                } else {
                    Err("WEATHER_UNAVAILABLE")
                }
            },
        )
    }

    fn current(
        &self,
        settings: impl Fn() -> Result<Value, String>,
        clock: impl Fn() -> u64,
        fetch: impl FnOnce(&str) -> Result<Value, &'static str>,
    ) -> Result<Value, String> {
        let mut cache = self.0.lock().map_err(|_| "WEATHER_UNAVAILABLE")?;
        let s = settings()?;
        let revision = s["revision"].as_u64().ok_or("PROVIDERS_SETTINGS_INVALID")?;
        let city = s["weather"]["city"]
            .as_str()
            .ok_or("PROVIDERS_SETTINGS_INVALID")?;
        if cache.city != city || cache.revision != Some(revision) {
            *cache = Cache {
                city: city.into(),
                revision: Some(revision),
                ..Cache::default()
            };
        }
        let enabled = s["weather"]["enabled"] == true;
        if !enabled {
            return Ok(
                json!({"schemaVersion":1,"enabled":false,"available":false,"source":"none","settingsRevision":revision}),
            );
        }
        let now = clock();
        let age = cache
            .observation
            .as_ref()
            .and_then(|(at, _)| now.checked_sub(*at));
        if age.is_none_or(|age| age >= 900000)
            && cache
                .attempt
                .is_none_or(|at| now.checked_sub(at).is_none_or(|age| age >= 60000))
        {
            cache.attempt = Some(now);
            let result = fetch(city).and_then(|v| parse(&v, clock()));
            let latest = settings()?;
            if latest["revision"] != s["revision"] {
                return Ok(
                    json!({"schemaVersion":1,"enabled":latest["weather"]["enabled"],"available":false,
                                 "source":"none","settingsRevision":latest["revision"],"configurationChanged":true}),
                );
            }
            match result {
                Ok(mut v) => {
                    let at = clock();
                    v["city"] = city.into();
                    v["fetchedAtMs"] = at.into();
                    cache.observation = Some((at, v));
                    cache.error = None;
                }
                Err(code) => cache.error = Some(code),
            }
        }
        let now = clock();
        let age = cache
            .observation
            .as_ref()
            .and_then(|(at, _)| now.checked_sub(*at));
        let available = age.is_some_and(|age| age < 86400000);
        let mut response = json!({"schemaVersion":1,"enabled":true,"available":available,"source":SOURCE,
            "settingsRevision":revision,"city":city,
            "stale":cache.observation.is_some() && (cache.error.is_some() || age.is_some_and(|age| age >= 900000)),
            "offline":cache.error.is_some(),"error":cache.error.map(|code| json!({"code":code})),
            "attribution":{"name":ATTRIBUTION,"url":"https://open-meteo.com/","license":"CC BY 4.0; hosted free API is non-commercial only"}});
        if available {
            response["data"] = cache.observation.as_ref().unwrap().1.clone();
            response["ageMs"] = age.unwrap().into();
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_opt_in_cache_expiry_and_offline_follow_existing_contract() {
        let weather = Weather::default();
        let policy = |enabled| json!({"revision":1,"weather":{"enabled":enabled,"city":"台北"}});
        let now = 1_800_000_000_000;
        let get = || {
            Ok(
                json!({"current":{"is_day":0,"temperature_2m":20.1,"relative_humidity_2m":80,"weather_code":61,"time":now/1000}}),
            )
        };
        let disabled = weather
            .current(
                || Ok(policy(false)),
                || now,
                |_| panic!("disabled weather requested network"),
            )
            .unwrap();
        assert_eq!(disabled["enabled"], false);
        let live = weather
            .current(|| Ok(policy(true)), || now, |_| get())
            .unwrap();
        assert_eq!(live["data"]["effect"], "rain");
        assert_eq!(live["data"]["isDay"], false);
        let cached = weather
            .current(
                || Ok(policy(true)),
                || now + 1000,
                |_| panic!("fresh cache fetched again"),
            )
            .unwrap();
        assert_eq!(cached["ageMs"], 1000);
        let offline = weather
            .current(
                || Ok(policy(true)),
                || now + 900001,
                |_| Err("WEATHER_UNAVAILABLE"),
            )
            .unwrap();
        assert_eq!(offline["available"], true);
        assert_eq!(offline["stale"], true);
        let expired = weather
            .current(
                || Ok(policy(true)),
                || now + 86400000,
                |_| Err("WEATHER_UNAVAILABLE"),
            )
            .unwrap();
        assert_eq!(expired["available"], false);
        assert!(expired.get("data").is_none());
    }
}
