//! Read-only CPU / physical-memory / interface-counter sampler. No sensors or packet capture.
use serde_json::json;
use std::collections::BTreeMap;
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
#[derive(Clone)]
struct NetworkSnapshot {
    scope: &'static str,
    counters: BTreeMap<u64, (u64, u64)>,
}
fn network_snapshot() -> Option<NetworkSnapshot> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
    };
    struct Table(*mut MIB_IF_TABLE2);
    impl Drop for Table {
        fn drop(&mut self) {
            unsafe { FreeMibTable(self.0.cast()) }
        }
    }
    let mut raw = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut raw) } != 0 || raw.is_null() {
        return None;
    }
    let table = Table(raw);
    let count = unsafe { (*table.0).NumEntries } as usize;
    if count == 0 || count > 4096 {
        return None;
    }
    // The SDK binding preserves the alignment padding before Table and between rows.
    let rows =
        unsafe { std::slice::from_raw_parts(std::ptr::addr_of!((*table.0).Table).cast(), count) };
    let mut selected = Vec::new();
    for row in rows {
        let row: &windows_sys::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2 = row;
        let flags = row.InterfaceAndOperStatusFlags._bitfield;
        if row.OperStatus != 1 || matches!(row.Type, 24 | 131) || flags & 2 != 0 {
            continue;
        }
        selected.push((
            unsafe { row.InterfaceLuid.Value },
            row.InOctets,
            row.OutOctets,
            flags & 1 != 0,
        ));
    }
    let physical = selected.iter().any(|row| row.3);
    let counters = selected
        .into_iter()
        .filter(|row| !physical || row.3)
        .map(|row| (row.0, (row.1, row.2)))
        .collect();
    Some(NetworkSnapshot {
        scope: if physical {
            "active-physical-interfaces"
        } else {
            "active-nonloopback-interfaces"
        },
        counters,
    })
}
fn network_rate(
    a: &NetworkSnapshot,
    b: &NetworkSnapshot,
    seconds: f64,
) -> Option<serde_json::Value> {
    if !seconds.is_finite()
        || seconds <= 0.0
        || a.scope != b.scope
        || a.counters.is_empty()
        || a.counters.keys().ne(b.counters.keys())
    {
        return None;
    }
    let (mut received, mut sent) = (0u64, 0u64);
    for (id, (rx, tx)) in &b.counters {
        let before = a.counters.get(id)?;
        received = received.checked_add(rx.checked_sub(before.0)?)?;
        sent = sent.checked_add(tx.checked_sub(before.1)?)?;
    }
    Some(
        json!({"available":true,"scope":b.scope,"adapterCount":b.counters.len(),"receiveBytesPerSecond":received as f64/seconds,"sendBytesPerSecond":sent as f64/seconds,"sampleWindowMs":seconds*1000.0}),
    )
}
pub(crate) fn sample() -> Result<serde_json::Value, String> {
    let start = Instant::now();
    let first = times()?;
    let network_first = network_snapshot();
    let network_start = Instant::now();
    thread::sleep(Duration::from_millis(500));
    let last = times()?;
    let network_last = network_snapshot();
    let network = network_first
        .as_ref()
        .zip(network_last.as_ref())
        .and_then(|(a, b)| network_rate(a, b, network_start.elapsed().as_secs_f64()));
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
    Ok(
        json!({"schemaVersion":1,"available":true,"source":"windows-native-readonly","sampledAtMs":crate::journal::now_ms(),"sampleWindowMs":start.elapsed().as_millis(),"cpuPercent":cpu(first,last),"cpuScope":"primary-processor-group (all processors on systems with at most 64)","memory":{"totalBytes":memory.total_physical,"usedBytes":memory.total_physical-memory.available_physical,"availableBytes":memory.available_physical},"network":network,"gpu":null,"power":null}),
    )
}
pub fn command() -> Result<i32, String> {
    println!("{}", sample()?);
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
    #[test]
    fn network_uses_64_bit_deltas_and_rejects_resets_or_adapter_changes() {
        let a = NetworkSnapshot {
            scope: "test",
            counters: BTreeMap::from([(7, (5_000_000_000, 9_000_000_000))]),
        };
        let mut b = a.clone();
        b.counters.insert(7, (5_000_001_000, 9_000_000_250));
        let rate = network_rate(&a, &b, 0.5).unwrap();
        assert_eq!(rate["receiveBytesPerSecond"], 2000.0);
        assert_eq!(rate["sendBytesPerSecond"], 500.0);
        assert!(network_rate(&a, &b, 0.0).is_none());
        b.counters.insert(7, (1, 1));
        assert!(network_rate(&a, &b, 1.0).is_none());
        b.counters.insert(8, (1, 1));
        assert!(network_rate(&a, &b, 1.0).is_none());
        assert_eq!(
            network_rate(&a, &a, 1.0).unwrap()["receiveBytesPerSecond"],
            0.0
        );
    }
}
