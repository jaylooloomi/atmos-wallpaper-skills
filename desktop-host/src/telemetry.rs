//! Read-only CPU / physical-memory sampler. No process enumeration or sensors.
use serde_json::json;
use std::{
    mem::size_of,
    thread,
    time::{Duration, Instant},
};
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}
#[repr(C)]
#[derive(Default)]
struct Memory {
    length: u32,
    load: u32,
    total_physical: u64,
    available_physical: u64,
    total_page: u64,
    available_page: u64,
    total_virtual: u64,
    available_virtual: u64,
    extended: u64,
}
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    fn GlobalMemoryStatusEx(memory: *mut Memory) -> i32;
}
fn ticks(f: FileTime) -> u64 {
    ((f.high as u64) << 32) | f.low as u64
}
fn times() -> Result<(u64, u64), String> {
    let (mut idle, mut kernel, mut user) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return Err("SYSTEM_SAMPLE_UNAVAILABLE".into());
    }
    Ok((
        ticks(idle),
        ticks(kernel)
            .checked_add(ticks(user))
            .ok_or("SYSTEM_SAMPLE_INVALID")?,
    ))
}
fn cpu(a: (u64, u64), b: (u64, u64)) -> Option<f64> {
    let total = b.1.checked_sub(a.1)?;
    let idle = b.0.checked_sub(a.0)?;
    if total == 0 || idle > total {
        return None;
    }
    Some((total - idle) as f64 * 100.0 / total as f64)
}
pub fn command() -> Result<i32, String> {
    let start = Instant::now();
    let first = times()?;
    thread::sleep(Duration::from_millis(500));
    let last = times()?;
    let mut memory = Memory {
        length: size_of::<Memory>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut memory) } == 0
        || memory.total_physical == 0
        || memory.available_physical > memory.total_physical
    {
        return Err("SYSTEM_SAMPLE_UNAVAILABLE".into());
    }
    println!(
        "{}",
        json!({"schemaVersion":1,"available":true,"source":"windows-native-readonly","sampledAtMs":crate::journal::now_ms(),"sampleWindowMs":start.elapsed().as_millis(),"cpuPercent":cpu(first,last),"cpuScope":"primary-processor-group (all processors on systems with at most 64)","memory":{"totalBytes":memory.total_physical,"usedBytes":memory.total_physical-memory.available_physical,"availableBytes":memory.available_physical},"network":null,"gpu":null,"power":null})
    );
    Ok(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_idle_in_total_and_rejects_resets() {
        assert_eq!(cpu((10, 20), (20, 60)), Some(75.0));
        assert_eq!(cpu((10, 20), (10, 20)), None);
        assert_eq!(cpu((10, 20), (9, 60)), None);
        assert_eq!(cpu((10, 20), (90, 60)), None)
    }
    #[test]
    fn filetime_high_bits() {
        assert_eq!(ticks(FileTime { low: 7, high: 1 }), 4_294_967_303)
    }
}
