//! 托盘状态图标预览：6 种状态 × 3 种样式 × 明/暗任务栏，16px 图标放大 8× 存 BMP。
//!
//! 用法：`cargo run --example icons -- tmp/status-icons.bmp`

use bh_core::status::{self, Aggregate, State};
use bh_core::tray::icons::{self, Style};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DT_LEFT, DT_SINGLELINE, DT_VCENTER,
    FW_NORMAL, HGDIOBJ, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};

fn main() {
    let states: [(&str, Aggregate); 6] = [
        ("good 90", Aggregate { state: State::Good, percent: Some(90) }),
        ("normal 55", Aggregate { state: State::Normal, percent: Some(55) }),
        ("low 12", Aggregate { state: State::Low, percent: Some(12) }),
        ("charging 55", Aggregate { state: State::Charging, percent: Some(55) }),
        ("stale 55", Aggregate { state: State::Stale, percent: Some(55) }),
        ("unknown", Aggregate { state: State::Unknown, percent: None }),
    ];
    let styles = [Style::Battery, Style::Number, Style::Ring];
    let themes = ["dark taskbar", "light taskbar"];

    let zoom = 8i32;
    let cell = 16 * zoom;
    let gap = 10i32;
    let pad = 12i32;
    let label_w = 128i32;
    let header_h = 48i32;
    let group_gap = 24i32;
    let cols = 6i32;
    let w = pad + label_w + cols * (cell + gap) + group_gap + pad;
    let h = pad + header_h + states.len() as i32 * (cell + gap) + pad;

    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
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
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        let old = SelectObject(dc, dib as HGDIOBJ);
        fill(dc, 0, 0, w, h, status::rgb(255, 255, 255));

        let font = CreateFontW(
            -14,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            windows_sys::core::w!("Microsoft YaHei UI"),
        );

        // 表头：两个主题分组 + 每列样式名
        for (gi, theme_name) in themes.iter().enumerate() {
            let x = pad + label_w + gi as i32 * (3 * (cell + gap) + group_gap);
            let mut r = RECT { left: x, top: pad, right: x + 3 * (cell + gap), bottom: pad + 20 };
            text(dc, font, theme_name, &mut r, status::rgb(32, 32, 32), DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
        for (c, s) in (0..cols).zip(styles.iter().chain(styles.iter())) {
            let x = col_x(c, pad, label_w, cell, gap, group_gap);
            let mut r = RECT { left: x, top: pad + 22, right: x + cell, bottom: pad + header_h };
            text(dc, font, s.name(), &mut r, status::rgb(96, 96, 96), DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }

        for (ri, (label, agg)) in states.iter().enumerate() {
            let y = pad + header_h + ri as i32 * (cell + gap);
            let mut r = RECT { left: pad, top: y, right: pad + label_w, bottom: y + cell };
            text(dc, font, label, &mut r, status::rgb(32, 32, 32), DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            for c in 0..cols {
                let dark = c < 3;
                let bg = if dark { status::rgb(32, 32, 32) } else { status::rgb(243, 243, 243) };
                let x = col_x(c, pad, label_w, cell, gap, group_gap);
                fill(dc, x, y, cell, cell, bg);
                let style = styles[(c % 3) as usize];
                let icon = icons::render_with(agg, style, dark);
                DrawIconEx(dc, x, y, icon as _, cell, cell, 0, std::ptr::null_mut(), DI_NORMAL);
            }
        }

        DeleteObject(font as HGDIOBJ);

        let px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
        let mut bmp: Vec<u8> = Vec::with_capacity((w * h * 3 + 54) as usize);
        let file_size = 54 + w * h * 3;
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&w.to_le_bytes());
        bmp.extend_from_slice(&h.to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&(w * h * 3).to_le_bytes());
        bmp.extend_from_slice(&2835u32.to_le_bytes());
        bmp.extend_from_slice(&2835u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        for row in (0..h).rev() {
            for col in 0..w {
                let i = ((row * w + col) * 4) as usize;
                bmp.push(px[i]);
                bmp.push(px[i + 1]);
                bmp.push(px[i + 2]);
            }
        }
        let out = std::env::args().nth(1).unwrap_or_else(|| "status-icons.bmp".into());
        std::fs::write(&out, &bmp).unwrap();
        println!("wrote {out} ({w}x{h})");
        SelectObject(dc, old);
        DeleteObject(dib as HGDIOBJ);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}

fn col_x(c: i32, pad: i32, label_w: i32, cell: i32, gap: i32, group_gap: i32) -> i32 {
    pad + label_w + c * (cell + gap) + if c >= 3 { group_gap } else { 0 }
}

unsafe fn fill(dc: *mut core::ffi::c_void, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let brush = CreateSolidBrush(color);
    let r = RECT { left: x, top: y, right: x + w, bottom: y + h };
    FillRect(dc, &r, brush);
    DeleteObject(brush as HGDIOBJ);
}

unsafe fn text(
    dc: *mut core::ffi::c_void,
    font: *mut core::ffi::c_void,
    value: &str,
    rc: &mut RECT,
    color: u32,
    flags: u32,
) {
    let old = SelectObject(dc, font as HGDIOBJ);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, color);
    let units: Vec<u16> = value.encode_utf16().collect();
    DrawTextW(dc, units.as_ptr(), units.len() as i32, rc, flags);
    SelectObject(dc, old);
}
