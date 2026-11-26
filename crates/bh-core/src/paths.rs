//! 路径解析：默认 %LOCALAPPDATA%\VitalTray，便携模式用 exe 同目录。
//!
//! 2026-11-26 由旧名 BatteryHub 更名，旧目录只在迁移时读取一次。

use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "VitalTray";

/// 旧产品名的数据目录名：只用于迁移，不再写入。
pub const LEGACY_APP_DIR: &str = "BatteryHub";

/// 把旧版（`%LOCALAPPDATA%\BatteryHub`）的配置迁移到新目录。
///
/// 仅在「新目录还没有 config.ini」且「旧目录里有」时复制一份，**不删旧目录**：
/// 用户可能还留着旧版 exe，删掉等于替他做决定。返回值 = 发生迁移时旧目录路径。
pub fn migrate_legacy_config(dir: &Path) -> Option<PathBuf> {
    if config_file(dir).is_file() {
        return None;
    }
    let old = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join(LEGACY_APP_DIR);
    let old_cfg = config_file(&old);
    if !old_cfg.is_file() || ensure_dir(dir).is_err() {
        return None;
    }
    match std::fs::copy(&old_cfg, config_file(dir)) {
        Ok(_) => Some(old),
        Err(_) => None,
    }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir(portable: bool) -> PathBuf {
    if portable {
        return exe_dir().join(APP_DIR);
    }
    match std::env::var_os("LOCALAPPDATA") {
        Some(v) => PathBuf::from(v).join(APP_DIR),
        None => exe_dir().join(APP_DIR),
    }
}

pub fn config_file(dir: &Path) -> PathBuf {
    dir.join("config.ini")
}

pub fn log_file(dir: &Path) -> PathBuf {
    dir.join("app.log")
}

pub fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}
