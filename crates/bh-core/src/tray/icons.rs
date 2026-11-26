//! 托盘图标：4× 超采样 + “遮罩/颜色”双通道合成，边缘平滑且适配明暗任务栏。
//!
//! 颜色由 `status` 决定（充足绿 / 正常中性 / 低电红 / 充电蓝 / 上次灰 / 无数据灰），
//! 样式（电池 / 数字 / 圆环）只决定图形。无数据时电池画斜线、数字/圆环画“—”，
//! 不再和“0%”混在一起。

use crate::status::{self, Aggregate, State};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    Arc, BitBlt, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, Ellipse, FillRect, GetDC, LineTo, MoveToEx, ReleaseDC,
    RoundRect, SelectObject, SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLACKNESS, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE,
    DT_VCENTER, FW_BOLD, HALFTONE, HGDIOBJ, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetSystemMetrics, ICONINFO, SM_CXSMICON, SM_CYSMICON,
};

type H = *mut core::ffi::c_void;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Battery,
    Number,
    Ring,
}

impl Style {
    pub fn parse(s: &str) -> Style {
        match s.trim().to_ascii_lowercase().as_str() {
            "number" => Style::Number,
            "ring" => Style::Ring,
            _ => Style::Battery,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Battery => "battery",
            Style::Number => "number",
            Style::Ring => "ring",
        }
    }

    pub fn next(self) -> Style {
        match self {
            Style::Battery => Style::Number,
            Style::Number => Style::Ring,
            Style::Ring => Style::Battery,
        }
    }
}

/// 与状态无关的三种中性色：外壳、内底、轨道。
struct Colors {
    shell: u32,
    bg: u32,
    track: u32,
}

fn colors(dark: bool) -> Colors {
    if dark {
        Colors { shell: status::rgb(154, 160, 168), bg: status::rgb(42, 44, 48), track: status::rgb(58, 61, 66) }
    } else {
        Colors { shell: status::rgb(104, 112, 120), bg: status::rgb(244, 244, 244), track: status::rgb(200, 200, 200) }
    }
}

unsafe fn fill_rect(dc: H, x: i32, y: i32, w: i32, h: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = CreateSolidBrush(color);
    let r = RECT { left: x, top: y, right: x + w, bottom: y + h };
    FillRect(dc, &r, brush);
    DeleteObject(brush as HGDIOBJ);
}

unsafe fn round_rect(dc: H, x: i32, y: i32, w: i32, h: i32, radius: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = CreateSolidBrush(color);
    let pen = CreatePen(0, 0, color);
    let old_b = SelectObject(dc, brush as HGDIOBJ);
    let old_p = SelectObject(dc, pen as HGDIOBJ);
    RoundRect(dc, x, y, x + w, y + h, radius, radius);
    SelectObject(dc, old_b);
    SelectObject(dc, old_p);
    DeleteObject(brush as HGDIOBJ);
    DeleteObject(pen as HGDIOBJ);
}

/// 一条直线（无数据状态的斜线用）。
unsafe fn line(dc: H, x1: i32, y1: i32, x2: i32, y2: i32, color: u32, width: i32) {
    let pen = CreatePen(0, width.max(1), color);
    let old = SelectObject(dc, pen as HGDIOBJ);
    MoveToEx(dc, x1, y1, std::ptr::null_mut());
    LineTo(dc, x2, y2);
    SelectObject(dc, old);
    DeleteObject(pen as HGDIOBJ);
}

fn make_dib(dc: H, w: i32, h: i32) -> (H, H) {
    unsafe {
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits: H = std::ptr::null_mut();
        let dib = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        (dib, bits)
    }
}

/// 图标中央的一行字（数字或“—”）。
fn draw_label(dc: H, size: i32, text: &str, color: u32) {
    unsafe {
        let font = CreateFontW(
            -(size * 58 / 100),
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            windows_sys::core::w!("Segoe UI"),
        );
        let old = SelectObject(dc, font as HGDIOBJ);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, color);
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: size, bottom: size };
        DrawTextW(dc, units.as_ptr(), units.len() as i32, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(dc, old);
        DeleteObject(font as HGDIOBJ);
    }
}

