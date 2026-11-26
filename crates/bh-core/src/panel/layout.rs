//! 面板布局：把模型排成竖排的行，并给出物理像素坐标（1×，绘制时再 ×2 超采样）。
//!
//! 逻辑尺寸按 96 DPI 定义，乘以面板所在显示器的缩放比；命中测试与绘制共用同一份
//! `Layout`，避免“画在这、点在别处”。

use super::model::Model;

#[derive(Debug, Clone, Copy)]
pub struct RowBox {
    pub section: usize,
    pub row: usize,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub pad: i32,
    pub header_top: i32,
    pub header_h: i32,
    pub title_h: i32,
    pub footer_top: i32,
    pub footer_h: i32,
    pub radius: i32,
    pub icon: i32,
    pub header_icon: i32,
    pub gap: i32,
    pub brand_px: i32,
    pub model_px: i32,
    pub section_px: i32,
    pub title_px: i32,
    pub detail_px: i32,
    pub value_px: i32,
    pub footer_px: i32,
    pub value_w: i32,
    pub rows: Vec<RowBox>,
    pub section_tops: Vec<i32>,
}

/// 逻辑尺寸（96 DPI）。
const WIDTH: f32 = 304.0;
const PAD: f32 = 12.0;
const HEADER_H: f32 = 50.0;
const TITLE_H: f32 = 24.0;
const ROW_H: f32 = 42.0;
const GAP: f32 = 8.0;
const FOOTER_H: f32 = 18.0;
const RADIUS: f32 = 10.0;
const ICON: f32 = 20.0;
const HEADER_ICON: f32 = 26.0;
const VALUE_W: f32 = 92.0;

pub fn compute(model: &Model, scale: f32) -> Layout {
    let scale = scale.clamp(0.75, 4.0);
    let px = |v: f32| (v * scale).round().max(1.0) as i32;
    let pad = px(PAD);
    let header_h = px(HEADER_H);
    let title_h = px(TITLE_H);
    let row_h = px(ROW_H);
    let gap = px(GAP);
    let footer_h = px(FOOTER_H);

    let mut rows = Vec::new();
    let mut section_tops = Vec::new();
    let mut y = pad;
    let header_top = y;
    y += header_h;

    for (si, section) in model.sections.iter().enumerate() {
        section_tops.push(y);
        if !section.title.is_empty() {
            y += title_h;
        }
        for (ri, _) in section.rows.iter().enumerate() {
            rows.push(RowBox { section: si, row: ri, top: y, bottom: y + row_h });
            y += row_h;
        }
        y += gap;
    }

    let footer_top = y + px(2.0);
    y = footer_top + footer_h;
    Layout {
        width: px(WIDTH),
        height: y + pad,
        pad,
        header_top,
        header_h,
        title_h,
        footer_top,
        footer_h,
        radius: px(RADIUS),
        icon: px(ICON),
        header_icon: px(HEADER_ICON),
        gap,
        brand_px: px(10.0),
        model_px: px(14.5),
        section_px: px(9.5),
        title_px: px(12.0),
        detail_px: px(10.0),
        value_px: px(12.0),
        footer_px: px(9.0),
        value_w: px(VALUE_W),
        rows,
        section_tops,
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Icon, Row, Section, Tone};
    use super::*;

    fn sample() -> Model {
        let row = |title: &str| Row {
            icon: Icon::Cpu,
            title: title.into(),
            detail: String::new(),
            value: "1%".into(),
            tone: Tone::Dim,
        };
        Model {
            brand: "ASUS".into(),
            model: "Zenbook".into(),
            sections: vec![
                Section { title: "系统", rows: vec![row("CPU"), row("GPU")] },
                Section { title: "设备", rows: vec![row("鼠标")] },
            ],
            updated: "10:00:00".into(),
        }
    }

    #[test]
    fn layout_stacks_every_row_above_the_footer() {
        let layout = compute(&sample(), 1.5);
        assert_eq!(layout.rows.len(), 3);
        assert!(layout.rows[1].top >= layout.rows[0].bottom);
        assert!(layout.rows[2].section == 1 && layout.rows[2].row == 0);
        assert!(layout.footer_top > layout.rows[2].bottom);
        assert!(layout.height > layout.footer_top + layout.footer_h);
        assert_eq!(layout.width, (304.0 * 1.5f32).round() as i32);
    }

    #[test]
    fn taller_scale_gives_taller_panel() {
        let small = compute(&sample(), 1.0);
        let big = compute(&sample(), 2.0);
        assert!(big.height > small.height);
    }
}
