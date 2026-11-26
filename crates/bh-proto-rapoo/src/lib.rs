//! Rapoo（雷柏）VT 系列鼠标电量：被动读取厂商集合 FF00:0002 的状态帧。
//!
//! 两种已知布局（来自公开逆向资料，见 docs/protocol-notes/rapoo.md）：
//! * Nordic 54L（VT3 Max、VT9、VT3 Pro 等）：report id 0x07
//!   byte7 = 状态（0 无效 / 1 放电 / 2 充电），byte8 = 电量百分比（0–100）
//! * 旧版 7 字节：BB B0 [mode] [dpi lo] [dpi hi] [状态] [百分比]
//!
//! 只读：从不发任何请求。设备空闲/睡眠时不推帧，调用方需要给足等待时间。

use bh_api::{DriverError, HidIo, Reading};

pub const VENDOR_RAPOO: u16 = 0x24AE;
pub const USAGE_PAGE_VENDOR: u16 = 0xFF00;
pub const USAGE_BATTERY: u16 = 0x0002;

/// 解析一帧；不是电量帧就返回 None。
pub fn parse(frame: &[u8]) -> Option<Reading> {
    if frame.is_empty() {
        return None;
    }
    match frame[0] {
        0x07 => {
            if frame.len() < 9 {
                return None;
            }
            let conn = frame[1] & 0x0F;
            if conn > 2 {
                return None;
            }
            let state = frame[7];
            let percent = frame[8];
            if !(1..=2).contains(&state) || percent > 100 {
                return None;
            }
            Some(Reading {
                soc: percent,
                voltage_mv: None,
                charging: Some(state == 2),
                at_ms: bh_api::now_ms(),
            })
        }
        0xBB => {
            // 旧版 7 字节：BB B0 mode dpiLo dpiHi state percent
            if frame.len() < 7 || frame[1] != 0xB0 {
                return None;
            }
            let state = frame[5];
            let percent = frame[6];
            if percent == 0x7F || percent > 100 {
                return None;
            }
            if !(1..=2).contains(&state) {
                return None;
            }
            Some(Reading {
                soc: percent,
                voltage_mv: None,
                charging: Some(state == 2),
                at_ms: bh_api::now_ms(),
            })
        }
        _ => None,
    }
}

/// 在当前句柄上等待一帧电量（被动，不发请求）。
pub fn read_battery(io: &mut dyn HidIo, wait_ms: u32) -> Result<Reading, DriverError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(wait_ms as u64);
    let mut buf = [0u8; 64];
    while std::time::Instant::now() < deadline {
        match io.read_timeout(&mut buf, 200) {
            Ok(n) if n > 0 => {
                if let Some(r) = parse(&buf[..n]) {
                    return Ok(r);
                }
            }
            Ok(_) => {}
            Err(DriverError::Io(_)) => return Err(DriverError::Io("read failed".into())),
            Err(_) => {}
        }
    }
    Err(DriverError::Timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn vt3m_wireless_25_percent() {
        let r = parse(&hex("07 20 03 20 03 20 03 01 19 00 04 84 00 17 46 00 02 03 BC")).unwrap();
        assert_eq!(r.soc, 25);
        assert_eq!(r.charging, Some(false));
    }

    #[test]
    fn vt3m_cable_charging() {
        let r = parse(&hex("07 22 03 20 03 20 03 02 19 00 04 84 00 17 46 00 02 03 00")).unwrap();
        assert_eq!(r.soc, 25);
        assert_eq!(r.charging, Some(true));
    }

    #[test]
    fn vt9_receiver_100_percent() {
        let r = parse(&hex("07 10 03 40 06 40 06 01 64 00 03 01 00 14 46 01")).unwrap();
        assert_eq!(r.soc, 100);
    }

    #[test]
    fn legacy_bb_100_percent() {
        let r = parse(&hex("BB B0 71 84 03 01 64")).unwrap();
        assert_eq!(r.soc, 100);
        assert_eq!(r.charging, Some(false));
    }

    #[test]
    fn legacy_bb_charging_95() {
        let r = parse(&hex("BB B0 71 84 03 02 5F")).unwrap();
        assert_eq!(r.soc, 95);
        assert_eq!(r.charging, Some(true));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(&hex("BC 00 00 00 00 00 00")).is_none());
        assert!(parse(&hex("07 20 03 20 03 20 03 00 19")).is_none()); // state 0 无效
        assert!(parse(&hex("07 20 03 20 03 20 03 01 65")).is_none()); // 101%
        assert!(parse(&hex("BB B0 71 84 03 01 7F")).is_none()); // 未知电量
    }
}
