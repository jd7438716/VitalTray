//! 机器品牌 / 型号 / CPU 型号：只读注册表，不需要管理员，也不依赖 WMI。
//!
//! 数据源：`HKLM\HARDWARE\DESCRIPTION\System\BIOS` 与 `...\CentralProcessor\0`，
//! 由固件在开机时写入，任何用户都能读。OEM 厂商字段经常是 "System manufacturer"
//! 之类的占位串，这里统一清洗掉。

use std::sync::OnceLock;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE,
};

const BIOS_KEY: &str = "HARDWARE\\DESCRIPTION\\System\\BIOS";
const CPU_KEY: &str = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// 品牌，如 "ASUS"；读不到时为空。
    pub vendor: String,
    /// 型号，如 "Zenbook 14 UX3405MA"；读不到时为空。
    pub model: String,
    /// CPU 名称，如 "Intel Core Ultra 9 185H"。
    pub cpu: String,
}

static INFO: OnceLock<Machine> = OnceLock::new();

pub fn info() -> &'static Machine {
    INFO.get_or_init(|| {
        let vendor = clean_vendor(&read(BIOS_KEY, "SystemManufacturer"));
        let raw_model = read(BIOS_KEY, "SystemProductName");
        let model = if raw_model.is_empty() || is_placeholder(&raw_model) {
            let board = read(BIOS_KEY, "BaseBoardProduct");
            if is_placeholder(&board) { String::new() } else { clean_model(&vendor, &board) }
        } else {
            clean_model(&vendor, &raw_model)
        };
        let cpu = {
            let name = read(CPU_KEY, "ProcessorNameString");
            if is_placeholder(&name) { String::new() } else { crate::util::tidy_name(&name) }
        };
        Machine { vendor, model, cpu }
    })
}

/// 占位串（OEM 没填字段时固件留下的模板文字）。
pub fn is_placeholder(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    ["to be filled", "system manufacturer", "system product name", "default string", "o.e.m", "not specified", "unknown"]
        .iter()
        .any(|p| lower.contains(p))
}

/// 把 OEM 的厂商全名映射成用户认识的品牌短名。
pub fn clean_vendor(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() || is_placeholder(s) {
        return String::new();
    }
    let u = s.to_ascii_uppercase();
    let known = [
        ("ASUSTEK", "ASUS"),
        ("ASUS", "ASUS"),
        ("MICRO-STAR", "MSI"),
        ("MICRO STAR", "MSI"),
        ("MSI", "MSI"),
        ("LENOVO", "Lenovo"),
        ("DELL", "Dell"),
        ("HEWLETT", "HP"),
        ("HP", "HP"),
        ("ACER", "Acer"),
        ("SAMSUNG", "Samsung"),
        ("HUAWEI", "Huawei"),
        ("XIAOMI", "Xiaomi"),
        ("GIGABYTE", "GIGABYTE"),
        ("ASROCK", "ASRock"),
        ("RAZER", "Razer"),
        ("MECHREVO", "MECHREVO"),
        ("HASEE", "HASEE"),
        ("TONG FANG", "TongFang"),
        ("THUNDEROBOT", "Thunderobot"),
        ("APPLE", "Apple"),
        ("SONY", "Sony"),
        ("TOSHIBA", "Toshiba"),
        ("FUJITSU", "Fujitsu"),
        ("CLEVO", "Clevo"),
    ];
    for (needle, short) in known {
        if u.contains(needle) {
            return short.to_string();
        }
    }
    if s.len() <= 16 && s.chars().all(|c| !c.is_ascii_lowercase()) {
        // 全大写短名（如 "MECHREVO"）转成首字母大写，避免标题栏全是吼叫体。
        let mut out = String::with_capacity(s.len());
        for word in s.split_whitespace() {
            if !out.is_empty() {
                out.push(' ');
            }
            let mut cs = word.chars();
            if let Some(first) = cs.next() {
                out.extend(first.to_uppercase());
                out.push_str(&cs.as_str().to_ascii_lowercase());
            }
        }
        return out;
    }
    s.to_string()
}

/// 去掉型号字符串里的 OEM 冗余：重复后缀（"..._UX3405MA_UX3405MA"）与重复品牌前缀。
pub fn clean_model(vendor: &str, raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.is_empty() || is_placeholder(&s) {
        return String::new();
    }
    if let Some((head, tail)) = s.rsplit_once('_') {
        if !tail.is_empty() && head.trim_end().ends_with(tail) {
            s = head.trim_end_matches([' ', '_']).to_string();
        }
    }
    if !vendor.is_empty() {
        if let Some(rest) = s.strip_prefix(vendor) {
            s = rest.trim_start_matches([' ', '-', '_']).to_string();
        }
    }
    s
}

fn read(key: &str, value: &str) -> String {
    let sub = wide(key);
    let name = wide(value);
    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey)
            != windows_sys::Win32::Foundation::ERROR_SUCCESS
        {
            return String::new();
        }
        let mut kind = 0u32;
        let mut len = 0u32;
        let rc = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut len,
        );
        if rc != windows_sys::Win32::Foundation::ERROR_SUCCESS || len == 0 || len > 4096 {
            RegCloseKey(hkey);
            return String::new();
        }
        let mut buf = vec![0u8; len as usize];
        let rc = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buf.as_mut_ptr(),
            &mut len,
        );
        RegCloseKey(hkey);
        if rc != windows_sys::Win32::Foundation::ERROR_SUCCESS {
            return String::new();
        }
        // REG_SZ / REG_EXPAND_SZ 都是 UTF-16；其他类型不处理
        if kind != 1 && kind != 2 {
            return String::new();
        }
        let units: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16_lossy(&units).trim().to_string()
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_aliases() {
        assert_eq!(clean_vendor("ASUSTeK COMPUTER INC."), "ASUS");
        assert_eq!(clean_vendor("LENOVO"), "Lenovo");
        assert_eq!(clean_vendor("Micro-Star International Co., Ltd."), "MSI");
        assert_eq!(clean_vendor("System manufacturer"), "");
        assert_eq!(clean_vendor(""), "");
    }

    #[test]
    fn model_cleanup() {
        assert_eq!(clean_model("ASUS", "ASUS Zenbook 14 UX3405MA_UX3405MA"), "Zenbook 14 UX3405MA");
        assert_eq!(clean_model("LENOVO", "82WM"), "82WM");
        assert_eq!(clean_model("Dell", "XPS 15 9520"), "XPS 15 9520");
        assert_eq!(clean_model("", "Default string"), "");
    }

    #[test]
    fn placeholders() {
        assert!(is_placeholder("To Be Filled By O.E.M."));
        assert!(is_placeholder("System Product Name"));
        assert!(!is_placeholder("Zenbook 14"));
    }
}
