//! 面板内容模型：把“设备电量 + 系统状态”组织成竖排的行。
//!
//! 面板是只读的：这里只有看得懂的信息（文字 + 语义色），没有命令、没有可点的行；
//! 所有动作都在托盘右键菜单里（`commands`）。绘制在 paint.rs，配色在 theme.rs。

use crate::i18n;
use crate::machine;
use crate::poller::DeviceReading;
use crate::state;
use crate::status;
use crate::sysmon::{self, Snapshot};
use crate::util;
use bh_api::Glyph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Cpu,
    Gpu,
    Disk,
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
    Laptop,
    Phone,
    Bluetooth,
}

/// 数值语义色。电池电量只有三档：充足绿、正常中性、低电红；充电中单独一档蓝；
/// 上次/无数据用灰。系统温度继续用面板自己的绿/琥珀/红。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Amber,
    Red,
    Neutral,
    Accent,
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
    let (readings, low) = state::with(|st| match st {
        Some(s) => (s.readings.clone(), s.cfg.low_battery),
        None => (Vec::new(), status::DEFAULT_LOW),
    });
    let snap = sysmon::latest();
    let info = machine::info();
    let brand = if info.vendor.is_empty() { "VitalTray".to_string() } else { info.vendor.clone() };
    let model = if info.model.is_empty() { i18n::t("panel.status").to_string() } else { info.model.clone() };
    Model {
        brand,
        model,
        sections: vec![
            Section { title: i18n::t("panel.system"), rows: system_rows(&snap, info.cpu.as_str()) },
            Section { title: i18n::t("panel.devices"), rows: device_rows(&readings, low) },
        ],
        updated: i18n::tf("panel.updated", &[("time", &util::clock())]),
    }
}

fn system_rows(snap: &Snapshot, cpu_name: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    rows.push(Row {
        icon: Icon::Cpu,
        title: i18n::t("panel.cpu").to_string(),
        detail: cpu_name.to_string(),
        value: temp_text(snap.cpu_temp),
        tone: temp_tone_of(snap.cpu_temp),
    });

    if snap.gpus.is_empty() {
        rows.push(Row {
            icon: Icon::Gpu,
            title: i18n::t("panel.gpu").to_string(),
            detail: String::new(),
            value: temp_text(None),
            tone: Tone::Dim,
        });
    } else {
        for gpu in snap.gpus.iter().take(2) {
            rows.push(Row {
                icon: Icon::Gpu,
                title: i18n::t("panel.gpu").to_string(),
                detail: gpu.name.clone(),
                value: temp_text(gpu.temp_c),
                tone: temp_tone_of(gpu.temp_c),
            });
        }
    }

    rows.push(match &snap.disk {
        Some(disk) => Row {
            icon: Icon::Disk,
            title: format!("{} ({}:)", i18n::t("panel.disk"), disk.letter),
            detail: i18n::tf(
                "panel.disk_detail",
                &[
                    ("free", &format!("{:.0}", disk.free_mb as f64 / 1024.0)),
                    ("total", &format!("{:.0}", disk.total_mb as f64 / 1024.0)),
                ],
            ),
            value: temp_text(disk.temp_c),
            tone: temp_tone_of(disk.temp_c),
        },
        None => missing_row(Icon::Disk, i18n::t("panel.disk")),
    });
    rows
}

fn missing_row(icon: Icon, title: &str) -> Row {
    Row { icon, title: title.to_string(), detail: String::new(), value: "—".into(), tone: Tone::Dim }
}

