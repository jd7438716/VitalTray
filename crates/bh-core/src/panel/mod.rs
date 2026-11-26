//! 状态面板：托盘左键弹出的**只读**信息层（系统状态 + 设备电量）。
//!
//! 分工：左键 = 看信息，右键 = 做功能。面板里没有命令、没有可点的行，
//! 点任意处 / `Esc` / 失去前台即收起；在面板上按右键也是收起面板、把菜单交出去。
//! 外观像右键菜单，但行为上它只是一张信息卡。

mod gdi;
mod icons;
mod layout;
mod model;
mod paint;
mod theme;
mod window;

use layout::Layout;
use model::Model;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetCursorPos, GetForegroundWindow, IsWindow, KillTimer, SetForegroundWindow,
    SetTimer, ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE,
};
use window::{clamp_into, create_window, dpi_scale, place, work_rect};

const TIMER_ID: usize = 1;
const VK_ESCAPE: usize = 0x1B;
/// 点托盘收起面板时，鼠标“按下”先让面板失活收起、“抬起”才回到托盘；
/// 用这个时间窗吸收那次抬起，避免同一次点击把面板关了又开。
const CLICK_GRACE: Duration = Duration::from_millis(400);

struct Content {
    model: Model,
    layout: Layout,
    x: i32,
    y: i32,
    scale: f32,
}

struct Panel {
    hwnd: HWND,
    visible: bool,
    content: Option<Content>,
}

thread_local! {
    static PANEL: std::cell::RefCell<Option<Panel>> = const { std::cell::RefCell::new(None) };
    /// 最近一次“因为失活而自动收起”的时刻。
    static AUTOHIDE: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
}

/// 托盘左键：开着就收，关着就开。刚因“点外面”收起的，忽略这一次抬起。
pub fn toggle() {
    if is_open() {
        crate::log::debug("panel", "toggle -> hide");
        hide();
    } else if autohid_within(CLICK_GRACE) {
        crate::log::debug("panel", "toggle ignored (just autohid)");
    } else {
        crate::log::debug("panel", "toggle -> show");
        show();
    }
}

/// 在鼠标位置弹出面板。已打开时先收起再按最新数据重开。
/// 返回 false 表示窗口建不起来。
pub fn show() -> bool {
    let Some(hwnd) = ensure() else { return false };
    // 打开面板 = 一次「立即刷新」：被动协议（雷柏接收器）只在设备被使用时才推帧，
    // 光靠退避轮询很难正好逮到那一刻。这里强制重试一轮并解除退避，
    // 6 秒内只要有帧，面板自己那个 1 秒定时器就会把它画出来。
    crate::poller::request_refresh();
    hide();

    let model = model::build();
    let mut pt = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut pt) };
    let monitor = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let work = if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
        info.rcWork
    } else {
        RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
    };
    let mut scale = dpi_scale(monitor);
    let mut layout = layout::compute(&model, scale);
    // 内容高于工作区时整体缩小，保证不越界（不引入滚动条）。
    let max_h = (work.bottom - work.top - 16).max(160);
    if layout.height > max_h {
        scale *= max_h as f32 / layout.height as f32;
        layout = layout::compute(&model, scale);
    }
    let (x, y) = place(pt, &work, &layout);

    if !paint::render(hwnd, x, y, &layout, &model) {
        crate::log::warn("panel", "render failed");
        return false;
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetForegroundWindow(hwnd);
        // “点外面就收”靠的是失活消息；拿得到前台就不必抢鼠标捕获，
        // 这样托盘右键在面板开着时也能直达菜单。抢不到前台时退回捕获兜底。
        if GetForegroundWindow() == hwnd {
            SetFocus(hwnd);
        } else {
            crate::log::debug("panel", "not foreground; falling back to mouse capture");
            SetCapture(hwnd);
        }
        SetTimer(hwnd, TIMER_ID, 1000, None);
    }
    let content = Content { model, layout, x, y, scale };
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().as_mut() {
            p.visible = true;
            p.content = Some(content);
        }
    });
    crate::log::info("panel", &format!("open at {x},{y}"));
    true
}

