//! 宿主主流程：参数解析、单实例、自启、窗口 + 托盘 + 菜单 + 轮询整合。

use crate::commands;
use crate::log::{self, Level};
use crate::poller::{self, DeviceReading};
use crate::state::{self, State};
use crate::{autostart, cli::Args, config::Config, i18n, paths, plugins, probe, runtime, single_instance, tray, window, VERSION};
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, KillTimer, PostQuitMessage, SetTimer, WM_CLOSE, WM_COMMAND,
    WM_DESTROY, WM_TIMER,
};

pub fn run() -> i32 {
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleOutputCP(65001);
        // 高 DPI：让坐标/字体/图标都按物理像素工作，否则 150% 缩放下任务栏位置会算错
        const PER_MONITOR_AWARE_V2: isize = -4;
        let ok = windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2 as windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT);
        if ok == 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
        }
    }
    let args = Args::parse(std::env::args().skip(1));
    let dir = paths::data_dir(args.portable);
    let _ = paths::ensure_dir(&dir);
    // 改名迁移：把旧 %LOCALAPPDATA%\BatteryHub\config.ini 复制过来（必须在 Config::load 之前）。
    let migrated_from = paths::migrate_legacy_config(&dir);
    let cfg_path = args
        .config
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| paths::config_file(&dir));
    let mut cfg = Config::load(&cfg_path);
    // 语言必须在任何输出之前定下来：--help / --once / --sysmon 的文案都跟着界面语言走。
    i18n::init(&cfg.lang);
    if args.help {
        print!("{}", Args::help_text());
        return 0;
    }
    if args.version {
        println!("VitalTray {VERSION}");
        return 0;
    }
    if args.self_test {
        return crate::selftest::run(&args);
    }
    let capture_dir = dir.join("capture");
    if args.dump {
        return probe::dump_hid();
    }
    if args.capture_report {
        return probe::capture_report(&capture_dir);
    }
    if let Some(target) = args.feature_get.clone() {
        return probe::feature_get(&target, args.feature_id);
    }
    if let Some(target) = args.watch.clone() {
        let secs = if args.watch_for == 0 { 180 } else { args.watch_for };
        return probe::watch(&target, secs, args.watch_out.as_deref());
    }
    if args.once {
        return probe::once();
    }
    if args.sysmon {
        return probe::sysmon_once();
    }

    let level = args
        .log_level
        .as_deref()
        .and_then(Level::parse)
        .or_else(|| Level::parse(&cfg.log_level))
        .unwrap_or(Level::Info);
    log::init(paths::log_file(&dir), level);
    log::info("app", &format!("start v{VERSION} portable={}", args.portable));
    if let Some(old) = &migrated_from {
        // 只在"刚刚发生过迁移"时记一行，之后每次启动都不再重复。
        log::info(
            "app",
            &format!("migrated config from legacy dir {}", old.display()),
        );
    }
    log::debug("app", &format!("config={} log_level={}", cfg_path.display(), level.name()));

    let _guard = match single_instance::acquire("Local\\VitalTray.SingleInstance") {
        Some(g) => g,
        None => {
            // 已经有一个实例在跑：把面板叫出来再退出，别让“双击没反应”变成死胡同。
            let woken = window::wake_existing();
            log::warn("app", &format!("another instance is running; wake existing = {woken}"));
            return 0;
        }
    };

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vitaltray.exe"));
    if !cfg_path.exists() {
        let _ = cfg.save(&cfg_path);
        log::info("config", "default written");
    }
    if !args.no_autostart {
        let _ = autostart::sync(&exe, cfg.autostart);
    }
    let autostart_now = autostart::is_enabled();
    cfg.autostart = autostart_now;

    let hwnd = match window::create_host_window(wndproc) {
        Some(h) => h,
        None => {
            log::error("app", "create window failed");
            return 2;
        }
    };

    let tray_wanted = cfg.tray && !args.no_tray;
    let tray = if tray_wanted {
        let mut t = tray::TrayIcon::new(hwnd, 1, i18n::t("tray.title"));
        if t.add() {
            log::info("tray", "icon added");
            Some(t)
        } else {
            log::warn("tray", "add failed");
            None
        }
    } else {
        None
    };

    let monitor = crate::monitor::Monitor::new(&dir.join("history"));
    let low_battery = cfg.low_battery;
    let (tx, rx) = std::sync::mpsc::channel::<Vec<DeviceReading>>();
    poller::spawn(cfg.refresh_secs, tx);
    crate::sysmon::spawn();
    unsafe { SetTimer(hwnd, 1, 1000, None) };

    // 抓帧是逆向工具：只有配置里显式列了 VID 才启动（默认不抓，也不占任何句柄）。
    let capture_vids = crate::capture::parse_watch(&cfg.capture_vids);
    if !args.no_capture && !capture_vids.is_empty() {
        crate::capture::spawn(capture_dir, capture_vids.clone());
        log::info("capture", &format!("watching vendor HID collections: {capture_vids:?}"));
    }

    state::install(State {
        hwnd,
        readings: Vec::new(),
        monitor,
        low_battery,
        rx,
        cfg,
        exe,
        cfg_path,
        log_dir: dir,
        tray,
        tray_wanted,
        autostart: autostart_now,
    });

    log::info("app", &format!("plugins: {:?}", plugins::describe()));
    runtime::refresh();
    if args.panel {
        let _ = crate::panel::show();
    }
    window::message_loop();
    log::info("app", "exit");
    0
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 资源管理器重启会广播 TaskbarCreated：不重新挂图标，托盘就再也回不来了。
    if tray::is_taskbar_created(msg) {
        log::info("tray", "taskbar recreated; re-adding icon");
        state::with_mut(|st| {
            if let Some(t) = st.and_then(|s| s.tray.as_mut()) {
                t.readd();
            }
        });
        runtime::refresh();
        return 0;
    }
    match msg {
        window::WAKE_MESSAGE => {
            log::info("app", "wake from second instance: show panel");
            crate::panel::show();
            0
        }
        WM_DESTROY => {
            unsafe { KillTimer(hwnd, 1) };
            state::with_mut(|st| {
                if let Some(s) = st {
                    if let Some(t) = s.tray.as_mut() {
                        t.remove();
                    }
                }
            });
            PostQuitMessage(0);
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_TIMER => {
            runtime::on_timer();
            0
        }
        tray::TRAY_CALLBACK => {
            let mouse_msg = (lparam & 0xFFFF) as u32;
            if log::enabled(Level::Trace) {
                log::trace("tray", &format!("callback 0x{mouse_msg:04X}"));
            }
            match mouse_msg {
                // 左键 = 看信息：只开/关只读面板，不做任何动作。
                0x0202 => crate::panel::toggle(),
                // 右键（含键盘菜单键）= 做功能：收起面板，直接弹原生菜单。
                0x0205 | 0x007B => {
                    crate::panel::hide();
                    commands::show_menu();
                }
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            commands::on_command((wparam & 0xFFFF) as usize);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}