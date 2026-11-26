//! 跟随系统明暗主题（任务栏背景）。

use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// true = 深色任务栏（用浅色前景）。
pub fn is_dark() -> bool {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut key)
            != windows_sys::Win32::Foundation::ERROR_SUCCESS
        {
            return true;
        }
        let name = wide("AppsUseLightTheme");
        let mut value: u32 = 1;
        let mut len = 4u32;
        let mut kind = 0u32;
        let rc = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            &mut value as *mut u32 as *mut u8,
            &mut len,
        );
        RegCloseKey(key);
        rc != windows_sys::Win32::Foundation::ERROR_SUCCESS || value == 0
    }
}