fn device_rows(readings: &[DeviceReading], low: u8) -> Vec<Row> {
    // 幽灵行抑制：接收器常驻但从未上报过的通道不占面板。
    let live: Vec<&DeviceReading> = readings.iter().filter(|r| !r.ghost).collect();
    if live.is_empty() {
        return vec![Row {
            icon: Icon::Bluetooth,
            title: i18n::t("panel.none").to_string(),
            detail: String::new(),
            value: String::new(),
            tone: Tone::Dim,
        }];
    }
    let info = machine::info();
    live.into_iter()
        .take(MAX_DEVICES)
        .map(|r| {
            let (value, tone) = device_value(r.soc, r.charging, r.stale, r.pending, r.error.as_deref(), low);
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
///
/// 电量色与托盘同口径：<= 低电阈值红、> 80% 绿、充电中蓝、正常中性；
/// 上次读数 / 读不到 / 待上报用灰。
pub fn device_value(
    soc: u8,
    charging: bool,
    stale: bool,
    pending: bool,
    error: Option<&str>,
    low: u8,
) -> (String, Tone) {
    if pending {
        return (i18n::t("panel.waiting").to_string(), Tone::Dim);
    }
    if error.is_some() {
        return ("—".into(), Tone::Dim);
    }
    if stale {
        return (format!("{}% · {}", soc, i18n::t("panel.stale")), Tone::Dim);
    }
    let tone = tone_of(status::state_of(soc, charging, false, low));
    let value = if charging { format!("{soc}% ⚡") } else { format!("{soc}%") };
    (value, tone)
}

/// 状态 → 面板色，和托盘图标共用 `status` 的判定。
pub fn tone_of(state: status::State) -> Tone {
    match state {
        status::State::Good => Tone::Green,
        status::State::Normal => Tone::Neutral,
        status::State::Low => Tone::Red,
        status::State::Charging => Tone::Accent,
        status::State::Stale | status::State::Unknown => Tone::Dim,
    }
}

/// "78°C"；温度拿不到就是 "—"（占用百分比不在这里，任务管理器负责那个）。
pub fn temp_text(celsius: Option<u8>) -> String {
    celsius.map(|t| format!("{t}°C")).unwrap_or_else(|| "—".into())
}

/// 温度决定颜色；没有温度就是灰的。
pub fn temp_tone_of(celsius: Option<u8>) -> Tone {
    celsius.map(temp_tone).unwrap_or(Tone::Dim)
}

pub fn compact_detail(brand: Option<&str>, product: &str) -> String {
    match brand {
        Some(b) if !b.is_empty() && !product.to_lowercase().contains(&b.to_lowercase()) => {
            format!("{b} · {product}")
        }
        _ => product.to_string(),
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
    fn temperature_text_variants() {
        assert_eq!(temp_text(Some(78)), "78°C");
        assert_eq!(temp_text(None), "—");
    }

    #[test]
    fn tones_follow_thresholds() {
        assert_eq!(tone_of(status::State::Low), Tone::Red);
        assert_eq!(tone_of(status::State::Good), Tone::Green);
        assert_eq!(tone_of(status::State::Normal), Tone::Neutral);
        assert_eq!(tone_of(status::State::Charging), Tone::Accent);
        assert_eq!(tone_of(status::State::Stale), Tone::Dim);
        assert_eq!(temp_tone(85), Tone::Red);
        assert_eq!(temp_tone(70), Tone::Amber);
        assert_eq!(temp_tone(40), Tone::Green);
        assert_eq!(temp_tone_of(None), Tone::Dim);
    }

    #[test]
    fn detail_does_not_repeat_brand() {
        assert_eq!(compact_detail(Some("Rapoo"), "Rapoo Gaming Device"), "Rapoo Gaming Device");
        assert_eq!(compact_detail(Some("Logitech"), "Wireless Mouse"), "Logitech · Wireless Mouse");
        assert_eq!(compact_detail(None, "MX Keys"), "MX Keys");
    }

    #[test]
    fn device_value_states() {
        // 85%：> 80，绿；充电中仍是绿（电量优先），但值里带 ⚡
        assert_eq!(device_value(85, false, false, false, None, 20).0, "85%");
        assert_eq!(device_value(85, false, false, false, None, 20).1, Tone::Green);
        assert_eq!(device_value(85, true, false, false, None, 20).1, Tone::Green);
        // 55% 充电中：中间档才显示蓝
        assert_eq!(device_value(55, true, false, false, None, 20).1, Tone::Accent);
        assert_eq!(device_value(55, false, false, false, None, 20).1, Tone::Neutral);
        // 20% 及以下：红（充电中也红，直到充过阈值）
        assert_eq!(device_value(20, false, false, false, None, 20).1, Tone::Red);
        assert_eq!(device_value(15, true, false, false, None, 20).1, Tone::Red);
        assert_eq!(device_value(85, false, true, false, None, 20).1, Tone::Dim);
        assert_eq!(device_value(0, false, false, false, Some("open failed"), 20).0, "—");
        assert_eq!(device_value(0, false, false, true, None, 20).0, i18n::t("panel.waiting"));
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

    fn reading(soc: u8, ghost: bool) -> DeviceReading {
        DeviceReading {
            key: format!("k{soc}{ghost}"),
            name: "Rapoo Gaming Device".into(),
            domain: "mouse".into(),
            glyph: Glyph::Mouse,
            brand: Some("Rapoo".into()),
            soc,
            charging: false,
            error: None,
            pending: false,
            stale: false,
            ghost,
        }
    }

    #[test]
    fn ghost_channels_are_hidden_from_panel() {
        // 接收器常驻但从未上报过的通道不占面板行（只有真读到过的才显示）。
        let rows = device_rows(&[reading(0, true), reading(85, false)], 20);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, "85%");
        // 全是幽灵时退回“没有可读设备”提示，而不是空列表。
        let only_ghost = device_rows(&[reading(0, true)], 20);
        assert_eq!(only_ghost.len(), 1);
        assert_eq!(only_ghost[0].title, i18n::t("panel.none"));
    }
}
