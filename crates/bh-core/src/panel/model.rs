//! 面板内容模型：把“设备电量 + 系统状态”组织成竖排的行。
//!
//! 面板是只读的：这里只有看得懂的信息（文字 + 语义色），没有命令、没有可点的行；
//! 所有动作都在托盘右键菜单里（`commands`）。绘制在 paint.rs，配色在 theme.rs。

use crate::i18n;
use crate::machine;
use crate::poller::DeviceReading;
use crate::state;
use crate::sysmon::{self, Snapshot};
use crate::util;
use bh_api::Glyph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Cpu,
    Gpu,
    Memory,
    Disk,
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
    Laptop,
    Phone,
    Bluetooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Amber,
    Red,
    Dim,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub icon: Icon,
    pub title: String,
    pub detail: String,
    pub value: String,
    pub tone: Tone,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub brand: String,
    pub model: String,
    pub sections: Vec<Section>,
    pub updated: String,
}

/// 面板最多展示的设备行（再多会撑出屏幕）。
const MAX_DEVICES: usize = 6;

pub fn build() -> Model {
    let readings = state::with(|st| st.map(|s| s.readings.clone()).unwrap_or_default());
    let snap = sysmon::latest();
    let info = machine::info();
    let brand = if info.vendor.is_empty() { "BatteryHub".to_string() } else { info.vendor.clone() };
    let model = if info.model.is_empty() { i18n::t("panel.status").to_string() } else { info.model.clone() };
    Model {
        brand,
        model,
        sections: vec![
            Section { title: i18n::t("panel.system"), rows: system_rows(&snap, info.cpu.as_str()) },
            Section { title: i18n::t("panel.devices"), rows: device_rows(&readings) },
        ],
        updated: format!("{} {}", i18n::t("panel.updated"), util::clock()),
    }
}

fn system_rows(snap: &Snapshot, cpu_name: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    rows.push(Row {
        icon: Icon::Cpu,
        title: i18n::t("panel.cpu").to_string(),
        detail: cpu_name.to_string(),
        value: metric(snap.cpu_use, snap.cpu_temp),
        tone: metric_tone(snap.cpu_use, snap.cpu_temp),
    });

    if snap.gpus.is_empty() {
        rows.push(Row {
            icon: Icon::Gpu,
            title: i18n::t("panel.gpu").to_string(),
            detail: String::new(),
            value: metric(snap.gpu_use, None),
            tone: snap.gpu_use.map(usage_tone).unwrap_or(Tone::Dim),
        });
    } else {
        for gpu in snap.gpus.iter().take(2) {
            rows.push(Row {
                icon: Icon::Gpu,
                title: i18n::t("panel.gpu").to_string(),
                detail: gpu.name.clone(),
                value: metric(gpu.use_percent, gpu.temp_c),
                tone: metric_tone(gpu.use_percent, gpu.temp_c),
            });
        }
    }

    rows.push(match &snap.mem {
        Some(mem) => Row {
            icon: Icon::Memory,
            title: i18n::t("panel.mem").to_string(),
            detail: format!("{:.1} / {:.1} GB", mem.used_mb as f64 / 1024.0, mem.total_mb as f64 / 1024.0),
            value: format!("{}%", mem.percent),
            tone: usage_tone(mem.percent),
        },
        None => missing_row(Icon::Memory, i18n::t("panel.mem")),
    });

    rows.push(match &snap.disk {
        Some(disk) => Row {
            icon: Icon::Disk,
            title: format!("{} ({}:)", i18n::t("panel.disk"), disk.letter),
            detail: format!(
                "{} {:.0} / {:.0} GB",
                i18n::t("panel.free"),
                disk.free_mb as f64 / 1024.0,
                disk.total_mb as f64 / 1024.0
            ),
            value: format!("{}%", disk.percent),
            tone: usage_tone(disk.percent),
        },
        None => missing_row(Icon::Disk, i18n::t("panel.disk")),
    });
    rows
}

fn missing_row(icon: Icon, title: &str) -> Row {
    Row { icon, title: title.to_string(), detail: String::new(), value: "—".into(), tone: Tone::Dim }
}

fn device_rows(readings: &[DeviceReading]) -> Vec<Row> {
    if readings.is_empty() {
        return vec![Row {
            icon: Icon::Bluetooth,
            title: i18n::t("panel.none").to_string(),
            detail: String::new(),
            value: String::new(),
            tone: Tone::Dim,
        }];
    }
    let info = machine::info();
    readings
        .iter()
        .take(MAX_DEVICES)
        .map(|r| {
            let (value, tone) = device_value(r.soc, r.charging, r.stale, r.error.as_deref());
            Row {
                icon: icon_of(r.glyph),
                title: i18n::device_label(r.glyph).to_string(),
                detail: match r.glyph {
                    Glyph::Laptop => {
                        let name = format!("{} {}", info.vendor, info.model).trim().to_string();
                        if name.is_empty() { r.name.clone() } else { name }
                    }
                    _ => compact_detail(r.brand.as_deref(), &r.name),
                },
                value,
                tone,
            }
        })
        .collect()
}

