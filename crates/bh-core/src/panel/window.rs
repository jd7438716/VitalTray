//! 面板窗口创建与几何计算（放在单独文件里，避免生命周期/消息处理被挤爆）。

use super::layout::Layout;
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, LoadCursorW, LoadIconW, RegisterClassW, WNDCLASSW, WNDPROC, CS_HREDRAW,
    CS_VREDRAW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

pub fn create_window(proc: WNDPROC) -> Option<HWND> {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide("VitalTrayPanel");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: proc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: LoadIconW(hinst, crate::window::int_resource(1)),
            hCursor: LoadCursorW(std::ptr::null_mut(), crate::window::int_resource(32512)),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class.as_ptr(),
            wide(crate::i18n::t("panel.title")).as_ptr(),
            WS_POPUP,
            0,
            0,
            10,
            10,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            crate::log::warn("panel", "CreateWindowExW failed");
            None
        } else {
            Some(hwnd)
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn dpi_scale(monitor: *mut core::ffi::c_void) -> f32 {
    let mut dx = 96u32;
    let mut dy = 96u32;
    let hr = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
    if hr == 0 { (dx as f32 / 96.0).clamp(0.75, 4.0) } else { 1.0 }
}

pub fn work_rect(hwnd: HWND) -> RECT {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
        info.rcWork
    } else {
        RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
    }
}

/// 像右键菜单一样贴着鼠标展开，并夹在工作区内。
pub fn place(pt: POINT, work: &RECT, layout: &Layout) -> (i32, i32) {
    let x = pt.x - layout.width + 12;
    let mut y = pt.y - layout.height - 8;
    if y < work.top + 8 {
        y = pt.y + 8;
    }
    clamp_into(*work, x, y, layout)
}

pub fn clamp_into(work: RECT, x: i32, y: i32, layout: &Layout) -> (i32, i32) {
    let min_x = work.left + 8;
    let max_x = (work.right - layout.width - 8).max(min_x);
    let min_y = work.top + 8;
    let max_y = (work.bottom - layout.height - 8).max(min_y);
    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_layout() -> Layout {
        let model = super::super::model::build();
        super::super::layout::compute(&model, 1.0)
    }

    #[test]
    fn places_menu_above_cursor() {
        let layout = sample_layout();
        let (w, h) = (layout.width, layout.height);
        let work = RECT { left: 0, top: 0, right: 1920, bottom: 1040 };
        let (x, y) = place(POINT { x: 1900, y: 1030 }, &work, &layout);
        assert!(x >= 8 && x + w <= 1920);
        assert!(y >= 8 && y + h <= 1040);
    }

    #[test]
    fn clamps_into_top_left() {
        let layout = sample_layout();
        let work = RECT { left: 0, top: 0, right: 800, bottom: 600 };
        let (x, y) = place(POINT { x: 0, y: 0 }, &work, &layout);
        assert!(x >= 8 && y >= 8);
    }
}