pub fn hide() {
    let hwnd = PANEL.with(|cell| {
        let mut guard = cell.borrow_mut();
        let p = guard.as_mut()?;
        if !p.visible {
            return None;
        }
        p.visible = false;
        Some(p.hwnd)
    });
    if let Some(hwnd) = hwnd {
        unsafe {
            ReleaseCapture();
            KillTimer(hwnd, TIMER_ID);
            ShowWindow(hwnd, SW_HIDE);
        }
        crate::log::debug("panel", "hide");
    }
}

pub fn is_open() -> bool {
    PANEL.with(|cell| cell.borrow().as_ref().map(|p| p.visible).unwrap_or(false))
}

fn autohid_within(window: Duration) -> bool {
    AUTOHIDE.with(|cell| cell.get().map(|at| at.elapsed() < window).unwrap_or(false))
}

/// 失活收起：记下时刻，好让紧接着的那次托盘抬起不把面板又打开。
fn autohide() {
    if is_open() {
        AUTOHIDE.with(|cell| cell.set(Some(Instant::now())));
        hide();
    }
}

fn ensure() -> Option<HWND> {
    let existing = PANEL.with(|cell| cell.borrow().as_ref().map(|p| p.hwnd));
    if let Some(hwnd) = existing {
        if unsafe { IsWindow(hwnd) } != 0 {
            return Some(hwnd);
        }
    }
    let hwnd = create_window(Some(panel_proc))?;
    PANEL.with(|cell| {
        *cell.borrow_mut() = Some(Panel { hwnd, visible: false, content: None });
    });
    Some(hwnd)
}

fn redraw() {
    let snapshot = PANEL.with(|cell| {
        let guard = cell.borrow();
        let p = guard.as_ref()?;
        let c = p.content.as_ref()?;
        Some((p.hwnd, c.x, c.y, c.layout.clone(), c.model.clone()))
    });
    if let Some((hwnd, x, y, layout, model)) = snapshot {
        paint::render(hwnd, x, y, &layout, &model);
    }
}

fn refresh() {
    if !is_open() {
        return;
    }
    let Some((hwnd, x, y, scale)) = PANEL.with(|cell| {
        let guard = cell.borrow();
        let p = guard.as_ref()?;
        let c = p.content.as_ref()?;
        Some((p.hwnd, c.x, c.y, c.scale))
    }) else {
        return;
    };
    let work = work_rect(hwnd);
    let model = model::build();
    let max_h = (work.bottom - work.top - 16).max(160);
    let mut next_scale = scale;
    let mut layout = layout::compute(&model, next_scale);
    if layout.height > max_h && layout.height > 0 {
        next_scale *= max_h as f32 / layout.height as f32;
        layout = layout::compute(&model, next_scale);
    }
    let (x, y) = clamp_into(work, x, y, &layout);
    PANEL.with(|cell| {
        if let Some(p) = cell.borrow_mut().as_mut() {
            if let Some(c) = p.content.as_mut() {
                c.model = model;
                c.layout = layout;
                c.x = x;
                c.y = y;
                c.scale = next_scale;
            }
        }
    });
    redraw();
}

/// 右键 = 功能：收起信息面板，把菜单交给 commands（菜单出现在光标处）。
fn hand_over_to_menu() {
    hide();
    crate::commands::show_menu();
}

unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // 左键 = 收起（纯信息，没有可点的行）。
        WM_LBUTTONUP => {
            hide();
            0
        }
        // 右键 / 菜单键 = 功能。
        WM_RBUTTONUP | WM_CONTEXTMENU => {
            hand_over_to_menu();
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE {
                hide();
            }
            0
        }
        WM_ACTIVATE => {
            if (wparam & 0xFFFF) == 0 {
                autohide();
            }
            0
        }
        WM_CAPTURECHANGED => {
            if is_open() {
                hide();
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_ID {
                refresh();
            }
            0
        }
        WM_MOUSEACTIVATE => 1, // MA_ACTIVATE
        WM_ERASEBKGND => 1,
        WM_PAINT => 0,
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// Windows 消息常量（windows-sys 里名字不统一，这里只用到的几个直接写值）。
const WM_ACTIVATE: u32 = 0x0006;
const WM_PAINT: u32 = 0x000F;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_MOUSEACTIVATE: u32 = 0x0021;
const WM_KEYDOWN: u32 = 0x0100;
const WM_TIMER: u32 = 0x0113;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_CAPTURECHANGED: u32 = 0x0215;
const WM_CONTEXTMENU: u32 = 0x007B;
