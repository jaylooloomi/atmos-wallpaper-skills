//! Native generated weather audio. No microphone, global volume or other players.
use crate::{host_settings, windows};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub enabled: bool,
    pub muted: bool,
    pub volume: u8,
    pub focus_only: bool,
    pub source: String,
    pub output: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub audio: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            last_request_id: None,
            audio: Policy {
                enabled: false,
                muted: true,
                volume: 50,
                focus_only: false,
                source: "weather".into(),
                output: "system-default".into(),
            },
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Update {
    schema_version: u32,
    request_id: String,
    expected_revision: u64,
    audio: Policy,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        && id.as_bytes()[0].is_ascii_alphanumeric()
}
fn valid_output(s: &str) -> bool {
    s == "system-default"
        || (s.len() == 71
            && s.starts_with("device-")
            && s[7..]
                .bytes()
                .all(|c| c.is_ascii_digit() || b"abcdef".contains(&c)))
}
fn validate(s: &Settings) -> Result<(), String> {
    if s.schema_version != 1
        || s.revision > 9_007_199_254_740_991
        || s.audio.volume > 100
        || !matches!(s.audio.source.as_str(), "weather" | "rain" | "thunder")
        || !valid_output(&s.audio.output)
        || s.last_request_id.as_ref().is_some_and(|id| !valid_id(id))
    {
        return Err("AUDIO_SETTINGS_INVALID".into());
    }
    Ok(())
}
pub fn read(root: &Path) -> Result<Settings, String> {
    let p = root.join("audio-settings.json");
    if !p.exists() {
        return Ok(Settings::default());
    }
    if fs::metadata(&p).map_err(|_| "AUDIO_READ_FAILED")?.len() > 16384 {
        return Err("AUDIO_SETTINGS_INVALID".into());
    }
    let s: Settings = serde_json::from_slice(&fs::read(p).map_err(|_| "AUDIO_READ_FAILED")?)
        .map_err(|_| "AUDIO_SETTINGS_INVALID")?;
    validate(&s)?;
    Ok(s)
}
fn receipt(root: &Path, id: &str) -> PathBuf {
    root.join("audio-requests")
        .join(format!("{:x}.json", Sha256::digest(id.as_bytes())))
}
fn update(root: &Path, u: Update) -> Result<Value, String> {
    if u.schema_version != 1 || !valid_id(&u.request_id) {
        return Err("AUDIO_SETTINGS_INVALID".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let _guard = windows::named_singleton(&format!(
        "Local\\AtmosWallpaper.Audio.{:x}",
        Sha256::digest(root.to_string_lossy().to_lowercase().as_bytes())
    ))
    .map_err(|_| "AUDIO_BUSY")?;
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&u).unwrap()));
    let old = read(&root)?;
    // Finish the previous pointer-bound receipt before permitting the next commit.
    if let Some(id) = &old.last_request_id {
        let p = receipt(&root, id);
        if let Ok(b) = fs::read(&p) {
            if let Ok(mut r) = serde_json::from_slice::<Value>(&b) {
                if r["pending"] == true && r["result"]["revision"] == old.revision {
                    r["pending"] = json!(false);
                    host_settings::atomic(
                        p.parent().unwrap(),
                        p.file_name().unwrap().to_str().unwrap(),
                        &r,
                    )?;
                }
            }
        }
    }
    let p = receipt(&root, &u.request_id);
    if let Ok(b) = fs::read(&p) {
        let r: Value = serde_json::from_slice(&b).map_err(|_| "AUDIO_RECEIPT_INVALID")?;
        if r["fingerprint"] != fingerprint {
            return Err("AUDIO_REQUEST_CONFLICT".into());
        }
        if r["pending"] == false {
            let mut out = r["result"].clone();
            out["replayed"] = json!(true);
            return Ok(out);
        }
    }
    if old.revision != u.expected_revision {
        return Err("AUDIO_REVISION_CONFLICT".into());
    }
    let next = Settings {
        schema_version: 1,
        revision: old.revision + 1,
        last_request_id: Some(u.request_id),
        audio: u.audio,
    };
    validate(&next)?;
    let result = json!({"committed":true,"revision":next.revision});
    let pending = json!({"fingerprint":fingerprint,"pending":true,"result":result});
    host_settings::atomic(
        p.parent().unwrap(),
        p.file_name().unwrap().to_str().unwrap(),
        &pending,
    )?;
    host_settings::atomic(&root, "audio-settings.previous.json", &old)?;
    host_settings::atomic(&root, "audio-settings.json", &next)?;
    host_settings::atomic(
        p.parent().unwrap(),
        p.file_name().unwrap().to_str().unwrap(),
        &json!({"fingerprint":fingerprint,"pending":false,"result":result}),
    )?;
    Ok(result)
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if ![1, 3].contains(&args.len())
        || !matches!(
            args[0].as_str(),
            "get" | "update" | "probe" | "diagnose" | "test"
        )
    {
        return Err("AUDIO_COMMAND_INVALID".into());
    }
    let root = if args.len() == 3 {
        let p = PathBuf::from(&args[2]);
        if args[1] != "--store" || !p.is_absolute() || p.to_string_lossy().starts_with(r"\\") {
            return Err("AUDIO_COMMAND_INVALID".into());
        }
        p
    } else {
        host_settings::directory()?
    };
    let result = if args[0] == "update" {
        let mut b = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        if b.len() > 16384 {
            return Err("AUDIO_SETTINGS_INVALID".into());
        }
        Some(update(
            &root,
            serde_json::from_slice(&b).map_err(|_| "AUDIO_SETTINGS_INVALID")?,
        )?)
    } else if args[0] == "test" {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct TestInput {
            schema_version: u32,
            expected_revision: u64,
        }
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(1025)
            .read_to_end(&mut bytes)
            .map_err(|_| "AUDIO_SETTINGS_INVALID")?;
        if bytes.len() > 1024 {
            return Err("AUDIO_SETTINGS_INVALID".into());
        }
        let input: TestInput =
            serde_json::from_slice(&bytes).map_err(|_| "AUDIO_SETTINGS_INVALID")?;
        let s = read(&root)?;
        if input.schema_version != 1 {
            return Err("AUDIO_SETTINGS_INVALID".into());
        }
        if input.expected_revision != s.revision {
            return Err("AUDIO_REVISION_CONFLICT".into());
        }
        if !s.audio.enabled || s.audio.muted || s.audio.volume == 0 {
            return Err("AUDIO_TEST_BLOCKED".into());
        }
        let now = crate::journal::now_ms() as u64;
        let runtime = fs::read(root.join("audio-status.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .filter(|v| {
                v["pid"]
                    .as_u64()
                    .is_some_and(|pid| pid <= u32::MAX as u64 && windows::process_alive(pid as u32))
                    && v["capturedAtMs"]
                        .as_u64()
                        .is_some_and(|at| at <= now && now - at < 3000)
                    && v["audioSession"].as_str().is_some_and(|id| !id.is_empty())
                    && v["state"] != "stopped"
            })
            .ok_or("AUDIO_HOST_UNAVAILABLE")?;
        let expires = crate::journal::now_ms() + 3000;
        host_settings::atomic(
            &root,
            "audio-test.json",
            &json!({"schemaVersion":1,"settingsRevision":s.revision,"expiresAtMs":expires,"hostPid":runtime["pid"],"audioSession":runtime["audioSession"]}),
        )?;
        Some(json!({"testRequested":true,"durationMs":3000,"settingsUnchanged":true}))
    } else if args[0] == "diagnose" {
        Some(crate::audio_diagnostics::read(&root)?)
    } else if args[0] == "probe" {
        let gain = Arc::new(AtomicU32::new(0f32.to_bits()));
        let source = Arc::new(AtomicU32::new(1));
        let error = Arc::new(AtomicBool::new(false));
        let selected = read(&root)?.audio.output;
        let stream = open(&selected, gain, source, error.clone())?;
        thread::sleep(Duration::from_millis(120));
        drop(stream);
        if error.load(Ordering::Relaxed) {
            return Err("AUDIO_DEVICE_ERROR".into());
        }
        Some(
            json!({"silentOutputProbe":true,"audibilityVerified":false,"backend":"WASAPI via CPAL","output":selected}),
        )
    } else {
        None
    };
    let runtime = fs::read(root.join("audio-status.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .filter(|v| {
            if !v["pid"]
                .as_u64()
                .is_some_and(|pid| pid <= u32::MAX as u64 && windows::process_alive(pid as u32))
            {
                return false;
            }
            v["capturedAtMs"]
                .as_u64()
                .is_some_and(|t| (crate::journal::now_ms() as u64).abs_diff(t) < 3000)
        });
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"settings":read(&root)?,"runtime":runtime,"devices":crate::audio_diagnostics::devices(),"defaultDeviceName":crate::audio_diagnostics::default_name(),"result":result})
    );
    Ok(0)
}
struct Synth {
    rng: u32,
    low: f32,
    deep: f32,
    gain: f32,
    frames: u64,
    next_boom: f64,
    boom_at: f64,
    rate: f32,
}
impl Synth {
    fn new(rate: f32) -> Self {
        Self {
            rng: 0x953f2341,
            low: 0.,
            deep: 0.,
            gain: 0.,
            frames: 0,
            next_boom: 3.,
            boom_at: -100.,
            rate,
        }
    }
    fn sample(&mut self, target: f32, source: u32) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        let noise = (self.rng as f32 / u32::MAX as f32) * 2. - 1.;
        self.low += 0.06 * (noise - self.low);
        self.deep += 0.003 * (noise - self.deep);
        let time = self.frames as f64 / self.rate as f64;
        self.frames += 1;
        if time >= self.next_boom {
            self.boom_at = time;
            self.next_boom = time + 9. + (self.rng % 9000) as f64 / 1000.;
        }
        let age = (time - self.boom_at) as f32;
        let boom = if (source == 2 || (source >= 8 && source & 16 != 0)) && age >= 0. && age < 5. {
            (1. - (-age * 20.).exp())
                * (-age * 1.3).exp()
                * (self.deep * 5. + (age * 47. * std::f32::consts::TAU).sin() * 0.08)
        } else {
            0.
        };
        let rain = if source == 4 {
            (time * 660. * std::f64::consts::TAU).sin() as f32 * 0.2
        } else if source == 3 {
            self.low * 0.35 + noise * 0.035
        } else {
            noise * 0.08 + self.low * 0.12
        };
        if source == 4 {
            self.gain = self.gain.min(0.3);
        }
        let a = 1. - (-1. / (self.rate * 0.025)).exp();
        self.gain += a * (target - self.gain);
        if target == 0. && self.gain < 0.000001 {
            self.gain = 0.
        }
        let sample = if source >= 8 {
            let mut total = 0.;
            let mut count = 0.;
            if source & 8 != 0 {
                total += noise * 0.08 + self.low * 0.12;
                count += 1.;
            }
            if source & 16 != 0 {
                total += noise * 0.08 + self.low * 0.12 + boom;
                count += 1.;
            }
            if source & 32 != 0 {
                total += self.low * 0.35 + noise * 0.035;
                count += 1.;
            }
            if count > 0. {
                total / count
            } else {
                0.
            }
        } else {
            rain + boom
        };
        sample.clamp(-0.6, 0.6) * self.gain
    }
}
fn stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    gain: Arc<AtomicU32>,
    source: Arc<AtomicU32>,
    error: Arc<AtomicBool>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut synth = Synth::new(config.sample_rate as f32);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let target = f32::from_bits(gain.load(Ordering::Relaxed));
                let kind = source.load(Ordering::Relaxed);
                for frame in data.chunks_mut(channels) {
                    let value = synth.sample(target, kind);
                    for s in frame {
                        *s = T::from_sample(value);
                    }
                }
            },
            move |_| error.store(true, Ordering::Relaxed),
            Some(Duration::from_secs(3)),
        )
        .map_err(|_| "AUDIO_DEVICE_ERROR".into())
}
fn open(
    output: &str,
    gain: Arc<AtomicU32>,
    source: Arc<AtomicU32>,
    error: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = if output == "system-default" {
        host.default_output_device().ok_or("AUDIO_NO_DEVICE")?
    } else {
        host.output_devices()
            .map_err(|_| "AUDIO_NO_DEVICE")?
            .find(|d| {
                d.id().ok().is_some_and(|id| {
                    format!("device-{:x}", Sha256::digest(id.to_string().as_bytes())) == output
                })
            })
            .ok_or("AUDIO_OUTPUT_OFFLINE")?
    };
    let supported = device
        .default_output_config()
        .map_err(|_| "AUDIO_DEVICE_ERROR")?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    if config.channels == 0
        || config.channels > 32
        || config.sample_rate < 8000
        || config.sample_rate > 192000
    {
        return Err("AUDIO_FORMAT_UNSUPPORTED".into());
    }
    macro_rules! make {
        ($t:ty) => {
            stream::<$t>(&device, config, gain, source, error)
        };
    }
    let s = match format {
        cpal::SampleFormat::F32 => make!(f32),
        cpal::SampleFormat::F64 => make!(f64),
        cpal::SampleFormat::I16 => make!(i16),
        cpal::SampleFormat::U16 => make!(u16),
        cpal::SampleFormat::I32 => make!(i32),
        cpal::SampleFormat::U32 => make!(u32),
        _ => return Err("AUDIO_FORMAT_UNSUPPORTED".into()),
    }?;
    s.play().map_err(|_| "AUDIO_DEVICE_ERROR")?;
    Ok(s)
}
fn effect(root: &Path) -> u32 {
    fs::read(root.join("live.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|v| match v["desired"]["visual"]["effect"].as_str() {
            Some("rain") => 1,
            Some("thunder") => 2,
            Some("typhoon") => 3,
            _ => 0,
        })
        .unwrap_or(0)
}
fn reason(p: &Policy, attached: bool, desktop: bool, unlocked: bool, source: u32) -> &'static str {
    if !p.enabled {
        "disabled"
    } else if p.muted {
        "muted"
    } else if p.volume == 0 {
        "volume-zero"
    } else if !unlocked {
        "session-locked-or-unknown"
    } else if !attached {
        "wallpaper-cleared-or-unavailable"
    } else if p.focus_only && !desktop {
        "desktop-not-focused"
    } else if source == 0 {
        "source-silent"
    } else {
        "playing"
    }
}
#[derive(Clone, Debug)]
pub struct AudioTarget {
    pub id: String,
    pub primary: bool,
    pub handle: isize,
    pub source_root: PathBuf,
}
fn policy_allows_target(status: &Value, id: &str, now: u64) -> bool {
    status["pid"].as_u64() == Some(std::process::id() as u64)
        && status["capturedAtMs"]
            .as_u64()
            .is_some_and(|at| at <= now && now - at < 3000)
        && status["targets"].as_array().is_some_and(|targets| {
            targets.iter().any(|t| {
                t["targetId"] == id
                    && t["nativeAttached"] == true
                    && t["runtimePolicy"]["paused"] == false
            })
        })
}
fn test_marker_active(v: &Value, revision: Option<u64>, session: &str, now: u64) -> bool {
    v["hostPid"].as_u64() == Some(std::process::id() as u64)
        && v["audioSession"].as_str() == Some(session)
        && v["settingsRevision"].as_u64() == revision
        && v["expiresAtMs"]
            .as_u64()
            .is_some_and(|e| e > now && e <= now + 3500)
}
pub struct AudioEngine {
    targets: Arc<Mutex<Vec<AudioTarget>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl AudioEngine {
    pub fn start(root: PathBuf, window: isize) -> Self {
        let targets = Arc::new(Mutex::new(vec![AudioTarget {
            id: "single".into(),
            primary: true,
            handle: window,
            source_root: root.clone(),
        }]));
        let target_views = targets.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let audio_session = format!("{}-{}", std::process::id(), crate::journal::now_ms());
        let handle = thread::spawn(move || {
            let gain = Arc::new(AtomicU32::new(0f32.to_bits()));
            let source = Arc::new(AtomicU32::new(0));
            let error = Arc::new(AtomicBool::new(false));
            let mut current = None;
            let mut opened_output = String::new();
            let mut quiet_since = Instant::now();
            let mut retry = Instant::now() - Duration::from_secs(3);
            let mut reported = Instant::now() - Duration::from_secs(2);
            loop {
                if signal.load(Ordering::Relaxed) {
                    gain.store(0f32.to_bits(), Ordering::Relaxed);
                    thread::sleep(Duration::from_millis(150));
                    drop(current.take());
                    break;
                }
                let config = read(&root);
                let (desktop, unlocked) = windows::audio_context();
                let routing = crate::audio_routing::read(&root);
                let host_policy = fs::metadata(root.join("host-status.json"))
                    .ok()
                    .filter(|m| m.len() <= 131072)
                    .and_then(|_| fs::read(root.join("host-status.json")).ok())
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                    .unwrap_or(Value::Null);
                let candidates = target_views
                    .lock()
                    .map(|ts| {
                        ts.iter()
                            .map(|t| crate::audio_routing::Candidate {
                                id: t.id.clone(),
                                primary: t.primary,
                                visible: windows::record(t.handle).visible
                                    && policy_allows_target(
                                        &host_policy,
                                        &t.id,
                                        crate::journal::now_ms() as u64,
                                    ),
                                kind: config
                                    .as_ref()
                                    .map(|s| match s.audio.source.as_str() {
                                        "rain" => 1,
                                        "thunder" => 2,
                                        _ => effect(&t.source_root),
                                    })
                                    .unwrap_or(0),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selected = routing
                    .as_ref()
                    .map(|r| crate::audio_routing::select(&r.routing, &candidates))
                    .unwrap_or(crate::audio_routing::Selection {
                        owners: Vec::new(),
                        source_kind: 0,
                    });
                let attached = !selected.owners.is_empty();
                let test_active = fs::read(root.join("audio-test.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                    .is_some_and(|v| {
                        test_marker_active(
                            &v,
                            config.as_ref().ok().map(|s| s.revision),
                            &audio_session,
                            crate::journal::now_ms() as u64,
                        )
                    });
                let mut kind = selected.source_kind;
                if test_active {
                    kind = 4;
                }
                let mut state = config
                    .as_ref()
                    .map(|s| reason(&s.audio, attached, desktop || test_active, unlocked, kind))
                    .unwrap_or("configuration-error");
                if routing.is_err() {
                    state = "routing-configuration-error";
                } else if !attached
                    && config
                        .as_ref()
                        .is_ok_and(|s| s.audio.enabled && !s.audio.muted && s.audio.volume > 0)
                    && unlocked
                {
                    state = "logical-target-unavailable";
                }
                if crate::screensaver::active() {
                    state = "screensaver-active";
                }
                let allowed = state == "playing";
                source.store(kind, Ordering::Relaxed);
                if allowed {
                    quiet_since = Instant::now();
                    if current.is_some() && opened_output != config.as_ref().unwrap().audio.output {
                        gain.store(0f32.to_bits(), Ordering::Relaxed);
                        thread::sleep(Duration::from_millis(150));
                        drop(current.take());
                        retry = Instant::now() - Duration::from_secs(2);
                    }
                    if error.swap(false, Ordering::Relaxed) {
                        drop(current.take());
                        retry = Instant::now();
                    }
                    if current.is_none() && retry.elapsed() > Duration::from_secs(1) {
                        match open(
                            &config.as_ref().unwrap().audio.output,
                            gain.clone(),
                            source.clone(),
                            error.clone(),
                        ) {
                            Ok(s) => {
                                opened_output = config.as_ref().unwrap().audio.output.clone();
                                current = Some(s)
                            }
                            Err(_) => state = "device-error",
                        }
                        retry = Instant::now();
                    }
                    if current.is_none() {
                        state = "device-error";
                        gain.store(0f32.to_bits(), Ordering::Relaxed)
                    } else {
                        let v = config.as_ref().unwrap().audio.volume as f32 / 100.;
                        gain.store(
                            (if test_active { v.min(0.3) } else { v * v }).to_bits(),
                            Ordering::Relaxed,
                        )
                    }
                } else {
                    gain.store(0f32.to_bits(), Ordering::Relaxed);
                    if quiet_since.elapsed() > Duration::from_millis(200) {
                        drop(current.take());
                    }
                }
                if reported.elapsed() > Duration::from_millis(900) {
                    let _ = host_settings::atomic(
                        &root,
                        "audio-status.json",
                        &json!({"schemaVersion":1,"pid":std::process::id(),"audioSession":audio_session,"capturedAtMs":crate::journal::now_ms(),"settingsRevision":config.as_ref().ok().map(|s|s.revision),"state":state,"streamOpen":current.is_some(),"desktopFocused":desktop,"sessionUnlocked":unlocked,"nativeWindowVisible":attached,"backend":"WASAPI via CPAL","output":config.as_ref().ok().map(|s|&s.audio.output),"activeOutput":if current.is_some(){Some(&opened_output)}else{None},"testActive":test_active,"sourceKind":kind,"routingRevision":routing.as_ref().ok().map(|r|r.revision),"routingMode":routing.as_ref().ok().map(|r|&r.routing.mode),"owners":selected.owners,"mixPolicy":"one normalized bus; identical source types deduplicated","trust":"stream/policy state; audible presentation unverified"}),
                    );
                    reported = Instant::now();
                }
                thread::sleep(Duration::from_millis(100));
            }
            let _ = host_settings::atomic(
                &root,
                "audio-status.json",
                &json!({"schemaVersion":1,"pid":std::process::id(),"capturedAtMs":crate::journal::now_ms(),"state":"stopped","streamOpen":false}),
            );
        });
        Self {
            targets,
            stop,
            thread: Some(handle),
        }
    }
    pub fn set_targets(&self, targets: Vec<AudioTarget>) {
        if let Ok(mut current) = self.targets.lock() {
            *current = targets;
        }
    }
}
impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let start = Instant::now();
            while !t.is_finished() && start.elapsed() < Duration::from_secs(1) {
                thread::sleep(Duration::from_millis(10))
            }
            if t.is_finished() {
                let _ = t.join();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_pause_stale_or_other_pid_policy_cannot_enable_audio() {
        let mut status = json!({"pid":std::process::id(),"capturedAtMs":1000,"targets":[{"targetId":"test","nativeAttached":true,"runtimePolicy":{"paused":false}}]});
        assert!(policy_allows_target(&status, "test", 1200));
        assert!(!policy_allows_target(&status, "other", 1200));
        assert!(!policy_allows_target(&status, "test", 5000));
        status["targets"][0]["runtimePolicy"]["paused"] = json!(true);
        assert!(!policy_allows_target(&status, "test", 1200));
        status["targets"][0]["runtimePolicy"]["paused"] = json!(false);
        status["pid"] = json!(0);
        assert!(!policy_allows_target(&status, "test", 1200));
    }
    #[test]
    fn test_tone_is_bound_to_one_host_session_revision_and_expiry() {
        let value = json!({"hostPid":std::process::id(),"audioSession":"a","settingsRevision":3,"expiresAtMs":2000});
        assert!(test_marker_active(&value, Some(3), "a", 1000));
        assert!(!test_marker_active(&value, Some(3), "b", 1000));
        assert!(!test_marker_active(&value, Some(4), "a", 1000));
        assert!(!test_marker_active(&value, Some(3), "a", 2000));
        let mut wrong = value.clone();
        wrong["hostPid"] = json!(0);
        assert!(!test_marker_active(&wrong, Some(3), "a", 1000));
    }
    #[test]
    fn multiple_source_mix_is_normalized_bounded_and_fades() {
        for mask in [8 | 16, 8 | 32, 16 | 32, 8 | 16 | 32] {
            let mut s = Synth::new(48000.);
            let mut energy = 0.;
            for _ in 0..48000 {
                let value = s.sample(1., mask);
                assert!(value.is_finite() && value.abs() <= 0.6001);
                energy += value * value;
            }
            assert!(energy > 0.1);
            for _ in 0..48000 {
                s.sample(0., mask);
            }
            assert_eq!(s.sample(0., mask), 0.);
        }
    }
    #[test]
    fn defaults_are_silent_and_strict() {
        let s = Settings::default();
        assert!(!s.audio.enabled && s.audio.muted);
        assert!(validate(&s).is_ok());
        let mut bad = s.clone();
        bad.audio.volume = 101;
        assert!(validate(&bad).is_err());
        bad = s;
        bad.audio.output = "arbitrary-device".into();
        assert!(validate(&bad).is_err());
    }
    #[test]
    fn policy_never_confuses_desktop_and_manager_or_lock() {
        let mut p = Settings::default().audio;
        p.enabled = true;
        p.muted = false;
        assert_eq!(reason(&p, true, false, true, 1), "playing");
        p.focus_only = true;
        assert_eq!(reason(&p, true, false, true, 1), "desktop-not-focused");
        assert_eq!(
            reason(&p, true, true, false, 1),
            "session-locked-or-unknown"
        );
        assert_eq!(
            reason(&p, false, true, true, 1),
            "wallpaper-cleared-or-unavailable"
        );
        assert_eq!(reason(&p, true, true, true, 0), "source-silent");
        p.volume = 0;
        assert_eq!(reason(&p, true, true, true, 2), "volume-zero");
    }
    #[test]
    fn native_audio_cas_and_receipts_are_independent_and_strict() {
        let root = std::env::temp_dir().join(format!("atmos-audio-cas-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let u = || Update {
            schema_version: 1,
            request_id: "audio.cas.1".into(),
            expected_revision: 0,
            audio: Settings::default().audio,
        };
        let r = update(&root, u()).unwrap();
        assert_eq!(r["revision"], 1);
        assert_eq!(update(&root, u()).unwrap()["replayed"], true);
        let mut conflict = u();
        conflict.audio.volume = 5;
        assert_eq!(
            update(&root, conflict).unwrap_err(),
            "AUDIO_REQUEST_CONFLICT"
        );
        let mut stale = u();
        stale.request_id = "audio.cas.2".into();
        assert_eq!(update(&root, stale).unwrap_err(), "AUDIO_REVISION_CONFLICT");
        assert_eq!(read(&root).unwrap().audio, Settings::default().audio);
        for name in ["audio-settings.json", "audio-settings.previous.json"] {
            std::fs::remove_file(root.join(name)).unwrap();
        }
        let requests = root.join("audio-requests");
        for e in std::fs::read_dir(&requests).unwrap() {
            std::fs::remove_file(e.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(requests).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn synthesis_is_bounded_continuous_and_fades_to_digital_silence() {
        let mut s = Synth::new(48000.);
        let mut energy = 0.;
        for _ in 0..48000 {
            let x = s.sample(1., 2);
            assert!(x.is_finite() && x.abs() <= 0.6001);
            energy += x * x;
        }
        assert!(energy > 1.);
        for _ in 0..48000 {
            s.sample(0., 2);
        }
        assert_eq!(s.sample(0., 2), 0.);
    }
}