/// mono = true 时所有图元画成白色，只用于生成 alpha 遮罩。
unsafe fn draw(dc: H, size: i32, agg: &Aggregate, style: Style, dark: bool, mono: bool) {
    let c = colors(dark);
    let paint = |col: u32| if mono { 0x00FF_FFFF } else { col };
    // 外壳：无数据变灰；低电整壳变红——低电时填充只剩一两个像素，
    // 只靠填充会看不见，必须让轮廓也带上警示色。
    let shell = paint(match agg.state {
        State::Unknown => status::state_color(State::Unknown, dark),
        State::Low => status::state_color(State::Low, dark),
        _ => c.shell,
    });
    let bg = paint(c.bg);
    let track = paint(c.track);
    let accent = paint(status::state_color(agg.state, dark));
    let percent = agg.percent.unwrap_or(0) as i32;

    match style {
        Style::Battery => {
            // 竖向电池，与应用图标（`assets/vitaltray.ico`）同一套比例：
            // 壳宽 46%、高 62%、壳厚 5.5%、触点宽 22%。电量从底部往上填。
            let body_x = size * 27 / 100;
            let body_y = size * 23 / 100;
            let body_w = size * 46 / 100;
            let body_h = size * 62 / 100;
            round_rect(dc, body_x, body_y, body_w, body_h, size / 10, shell);
            let pad = size * 11 / 200;
            let inner_w = body_w - pad * 2;
            let inner_h = body_h - pad * 2;
            round_rect(dc, body_x + pad, body_y + pad, inner_w, inner_h, size / 18, bg);
            match agg.percent {
                Some(_) => {
                    let filled = (inner_h * percent.min(100) / 100).max(1);
                    round_rect(dc, body_x + pad, body_y + pad + inner_h - filled, inner_w, filled, size / 18, accent);
                }
                None => {
                    // 空心 + 斜线 = “没有数据”，跟 0% 的满格内底区分开。
                    line(dc, body_x + pad, body_y + pad + inner_h, body_x + pad + inner_w, body_y + pad, shell, (size / 14).max(2));
                }
            }
            let nub_w = size * 22 / 100;
            round_rect(dc, (size - nub_w) / 2, body_y - size * 7 / 100, nub_w, size * 7 / 100, size / 28, shell);
        }
        Style::Number => {
            round_rect(dc, size * 3 / 100, size * 3 / 100, size * 94 / 100, size * 94 / 100, size / 6, bg);
            let bar_h = (size * 20 / 100).max(3);
            fill_rect(dc, size * 3 / 100, size - bar_h - size * 3 / 100, size * 94 / 100, bar_h, track);
            if agg.percent.is_some() {
                let fill = (size * 94 / 100) * percent.min(100) / 100;
                fill_rect(dc, size * 3 / 100, size - bar_h - size * 3 / 100, fill, bar_h, accent);
            }
            let label = agg.percent.map(|p| p.to_string()).unwrap_or_else(|| "—".to_string());
            draw_label(dc, size, &label, accent);
        }
        Style::Ring => {
            let pad = size * 8 / 100;
            let track_pen = CreatePen(0, (size / 8).max(2), track);
            let old_p = SelectObject(dc, track_pen as HGDIOBJ);
            let brush = CreateSolidBrush(bg);
            let old_b = SelectObject(dc, brush as HGDIOBJ);
            Ellipse(dc, pad, pad, size - pad, size - pad);
            SelectObject(dc, old_b);
            SelectObject(dc, old_p);
            DeleteObject(brush as HGDIOBJ);
            DeleteObject(track_pen as HGDIOBJ);

            if agg.percent.is_some() {
                let r = (size - pad * 2) as f64 / 2.0;
                let cx = size as f64 / 2.0;
                let cy = size as f64 / 2.0;
                let sweep = 3.6 * percent.min(100) as f64;
                let end = (-90.0 + sweep).to_radians();
                let arc_pen = CreatePen(0, (size / 8).max(2), accent);
                let old_arc = SelectObject(dc, arc_pen as HGDIOBJ);
                Arc(
                    dc,
                    pad,
                    pad,
                    size - pad,
                    size - pad,
                    cx as i32,
                    (cy - r) as i32,
                    (cx + r * end.cos()) as i32,
                    (cy + r * end.sin()) as i32,
                );
                SelectObject(dc, old_arc);
                DeleteObject(arc_pen as HGDIOBJ);
            }
            let label = agg.percent.map(|p| p.to_string()).unwrap_or_else(|| "—".to_string());
            draw_label(dc, size, &label, accent);
        }
    }
}

