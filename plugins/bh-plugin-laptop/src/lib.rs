//! 笔记本设备域插件：读取本机电池（数据由宿主的 SystemInfo 注入）。

use bh_api::{
    DeviceDomain, DeviceInfo, DriverError, Glyph, Match, Plugin, PluginMeta, ReadContext, Reading,
};

pub struct Domain;

impl Plugin for Domain {
    fn meta(&self) -> PluginMeta {
        PluginMeta { id: "laptop", name_key: "plugin.laptop.name", version: env!("CARGO_PKG_VERSION") }
    }
}

impl DeviceDomain for Domain {
    fn glyph(&self) -> Glyph {
        Glyph::Laptop
    }

    fn matches(&self, device: &DeviceInfo) -> Match {
        // 本机电池由宿主以 Transport::System + key 前缀 sys: 注入
        if device.transport == bh_api::Transport::System && device.key.starts_with("sys:") {
            Match::Strong
        } else {
            Match::None
        }
    }

    fn read(&self, _device: &DeviceInfo, ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError> {
        let sys = ctx.system.ok_or(DriverError::Unsupported)?;
        let b = sys.battery().ok_or(DriverError::Unsupported)?;
        Ok(Reading {
            soc: b.percent,
            voltage_mv: None,
            charging: Some(b.charging),
            at_ms: ctx.now_ms,
        })
    }
}
