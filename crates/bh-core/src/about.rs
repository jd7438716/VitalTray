//! 关于对话框。

use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn show(plugin_lines: &[String], config_path: &str, log_dir: &str) {
    let text = format!(
        "BatteryHub {}\n\n{}：\n{}\n\n{}：{config_path}\n{}：{log_dir}",
        crate::VERSION,
        crate::i18n::t("about.plugins"),
        plugin_lines.join("\n"),
        crate::i18n::t("about.config"),
        crate::i18n::t("about.logdir")
    );
    let t = wide(&text);
    let c = wide(crate::i18n::t("about.title"));
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONINFORMATION); }
}
