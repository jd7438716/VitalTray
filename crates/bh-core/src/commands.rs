//! 菜单命令：托盘右键、开关项、关于、打开日志目录。

use crate::about;
use crate::autostart;
use crate::i18n;
use crate::log::{self, Level};
use crate::plugins;
use crate::state;
use crate::tray::{self, menu};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn show_menu() {
    let (hwnd, autostart_on, tray_on, debug_on, icon_style, follows_system) = state::with(|st| match st {
        Some(s) => (
            s.hwnd,
            s.cfg.autostart,
            s.tray_wanted,
            log::level() >= Level::Debug,
            s.cfg.icon_style.clone(),
            i18n::is_auto(&s.cfg.lang),
        ),
        None => (std::ptr::null_mut(), false, false, false, String::new(), true),
    });
    if hwnd.is_null() {
        return;
    }
    if let Some(cmd) =
        unsafe { menu::show(hwnd, autostart_on, tray_on, debug_on, &icon_style, follows_system) }
    {
        log::debug("menu", &format!("command {cmd}"));
        on_command(cmd);
    }
}

pub fn on_command(cmd: usize) {
    match cmd {
        menu::CMD_REFRESH => {
            log::info("menu", "manual refresh");
            crate::poller::request_refresh();
            crate::runtime::on_timer();
        }
        menu::CMD_OPEN_LOG => open_log_dir(),
        menu::CMD_AUTOSTART => toggle_autostart(),
        menu::CMD_TRAY_TOGGLE => toggle_tray(),
        menu::CMD_LOG_LEVEL => toggle_log_level(),
        menu::CMD_ICON_STYLE => cycle_icon_style(),
        menu::CMD_LANG_AUTO => set_language(None),
        menu::CMD_ABOUT => show_about(),
        menu::CMD_QUIT => {
            let hwnd = state::with(|st| st.map(|s| s.hwnd).unwrap_or(std::ptr::null_mut()));
            if !hwnd.is_null() {
                unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
            }
        }
        // 语言子菜单里每一条：CMD_LANG_BASE + 下标
        cmd if (menu::CMD_LANG_BASE..menu::CMD_LANG_BASE + i18n::Lang::ALL.len()).contains(&cmd) => {
            set_language(Some(i18n::Lang::ALL[cmd - menu::CMD_LANG_BASE]))
        }
        _ => {}
    }
}

/// 切语言：写回配置（`auto` 或标签），立刻生效。
///
/// 托盘提示是固定标题、面板下次弹出才重排，所以这里只需要收起已经打开的面板，
/// 免得新旧两种语言同屏。
fn set_language(lang: Option<i18n::Lang>) {
    let tag = lang.map(|l| l.tag().to_string()).unwrap_or_else(|| "auto".to_string());
    state::with_mut(|st| {
        if let Some(s) = st {
            s.cfg.lang = tag.clone();
            let _ = s.cfg.save(&s.cfg_path);
        }
    });
    i18n::init(&tag);
    crate::panel::hide();
    log::info(
        "i18n",
        &format!("language -> {tag} (resolved: {} {})", i18n::current().tag(), i18n::current().native_name()),
    );
}

pub fn show_about() {
    let (cfg_path, log_dir) = state::with(|st| match st {
        Some(s) => (s.cfg_path.display().to_string(), s.log_dir.display().to_string()),
        None => (String::new(), String::new()),
    });
    about::show(&plugins::describe(), &cfg_path, &log_dir);
}

fn toggle_autostart() {
    state::with_mut(|st| {
        if let Some(s) = st {
            let want = !s.cfg.autostart;
            let ok = autostart::sync(&s.exe, want);
            s.cfg.autostart = autostart::is_enabled();
            s.autostart = s.cfg.autostart;
            let _ = s.cfg.save(&s.cfg_path);
            log::info("autostart", &format!("-> {} (registry ok={ok})", s.cfg.autostart));
        }
    });
    crate::runtime::refresh();
}

fn toggle_tray() {
    // 托盘是唯一的菜单入口：关掉之后必须留一条回得来的路（再运行一次 exe 唤出面板）。
    let turned_off = state::with_mut(|st| {
        let Some(s) = st else { return false };
        s.tray_wanted = !s.tray_wanted;
        s.cfg.tray = s.tray_wanted;
        let _ = s.cfg.save(&s.cfg_path);
        if s.tray_wanted {
            let mut t = tray::TrayIcon::new(s.hwnd, 1, crate::i18n::t("tray.title"));
            t.add();
            s.tray = Some(t);
        } else if let Some(mut t) = s.tray.take() {
            t.remove();
        }
        log::info("tray", &format!("-> {}", s.tray_wanted));
        !s.tray_wanted
    });
    if turned_off {
        about::info(crate::i18n::t("tray.off.title"), crate::i18n::t("tray.off.body"));
    }
}

fn cycle_icon_style() {
    state::with_mut(|st| {
        if let Some(s) = st {
            let next = crate::tray::icons::Style::parse(&s.cfg.icon_style).next();
            s.cfg.icon_style = next.name().to_string();
            let _ = s.cfg.save(&s.cfg_path);
            log::info("tray", &format!("icon style -> {}", next.name()));
        }
    });
    crate::runtime::refresh();
}

fn toggle_log_level() {
    let next = if log::level() >= Level::Debug { Level::Info } else { Level::Debug };
    log::set_level(next);
    state::with(|st| {
        if let Some(s) = st {
            let mut cfg = s.cfg.clone();
            cfg.log_level = next.name().to_string();
            let _ = cfg.save(&s.cfg_path);
        }
    });
    log::info("log", &format!("level -> {}", next.name()));
}

fn open_log_dir() {
    let dir = state::with(|st| st.map(|s| s.log_dir.display().to_string()).unwrap_or_default());
    if dir.is_empty() {
        return;
    }
    let op = wide("open");
    let path = wide(&dir);
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        );
    }
}