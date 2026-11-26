//! Razer（雷蛇）鼠标电量：90 字节 feature report 上的命令类 0x07。
//!
//! 协议要点（公开逆向资料，OpenRazer 系列实现一致）：
//! * 报文固定 90 字节：`[status, txn, pkts_hi, pkts_lo, proto, size, class, cmd, args(80), crc, 0]`
//! * `crc` = 第 2..88 字节的异或；不匹配说明设备没按协议应答，直接判失败。
//! * 命令类 `0x07`（电池）：`0x80` 取电量、`0x84` 取充电状态。
//! * 读用 feature report id `0x02`，写用 `0x03`；payload 前面还有 1 字节 report id。
//!
//! 实测限制（2026-11-26，Viper Ultimate 1532:007a 有线）：
//! * MI_02 控制接口被雷蛇自家驱动占用，用户态 feature report 直接 `0x1 函数不正确`；
//! * 鼠标集合能写入 90 字节命令，但设备一律回 `0x1F`（不响应）；
//! * 有线模式下鼠标由线供电，电量本身没有意义（OpenRazer 在有线时也只给异常值）。
//!
//! 所以本协议只认无线接收器（`1532:007b`）；插上 dongle 后用
//! `cargo run -p bh-proto-razer --example razer_probe` 复核 usage 与应答布局。
//!
//! 只读：不修改任何设备设置。

use bh_api::{DriverError, HidIo, Reading};
use std::time::{Duration, Instant};

pub const REPORT_GET: u8 = 0x02;
pub const REPORT_SET: u8 = 0x03;
pub const FRAME_LEN: usize = 90;
pub const CLASS_BATTERY: u8 = 0x07;
pub const CMD_BATTERY_LEVEL: u8 = 0x80;
pub const CMD_CHARGING: u8 = 0x84;

/// 无线接收器（Viper Ultimate 的 dongle）。
pub const PID_WIRELESS: u16 = 0x007B;
/// 有线模式：电量为线供电，不做读取。
pub const PID_WIRED: u16 = 0x007A;
/// 控制通道的 HID usage（无线鼠标集合，待 dongle 复核）。
pub const CONTROL_USAGE: u16 = 0x0002;

/// 组一帧请求（未含 report id）。
pub fn frame(class: u8, cmd: u8, args: &[u8], txn: u8) -> [u8; FRAME_LEN] {
    let mut f = [0u8; FRAME_LEN];
    f[0] = 0x00; // status：新命令
    f[1] = txn & 0x1F;
    // f[2..4] = 剩余包数（单包 = 0）
    f[4] = 0x00; // 协议类型：默认
    let n = args.len().min(80);
    f[5] = n as u8;
    f[6] = class;
    f[7] = cmd;
    f[8..8 + n].copy_from_slice(&args[..n]);
    f[88] = crc(&f);
    f
}

/// 第 2..88 字节的异或。
pub fn crc(f: &[u8]) -> u8 {
    let mut c = 0u8;
    for &b in f.get(2..88).unwrap_or(&[]) {
        c ^= b;
    }
    c
}

/// 校验并切出 (class, cmd, args)。
pub fn parse(f: &[u8]) -> Option<(u8, u8, &[u8])> {
    if f.len() < FRAME_LEN || crc(f) != f[88] {
        return None;
    }
    let size = (f[5] as usize).min(80);
    Some((f[6], f[7], &f[8..8 + size]))
}

/// 设备回报的原始值：有的型号直接给百分比，有的给 0–255。
pub fn normalize(raw: u8) -> u8 {
    match raw {
        0..=100 => raw,
        _ => ((raw as u16 * 100 + 127) / 255) as u8,
    }
}

/// 发一条命令并校验应答。`deadline` 到了就返回 `Timeout`。
fn query(
    io: &mut dyn HidIo,
    class: u8,
    cmd: u8,
    args: &[u8],
    txn: u8,
    deadline: Instant,
) -> Result<Vec<u8>, DriverError> {
    if Instant::now() >= deadline {
        return Err(DriverError::Timeout);
    }
    let req = frame(class, cmd, args, txn);
    io.feature_set(REPORT_SET, &req)?;

    let mut buf = [0u8; FRAME_LEN + 1];
    let n = io.feature_get(REPORT_GET, &mut buf)?;
    let payload: &[u8] = if n >= FRAME_LEN + 1 {
        &buf[1..FRAME_LEN + 1]
    } else if n >= FRAME_LEN {
        &buf[..FRAME_LEN]
    } else {
        return Err(DriverError::Malformed);
    };
    let (rc, rcmd, out) = parse(payload).ok_or(DriverError::Malformed)?;
    if rc != class || rcmd != cmd {
        return Err(DriverError::Malformed);
    }
    if Instant::now() >= deadline {
        return Err(DriverError::Timeout);
    }
    Ok(out.to_vec())
}

