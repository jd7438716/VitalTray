//! Logitech HID++ 2.0 最简客户端：只做"发现特性 + 读电量"。
//!
//! 协议要点（公开文档 + 社区逆向，均为事实性描述）：
//! * 长报文 report id = 0x11，短报文 0x10；帧 = [id, deviceIndex, featureIndex, function<<4|swId, params..]
//! * featureIndex 0x00 = IRoot，function 0x00 = getFeature(featureId)，返回 [featureIndex, type, version]
//! * 特性 0x1000 = BATTERY_STATUS，function 0x00 = getBatteryLevelStatus
//! * 应答里 featureIndex == 0xFF 表示错误，params[0] 为错误码
//!
//! 本模块不依赖平台，只通过 bh_api::HidIo 说话，便于单测（见 tests 里的 MockIo）。

use bh_api::{DriverError, HidIo, Reading};

pub const FEATURE_ROOT: u8 = 0x00;
pub const FEATURE_BATTERY_STATUS: u16 = 0x1000;
pub const FEATURE_BATTERY_VOLTAGE: u16 = 0x1001;
pub const REPORT_LONG: u8 = 0x11;
pub const REPORT_SHORT: u8 = 0x10;
pub const DEVICE_DIRECT: u8 = 0xFF;

/// 一次 HID++ 事务的结果。
#[derive(Debug, PartialEq, Eq)]
pub struct Response {
    pub feature_index: u8,
    pub function: u8,
    pub params: Vec<u8>,
}

pub struct Client {
    device_index: u8,
    sw_id: u8,
}

impl Client {
    pub fn new(device_index: u8) -> Self {
        Self { device_index, sw_id: 0x0A }
    }

    /// 组装一帧长报文（20 字节）。
    pub fn frame(&self, feature_index: u8, function: u8, params: &[u8]) -> [u8; 20] {
        let mut f = [0u8; 20];
        f[0] = REPORT_LONG;
        f[1] = self.device_index;
        f[2] = feature_index;
        f[3] = (function << 4) | (self.sw_id & 0x0F);
        let n = params.len().min(16);
        f[4..4 + n].copy_from_slice(&params[..n]);
        f
    }

    /// 发送请求并等待匹配的应答（超时返回 Timeout）。
    pub fn request(
        &self,
        io: &mut dyn HidIo,
        feature_index: u8,
        function: u8,
        params: &[u8],
    ) -> Result<Response, DriverError> {
        let frame = self.frame(feature_index, function, params);
        io.write(&frame)?;
        let mut buf = [0u8; 32];
        for _ in 0..3 {
            let n = io.read_timeout(&mut buf, 800)?;
            if n < 4 {
                continue;
            }
            let parsed = parse_response(&buf[..n], self.device_index);
            if let Some(r) = parsed {
                if r.feature_index == 0xFF {
                    return Err(DriverError::Unsupported);
                }
                return Ok(r);
            }
        }
        Err(DriverError::Timeout)
    }

    /// 用 IRoot.getFeature 找特性索引（找不到返回 Unsupported）。
    pub fn feature_index(&self, io: &mut dyn HidIo, feature: u16) -> Result<u8, DriverError> {
        let params = [(feature >> 8) as u8, (feature & 0xFF) as u8];
        let r = self.request(io, FEATURE_ROOT, 0x00, &params)?;
        if r.params.is_empty() || r.params[0] == 0 {
            return Err(DriverError::Unsupported);
        }
        Ok(r.params[0])
    }

    /// 读电池状态：SOC、充电标志、电压。
    pub fn battery(&self, io: &mut dyn HidIo) -> Result<Reading, DriverError> {
        let idx = self.feature_index(io, FEATURE_BATTERY_STATUS)?;
        let r = self.request(io, idx, 0x00, &[])?;
        if r.params.len() < 2 {
            return Err(DriverError::Malformed);
        }
        let soc = r.params[0].min(100);
        let status = r.params[1];
        let charging = matches!(status, 1 | 2 | 4 | 5);
        let voltage_mv = if r.params.len() >= 5 {
            Some(u16::from_le_bytes([r.params[3], r.params[4]]))
        } else {
            None
        };
        Ok(Reading { soc, voltage_mv, charging: Some(charging), at_ms: 0 })
    }
}

