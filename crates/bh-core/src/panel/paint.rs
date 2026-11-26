//! 面板绘制：2× 超采样 + 圆角背景 + 图标 + 两行文字 + 右对齐数值。
//!
//! 透明策略与任务栏 widget 相同：颜色画在黑底上（边缘自然成为预乘 alpha），
//! 形状遮罩单独画一张，最后取遮罩亮度作为 alpha 通道交给 UpdateLayeredWindow。

use super::gdi::{
    downscale, draw_text, fill, font, make_dib, release_screen_dc, round_fill, round_stroke,
    screen_dc, stroke_line, H,
};
use super::icons;
use super::layout::Layout;
use super::model::{Icon, Model, Tone};
use super::theme::{palette, Palette};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, BLENDFUNCTION, HGDIOBJ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};

const SS: i32 = 2;
const DT_LEFT: u32 = 0;
const DT_RIGHT: u32 = 2;
const DT_VCENTER: u32 = 4;
const DT_SINGLELINE: u32 = 0x20;
const DT_END_ELLIPSIS: u32 = 0x8000;
const DT_NOPREFIX: u32 = 0x800;

/// 渲染面板到窗口（同时设置位置与大小）。面板只读，没有悬停态。
pub fn render(hwnd: HWND, x: i32, y: i32, layout: &Layout, model: &Model) -> bool {
    let pal = palette(crate::theme::is_dark());
    let bw = layout.width * SS;
    let bh = layout.height * SS;
    if bw <= 2 || bh <= 2 {
        return false;
    }
    let b = |v: i32| v * SS;

    unsafe {
        let screen = screen_dc();
        let cdc = CreateCompatibleDC(screen);
        let mdc = CreateCompatibleDC(screen);
        let (cdib, _) = make_dib(cdc, bw, bh);
        let (mdib, _) = make_dib(mdc, bw, bh);
        let oc = windows_sys::Win32::Graphics::Gdi::SelectObject(cdc, cdib as HGDIOBJ);
        let om = windows_sys::Win32::Graphics::Gdi::SelectObject(mdc, mdib as HGDIOBJ);
        fill(cdc, 0, 0, bw, bh, 0);
        fill(mdc, 0, 0, bw, bh, 0);
        round_fill(cdc, 0, 0, bw, bh, b(layout.radius), pal.bg);
        round_stroke(cdc, b(1), b(1), bw - b(2), bh - b(2), b(layout.radius), pal.border, SS);
        round_fill(mdc, 0, 0, bw, bh, b(layout.radius), 0x00FF_FFFF);

        let fonts = Fonts::new(layout);
        draw_header(cdc, layout, model, &pal, &fonts, &b);
        draw_sections(cdc, layout, model, &pal, &fonts, &b);
        draw_rows(cdc, layout, model, &pal, &fonts, &b);
        draw_footer(cdc, layout, model, &pal, &fonts, &b);
        fonts.drop();

        let (scdib, cbits) = downscale(cdc, bw, bh, layout.width, layout.height);
        let (smdib, mbits) = downscale(mdc, bw, bh, layout.width, layout.height);
        windows_sys::Win32::Graphics::Gdi::SelectObject(cdc, oc);
        windows_sys::Win32::Graphics::Gdi::SelectObject(mdc, om);

        if !cbits.is_null() && !mbits.is_null() {
            let n = (layout.width * layout.height * 4) as usize;
            let cpx = std::slice::from_raw_parts_mut(cbits as *mut u8, n);
            let mpx = std::slice::from_raw_parts(mbits as *const u8, n);
            for i in (0..n).step_by(4) {
                let a = mpx[i].max(mpx[i + 1]).max(mpx[i + 2]);
                cpx[i + 3] = a;
            }
        }

        let mdc2 = CreateCompatibleDC(screen);
        let old2 = windows_sys::Win32::Graphics::Gdi::SelectObject(mdc2, scdib as HGDIOBJ);
        let dst = POINT { x, y };
        let src = POINT { x: 0, y: 0 };
        let size = SIZE { cx: layout.width, cy: layout.height };
        let blend = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 };
        let ok = UpdateLayeredWindow(hwnd, screen, &dst, &size, mdc2, &src, 0, &blend, ULW_ALPHA);
        windows_sys::Win32::Graphics::Gdi::SelectObject(mdc2, old2);

        DeleteObject(cdib as HGDIOBJ);
        DeleteObject(mdib as HGDIOBJ);
        DeleteObject(scdib as HGDIOBJ);
        DeleteObject(smdib as HGDIOBJ);
        DeleteDC(cdc);
        DeleteDC(mdc);
        DeleteDC(mdc2);
        release_screen_dc(screen);
        ok != 0
    }
}

struct Fonts {
    brand: H,
    model: H,
    section: H,
    title: H,
    detail: H,
    value: H,
    footer: H,
}

