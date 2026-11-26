//! 隐藏窗口与消息循环。

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, FindWindowW, GetMessageW, LoadIconW, PostMessageW, MSG,
    RegisterClassW, TranslateMessage, CS_HREDRAW, CS_VREDRAW, WM_APP, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

const CLASS_NAME: &str = "VitalTrayHostWindow";

/// “把面板/菜单交出来”的消息：第二个实例已经退出托盘的情况下，
/// 再运行一次 exe 就用它唤出面板（面板上右键即出菜单）。
pub const WAKE_MESSAGE: u32 = WM_APP + 10;

/// exe 内嵌的图标资源 ID（见 `crates/bh-app/build.rs`）。
const APP_ICON_ID: usize = 1;

/// Windows 的 `MAKEINTRESOURCE`：把小整数当资源指针传（图标 / 光标 / 字符串编号）。
/// 这些值只被系统拿去查资源表，永远不会解引用。
pub fn int_resource(id: usize) -> *const u16 {
    std::ptr::without_provenance(id)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn create_host_window(wndproc: unsafe extern "system" fn(HWND, u32, usize, isize) -> isize) -> Option<HWND> {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide(CLASS_NAME);
        let icon = LoadIconW(hinst, int_resource(APP_ICON_ID));
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: icon,
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            return None;
        }
        let title = wide("VitalTray");
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() { None } else { Some(hwnd) }
    }
}

/// 另一个实例已经在跑：让它把面板交出来，本次进程安静退出。
///
/// 用户把托盘图标关掉之后，这是唯一还能摸到菜单的入口（面板上右键 = 菜单），
/// 顺带解决“双击 exe 毫无反应”的老毛病。
pub fn wake_existing() -> bool {
    let class = wide(CLASS_NAME);
    for _ in 0..10 {
        unsafe {
            let hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
            if !hwnd.is_null() {
                PostMessageW(hwnd, WAKE_MESSAGE, 0, 0);
                return true;
            }
        }
        // 老实例可能还在启动：等窗口出现，最多轮询 1 秒。
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

pub fn message_loop() {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if r == 0 || r == -1 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
