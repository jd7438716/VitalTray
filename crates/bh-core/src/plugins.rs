//! 插件注册表（编译期）。加一个域插件 = 加一个 crate + 这里一行。

use bh_api::DeviceDomain;
use bh_plugin_bluetooth::Domain as BluetoothDomain;
use bh_plugin_keyboard::Domain as KeyboardDomain;
use bh_plugin_laptop::Domain as LaptopDomain;
use bh_plugin_mouse::Domain as MouseDomain;

static MOUSE: MouseDomain = MouseDomain;
static KEYBOARD: KeyboardDomain = KeyboardDomain;
static BLUETOOTH: BluetoothDomain = BluetoothDomain;
static LAPTOP: LaptopDomain = LaptopDomain;

/// 所有设备域插件（顺序即菜单顺序）。
pub fn domains() -> Vec<&'static dyn DeviceDomain> {
    vec![&MOUSE, &KEYBOARD, &LAPTOP, &BLUETOOTH]
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
