//! 键盘设备域插件。
//!
//! M1 支持：Logitech HID++ 2.0（直连或接收器首个应答的配对槽位）。
//! 其他品牌在后续里程碑按协议库接入。

use bh_api::{
    DeviceDomain, DeviceInfo, DriverError, Glyph, Match, Plugin, PluginMeta, ReadContext, Reading,
};

pub struct Domain;

impl Plugin for Domain {
    fn meta(&self) -> PluginMeta {
        PluginMeta { id: "keyboard", name_key: "plugin.keyboard.name", version: env!("CARGO_PKG_VERSION") }
    }
}

impl DeviceDomain for Domain {
    fn glyph(&self) -> Glyph {
        Glyph::Keyboard
    }

    fn matches(&self, device: &DeviceInfo) -> Match {
        // Logitech 的 HID++ 集合用 vendor page 0xFF00，是读电量最可靠的入口
        if device.vendor_id == 0x046D && device.usage_page == 0xFF00 {
            return Match::Strong;
        }
        // Rapoo 接收器的键盘通道
        if device.vendor_id == bh_proto_rapoo::VENDOR_RAPOO
            && device.usage_page == bh_proto_rapoo::USAGE_PAGE_VENDOR
            && device.usage == 0x000E
        {
            return Match::Strong;
        }
        if device.usage_page == 0x01 && device.usage == 0x06 {
            return Match::Weak;
        }
        Match::None
    }

    fn read(&self, _device: &DeviceInfo, ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError> {
        let hid = ctx.hid.as_mut().ok_or(DriverError::Unsupported)?;
        if _device.vendor_id == bh_proto_rapoo::VENDOR_RAPOO {
            return bh_proto_rapoo::read_battery(&mut **hid, 2000);
        }
        bh_proto_logitech::read_battery(&mut **hid)
    }
}
