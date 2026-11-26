//! 面板用的 GDI 小工具（纯绘制，不含业务逻辑）。

use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, Ellipse, FillRect, GetDC, GetStockObject, LineTo, MoveToEx,
    ReleaseDC, RoundRect, SelectObject, SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, FW_BOLD, FW_NORMAL, HALFTONE, HGDIOBJ,
    NULL_BRUSH, PS_SOLID, SRCCOPY, TRANSPARENT,
};

pub type H = *mut core::ffi::c_void;

/// COLORREF：0x00BBGGRR，用函数组色避免手写错字节序。
pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((b as u32) << 16) | ((g as u32) << 8) | r as u32
}

pub fn font(height: i32, bold: bool) -> H {
    // 字体跟着语言走：中日韩各用一套带该文种字形的系统字体（见 i18n::ui_font）。
    let face: Vec<u16> = crate::i18n::ui_font().encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateFontW(
            -height.max(1),
            0,
            0,
            0,
            if bold { FW_BOLD } else { FW_NORMAL } as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            face.as_ptr(),
        )
    }
}

pub unsafe fn fill(dc: H, x: i32, y: i32, w: i32, h: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = unsafe { CreateSolidBrush(color) };
    let r = RECT { left: x, top: y, right: x + w, bottom: y + h };
    unsafe {
        FillRect(dc, &r, brush);
        DeleteObject(brush as HGDIOBJ);
    }
}

pub unsafe fn round_fill(dc: H, x: i32, y: i32, w: i32, h: i32, radius: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = unsafe { CreateSolidBrush(color) };
    let pen = unsafe { CreatePen(PS_SOLID as i32, 1, color) };
    let ob = unsafe { SelectObject(dc, brush as HGDIOBJ) };
    let op = unsafe { SelectObject(dc, pen as HGDIOBJ) };
    unsafe {
        RoundRect(dc, x, y, x + w, y + h, radius, radius);
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(brush as HGDIOBJ);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub unsafe fn round_stroke(dc: H, x: i32, y: i32, w: i32, h: i32, radius: i32, color: u32, width: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = unsafe { GetStockObject(NULL_BRUSH) };
    let pen = unsafe { CreatePen(PS_SOLID as i32, width.max(1), color) };
    let ob = unsafe { SelectObject(dc, brush) };
    let op = unsafe { SelectObject(dc, pen as HGDIOBJ) };
    unsafe {
        RoundRect(dc, x, y, x + w, y + h, radius, radius);
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub unsafe fn stroke_rect(dc: H, x: i32, y: i32, w: i32, h: i32, color: u32, width: i32) {
    round_stroke(dc, x, y, w, h, 0, color, width);
}

pub unsafe fn stroke_line(dc: H, x1: i32, y1: i32, x2: i32, y2: i32, color: u32, width: i32) {
    let pen = unsafe { CreatePen(PS_SOLID as i32, width.max(1), color) };
    let op = unsafe { SelectObject(dc, pen as HGDIOBJ) };
    unsafe {
        MoveToEx(dc, x1, y1, std::ptr::null_mut());
        LineTo(dc, x2, y2);
        SelectObject(dc, op);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub unsafe fn stroke_ellipse(dc: H, x: i32, y: i32, w: i32, h: i32, color: u32, width: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let brush = unsafe { GetStockObject(NULL_BRUSH) };
    let pen = unsafe { CreatePen(PS_SOLID as i32, width.max(1), color) };
    let ob = unsafe { SelectObject(dc, brush) };
    let op = unsafe { SelectObject(dc, pen as HGDIOBJ) };
    unsafe {
        Ellipse(dc, x, y, x + w, y + h);
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub unsafe fn stroke_arc(dc: H, cx: f32, cy: f32, r: f32, start_deg: f32, end_deg: f32, color: u32, width: i32) {
    // 用折线近似：GDI 的 Arc 对角度/坐标系方向太挑，图标只需要“看起来圆”。
    let steps = 28;
    let pen = unsafe { CreatePen(PS_SOLID as i32, width.max(1), color) };
    let op = unsafe { SelectObject(dc, pen as HGDIOBJ) };
    unsafe {
        let mut prev: Option<POINT> = None;
        for i in 0..=steps {
            let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
            let a = t.to_radians();
            let p = POINT { x: (cx + r * a.cos()).round() as i32, y: (cy + r * a.sin()).round() as i32 };
            if let Some(q) = prev {
                MoveToEx(dc, q.x, q.y, std::ptr::null_mut());
                LineTo(dc, p.x, p.y);
            }
            prev = Some(p);
        }
        SelectObject(dc, op);
        DeleteObject(pen as HGDIOBJ);
    }
}

pub unsafe fn draw_text(dc: H, text: &str, rc: &mut RECT, font: H, color: u32, flags: u32) {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let old = unsafe { SelectObject(dc, font as HGDIOBJ) };
    unsafe {
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, color);
        DrawTextW(dc, wide.as_ptr(), wide.len() as i32, rc, flags);
        SelectObject(dc, old);
    }
}

pub unsafe fn make_dib(dc: H, w: i32, h: i32) -> (H, H) {
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

pub unsafe fn downscale(src_dc: H, big_w: i32, big_h: i32, w: i32, h: i32) -> (H, H) {
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let (dib, bits) = make_dib(dc, w, h);
        let old = SelectObject(dc, dib as HGDIOBJ);
        SetStretchBltMode(dc, HALFTONE as i32);
        StretchBlt(dc, 0, 0, w, h, src_dc, 0, 0, big_w, big_h, SRCCOPY);
        SelectObject(dc, old);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        (dib, bits)
    }
}

/// 供 paint 使用的屏幕 DC（调用方负责 ReleaseDC）。
pub unsafe fn screen_dc() -> H {
    unsafe { GetDC(std::ptr::null_mut()) }
}

pub unsafe fn release_screen_dc(dc: H) {
    unsafe { ReleaseDC(std::ptr::null_mut(), dc) };
}
