//! 托盘图标：Shell_NotifyIcon 的最小封装。

pub mod icons;
pub mod menu;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_WARNING, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    LoadIconW, RegisterWindowMessageW, WM_APP,
};

/// 托盘回调消息号（lParam 低字为鼠标事件）。
pub const TRAY_CALLBACK: u32 = WM_APP + 1;

/// exe 内嵌的图标资源 ID（见 `crates/bh-app/build.rs`）。
const APP_ICON_ID: usize = 1;

pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    added: bool,
}

impl TrayIcon {
    pub fn new(hwnd: HWND, id: u32, tip: &str) -> Self {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = id;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_CALLBACK;
        data.hIcon = default_icon();
        set_tip_into(&mut data, tip);
        Self { data, added: false }
    }

    pub fn add(&mut self) -> bool {
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) != 0 };
        self.added = ok;
        ok
    }

    /// 资源管理器重启后托盘区被重建，图标要重新挂一次。
    pub fn readd(&mut self) -> bool {
        self.added = false;
        self.add()
    }

    pub fn set_icon(&mut self, icon: *mut core::ffi::c_void) {
        self.data.hIcon = icon;
        if self.added {
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data); }
        }
    }

    pub fn set_tip(&mut self, tip: &str) {
        set_tip_into(&mut self.data, tip);
        if self.added {
            unsafe {
                Shell_NotifyIconW(NIM_MODIFY, &self.data);
            }
        }
    }

    /// 气泡通知（Win10/11 会转成通知中心 toast）。
    pub fn balloon(&mut self, title: &str, text: &str) {
        let t: Vec<u16> = title.encode_utf16().take(63).collect();
        let b: Vec<u16> = text.encode_utf16().take(255).collect();
        self.data.uFlags |= NIF_INFO;
        self.data.dwInfoFlags = NIIF_WARNING;
        self.data.szInfoTitle = [0; 64];
        self.data.szInfoTitle[..t.len()].copy_from_slice(&t);
        self.data.szInfo = [0; 256];
        self.data.szInfo[..b.len()].copy_from_slice(&b);
        if self.added {
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data); }
        }
    }

    pub fn remove(&mut self) {
        if self.added {
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &self.data);
            }
            self.added = false;
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}

fn set_tip_into(data: &mut NOTIFYICONDATAW, tip: &str) {
    let units: Vec<u16> = tip.encode_utf16().take(127).collect();
    data.szTip = [0; 128];
    data.szTip[..units.len()].copy_from_slice(&units);
}

fn default_icon() -> *mut core::ffi::c_void {
    // 先用 exe 内嵌的图标（build.rs 编进去的），没有资源才退回系统默认。
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let icon = LoadIconW(hinst, crate::window::int_resource(APP_ICON_ID));
        if icon.is_null() {
            LoadIconW(std::ptr::null_mut(), crate::window::int_resource(32512))
        } else {
            icon
        }
    }
}

/// 资源管理器广播的 `TaskbarCreated` 消息号（系统范围内同一个 ID，取一次缓存）。
///
/// 收不到它，Explorer 重启后托盘图标就永久消失了——只能重启应用。
pub fn is_taskbar_created(msg: u32) -> bool {
    static ID: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let id = *ID.get_or_init(|| {
        let name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    });
    id != 0 && msg == id
}
