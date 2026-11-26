//! BatteryHub 插件 API。
//!
//! 设计约束：
//! * 本 crate 不依赖任何平台 API，只依赖 std。以后要做 sidecar / WASM 插件也不用改。
//! * 插件按“设备域”切分（鼠标 / 键盘 / 笔记本 / 耳机 / 手柄 …），
//!   协议库（logitech、razer …）是普通库 crate，被多个域插件共用。
//! * 命令（菜单项）用稳定 id + i18n key 描述，宿主负责渲染与分发。

#![forbid(unsafe_code)]

/// 插件元信息，用于日志、设置页与许可证展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginMeta {
    pub id: &'static str,
    pub name_key: &'static str,
    pub version: &'static str,
}

/// 设备连接方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Usb,
    Receiver,
    Bluetooth,
    System,
}

/// 一个候选设备（由宿主的 HID 枚举产生，插件只读）。
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub product: String,
    pub serial: Option<String>,
    pub usage_page: u16,
    pub usage: u16,
    pub transport: Transport,
    /// 宿主的稳定设备键（接收器场景会做聚合）。
    pub key: String,
}

/// 一次电量读数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub soc: u8,
    pub voltage_mv: Option<u16>,
    pub charging: Option<bool>,
    pub at_ms: i64,
}

impl Reading {
    pub fn new(soc: u8, at_ms: i64) -> Self {
        Self {
            soc,
            voltage_mv: None,
            charging: None,
            at_ms,
        }
    }
}

/// 领域插件读取设备时的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    Unsupported,
    Timeout,
    Io(String),
    Malformed,
}

/// 设备类别图标（宿主负责映射到字体/GDI 字形）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
    Laptop,
    Phone,
    Unknown,
}

/// 设备与插件的匹配强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Match {
    None,
    Weak,
    Strong,
}

/// 宿主提供给插件的 HID 读写句柄（由 bh-hid 实现）。
pub trait HidIo {
    fn feature_get(&mut self, report_id: u8, buf: &mut [u8]) -> Result<usize, DriverError>;
    fn feature_set(&mut self, report_id: u8, buf: &[u8]) -> Result<(), DriverError>;
    fn write(&mut self, buf: &[u8]) -> Result<(), DriverError>;
    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, DriverError>;
}

/// 宿主提供的本机电源信息（笔记本域用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemBattery {
    pub percent: u8,
    pub charging: bool,
    pub ac_online: bool,
    pub seconds_left: Option<u32>,
}

/// 宿主实现：读取本机系统电量与电源状态。
pub trait SystemInfo: Send + Sync {
    fn battery(&self) -> Option<SystemBattery>;

    /// 蓝牙设备电量（key = 宿主给出的 bt: 前缀设备键）。
    fn bluetooth_battery(&self, _key: &str) -> Option<u8> {
        None
    }
}

/// 一次读取的上下文：HID 句柄与系统信息，由宿主注入。
pub struct ReadContext<'a> {
    pub hid: Option<&'a mut dyn HidIo>,
    pub system: Option<&'a dyn SystemInfo>,
    pub now_ms: i64,
}

/// 菜单命令定义（宿主统一渲染，id 全局唯一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label_key: &'static str,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Action,
    Toggle,
    Submenu,
}

/// 所有插件的最小契约。
pub trait Plugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
}

/// 设备域插件：负责一类设备（鼠标 / 键盘 / 笔记本 …）的匹配与读取。
pub trait DeviceDomain: Plugin {
    /// 该类设备的图标。
    fn glyph(&self) -> Glyph;

    /// 判断一个候选设备是否属于本域。
    fn matches(&self, device: &DeviceInfo) -> Match;

    /// 读取电量。只在 matches 非 None 时调用。
    fn read(&self, device: &DeviceInfo, ctx: &mut ReadContext<'_>) -> Result<Reading, DriverError>;

    /// 本域贡献的菜单命令（宿主插到“设备”子菜单下）。
    fn commands(&self) -> &'static [CommandSpec] {
        &[]
    }
}

/// 功能插件：不读设备，只消费读数/事件（历史、续航估算、通知 …）。
pub trait Feature: Plugin {
    fn commands(&self) -> &'static [CommandSpec] {
        &[]
    }
}

/// 当前 UTC 毫秒（插件里不直接依赖 std::time 之外的时钟）。
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
