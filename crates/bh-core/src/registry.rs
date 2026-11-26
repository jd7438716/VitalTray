//! 协议注册表（编译期）：加一个厂商 = 加一个 `bh-proto-*` crate + 这里一行。
//!
//! 分发只做一件事：按 VID 收敛候选协议。没插的厂商 = 一次 `HashMap` miss，零成本。
//! 这就是"检测到才增加"的等价实现 —— 不需要动态加载。

use bh_api::{DeviceInfo, Endpoint, VendorProtocol};
use std::collections::HashMap;
use std::sync::OnceLock;

/// 全部厂商协议。同一 VID 有多个协议时按此顺序取第一个认领者。
static PROTOCOLS: &[&'static dyn VendorProtocol] = &[
    &bh_proto_logitech::PROTO,
    &bh_proto_rapoo::PROTO,
    &bh_proto_razer::PROTO,
];

type Index = HashMap<u16, Vec<&'static dyn VendorProtocol>>;

static BY_VID: OnceLock<Index> = OnceLock::new();

fn by_vid() -> &'static Index {
    BY_VID.get_or_init(|| {
        let mut m: Index = HashMap::new();
        for p in PROTOCOLS {
            m.entry(p.vid()).or_default().push(*p);
        }
        m
    })
}

pub fn protocols() -> &'static [&'static dyn VendorProtocol] {
    PROTOCOLS
}

/// 认领一条 collection：VID 命中才进入协议。
pub fn claim(dev: &DeviceInfo) -> Option<(&'static dyn VendorProtocol, Endpoint)> {
    let list = by_vid().get(&dev.vendor_id)?;
    list.iter().find_map(|p| p.endpoint(dev).map(|ep| (*p, ep)))
}

/// 注册表清单（日志 / 关于框用）。
pub fn describe() -> Vec<String> {
    PROTOCOLS.iter().map(|p| format!("proto {} vid={:04x}", p.id(), p.vid())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bh_api::Transport;

    fn dev(vid: u16, usage_page: u16, usage: u16) -> DeviceInfo {
        DeviceInfo {
            vendor_id: vid,
            product_id: 0x185A,
            product: "test".into(),
            serial: None,
            usage_page,
            usage,
            transport: Transport::Receiver,
            key: "k".into(),
        }
    }

    #[test]
    fn every_protocol_is_indexed() {
        for p in protocols() {
            let hit = by_vid().get(&p.vid()).map(|l| l.iter().any(|q| q.id() == p.id())).unwrap_or(false);
            assert!(hit, "协议 {} 没进索引", p.id());
        }
    }

    #[test]
    fn unknown_vid_is_free() {
        assert!(claim(&dev(0xFFFF, 0xFF00, 0x0002)).is_none());
    }

    #[test]
    fn rapoo_channels_split_by_usage() {
        assert_eq!(claim(&dev(0x24AE, 0xFF00, 0x0002)).map(|(_, e)| e.glyph), Some(bh_api::Glyph::Mouse));
        assert_eq!(claim(&dev(0x24AE, 0xFF00, 0x000E)).map(|(_, e)| e.glyph), Some(bh_api::Glyph::Keyboard));
        assert!(claim(&dev(0x24AE, 0xFF00, 0x0001)).is_none());
    }

    #[test]
    fn razer_wired_mode_is_not_claimed() {
        // 有线 007A：电量无意义，不认领（否则会多一行永远读不到的设备）。
        let mut d = dev(0x1532, 0x0001, 0x0002);
        d.product_id = 0x007A;
        assert!(claim(&d).is_none());
        d.product_id = 0x007B;
        assert!(claim(&d).is_some());
    }
}