impl Fonts {
    fn new(l: &Layout) -> Self {
        Self {
            brand: font(l.brand_px * SS, false),
            model: font(l.model_px * SS, true),
            section: font(l.section_px * SS, false),
            title: font(l.title_px * SS, true),
            detail: font(l.detail_px * SS, false),
            value: font(l.value_px * SS, true),
            footer: font(l.footer_px * SS, false),
        }
    }
    fn drop(&self) {
        for f in [self.brand, self.model, self.section, self.title, self.detail, self.value, self.footer] {
            unsafe { DeleteObject(f as HGDIOBJ) };
        }
    }
}

fn tone_color(pal: &Palette, tone: Tone) -> u32 {
    match tone {
        Tone::Green => pal.green,
        Tone::Amber => pal.amber,
        Tone::Red => pal.red,
        Tone::Neutral => pal.text,
        Tone::Accent => pal.accent,
        Tone::Dim => pal.stale,
    }
}

fn draw_header(dc: H, l: &Layout, m: &Model, pal: &Palette, f: &Fonts, b: &dyn Fn(i32) -> i32) {
    let top = b(l.header_top);
    unsafe {
        icons::draw(
            dc,
            Icon::Laptop,
            b(l.pad),
            top + (b(l.header_h) - b(l.header_icon)) / 2,
            b(l.header_icon),
            pal.muted,
        );
        let left = b(l.pad + l.header_icon + 10);
        let right = b(l.width - l.pad);
        let mut brand = RECT { left, top: top + b(3), right, bottom: top + b(3 + l.brand_px + 2) };
        draw_text(dc, &m.brand, &mut brand, f.brand, pal.muted, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
        let mut name = RECT { left, top: brand.bottom, right, bottom: top + b(l.header_h - 4) };
        draw_text(dc, &m.model, &mut name, f.model, pal.text, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
    }
}

fn draw_sections(dc: H, l: &Layout, m: &Model, pal: &Palette, f: &Fonts, b: &dyn Fn(i32) -> i32) {
    for (si, section) in m.sections.iter().enumerate() {
        let top = b(l.section_tops[si]);
        if si > 0 {
            let y = top - b(l.gap / 2).max(1);
            unsafe { stroke_line(dc, b(l.pad), y, b(l.width - l.pad), y, pal.separator, SS) };
        }
        if section.title.is_empty() {
            continue;
        }
        let mut rc = RECT { left: b(l.pad), top, right: b(l.width - l.pad), bottom: top + b(l.title_h) };
        unsafe {
            draw_text(dc, section.title, &mut rc, f.section, pal.subdued, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }
}

fn draw_rows(dc: H, l: &Layout, m: &Model, pal: &Palette, f: &Fonts, b: &dyn Fn(i32) -> i32) {
    for r in &l.rows {
        let row = &m.sections[r.section].rows[r.row];
        let top = b(r.top);
        let bottom = b(r.bottom);
        let h = bottom - top;
        unsafe {
            icons::draw(dc, row.icon, b(l.pad + 1), top + (h - b(l.icon)) / 2, b(l.icon), pal.muted);
        }
        let left = b(l.pad + l.icon + 10);
        let right = b(l.width - l.pad);
        let split = top + h * 55 / 100;
        if !row.detail.is_empty() {
            let mut title = RECT { left, top: top + b(2), right: right - b(l.value_w), bottom: split };
            let mut detail = RECT { left, top: split - b(1), right: right - b(l.value_w), bottom: bottom - b(2) };
            unsafe {
                draw_text(dc, &row.title, &mut title, f.title, pal.text, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
                draw_text(dc, &row.detail, &mut detail, f.detail, pal.muted, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            }
        } else {
            let mut title = RECT { left, top, right: right - b(l.value_w), bottom };
            unsafe { draw_text(dc, &row.title, &mut title, f.title, pal.text, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX) };
        }
        if !row.value.is_empty() {
            let mut val = RECT { left: right - b(l.value_w), top, right, bottom };
            unsafe {
                draw_text(dc, &row.value, &mut val, f.value, tone_color(pal, row.tone), DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            }
        }
    }
}

fn draw_footer(dc: H, l: &Layout, m: &Model, pal: &Palette, f: &Fonts, b: &dyn Fn(i32) -> i32) {
    let top = b(l.footer_top);
    let y = top - b(3);
    unsafe {
        stroke_line(dc, b(l.pad), y, b(l.width - l.pad), y, pal.separator, SS);
        let mut left = RECT { left: b(l.pad), top, right: b(l.width / 2), bottom: top + b(l.footer_h) };
        draw_text(dc, &m.updated, &mut left, f.footer, pal.subdued, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        let mut right = RECT { left: b(l.width / 2), top, right: b(l.width - l.pad), bottom: top + b(l.footer_h) };
        draw_text(dc, &format!("BatteryHub v{}", crate::VERSION), &mut right, f.footer, pal.subdued, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }
}
