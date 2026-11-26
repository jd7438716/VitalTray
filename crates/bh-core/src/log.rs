//! 开发日志：分级 + 模块标签 + 按大小滚动（保留 3 个文件）。
//!
//! 格式：`2026-11-23T22:50:01.123+08:00 [INFO ] [poll] message`
//! 级别：error < warn < info < debug < trace；运行时可通过菜单/配置切换。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN ",
            Level::Info => "INFO ",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Level::Error),
            "warn" | "warning" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }
}

const MAX_BYTES: u64 = 1024 * 1024;
const KEEP: usize = 3;

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);
static SINK: OnceLock<Mutex<PathBuf>> = OnceLock::new();

pub fn init(path: PathBuf, level: Level) {
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    let _ = SINK.set(Mutex::new(path.clone()));
    set_level(level);
    info("app", &format!("---- log started ({}) ----", level.name()));
}

pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn level() -> Level {
    match LEVEL.load(Ordering::Relaxed) {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        _ => Level::Trace,
    }
}

pub fn enabled(level: Level) -> bool {
    level <= self::level()
}

pub fn error(module: &str, msg: &str) {
    write(Level::Error, module, msg);
}
pub fn warn(module: &str, msg: &str) {
    write(Level::Warn, module, msg);
}
pub fn info(module: &str, msg: &str) {
    write(Level::Info, module, msg);
}
pub fn debug(module: &str, msg: &str) {
    write(Level::Debug, module, msg);
}
pub fn trace(module: &str, msg: &str) {
    write(Level::Trace, module, msg);
}

fn write(level: Level, module: &str, msg: &str) {
    if !enabled(level) {
        return;
    }
    let Some(sink) = SINK.get() else { return };
    let Ok(path) = sink.lock() else { return };
    rotate(&path);
    let ts = crate::util::timestamp_ms();
    let line = format!("{ts} [{}] [{module}] {msg}", level.as_str());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&*path) {
        let _ = writeln!(f, "{line}");
    }
}

fn rotate(path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.len() < MAX_BYTES {
        return;
    }
    for i in (1..KEEP).rev() {
        let from = path.with_extension(format!("log.{}", i));
        let to = path.with_extension(format!("log.{}", i + 1));
        let _ = std::fs::rename(&from, &to);
    }
    let _ = std::fs::rename(path, path.with_extension("log.1"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Info < Level::Debug);
        assert_eq!(Level::parse("DEBUG"), Some(Level::Debug));
        assert_eq!(Level::parse("nope"), None);
    }

    #[test]
    fn writes_when_enabled() {
        let dir = std::env::temp_dir().join("bh-log-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("app.log");
        init(p.clone(), Level::Debug);
        set_level(Level::Debug);
        info("test", "hello");
        debug("test", "detail");
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("[INFO ] [test] hello"));
        assert!(text.contains("[DEBUG] [test] detail"));
    }
}