/// src 必须是“已选好位图”的内存 DC。
fn downscale(src_dc: H, big: i32, target: i32) -> (H, H) {
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let (dib, bits) = make_dib(dc, target, target);
        let old = SelectObject(dc, dib as HGDIOBJ);
        SetStretchBltMode(dc, HALFTONE as i32);
        StretchBlt(dc, 0, 0, target, target, src_dc, 0, 0, big, big, SRCCOPY);
        SelectObject(dc, old);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        (dib, bits)
    }
}

/// 生成托盘图标；颜色跟随任务栏明暗。调用方负责 DestroyIcon。
pub fn render(agg: &Aggregate, style: Style) -> H {
    render_with(agg, style, crate::theme::taskbar_is_dark())
}

/// 指定明暗主题的版本（预览/测试用）。
pub fn render_with(agg: &Aggregate, style: Style, dark: bool) -> H {
    let target = unsafe { GetSystemMetrics(SM_CXSMICON).max(16) };
    let _ = unsafe { GetSystemMetrics(SM_CYSMICON) };
    let ss = 4;
    let big = target * ss;

    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let color_dc = CreateCompatibleDC(screen);
        let mask_dc = CreateCompatibleDC(screen);

        let (color_dib, _) = make_dib(color_dc, big, big);
        let old_c = SelectObject(color_dc, color_dib as HGDIOBJ);
        fill_rect(color_dc, 0, 0, big, big, 0x0000_0000);
        draw(color_dc, big, agg, style, dark, false);

        let (mask_dib, _) = make_dib(mask_dc, big, big);
        let old_m = SelectObject(mask_dc, mask_dib as HGDIOBJ);
        fill_rect(mask_dc, 0, 0, big, big, 0x0000_0000);
        draw(mask_dc, big, agg, style, dark, true);

        let (small_color, color_bits) = downscale(color_dc, big, target);
        let (small_mask, mask_bits) = downscale(mask_dc, big, target);

        if !color_bits.is_null() && !mask_bits.is_null() {
            let cpx = std::slice::from_raw_parts_mut(color_bits as *mut u8, (target * target * 4) as usize);
            let mpx = std::slice::from_raw_parts(mask_bits as *const u8, (target * target * 4) as usize);
            for i in (0..cpx.len()).step_by(4) {
                let a = (mpx[i].max(mpx[i + 1]).max(mpx[i + 2])) as u32;
                cpx[i] = ((cpx[i] as u32 * a) / 255) as u8;
                cpx[i + 1] = ((cpx[i + 1] as u32 * a) / 255) as u8;
                cpx[i + 2] = ((cpx[i + 2] as u32 * a) / 255) as u8;
                cpx[i + 3] = a as u8;
            }
        }

        let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: small_mask, hbmColor: small_color };
        let icon = CreateIconIndirect(&info);

        SelectObject(color_dc, old_c);
        SelectObject(mask_dc, old_m);
        DeleteObject(color_dib as HGDIOBJ);
        DeleteObject(mask_dib as HGDIOBJ);
        DeleteObject(small_color as HGDIOBJ);
        DeleteObject(small_mask as HGDIOBJ);
        DeleteDC(color_dc);
        DeleteDC(mask_dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        let _ = BitBlt;
        let _ = BLACKNESS;
        icon
    }
}