fn icon_of(glyph: Glyph) -> Icon {
    match glyph {
        Glyph::Mouse => Icon::Mouse,
        Glyph::Keyboard => Icon::Keyboard,
        Glyph::Headset => Icon::Headset,
        Glyph::Gamepad => Icon::Gamepad,
        Glyph::Laptop => Icon::Laptop,
        Glyph::Phone => Icon::Phone,
        Glyph::Unknown => Icon::Bluetooth,
    }
}

/// "85%" / "85% ⚡" / "待上报" / "—"，以及对应颜色语义。
pub fn device_value(soc: u8, charging: bool, stale: bool, error: Option<&str>) -> (String, Tone) {
    if let Some(text) = error {
        if text.contains("等待") {
            return (i18n::t("panel.waiting").to_string(), Tone::Dim);
        }
        return ("—".into(), Tone::Dim);
    }
    if stale {
        (format!("{}% · {}", soc, i18n::t("panel.stale")), Tone::Dim)
    } else if charging {
        (format!("{}% ⚡", soc), Tone::Amber)
    } else {
        (format!("{soc}%"), level_tone(soc))
    }
}

/// "23% · 78°C"：有占用有温度、只有其一、都没有。
pub fn metric(usage: Option<u8>, temp: Option<u8>) -> String {
    match (usage, temp) {
        (Some(u), Some(t)) => format!("{u}% · {t}°C"),
        (Some(u), None) => format!("{u}%"),
        (None, Some(t)) => format!("{t}°C"),
        (None, None) => "—".into(),
    }
}

pub fn metric_tone(usage: Option<u8>, temp: Option<u8>) -> Tone {
    if let Some(t) = temp {
        return temp_tone(t);
    }
    usage.map(usage_tone).unwrap_or(Tone::Dim)
}

pub fn compact_detail(brand: Option<&str>, product: &str) -> String {
    match brand {
        Some(b) if !b.is_empty() && !product.to_lowercase().contains(&b.to_lowercase()) => {
            format!("{b} · {product}")
        }
        _ => product.to_string(),
    }
}

pub fn level_tone(soc: u8) -> Tone {
    if soc <= 15 {
        Tone::Red
    } else if soc <= 40 {
        Tone::Amber
    } else {
        Tone::Green
    }
}

pub fn usage_tone(percent: u8) -> Tone {
    if percent >= 90 {
        Tone::Red
    } else if percent >= 70 {
        Tone::Amber
    } else {
        Tone::Green
    }
}

pub fn temp_tone(celsius: u8) -> Tone {
    if celsius >= 80 {
        Tone::Red
    } else if celsius >= 65 {
        Tone::Amber
    } else {
        Tone::Green
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_text_variants() {
        assert_eq!(metric(Some(23), Some(78)), "23% · 78°C");
        assert_eq!(metric(Some(23), None), "23%");
        assert_eq!(metric(None, Some(78)), "78°C");
        assert_eq!(metric(None, None), "—");
    }

    #[test]
    fn tones_follow_thresholds() {
        assert_eq!(level_tone(10), Tone::Red);
        assert_eq!(level_tone(30), Tone::Amber);
        assert_eq!(level_tone(80), Tone::Green);
        assert_eq!(usage_tone(95), Tone::Red);
        assert_eq!(usage_tone(75), Tone::Amber);
        assert_eq!(usage_tone(20), Tone::Green);
        assert_eq!(temp_tone(85), Tone::Red);
        assert_eq!(temp_tone(70), Tone::Amber);
        assert_eq!(temp_tone(40), Tone::Green);
        assert_eq!(metric_tone(None, None), Tone::Dim);
    }

    #[test]
    fn detail_does_not_repeat_brand() {
        assert_eq!(compact_detail(Some("Rapoo"), "Rapoo Gaming Device"), "Rapoo Gaming Device");
        assert_eq!(compact_detail(Some("Logitech"), "Wireless Mouse"), "Logitech · Wireless Mouse");
        assert_eq!(compact_detail(None, "MX Keys"), "MX Keys");
    }

    #[test]
    fn device_value_states() {
        assert_eq!(device_value(85, false, false, None).0, "85%");
        assert_eq!(device_value(85, true, false, None).1, Tone::Amber);
        assert_eq!(device_value(85, false, true, None).1, Tone::Dim);
        assert_eq!(device_value(0, false, false, Some("打开失败")).0, "—");
        assert_eq!(device_value(0, false, false, Some("等待设备上报（用一下设备）")).0, i18n::t("panel.waiting"));
    }

    #[test]
    fn model_is_info_only() {
        // 回归：面板只有“系统 / 设备”两段信息，没有第三段动作区。
        let model = build();
        assert_eq!(model.sections.len(), 2);
        assert_eq!(model.sections[0].title, i18n::t("panel.system"));
        assert_eq!(model.sections[1].title, i18n::t("panel.devices"));
        assert!(model.sections.iter().all(|s| s.rows.iter().all(|r| !r.title.is_empty())));
    }
}
