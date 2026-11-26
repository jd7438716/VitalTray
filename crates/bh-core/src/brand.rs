//! HID 厂商号（VID）→ 品牌短名。仅用于展示（面板里的“品牌 · 型号”），
//! 匹配逻辑不依赖它；未知厂商返回 None，由调用方退回产品字符串。

/// 常见外设厂商。表持久稳定，新增品牌只需一行。
pub fn vendor_name(vid: u16) -> Option<&'static str> {
    Some(match vid {
        0x046d => "Logitech",
        0x24ae => "Rapoo",
        0x045e => "Microsoft",
        0x05ac => "Apple",
        0x1532 => "Razer",
        0x1b1c => "Corsair",
        0x1038 => "SteelSeries",
        0x0b05 => "ASUS",
        0x413c => "Dell",
        0x03f0 => "HP",
        0x17ef => "Lenovo",
        0x0951 => "HyperX",
        0x3434 => "Keychron",
        0x09da => "A4Tech",
        0x3151 => "Ajazz",
        0x258a => "SinoWealth",
        0x0c45 => "Sonix",
        0x04d9 => "Holtek",
        0x04f2 => "Chicony",
        0x8087 => "Intel",
        0x0a5c => "Broadcom",
        0x0bda => "Realtek",
        0x1a2c => "Semico",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_unknown() {
        assert_eq!(vendor_name(0x046d), Some("Logitech"));
        assert_eq!(vendor_name(0x24ae), Some("Rapoo"));
        assert_eq!(vendor_name(0xffff), None);
    }
}
