//! BatteryHub 宿主：命令行、配置、单实例、自启、托盘与消息循环。
//!
//! M0 目标：可双击运行、托盘有图标、右键菜单可用、开机自启可开关。

pub mod about;
pub mod probe;
pub mod window;
pub mod app;
pub mod autostart;
pub mod brand;
pub mod bt;
pub mod capture;
pub mod cli;
pub mod commands;
pub mod console;
pub mod machine;
pub mod panel;
pub mod runtime;
pub mod state;
pub mod config;
pub mod estimate;
pub mod history;
pub mod monitor;
pub mod i18n;
pub mod log;
pub mod paths;
pub mod poller;
pub mod plugins;
pub mod registry;
pub mod selftest;
pub mod single_instance;
pub mod status;
pub mod sysmon;
pub mod system;
pub mod theme;
pub mod tray;
pub mod util;

pub fn run() -> i32 {
    app::run()
}

/// 版本号（来自 Cargo）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
