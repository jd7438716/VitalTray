//! 系统盘：空间用 `GetDiskFreeSpaceExW`，温度走 NVMe SMART（见 `smart.rs`）。
//!
//! 两样都缓存：空间 30 秒、温度 20 秒（温度在 `smart.rs` 里缓存）。
//! 面板每 2 秒采样一次，但“盘还剩多少”这种东西没必要每 2 秒问一次卷——
//! 单次 `GetDiskFreeSpaceExW` 在 4 TB 卷上实测约 55 µs，是温度那几项里最贵的一笔。

use super::smart;
use std::sync::Mutex;
use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW;

const SPACE_TTL_MS: i64 = 30_000;

static SPACE: Mutex<Option<(DiskInfo, i64)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskInfo {
    pub letter: char,
    pub percent: u8,
    pub free_mb: u64,
    pub total_mb: u64,
    /// 盘温（°C）；非 NVMe 或驱动不提供时为 `None`。
    pub temp_c: Option<u8>,
}

pub fn system_disk() -> Option<DiskInfo> {
    let now = now_ms();
    if let Ok(cache) = SPACE.lock() {
        if let Some((info, at)) = *cache {
            if now.saturating_sub(at) < SPACE_TTL_MS {
                return Some(info);
            }
        }
    }
    let mut info = read_space()?;
    info.temp_c = smart::volume_temp(info.letter);
    if let Ok(mut cache) = SPACE.lock() {
        *cache = Some((info, now));
    }
    Some(info)
}

/// 只读空间，温度留空（温度单独缓存）。
fn read_space() -> Option<DiskInfo> {
    let letter = windows_drive()?;
    let path = [letter as u16, b':' as u16, b'\\' as u16, 0];
    let mut free = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut free, &mut total, &mut total_free) };
    if ok == 0 || total == 0 {
        return None;
    }
    let used = total.saturating_sub(total_free);
    let percent = ((used as u128 * 100 + total as u128 / 2) / total as u128) as u8;
    Some(DiskInfo {
        letter,
        percent: percent.min(100),
        free_mb: total_free / (1024 * 1024),
        total_mb: total / (1024 * 1024),
        temp_c: None,
    })
}

fn windows_drive() -> Option<char> {
    let mut buf = [0u16; 260];
    let len = unsafe { GetWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        return None;
    }
    let c = char::from_u32(buf[0] as u32)?.to_ascii_uppercase();
    if c.is_ascii_alphabetic() { Some(c) } else { None }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
