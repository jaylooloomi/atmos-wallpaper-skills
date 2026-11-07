//! Read-only capacity of the two explicitly supported local fixed volumes.
use serde_json::{json, Value};
use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};
fn capacity(drive: &str, total: u64, free: u64) -> Value {
    if total == 0 || free > total || total > 9_007_199_254_740_991 { return json!({"drive":drive,"available":false}); }
    json!({"drive":drive,"available":true,"totalBytes":total,"freeBytes":free,"usedBytes":total-free})
}
fn read(drive: &str) -> Value {
    let root: Vec<u16> = format!("{}\\",drive).encode_utf16().chain(Some(0)).collect();
    // Do not touch mapped/network drives, optical media or missing/removable roots.
    if unsafe { GetDriveTypeW(root.as_ptr()) } != 3 { return json!({"drive":drive,"available":false}); }
    let (mut free, mut total, mut volume_free) = (0u64,0u64,0u64);
    if unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut free, &mut total, &mut volume_free) } == 0 { return json!({"drive":drive,"available":false}); }
    capacity(drive,total,free)
}
pub fn command() -> Result<i32,String> {
    println!("{}",json!({"schemaVersion":1,"source":"windows-native-readonly","sampledAtMs":crate::journal::now_ms(),"disks":[read("C:"),read("D:")]}));Ok(0)
}
#[cfg(test)] mod tests { use super::*;
 #[test] fn validates_64_bit_capacity_without_fake_unknown_values(){let d=capacity("C:",6_000_000_000,1_000_000_000);assert_eq!(d["usedBytes"],5_000_000_000u64);for (total,free) in [(0,0),(10,11),(u64::MAX,0)] {let v=capacity("D:",total,free);assert_eq!(v["available"],false);assert!(v.get("freeBytes").is_none());}}
}
