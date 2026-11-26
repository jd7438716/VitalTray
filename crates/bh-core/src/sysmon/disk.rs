//! 系统盘空间：`GetDiskFreeSpaceExW`。

use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskInfo {
    pub letter: char,
    pub percent: u8,
    pub free_mb: u64,
    pub total_mb: u64,
}

pub fn system_disk() -> Option<DiskInfo> {
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
