//! 实验：验证跨进程父子窗口是否可行（排查 taskbar widget 挂载失败）。

use windows_sys::Win32::Foundation::{GetLastError, SetLastError, HWND};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowW, GetDesktopWindow, RegisterClassW, WNDCLASSW,
    WS_CHILD, WS_EX_LAYERED, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

unsafe extern "system" fn proc_(h: HWND, m: u32, w: usize, l: isize) -> isize {
    DefWindowProcW(h, m, w, l)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn try_create(label: &str, parent: HWND, style: u32, ex: u32) {
    let class = wide("BhProbeClass");
    unsafe {
        SetLastError(0);
        let h = CreateWindowExW(
            ex,
            class.as_ptr(),
            wide("probe").as_ptr(),
            style,
            0,
            0,
            100,
            30,
            parent,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        println!("{label}: hwnd={h:?} err={}", GetLastError());
    }
}

fn main() {
    unsafe {
        let class = wide("BhProbeClass");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        let atom = RegisterClassW(&wc);
        println!("register atom={atom} err={}", GetLastError());

        let tray = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        println!("tray hwnd={tray:?}");
        let desktop = GetDesktopWindow();
        println!("desktop hwnd={desktop:?}");

        try_create("toplevel", std::ptr::null_mut(), WS_OVERLAPPEDWINDOW | WS_VISIBLE, 0);
        try_create("child-of-desktop", desktop, WS_CHILD | WS_VISIBLE, 0);
        try_create("child-of-tray", tray, WS_CHILD | WS_VISIBLE, 0);
        try_create("child-of-tray-layered", tray, WS_CHILD | WS_VISIBLE, WS_EX_LAYERED);

        // 变体：先普通 child，再动态加 WS_EX_LAYERED
        let child = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("probe2").as_ptr(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            100,
            30,
            tray,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        println!("child2={child:?}");
        if !child.is_null() {
            let old = windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                child,
                windows_sys::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
                (WS_EX_LAYERED as isize) | (windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE as isize),
            );
            let ok = windows_sys::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                child,
                0x00FF00FF,
                0,
                windows_sys::Win32::UI::WindowsAndMessaging::LWA_COLORKEY,
            );
            println!("dynamic layered: oldex={old} setattr={ok} err={}", GetLastError());
        }
        // 顶层 layered 窗口（覆盖任务栏区域）是否可行
        let top = CreateWindowExW(
            WS_EX_LAYERED | windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW
                | windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE,
            class.as_ptr(),
            wide("probe3").as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::WS_POPUP | WS_VISIBLE,
            100,
            100,
            120,
            30,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        println!("toplevel-layered hwnd={top:?} err={}", GetLastError());
        if !top.is_null() {
            let ok = windows_sys::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                top,
                0x00FF00FF,
                0,
                windows_sys::Win32::UI::WindowsAndMessaging::LWA_COLORKEY,
            );
            println!("toplevel-layered setattr={ok} err={}", GetLastError());
            let mut r: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(top, &mut r);
            println!("toplevel rect={:?}", (r.left, r.top, r.right, r.bottom));
        }
        let mut ver = [0u16; 64];
        let n = windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW(std::ptr::null_mut(), ver.as_mut_ptr(), 64);
        let _ = n;
    }
}
