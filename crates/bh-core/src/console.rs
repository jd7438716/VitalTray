//! GUI 子系统下的控制台挂接：双击/自启不再弹控制台窗口，带参数跑 CLI 时输出照常。
//!
//! 规则：只在当前 std 句柄不可用时才挂回父控制台。被重定向（管道/文件）时保持原句柄，
//! 免得把 `--self-test` 的输出抢到控制台上。

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

fn usable(h: HANDLE) -> bool {
    !h.is_null() && h != INVALID_HANDLE_VALUE
}

/// 父进程有控制台而自己没有可用的 std 句柄时，挂上去并重绑 CONOUT$/CONIN$。
/// 没有父控制台（Explorer 双击、计划任务）时静默返回。
pub fn attach_parent_console() {
    unsafe {
        if usable(GetStdHandle(STD_OUTPUT_HANDLE)) && usable(GetStdHandle(STD_ERROR_HANDLE)) {
            return;
        }
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        rebind("CONOUT$", STD_OUTPUT_HANDLE);
        rebind("CONOUT$", STD_ERROR_HANDLE);
        rebind("CONIN$", STD_INPUT_HANDLE);
    }
}

unsafe fn rebind(device: &str, which: u32) {
    use std::os::windows::io::AsRawHandle;
    if let Ok(f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(device)
    {
        SetStdHandle(which, f.as_raw_handle() as HANDLE);
        // 句柄要继续有效，文件不能在这里被关掉。
        std::mem::forget(f);
    }
}
