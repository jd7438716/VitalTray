//! 插件注册表（编译期）。加一个域插件 = 加一个 crate + 这里一行。
//!
//! 注意分工（ADR-0002）：域插件只回答"这是什么设备"（图标 / 菜单），
//! 厂商协议（能不能读、怎么读）在 `crate::registry` 里注册。

use bh_api::DeviceDomain;
use bh_plugin_bluetooth::Domain as BluetoothDomain;
use bh_plugin_keyboard::Domain as KeyboardDomain;
use bh_plugin_laptop::Domain as LaptopDomain;
use bh_plugin_mouse::Domain as MouseDomain;

static MOUSE: MouseDomain = MouseDomain;
static KEYBOARD: KeyboardDomain = KeyboardDomain;
static BLUETOOTH: BluetoothDomain = BluetoothDomain;
static LAPTOP: LaptopDomain = LaptopDomain;

/// 所有设备域插件（顺序即菜单顺序）。静态表，热路径上零分配。
static DOMAINS: [&'static dyn DeviceDomain; 4] = [&MOUSE, &KEYBOARD, &LAPTOP, &BLUETOOTH];

pub fn domains() -> &'static [&'static dyn DeviceDomain] {
    &DOMAINS
}

/// 插件清单（日志/关于框用）。
pub fn describe() -> Vec<String> {
    domains()
        .iter()
        .map(|d| {
            let m = d.meta();
            format!("{} v{} ({:?})", m.id, m.version, d.glyph())
        })
        .collect()
}
