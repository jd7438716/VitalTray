//! 厂商协议契约（ADR-0002）：硬件检测由宿主统一做，协议只回答"怎么读"。
//!
//! 分工：
//! * 宿主（bh-core）：一次枚举 → [`DeviceTable`] 快照 → 句柄生命周期、超时预算、退避、UI 行。
//! * 域插件（mouse / keyboard / laptop / bluetooth）：图标与菜单命令，不碰 HID 细节。
//! * 协议插件（bh-proto-*）：按 VID 认领电量通道（[`Endpoint`]），实现 [`VendorProtocol::read`]。
//!
//! 加一个新品牌 = 新建一个 `bh-proto-*` crate + 在 `bh-core::registry` 的静态表里加一行；
//! 域插件和轮询代码一行都不用改。

use crate::{DeviceInfo, DriverError, Glyph, HidIo, Reading};
use std::time::Duration;

/// 读取模式，决定宿主给的等待预算与退避节奏。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 被动：设备被使用时才推帧（如雷柏接收器），宿主必须给足等待时间。
    Passive,
    /// 主动：发 feature report / 写请求后设备立刻应答（如 HID++、Razer）。
    Feature,
}

/// 一条电量通道的语义：代表哪类设备 + 怎么读。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub glyph: Glyph,
    pub mode: Mode,
}

/// 宿主一次枚举产出的不可变设备快照，统一下发给所有插件（谁都不许自己再枚举）。
#[derive(Debug, Default, Clone)]
pub struct DeviceTable {
    pub devices: Vec<DeviceInfo>,
}

impl DeviceTable {
    pub fn new(devices: Vec<DeviceInfo>) -> Self {
        Self { devices }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DeviceInfo> {
        self.devices.iter()
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }
}

impl<'a> IntoIterator for &'a DeviceTable {
    type Item = &'a DeviceInfo;
    type IntoIter = std::slice::Iter<'a, DeviceInfo>;

    fn into_iter(self) -> Self::IntoIter {
        self.devices.iter()
    }
}

/// 厂商协议插件。注册一次即自动按 VID 参与分发。
pub trait VendorProtocol: Send + Sync {
    /// 稳定 id（日志 / 关于框用）。
    fn id(&self) -> &'static str;

    /// 负责的厂商号；宿主拿它建 `VID → 协议` 索引。
    fn vid(&self) -> u16;

    /// 这条 collection 是不是本协议的电量入口；是则给出语义。
    ///
    /// 必须是纯计算：不许开句柄、不许枚举、不许 sleep —— 它每轮对每条 collection 都会跑。
    fn endpoint(&self, device: &DeviceInfo) -> Option<Endpoint>;

    /// 读取一次电量。`budget` 是宿主给的等待上限，实现必须尊重（超了就返回 `Timeout`）。
    fn read(
        &self,
        device: &DeviceInfo,
        io: &mut dyn HidIo,
        budget: Duration,
    ) -> Result<Reading, DriverError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_table_iterates() {
        let t = DeviceTable::default();
        assert!(t.is_empty());
        assert_eq!(t.iter().count(), 0);
        assert_eq!((&t).into_iter().count(), 0);
    }
}
