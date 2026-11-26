//! 鼠标设备域插件。
//!
//! 只负责"这类设备长什么样"（图标 / 将来贡献菜单）。**读电量不在这里**：
//! 厂商协议由 `bh-proto-*` 实现、由宿主 `registry` 按 VID 分发。
//! 所以加一个新品牌（雷蛇 / 双飞燕 …）不需要改本文件（ADR-0002）。

use bh_api::{
    DeviceDomain, DeviceInfo, DriverError, Glyph, Match, Plugin, PluginMeta, ReadContext, Reading,
};

pub struct Domain;

impl Plugin for Domain {
    fn meta(&self) -> PluginMeta {
        PluginMeta { id: "mouse", name_key: "plugin.mouse.name", version: env!("CARGO_PKG_VERSION") }
    }
}

impl DeviceDomain for Domain {
    fn glyph(&self) -> Glyph {
        Glyph::Mouse
    }

    fn matches(&self, device: &DeviceInfo) -> Match {
        // 纯分类：标准鼠标集合。Weak 表示"认得出类别"，但宿主不会直接读它
        // （HID 电量必须走协议插件的厂商通道）。
        if device.usage_page == 0x01 && device.usage == 0x02 {
            Match::Weak
        } else {
            Match::None
        }
    }

    fn read(&self, _device: &DeviceInfo, _ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError> {
        Err(DriverError::Unsupported)
    }
}
