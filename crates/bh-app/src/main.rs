// 打包/安装版用 GUI 子系统：双击或开机自启不再弹控制台窗口。
// 带参数跑 CLI 时由 bh_core::console 挂回父控制台，输出照常。
#![windows_subsystem = "windows"]

fn main() {
    if std::env::args().nth(1).is_some() {
        bh_core::console::attach_parent_console();
    }
    std::process::exit(bh_core::run());
}
