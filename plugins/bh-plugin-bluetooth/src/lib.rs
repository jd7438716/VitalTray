//! 蓝牙设备域插件：电量由 Windows 提供（DEVPKEY_Bluetooth_Battery），无需私有协议。

use bh_api::{
    DeviceDomain, DeviceInfo, DriverError, Glyph, Match, Plugin, PluginMeta, ReadContext, Reading,
};

pub struct Domain;

impl Plugin for Domain {
    fn meta(&self) -> PluginMeta {
        PluginMeta { id: "bluetooth", name_key: "plugin.bluetooth.name", version: env!("CARGO_PKG_VERSION") }
    }
}

impl DeviceDomain for Domain {
    fn glyph(&self) -> Glyph {
        Glyph::Unknown
    }

    fn matches(&self, device: &DeviceInfo) -> Match {
        if device.key.starts_with("bt:") {
            Match::Strong
        } else {
            Match::None
        }
    }

    fn read(&self, device: &DeviceInfo, ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError> {
        let sys = ctx.system.ok_or(DriverError::Unsupported)?;
        let soc = sys.bluetooth_battery(&device.key).ok_or(DriverError::Unsupported)?;
        Ok(Reading { soc, voltage_mv: None, charging: None, at_ms: ctx.now_ms })
    }
}
