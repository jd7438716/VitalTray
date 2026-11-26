//! 面板图标：全部用 GDI 线段/圆/多边形画，不依赖图标字体或外部资源。
//!
//! 坐标用 0..1 的比例表达，按实际像素尺寸缩放；上层在 2× 画布上绘制后缩半，
//! 边缘自然带抗锯齿。

use super::gdi::{round_stroke, stroke_arc, stroke_ellipse, stroke_line, stroke_rect, H};
use super::model::Icon;

pub unsafe fn draw(dc: H, icon: Icon, x: i32, y: i32, size: i32, color: u32) {
    if size <= 2 {
        return;
    }
    let u = size as f32;
    let px = |v: f32| x + (u * v).round() as i32;
    let py = |v: f32| y + (u * v).round() as i32;
    let len = |v: f32| (u * v).round().max(1.0) as i32;
    let w = (size / 13).max(2);
    let line = |x1: f32, y1: f32, x2: f32, y2: f32| unsafe {
        stroke_line(dc, px(x1), py(y1), px(x2), py(y2), color, w);
    };
    let ellipse = |x1: f32, y1: f32, ww: f32, hh: f32| unsafe {
        stroke_ellipse(dc, px(x1), py(y1), len(ww), len(hh), color, w);
    };

    match icon {
        Icon::Cpu => {
            unsafe { round_stroke(dc, px(0.14), py(0.14), len(0.72), len(0.72), len(0.12), color, w) };
            unsafe { stroke_rect(dc, px(0.36), py(0.36), len(0.28), len(0.28), color, w) };
            for i in 0..3 {
                let t = 0.30 + i as f32 * 0.20;
                line(0.03, t, 0.14, t);
                line(0.86, t, 0.97, t);
                line(t, 0.03, t, 0.14);
                line(t, 0.86, t, 0.97);
            }
        }
        Icon::Gpu => {
            unsafe { round_stroke(dc, px(0.06), py(0.20), len(0.88), len(0.48), len(0.10), color, w) };
            ellipse(0.62, 0.30, 0.26, 0.26);
            line(0.06, 0.78, 0.06, 0.90);
            line(0.34, 0.78, 0.34, 0.90);
            line(0.62, 0.78, 0.62, 0.90);
            line(0.18, 0.90, 0.82, 0.90);
        }
        Icon::Memory => {
            unsafe { round_stroke(dc, px(0.10), py(0.30), len(0.80), len(0.40), len(0.06), color, w) };
            for i in 0..5 {
                let t = 0.18 + i as f32 * 0.16;
                line(t, 0.70, t, 0.84);
            }
            line(0.46, 0.70, 0.54, 0.70);
        }
        Icon::Disk => {
            unsafe { round_stroke(dc, px(0.10), py(0.22), len(0.80), len(0.56), len(0.10), color, w) };
            line(0.10, 0.62, 0.90, 0.62);
            ellipse(0.68, 0.70, 0.12, 0.12);
        }
        Icon::Mouse => {
            unsafe { round_stroke(dc, px(0.28), py(0.10), len(0.44), len(0.80), len(0.22), color, w) };
            line(0.28, 0.46, 0.72, 0.46);
            line(0.50, 0.14, 0.50, 0.46);
        }
        Icon::Keyboard => {
            unsafe { round_stroke(dc, px(0.06), py(0.30), len(0.88), len(0.40), len(0.08), color, w) };
            for row in 0..2 {
                let t = 0.44 + row as f32 * 0.14;
                for col in 0..4 {
                    let c = 0.16 + col as f32 * 0.20;
                    line(c, t, c + 0.10, t);
                }
            }
            line(0.32, 0.66, 0.68, 0.66);
        }
        Icon::Laptop => {
            unsafe { round_stroke(dc, px(0.18), py(0.12), len(0.64), len(0.48), len(0.06), color, w) };
            line(0.08, 0.76, 0.92, 0.76);
            line(0.16, 0.84, 0.84, 0.84);
            line(0.08, 0.76, 0.16, 0.84);
            line(0.92, 0.76, 0.84, 0.84);
        }
        Icon::Headset => {
            unsafe { stroke_arc(dc, x as f32 + u * 0.5, y as f32 + u * 0.52, u * 0.34, 180.0, 360.0, color, w) };
            unsafe { round_stroke(dc, px(0.10), py(0.46), len(0.18), len(0.34), len(0.06), color, w) };
            unsafe { round_stroke(dc, px(0.72), py(0.46), len(0.18), len(0.34), len(0.06), color, w) };
        }
        Icon::Gamepad => {
            unsafe { round_stroke(dc, px(0.04), py(0.30), len(0.92), len(0.44), len(0.22), color, w) };
            ellipse(0.18, 0.40, 0.18, 0.24);
            ellipse(0.62, 0.40, 0.18, 0.24);
            line(0.50, 0.40, 0.50, 0.64);
        }
        Icon::Phone => {
            unsafe { round_stroke(dc, px(0.30), py(0.08), len(0.40), len(0.84), len(0.10), color, w) };
            line(0.44, 0.16, 0.56, 0.16);
            line(0.44, 0.82, 0.56, 0.82);
        }
        Icon::Bluetooth => {
            line(0.50, 0.08, 0.50, 0.92);
            line(0.50, 0.08, 0.74, 0.30);
            line(0.74, 0.30, 0.26, 0.70);
            line(0.50, 0.92, 0.74, 0.70);
            line(0.74, 0.70, 0.26, 0.30);
        }
    }
}
