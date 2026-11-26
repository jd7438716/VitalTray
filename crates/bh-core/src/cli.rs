//! 命令行参数（对齐参考项目的调试开关风格）。

#[derive(Debug, Clone, Default)]
pub struct Args {
    pub no_tray: bool,
    pub no_autostart: bool,
    pub portable: bool,
    pub debug: bool,
    pub log_level: Option<String>,
    pub self_test: bool,
    pub once: bool,
    pub dump: bool,
    pub sysmon: bool,
    pub panel: bool,
    pub no_capture: bool,
    pub capture_report: bool,
    pub watch: Option<String>,
    pub watch_for: u64,
    pub watch_out: Option<String>,
    pub feature_get: Option<String>,
    pub feature_id: u8,
    pub config: Option<String>,
    pub help: bool,
    pub version: bool,
}

impl Args {
    pub fn parse<I: IntoIterator<Item = String>>(it: I) -> Self {
        let mut a = Args::default();
        let mut rest = it.into_iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--no-tray" => a.no_tray = true,
                "--no-autostart" => a.no_autostart = true,
                "--portable" => a.portable = true,
                "--debug" => a.debug = true,
                "--log-level" => a.log_level = rest.next(),
                "--self-test" => a.self_test = true,
                "--once" => a.once = true,
                "--dump" => a.dump = true,
                "--sysmon" => a.sysmon = true,
                "--panel" => a.panel = true,
                "--no-capture" => a.no_capture = true,
                "--capture-report" => a.capture_report = true,
                "--watch" => a.watch = rest.next(),
                "--watch-for" => a.watch_for = rest.next().and_then(|s| s.parse().ok()).unwrap_or(180),
                "--watch-out" => a.watch_out = rest.next(),
                "--feature-get" => a.feature_get = rest.next(),
                "--feature-id" => a.feature_id = rest.next().and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(0),
                "--config" => a.config = rest.next(),
                "--help" | "-h" | "/?" => a.help = true,
                "--version" | "-V" => a.version = true,
                _ => {}
            }
        }
        a
    }

    pub fn help_text() -> String {
        let mut s = String::new();
        s.push_str("BatteryHub ");
        s.push_str(crate::VERSION);
        s.push('\n');
        s.push_str("\n用法: batteryhub.exe [options]\n\n");
        s.push_str("  --no-tray        不创建托盘图标（调试用）\n");
        s.push_str("  --no-autostart   本次运行不修改开机自启设置\n");
        s.push_str("  --portable       便携模式：配置与日志放在 exe 同目录\n");
        s.push_str("  --config <path>  指定配置文件路径\n");
        s.push_str("  --debug          等同 --log-level debug\n");
        s.push_str("  --log-level <lvl> error|warn|info|debug|trace\n");
        s.push_str("  --self-test      无界面自检，返回码 0 表示通过\n");
        s.push_str("  --once           枚举并读取一次，打印结果后退出\n");
        s.push_str("  --dump           列出所有 HID 设备（VID/PID/usage）后退出\n");
        s.push_str("  --sysmon         采样一次 CPU/内存/磁盘/GPU/温度并打印后退出\n");
        s.push_str("  --panel          启动后立即在鼠标位置弹出状态面板（调试/截图）\n");
        s.push_str("  --watch <vid:pid> 只读监听该设备的 HID 输入报文（默认 180 秒）\n");
        s.push_str("  --watch-for <sec> 监听时长，默认 180\n");
        s.push_str("  --watch-out <file> 把监听结果同时写入文件\n");
        s.push_str("  --feature-get <vid:pid> --feature-id <hex>  读一次 feature report（排障）\n");
        s.push_str("  --capture-report 分析 capture/ 下的自动抓帧日志（找候选电量字节）\n");
        s.push_str("  --version, -V    打印版本\n");
        s.push_str("  --help, -h       打印本帮助\n");
        s
    }
}
