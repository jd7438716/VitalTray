//! 全局状态（UI 线程独占）。

use crate::config::Config;
use crate::monitor::Monitor;
use crate::poller::DeviceReading;
use crate::tray::TrayIcon;
use std::cell::RefCell;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::HWND;

pub struct State {
    pub hwnd: HWND,
    pub readings: Vec<DeviceReading>,
    pub monitor: Monitor,
    pub low_battery: u8,
    pub rx: std::sync::mpsc::Receiver<Vec<DeviceReading>>,
    pub cfg: Config,
    pub exe: PathBuf,
    pub cfg_path: PathBuf,
    pub log_dir: PathBuf,
    pub tray: Option<TrayIcon>,
    pub tray_wanted: bool,
    pub autostart: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

pub fn install(state: State) {
    STATE.with(|s| *s.borrow_mut() = Some(state));
}

pub fn with<R>(f: impl FnOnce(Option<&State>) -> R) -> R {
    STATE.with(|s| f(s.borrow().as_ref()))
}

pub fn with_mut<R>(f: impl FnOnce(Option<&mut State>) -> R) -> R {
    STATE.with(|s| f(s.borrow_mut().as_mut()))
}
