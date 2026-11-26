//! 路径解析：默认 %LOCALAPPDATA%\BatteryHub，便携模式用 exe 同目录。

use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "BatteryHub";

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
