//! 托盘图标：4× 超采样 + "遮罩/颜色"双通道合成，边缘平滑且适配明暗主题。

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    Arc, BitBlt, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, Ellipse, FillRect, GetDC, ReleaseDC, RoundRect, SelectObject,
    SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLACKNESS, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FW_BOLD, HALFTONE, HGDIOBJ,
    SRCCOPY, TRANSPARENT,
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

struct Colors {
    text: u32,
    shell: u32,
    bg: u32,
    track: u32,
}

fn colors(dark: bool) -> Colors {
    if dark {
        Colors { text: 0x00F2_F2F2, shell: 0x009A_A0A8, bg: 0x002A_2C30, track: 0x003A_3D42 }
    } else {
        Colors { text: 0x0012_1212, shell: 0x0068_7078, bg: 0x00F4_F4F4, track: 0x00C8_C8C8 }
    }
}

fn level_color(percent: u8, charging: bool) -> u32 {
    if charging {
        return 0x00E8_9A_3C;
    }
    if percent <= 15 {
        0x0038_3D_E5
    } else if percent <= 40 {
        0x0020_B8_F0
    } else {
        0x0048_C8_5A
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

fn draw_number(dc: H, size: i32, percent: u8, color: u32) {
    unsafe {
        let font = CreateFontW(-(size * 58 / 100), 0, 0, 0, FW_BOLD as i32, 0, 0, 0, 1, 0, 0, 0, 0, windows_sys::core::w!("Segoe UI"));
        let old = SelectObject(dc, font as HGDIOBJ);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, color);
        let text: Vec<u16> = format!("{percent}").encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: size, bottom: size };
        DrawTextW(dc, text.as_ptr(), text.len() as i32, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(dc, old);
        DeleteObject(font as HGDIOBJ);
    }
}

/// mono = true 时所有图元画成白色，只用于生成 alpha 遮罩。
unsafe fn draw(dc: H, size: i32, percent: u8, charging: bool, style: Style, mono: bool) {
    let dark = crate::theme::is_dark();
    let c = colors(dark);
    let paint = |col: u32| if mono { 0x00FF_FFFF } else { col };
    let text = paint(c.text);
    let shell = paint(c.shell);
    let bg = paint(c.bg);
    let track = paint(c.track);
    let accent = paint(level_color(percent, charging));

    match style {
        Style::Battery => {
            let body_x = size * 5 / 100;
            let body_y = size * 20 / 100;
            let body_w = size * 78 / 100;
            let body_h = size * 56 / 100;
            round_rect(dc, body_x, body_y, body_w, body_h, size / 9, shell);
            let pad = size * 6 / 100;
            round_rect(dc, body_x + pad, body_y + pad, body_w - pad * 2, body_h - pad * 2, size / 16, bg);
            let inner_w = (body_w - pad * 2) * percent.min(100) as i32 / 100;
            round_rect(dc, body_x + pad, body_y + pad, inner_w.max(1), body_h - pad * 2, size / 16, accent);
            round_rect(dc, body_x + body_w + size * 3 / 100, body_y + body_h / 3, size * 7 / 100, body_h / 3, size / 24, shell);
        }
        Style::Number => {
            round_rect(dc, size * 3 / 100, size * 3 / 100, size * 94 / 100, size * 94 / 100, size / 6, bg);
            let bar_h = (size * 20 / 100).max(3);
            fill_rect(dc, size * 3 / 100, size - bar_h - size * 3 / 100, size * 94 / 100, bar_h, track);
            let fill = (size * 94 / 100) * percent.min(100) as i32 / 100;
            fill_rect(dc, size * 3 / 100, size - bar_h - size * 3 / 100, fill, bar_h, accent);
            draw_number(dc, size, percent, text);
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
            draw_number(dc, size, percent, text);
        }
    }
}

/// src 必须是"已选好位图"的内存 DC。
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

/// 生成托盘图标；调用方负责 DestroyIcon。
pub fn render(percent: u8, charging: bool, style: Style) -> H {
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
        draw(color_dc, big, percent, charging, style, false);

        let (mask_dib, _) = make_dib(mask_dc, big, big);
        let old_m = SelectObject(mask_dc, mask_dib as HGDIOBJ);
        fill_rect(mask_dc, 0, 0, big, big, 0x0000_0000);
        draw(mask_dc, big, percent, charging, style, true);

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