/// 解析应答：兼容带/不带 report id 两种读法。
pub fn parse_response(buf: &[u8], device_index: u8) -> Option<Response> {
    let (id, rest) = match buf.first() {
        Some(&REPORT_LONG) | Some(&REPORT_SHORT) => (buf[0], &buf[1..]),
        _ => (REPORT_LONG, buf),
    };
    let _ = id;
    if rest.len() < 3 {
        return None;
    }
    if rest[0] != device_index && device_index != DEVICE_DIRECT {
        return None;
    }
    let feature_index = rest[1];
    let function = rest[2] >> 4;
    Some(Response { feature_index, function, params: rest[3..].to_vec() })
}

/// 便捷入口：对 0xFF（直连）先试，再试 1..=6（接收器配对槽位）。
pub fn read_battery(io: &mut dyn HidIo) -> Result<Reading, DriverError> {
    let mut last = DriverError::Unsupported;
    for idx in [DEVICE_DIRECT, 1, 2, 3, 4, 5, 6] {
        let client = Client::new(idx);
        match client.battery(io) {
            Ok(mut r) => {
                r.at_ms = bh_api::now_ms();
                return Ok(r);
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIo {
        written: Vec<Vec<u8>>,
        replies: Vec<Vec<u8>>,
    }

    impl HidIo for MockIo {
        fn feature_get(&mut self, _id: u8, _buf: &mut [u8]) -> Result<usize, DriverError> { Err(DriverError::Unsupported) }
        fn feature_set(&mut self, _id: u8, _buf: &[u8]) -> Result<(), DriverError> { Err(DriverError::Unsupported) }
        fn write(&mut self, buf: &[u8]) -> Result<(), DriverError> {
            self.written.push(buf.to_vec());
            Ok(())
        }
        fn read_timeout(&mut self, buf: &mut [u8], _ms: u32) -> Result<usize, DriverError> {
            if self.replies.is_empty() {
                return Err(DriverError::Timeout);
            }
            let r = self.replies.remove(0);
            buf[..r.len()].copy_from_slice(&r);
            Ok(r.len())
        }
    }

    #[test]
    fn frame_layout() {
        let c = Client::new(1);
        let f = c.frame(0x05, 0x02, &[0xAA, 0xBB]);
        assert_eq!(f[0], REPORT_LONG);
        assert_eq!(f[1], 1);
        assert_eq!(f[2], 0x05);
        assert_eq!(f[3] >> 4, 0x02);
        assert_eq!(&f[4..6], &[0xAA, 0xBB]);
    }

    #[test]
    fn parses_feature_index() {
        let mut io = MockIo { written: vec![], replies: vec![vec![0x11, 0x01, 0x00, 0x00, 0x06, 0x00, 0x00]] };
        let c = Client::new(1);
        let idx = c.feature_index(&mut io, FEATURE_BATTERY_STATUS).unwrap();
        assert_eq!(idx, 0x06);
        // 发出的请求应包含特性号 0x1000
        assert_eq!(&io.written[0][4..6], &[0x10, 0x00]);
    }

    #[test]
    fn battery_status_parsing_discharging() {
        let mut io = MockIo {
            written: vec![],
            replies: vec![
                vec![0x11, 0x01, 0x00, 0x00, 0x06, 0x00, 0x00],          // getFeature -> 0x06
                vec![0x11, 0x01, 0x06, 0x00, 73, 0x00, 0x00, 0x34, 0x12], // 73%, discharging, 0x1234 mV
            ],
        };
        let c = Client::new(1);
        let r = c.battery(&mut io).unwrap();
        assert_eq!(r.soc, 73);
        assert_eq!(r.charging, Some(false));
        assert_eq!(r.voltage_mv, Some(0x1234));
    }

    #[test]
    fn charging_flag() {
        let mut io = MockIo {
            written: vec![],
            replies: vec![
                vec![0x11, 0x01, 0x00, 0x00, 0x06, 0x00, 0x00],
                vec![0x11, 0x01, 0x06, 0x00, 40, 0x01, 0x01, 0x00, 0x10],
            ],
        };
        let r = Client::new(1).battery(&mut io).unwrap();
        assert_eq!(r.charging, Some(true));
    }

    #[test]
    fn error_reply_is_unsupported() {
        let mut io = MockIo {
            written: vec![],
            replies: vec![vec![0x11, 0x01, 0xFF, 0x00, 0x06]],
        };
        let c = Client::new(1);
        assert_eq!(c.feature_index(&mut io, FEATURE_BATTERY_STATUS), Err(DriverError::Unsupported));
    }
}
