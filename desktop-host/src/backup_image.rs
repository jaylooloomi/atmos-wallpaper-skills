//! Reuse the bundled Pillow normalizer to verify imported backup assets.
use crate::portable_store;
use serde_json::Value;
use std::{
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(crate) struct Decoder {
    pub python: PathBuf,
    pub script: PathBuf,
}

impl Decoder {
    pub(crate) fn verify(&self, bytes: &[u8], width: u64, height: u64) -> Result<(), String> {
        let (normalized, actual_width, actual_height) = self.decode(bytes, true)?;
        if normalized != bytes || actual_width != width || actual_height != height {
            return Err("IMAGE_REJECTED".into());
        }
        Ok(())
    }

    pub(crate) fn decode(
        &self,
        bytes: &[u8],
        normalized: bool,
    ) -> Result<(Vec<u8>, u64, u64), String> {
        if !self.python.is_absolute() || !self.script.is_absolute() {
            return Err("IMAGE_DECODER_UNAVAILABLE".into());
        }
        portable_store::no_links(&self.python)?;
        portable_store::no_links(&self.script)?;
        let mut command = Command::new(&self.python);
        command.args(["-P", "-B"]).arg(&self.script);
        if normalized {
            command.arg("--normalized");
        }
        let mut child = command
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| "IMAGE_DECODER_UNAVAILABLE")?;
        let _job = match crate::windows::child_job(child.as_raw_handle() as isize) {
            Ok(job) => job,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("IMAGE_DECODER_UNAVAILABLE".into());
            }
        };
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        thread::scope(|scope| {
            let writer = scope.spawn(move || stdin.write_all(bytes));
            let output = scope.spawn(move || {
                let mut output = Vec::new();
                stdout
                    .take(16 * 1048576 + 1)
                    .read_to_end(&mut output)
                    .map(|_| output)
            });
            let metadata = scope.spawn(move || {
                let mut metadata = Vec::new();
                stderr
                    .take(4097)
                    .read_to_end(&mut metadata)
                    .map(|_| metadata)
            });
            let deadline = Instant::now() + Duration::from_secs(15);
            let success = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status.success(),
                    Ok(None) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break false;
                    }
                }
            };
            let written = writer.join().is_ok_and(|r| r.is_ok());
            let normalized = output.join().ok().and_then(Result::ok);
            let metadata: Option<Value> = metadata
                .join()
                .ok()
                .and_then(Result::ok)
                .and_then(|data| serde_json::from_slice(&data).ok());
            if !success || !written {
                return Err("IMAGE_REJECTED".into());
            }
            let output = normalized
                .filter(|b| !b.is_empty() && b.len() <= 16 * 1048576)
                .ok_or("IMAGE_REJECTED")?;
            let info = metadata
                .filter(|v| v["format"] == "PNG")
                .ok_or("IMAGE_REJECTED")?;
            let dimension = |k: &str| {
                info[k]
                    .as_u64()
                    .filter(|n| (1..=4096).contains(n))
                    .ok_or("IMAGE_REJECTED")
            };
            let (width, height) = (dimension("width")?, dimension("height")?);
            if width * height > 16_000_000 {
                return Err("IMAGE_REJECTED".into());
            }
            Ok((output, width, height))
        })
    }
}