/// 读一次电量：先电量、再尽量拿充电状态（拿不到不算失败）。
pub fn read_battery(io: &mut dyn HidIo, budget: Duration) -> Result<Reading, DriverError> {
    let deadline = Instant::now() + budget;
    let level = query(io, CLASS_BATTERY, CMD_BATTERY_LEVEL, &[], 0x1F, deadline)?;
    let raw = *level.first().ok_or(DriverError::Malformed)?;
    let charging = query(io, CLASS_BATTERY, CMD_CHARGING, &[], 0x1E, deadline)
        .ok()
        .and_then(|a| a.first().copied())
        .map(|v| v != 0);
    Ok(Reading { soc: normalize(raw), voltage_mv: None, charging, at_ms: bh_api::now_ms() })
}

// ---- 协议插件入口 ----

pub struct Proto;

impl bh_api::VendorProtocol for Proto {
    fn id(&self) -> &'static str {
        "razer"
    }

    fn vid(&self) -> u16 {
        0x1532
    }

    fn endpoint(&self, device: &bh_api::DeviceInfo) -> Option<bh_api::Endpoint> {
        // 只认无线接收器：有线模式（007A）电量无意义，且实测控制接口被厂商驱动占用。
        if device.vendor_id == 0x1532
            && device.product_id == PID_WIRELESS
            && device.usage_page == 0x0001
            && device.usage == CONTROL_USAGE
        {
            Some(bh_api::Endpoint { glyph: bh_api::Glyph::Mouse, mode: bh_api::Mode::Feature })
        } else {
            None
        }
    }

    fn read(
        &self,
        _device: &bh_api::DeviceInfo,
        io: &mut dyn HidIo,
        budget: Duration,
    ) -> Result<Reading, DriverError> {
        read_battery(io, budget)
    }
}

pub static PROTO: Proto = Proto;

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIo {
        reply: [u8; FRAME_LEN],
        sent: Vec<Vec<u8>>,
        ids: Vec<u8>,
    }

    impl HidIo for MockIo {
        fn feature_get(&mut self, id: u8, buf: &mut [u8]) -> Result<usize, DriverError> {
            buf[0] = id;
            buf[1..FRAME_LEN + 1].copy_from_slice(&self.reply);
            Ok(FRAME_LEN + 1)
        }
        fn feature_set(&mut self, id: u8, buf: &[u8]) -> Result<(), DriverError> {
            self.ids.push(id);
            self.sent.push(buf.to_vec());
            Ok(())
        }
        fn write(&mut self, _buf: &[u8]) -> Result<(), DriverError> {
            Err(DriverError::Unsupported)
        }
        fn read_timeout(&mut self, _buf: &mut [u8], _ms: u32) -> Result<usize, DriverError> {
            Err(DriverError::Unsupported)
        }
    }

    fn reply(class: u8, cmd: u8, args: &[u8]) -> [u8; FRAME_LEN] {
        let mut f = frame(class, cmd, args, 0x00);
        f[0] = 0x02; // 应答状态
        f[88] = crc(&f);
        f
    }

    #[test]
    fn crc_covers_bytes_2_to_88() {
        let f = frame(CLASS_BATTERY, CMD_BATTERY_LEVEL, &[], 0x1F);
        assert_eq!(f[88], crc(&f));
    }

    #[test]
    fn parse_roundtrip() {
        let r = reply(CLASS_BATTERY, CMD_BATTERY_LEVEL, &[73]);
        let (c, cmd, args) = parse(&r).unwrap();
        assert_eq!((c, cmd), (CLASS_BATTERY, CMD_BATTERY_LEVEL));
        assert_eq!(args, &[73]);
    }

    #[test]
    fn bad_crc_is_rejected() {
        let mut r = reply(CLASS_BATTERY, CMD_BATTERY_LEVEL, &[73]);
        r[88] ^= 0xFF;
        assert!(parse(&r).is_none());
    }

    #[test]
    fn normalize_scales_raw_values() {
        assert_eq!(normalize(0), 0);
        assert_eq!(normalize(73), 73);
        assert_eq!(normalize(255), 100);
        assert_eq!(normalize(128), 50);
    }

    #[test]
    fn read_battery_uses_level_command() {
        let mut io = MockIo { reply: reply(CLASS_BATTERY, CMD_BATTERY_LEVEL, &[73]), sent: vec![], ids: vec![] };
        let r = read_battery(&mut io, Duration::from_millis(500)).unwrap();
        assert_eq!(r.soc, 73);
        // 写用 report id 0x03，帧里的命令字节是 class 0x07 / cmd 0x80。
        assert_eq!(io.ids[0], REPORT_SET);
        assert_eq!(io.sent[0][6], CLASS_BATTERY);
        assert_eq!(io.sent[0][7], CMD_BATTERY_LEVEL);
        assert_eq!(io.sent[0][88], crc(&io.sent[0]));
    }

    #[test]
    fn only_wireless_is_claimed() {
        use bh_api::{Transport, VendorProtocol};
        let mk = |pid: u16| bh_api::DeviceInfo {
            vendor_id: 0x1532,
            product_id: pid,
            product: "Razer".into(),
            serial: None,
            usage_page: 0x0001,
            usage: CONTROL_USAGE,
            transport: Transport::Usb,
            key: "k".into(),
        };
        assert!(PROTO.endpoint(&mk(PID_WIRED)).is_none());
        assert!(PROTO.endpoint(&mk(PID_WIRELESS)).is_some());
    }
}
