//! 图标预览：把三种样式在 16px 下渲染，再 8× 放大存 BMP，便于人工检查。

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateSolidBrush, DeleteDC, DeleteObject, FillRect,
    GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};

fn main() {
    let styles = [bh_core::tray::icons::Style::Battery, bh_core::tray::icons::Style::Number, bh_core::tray::icons::Style::Ring];
    let pcts = [85u8, 20, 5];
    let zoom = 8i32;
    let cell = 16 * zoom + 8;
    let w = cell * 3;
    let h = cell * 3;
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
        let bg = CreateSolidBrush(0x00F0F0F0);
        let r = RECT { left: 0, top: 0, right: w, bottom: h };
        FillRect(dc, &r, bg);
        DeleteObject(bg as HGDIOBJ);

        for (si, style) in styles.iter().enumerate() {
            for (pi, pct) in pcts.iter().enumerate() {
                let icon = bh_core::tray::icons::render(*pct, false, *style);
                let x = si as i32 * cell + 4;
                let y = pi as i32 * cell + 4;
                DrawIconEx(dc, x, y, icon as _, 16 * zoom, 16 * zoom, 0, std::ptr::null_mut(), DI_NORMAL);
            }
        }
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
        let out = std::env::args().nth(1).unwrap_or_else(|| "icons.bmp".into());
        std::fs::write(&out, &bmp).unwrap();
        println!("wrote {out} ({w}x{h})");
        SelectObject(dc, old);
        DeleteObject(dib as HGDIOBJ);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}