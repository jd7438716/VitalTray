//! 极简 INI 配置：人可以直接编辑（对齐参考项目的风格）。

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub autostart: bool,
    pub tray: bool,
    pub history: bool,
    pub low_battery: u8,
    pub log_level: String,
    pub icon_style: String,
    pub start_minimized: bool,
    pub refresh_secs: u32,
    pub lang: String,
    /// 抓帧白名单（VID 十六进制列表，如 `09da,24ae`）；空 = 不抓。
    pub capture_vids: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: true,
            tray: true,
            history: true,
            low_battery: 20,
            log_level: "info".into(),
            icon_style: "battery".into(),
            start_minimized: true,
            refresh_secs: 60,
            lang: "zh-CN".into(),
            capture_vids: String::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let mut cfg = Config::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            return cfg;
        };
        let map = parse_ini(&text);
        if let Some(v) = map.get("system.autostart") {
            cfg.autostart = v == "1" || v.eq_ignore_ascii_case("true")
        }
        if let Some(v) = map.get("system.tray") {
            cfg.tray = v == "1" || v.eq_ignore_ascii_case("true")
        }
        if let Some(v) = map.get("system.start_minimized") {
            cfg.start_minimized = v == "1" || v.eq_ignore_ascii_case("true")
        }
        if let Some(v) = map.get("poll.refresh_secs") {
            if let Ok(n) = v.parse::<u32>() {
                cfg.refresh_secs = n.clamp(10, 3600);
            }
        }
        if let Some(v) = map.get("system.log_level") { cfg.log_level = v.clone(); }
        if let Some(v) = map.get("ui.icon_style").or_else(|| map.get("system.icon_style")) { cfg.icon_style = v.clone(); }
        if let Some(v) = map.get("ui.lang") {
            if !v.is_empty() {
                cfg.lang = v.clone();
            }
        }
        if let Some(v) = map.get("capture.vids") { cfg.capture_vids = v.clone(); }
        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = format!(
            "; BatteryHub 配置（改完重启生效）\n[system]\nautostart={}\ntray={}\nhistory={}\nlow_battery={}\nlog_level={}\nstart_minimized={}\n\n[poll]\nrefresh_secs={}\n\n[ui]\nlang={}\nicon_style={}\n\n; 逆向用：只抓这些厂商(VID, 16 进制)的厂商页 HID 报文，留空则不抓\n[capture]\nvids={}\n",
            b(self.autostart),
            b(self.tray),
            b(self.history),
            self.low_battery,
            self.log_level,
            b(self.start_minimized),
            self.refresh_secs,
            self.lang,
            self.icon_style,
            self.capture_vids
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)
    }
}

fn b(v: bool) -> u8 {
    if v {
        1
    } else {
        0
    }
}

fn parse_ini(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = format!("{}.{}", section, k.trim().to_ascii_lowercase());
            map.insert(key, v.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_values() {
        let m = parse_ini("[system]\nautostart=0\ntray = 1\n[poll]\nrefresh_secs=30\n");
        assert_eq!(m.get("system.autostart").unwrap(), "0");
        assert_eq!(m.get("system.tray").unwrap(), "1");
        assert_eq!(m.get("poll.refresh_secs").unwrap(), "30");
    }

    #[test]
    fn roundtrip_save_and_load() {
        let dir = std::env::temp_dir().join("bh-config-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.ini");
        let mut c = Config::default();
        c.autostart = false;
        c.refresh_secs = 120;
        c.save(&path).unwrap();
        let back = Config::load(&path);
        assert_eq!(back, c);
    }
}
