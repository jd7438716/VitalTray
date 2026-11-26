//! CPU 占用：`GetSystemTimes` 两次采样的差值（含 idle，kernel 已包含 idle）。

use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Threading::GetSystemTimes;

#[derive(Default)]
pub struct CpuUsage {
    prev: Option<(u64, u64, u64)>,
}

impl CpuUsage {
    /// 返回 0–100；第一次采样没有基准，返回 None。
    pub fn sample(&mut self) -> Option<u8> {
        let now = read()?;
        let prev = self.prev.replace(now)?;
        percent(prev, now)
    }
}

fn read() -> Option<(u64, u64, u64)> {
    let mut idle: FILETIME = unsafe { std::mem::zeroed() };
    let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
    let mut user: FILETIME = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }
    Some((ticks(idle), ticks(kernel), ticks(user)))
}

fn ticks(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

/// (idle, kernel, user) 前后差值 → 占用百分比。
pub fn percent(prev: (u64, u64, u64), now: (u64, u64, u64)) -> Option<u8> {
    let idle = now.0.saturating_sub(prev.0);
    let kernel = now.1.saturating_sub(prev.1);
    let user = now.2.saturating_sub(prev.2);
    let total = kernel + user;
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle.min(total));
    Some(((busy * 100 + total / 2) / total).min(100) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_busy() {
        // 1000 个时间片，500 个 idle → 50%
        assert_eq!(percent((0, 0, 0), (500, 600, 400)), Some(50));
        assert_eq!(percent((0, 0, 0), (0, 0, 0)), None);
    }

    #[test]
    fn busy_is_clamped() {
        // 保护异常样本：busy 不会超过 100%
        assert_eq!(percent((0, 0, 0), (0, 100, 100)), Some(100));
    }
}
