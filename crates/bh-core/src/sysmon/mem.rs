//! 物理内存：`GlobalMemoryStatusEx`。

use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemInfo {
    pub percent: u8,
    pub used_mb: u64,
    pub total_mb: u64,
}

pub fn read() -> Option<MemInfo> {
    let mut st = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    if unsafe { GlobalMemoryStatusEx(&mut st) } == 0 || st.ullTotalPhys == 0 {
        return None;
    }
    let total_mb = st.ullTotalPhys / (1024 * 1024);
    let used_mb = (st.ullTotalPhys - st.ullAvailPhys) / (1024 * 1024);
    Some(MemInfo { percent: st.dwMemoryLoad.min(100) as u8, used_mb, total_mb })
}
