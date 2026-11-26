//! 右键菜单：托盘与任务栏悬浮窗共用同一套构建器。
//!
//! 菜单每次弹出时按当前语言现拼文案（换语言不用重启）。

use crate::i18n::{self, Lang};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, PostMessageW, SetForegroundWindow,
    TrackPopupMenuEx, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, TPM_RETURNCMD,
    TPM_RIGHTBUTTON,
};

pub const CMD_REFRESH: usize = 1001;
pub const CMD_OPEN_LOG: usize = 1002;
pub const CMD_AUTOSTART: usize = 1010;
pub const CMD_TRAY_TOGGLE: usize = 1011;
pub const CMD_LOG_LEVEL: usize = 1013;
pub const CMD_ICON_STYLE: usize = 1014;
/// “跟随系统”（`ui.lang=auto`）。
pub const CMD_LANG_AUTO: usize = 1100;
/// 各语言：`CMD_LANG_BASE + Lang::ALL 的下标`。
pub const CMD_LANG_BASE: usize = 1110;
pub const CMD_ABOUT: usize = 1090;
pub const CMD_QUIT: usize = 1099;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn add(menu: *mut core::ffi::c_void, text: &str, id: usize, flags: u32) {
    let w = wide(text);
    unsafe {
        AppendMenuW(menu, flags, id, w.as_ptr());
    }
}

fn separator(menu: *mut core::ffi::c_void) {
    unsafe {
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    }
}

fn submenu(menu: *mut core::ffi::c_void, text: &str, sub: *mut core::ffi::c_void) {
    let w = wide(text);
    unsafe {
        AppendMenuW(menu, MF_POPUP | MF_STRING, sub as usize, w.as_ptr());
    }
}

fn check(on: bool) -> u32 {
    if on {
        MF_CHECKED
    } else {
        0
    }
}

/// 语言子菜单：所有语言都用它自己的名字写（不翻译），当前生效的打勾。
fn add_language_menu(menu: *mut core::ffi::c_void, follows_system: bool) {
    let sub = unsafe { CreatePopupMenu() };
    if sub.is_null() {
        return;
    }
    let current = i18n::current();
    add(sub, i18n::t("menu.follow_system"), CMD_LANG_AUTO, MF_STRING | check(follows_system));
    separator(sub);
    for (index, lang) in Lang::ALL.iter().enumerate() {
        let on = !follows_system && current == *lang;
        add(sub, lang.native_name(), CMD_LANG_BASE + index, MF_STRING | check(on));
    }
    submenu(menu, i18n::t("menu.language"), sub);
}

/// 弹出菜单并返回被选中的命令 id（0 = 取消）。
///
/// # Safety
/// `hwnd` 必须是本进程创建、仍然有效的窗口句柄（调用期间不得销毁）。
pub unsafe fn show(
    hwnd: HWND,
    autostart_on: bool,
    tray_on: bool,
    debug_log: bool,
    icon_style: &str,
    follows_system: bool,
) -> Option<usize> {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }
        add(menu, i18n::t("menu.refresh"), CMD_REFRESH, MF_STRING);
        add(menu, i18n::t("menu.open_log"), CMD_OPEN_LOG, MF_STRING);
        separator(menu);
        add(
            menu,
            i18n::t("menu.autostart"),
            CMD_AUTOSTART,
            MF_STRING | check(autostart_on),
        );
        add(
            menu,
            i18n::t("menu.tray"),
            CMD_TRAY_TOGGLE,
            MF_STRING | check(tray_on),
        );
        add(
            menu,
            i18n::icon_style_label(icon_style),
            CMD_ICON_STYLE,
            MF_STRING,
        );
        add(
            menu,
            i18n::t("menu.log_level"),
            CMD_LOG_LEVEL,
            MF_STRING | check(debug_log),
        );
        add_language_menu(menu, follows_system);
        separator(menu);
        add(menu, i18n::t("menu.about"), CMD_ABOUT, MF_STRING);
        add(menu, i18n::t("menu.quit"), CMD_QUIT, MF_STRING);

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenuEx(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            hwnd,
            std::ptr::null(),
        );
        PostMessageW(hwnd, 0, 0, 0);
        DestroyMenu(menu);
        if cmd == 0 {
            None
        } else {
            Some(cmd as usize)
        }
    }
}
