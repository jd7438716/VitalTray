//! 开机自启：只写 HKCU\...\Run，绝不碰 HKLM，也不需要管理员。

use std::path::Path;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "BatteryHub";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 读取当前 Run 值（None = 未设置）。
pub fn current() -> Option<String> {
    unsafe {
        let sub = wide(RUN_KEY);
        let name = wide(VALUE_NAME);
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        let mut kind = 0u32;
        let rc = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buf.as_mut_ptr(),
            &mut len,
        );
        RegCloseKey(key);
        if rc != ERROR_SUCCESS || len < 2 {
            return None;
        }
        buf.truncate(len as usize);
        let u16s: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = String::from_utf16_lossy(&u16s);
        Some(s.trim_end_matches('\0').to_string())
    }
}

pub fn is_enabled() -> bool {
    current().is_some()
}

/// 写入自启项。命令行 = "exe" --minimized。
pub fn enable(exe: &Path) -> bool {
    let cmd = format!("\"{}\" --minimized", exe.display());
    let data = wide(&cmd);
    unsafe {
        let sub = wide(RUN_KEY);
        let name = wide(VALUE_NAME);
        let mut key: HKEY = std::ptr::null_mut();
        let mut disp = 0u32;
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE | KEY_QUERY_VALUE,
            std::ptr::null(),
            &mut key,
            &mut disp,
        ) != ERROR_SUCCESS
        {
            return false;
        }
        let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
        let rc = RegSetValueExW(
            key,
            name.as_ptr(),
            0,
            REG_SZ,
            bytes.as_ptr(),
            bytes.len() as u32,
        );
        RegCloseKey(key);
        rc == ERROR_SUCCESS
    }
}

pub fn disable() -> bool {
    unsafe {
        let sub = wide(RUN_KEY);
        let name = wide(VALUE_NAME);
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_SET_VALUE, &mut key)
            != ERROR_SUCCESS
        {
            return false;
        }
        let rc = RegDeleteValueW(key, name.as_ptr());
        RegCloseKey(key);
        rc == ERROR_SUCCESS
    }
}

/// 按期望状态同步；exe 路径变化时自动修正。
pub fn sync(exe: &Path, wanted: bool) -> bool {
    if !wanted {
        return if is_enabled() { disable() } else { true };
    }
    let expected = format!("\"{}\" --minimized", exe.display());
    if current().as_deref() == Some(expected.as_str()) {
        return true;
    }
    enable(exe)
}
