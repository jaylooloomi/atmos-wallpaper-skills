//! Read-only own-session diagnostics. No audio capture or mixer changes.
use ::windows::{
    core::Interface,
    Win32::{
        Media::Audio::{
            Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
            *,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        },
    },
};
use serde_json::{json, Value};
use std::{fs, path::Path};
pub fn read(root: &Path) -> Result<Value, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() {
            return Err("AUDIO_DIAGNOSTICS_UNAVAILABLE".into());
        }
        struct Com;
        impl Drop for Com {
            fn drop(&mut self) {
                unsafe { CoUninitialize() }
            }
        }
        let _com = Com;
        let pid = fs::read(root.join("audio-status.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["pid"].as_u64())
            .ok_or("AUDIO_HOST_UNAVAILABLE")?;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|_| "AUDIO_NO_DEVICE")?;
        let endpoint = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|_| "AUDIO_NO_DEVICE")?;
        let volume: IAudioEndpointVolume = endpoint
            .Activate(CLSCTX_ALL, None)
            .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
        let manager: IAudioSessionManager2 = endpoint
            .Activate(CLSCTX_ALL, None)
            .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
        let sessions = manager
            .GetSessionEnumerator()
            .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
        let mut own = Vec::new();
        for i in 0..sessions
            .GetCount()
            .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?
        {
            let s = sessions
                .GetSession(i)
                .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
            let s2 = s
                .cast::<IAudioSessionControl2>()
                .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
            if s2.GetProcessId().unwrap_or(0) as u64 != pid {
                continue;
            }
            let simple = s
                .cast::<ISimpleAudioVolume>()
                .map_err(|_| "AUDIO_DIAGNOSTICS_UNAVAILABLE")?;
            let meter = s.cast::<IAudioMeterInformation>().ok();
            let mut peak = 0f32;
            for _ in 0..12 {
                if let Some(m) = &meter {
                    peak = peak.max(m.GetPeakValue().unwrap_or(0.))
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            own.push(json!({"sessionMute":simple.GetMute().map(|v|v.as_bool()).ok(),"sessionVolume":simple.GetMasterVolume().ok(),"sessionPeak":peak,"meterAvailable":meter.is_some()}));
        }
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let name = host
            .default_output_device()
            .and_then(|d| d.description().ok().map(|v| v.name().to_owned()));
        Ok(
            json!({"readOnly":true,"capturePerformed":false,"defaultDeviceName":name,"endpointMuted":volume.GetMute().map(|v|v.as_bool()).ok(),"endpointVolume":volume.GetMasterVolumeLevelScalar().ok(),"ownSessions":own}),
        )
    }
}

pub fn devices() -> Vec<Value> {
    use cpal::traits::{DeviceTrait, HostTrait};
    use sha2::{Digest, Sha256};
    let host = cpal::default_host();
    match host.output_devices() {
        Ok(ds) => ds
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                let name = d.description().ok()?.name().to_owned();
                Some(json!({"id":format!("device-{:x}",Sha256::digest(id.as_bytes())),"name":name}))
            })
            .take(64)
            .collect(),
        Err(_) => Vec::new(),
    }
}
pub fn default_name() -> Option<String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.description().ok().map(|v| v.name().to_owned()))
}
