//! 跟随系统明暗主题：面板跟应用主题走，托盘图标跟任务栏主题走。
//!
//! Windows 把两者分开存：`AppsUseLightTheme` 管应用窗口，`SystemUsesLightTheme` 管
//! 任务栏/开始菜单。托盘图标画在任务栏上，所以必须看后者，否则「应用浅色 + 任务栏深色」
//! 的组合下图标会画反（白底图标浮在深色任务栏上）。

use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 读 `Personalize` 下的 0/1 开关；键或值不存在时返回 `None`。
fn personalize_flag(name: &str) -> Option<bool> {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut key)
            != windows_sys::Win32::Foundation::ERROR_SUCCESS
        {
            return None;
        }
        let value_name = wide(name);
        let mut value: u32 = 1;
        let mut len = 4u32;
        let mut kind = 0u32;
        let rc = RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            &mut value as *mut u32 as *mut u8,
            &mut len,
        );
        RegCloseKey(key);
        if rc != windows_sys::Win32::Foundation::ERROR_SUCCESS {
            None
        } else {
            Some(value != 0)
        }
    }
}

/// true = 应用是深色（面板配色用）。读不到时按深色处理。
pub fn is_dark() -> bool {
    personalize_flag("AppsUseLightTheme").map(|light| !light).unwrap_or(true)
}

/// true = 任务栏是深色（托盘图标配色用）。读不到时退回应用主题。
pub fn taskbar_is_dark() -> bool {
    personalize_flag("SystemUsesLightTheme")
        .or_else(|| personalize_flag("AppsUseLightTheme"))
        .map(|light| !light)
        .unwrap_or(true)
}
