//! 单实例：命名互斥体。第二个实例只打印提示并退出。

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

pub struct Guard(HANDLE);

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                CloseHandle(self.0);
            }
        }
    }
}

/// 拿到锁返回 Some；已有实例在跑返回 None。
pub fn acquire(name: &str) -> Option<Guard> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let h = CreateMutexW(std::ptr::null(), 0, wide.as_ptr());
        if h.is_null() {
            // 创建失败：宁可不锁，也不要阻止启动
            return Some(Guard(std::ptr::null_mut()));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(h);
            return None;
        }
        Some(Guard(h))
    }
}
