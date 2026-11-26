//! 键盘设备域插件。
//!
//! 只负责"这类设备长什么样"（图标 / 将来贡献菜单）。**读电量不在这里**：
//! 厂商协议由 `bh-proto-*` 实现、由宿主 `registry` 按 VID 分发（ADR-0002）。

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
        // 纯分类：标准键盘集合（Weak = 认得出，但不直接读）。
        if device.usage_page == 0x01 && device.usage == 0x06 {
            Match::Weak
        } else {
            Match::None
        }
    }

    fn read(&self, _device: &DeviceInfo, _ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError> {
        Err(DriverError::Unsupported)
    }
}
