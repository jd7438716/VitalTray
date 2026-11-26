//! 极简 i18n：中英两套字符串，启动时按配置选择。

use std::sync::atomic::{AtomicBool, Ordering};

static EN: AtomicBool = AtomicBool::new(false);

pub fn init(lang: &str) {
    EN.store(lang.to_ascii_lowercase().starts_with("en"), Ordering::Relaxed);
}

pub fn icon_style_label(style: &str) -> &'static str {
    let en = EN.load(Ordering::Relaxed);
    match (style, en) {
        ("number", false) => "图标样式：数字",
        ("number", true) => "Icon style: number",
        ("ring", false) => "图标样式：圆环",
        ("ring", true) => "Icon style: ring",
        (_, false) => "图标样式：电池",
        (_, true) => "Icon style: battery",
    }
}

/// 设备类别短名（面板与提示共用）。
pub fn device_label(glyph: bh_api::Glyph) -> &'static str {
    let en = EN.load(Ordering::Relaxed);
    match (glyph, en) {
        (bh_api::Glyph::Mouse, false) => "鼠标",
        (bh_api::Glyph::Mouse, true) => "Mouse",
        (bh_api::Glyph::Keyboard, false) => "键盘",
        (bh_api::Glyph::Keyboard, true) => "Keyboard",
        (bh_api::Glyph::Headset, false) => "耳机",
        (bh_api::Glyph::Headset, true) => "Headset",
        (bh_api::Glyph::Gamepad, false) => "手柄",
        (bh_api::Glyph::Gamepad, true) => "Gamepad",
        (bh_api::Glyph::Laptop, false) => "本机电池",
        (bh_api::Glyph::Laptop, true) => "Laptop battery",
        (bh_api::Glyph::Phone, false) => "手机",
        (bh_api::Glyph::Phone, true) => "Phone",
        (bh_api::Glyph::Unknown, false) => "蓝牙设备",
        (bh_api::Glyph::Unknown, true) => "Bluetooth",
    }
}

pub fn t(key: &str) -> &'static str {
    let en = EN.load(Ordering::Relaxed);
    match (key, en) {
        ("menu.refresh", false) => "立即刷新",
        ("menu.refresh", true) => "Refresh now",
        ("menu.open_log", false) => "打开日志目录",
        ("menu.open_log", true) => "Open log folder",
        ("menu.autostart", false) => "开机自启",
        ("menu.autostart", true) => "Start with Windows",
        ("menu.tray", false) => "显示托盘图标",
        ("menu.tray", true) => "Tray icons",
        ("menu.log_level", false) => "调试日志（详细）",
        ("menu.log_level", true) => "Debug logging",
        ("menu.about", false) => "关于 BatteryHub",
        ("menu.about", true) => "About BatteryHub",
        ("menu.quit", false) => "退出",
        ("menu.quit", true) => "Quit",
        ("about.title", false) => "关于 BatteryHub",
        ("about.title", true) => "About BatteryHub",
        ("about.plugins", false) => "插件",
        ("about.plugins", true) => "Plugins",
        ("about.config", false) => "配置",
        ("about.config", true) => "Config",
        ("about.logdir", false) => "日志目录",
        ("about.logdir", true) => "Log folder",
        ("tooltip.none", false) => "BatteryHub — 未发现设备",
        ("tooltip.none", true) => "BatteryHub — no devices found",
        ("low.title", false) => "电量偏低",
        ("low.title", true) => "Low battery",
        ("low.body", false) => " 剩余 {}%，该充电了",
        ("low.body", true) => " is at {}% — time to charge",
        ("low.log", false) => "低电提醒",
        ("low.log", true) => "low battery alert",
        ("panel.system", false) => "系统",
        ("panel.system", true) => "System",
        ("panel.status", false) => "系统状态",
        ("panel.status", true) => "System status",
        ("panel.devices", false) => "设备",
        ("panel.devices", true) => "Devices",
        ("panel.cpu", false) => "CPU",
        ("panel.cpu", true) => "CPU",
        ("panel.gpu", false) => "GPU",
        ("panel.gpu", true) => "GPU",
        ("panel.mem", false) => "内存",
        ("panel.mem", true) => "Memory",
        ("panel.disk", false) => "系统盘",
        ("panel.disk", true) => "System disk",
        ("panel.updated", false) => "更新于",
        ("panel.updated", true) => "Updated",
        ("panel.none", false) => "未发现电量设备",
        ("panel.none", true) => "No battery devices",
        ("panel.waiting", false) => "待上报",
        ("panel.waiting", true) => "waiting",
        ("panel.stale", false) => "上次",
        ("panel.stale", true) => "last",
        ("panel.free", false) => "可用",
        ("panel.free", true) => "free",
        _ => "",
    }
}
