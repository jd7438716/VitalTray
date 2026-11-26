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

    /// `--help`：标题 + 用法 + 每个选项一行，全部走 i18n（跟随界面语言）。
    pub fn help_text() -> String {
        const OPTIONS: &[&str] = &[
            "cli.opt.no_tray",
            "cli.opt.no_autostart",
            "cli.opt.portable",
            "cli.opt.config",
            "cli.opt.debug",
            "cli.opt.log_level",
            "cli.opt.self_test",
            "cli.opt.once",
            "cli.opt.dump",
            "cli.opt.sysmon",
            "cli.opt.panel",
            "cli.opt.watch",
            "cli.opt.watch_for",
            "cli.opt.watch_out",
            "cli.opt.feature_get",
            "cli.opt.capture_report",
            "cli.opt.version",
            "cli.opt.help",
        ];
        let mut s = format!("VitalTray {}\n\n{}\n\n", crate::VERSION, crate::i18n::t("cli.usage"));
        for key in OPTIONS {
            s.push_str("  ");
            s.push_str(crate::i18n::t(key));
            s.push('\n');
        }
        s
    }
}
