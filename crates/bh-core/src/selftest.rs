//! 无界面自检：CI / 首次安装验证用（不创建窗口）。

use crate::{autostart, cli::Args, config::Config, paths, plugins};

pub fn run(args: &Args) -> i32 {
    let mut failed = 0;
    let dir = std::env::temp_dir().join("bh-selftest");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    let cfg_path = dir.join("config.ini");
    let mut cfg = Config::default();
    cfg.autostart = false;
    cfg.refresh_secs = 90;
    match cfg.save(&cfg_path) {
        Ok(()) => println!("PASS config write"),
        Err(e) => {
            println!("FAIL config write: {e}");
            failed += 1;
        }
    }
    if Config::load(&cfg_path) == cfg {
        println!("PASS config roundtrip");
    } else {
        println!("FAIL config roundtrip");
        failed += 1;
    }

    println!("INFO autostart registry = {:?}", autostart::current());
    println!("INFO exe = {:?}", std::env::current_exe());
    println!("INFO config dir = {:?}", paths::data_dir(args.portable));

    let names = plugins::describe();
    if names.len() >= 3 {
        println!("PASS plugins: {}", names.join(", "));
    } else {
        println!("FAIL plugins: {names:?}");
        failed += 1;
    }

    if std::env::var_os("LOCALAPPDATA").is_some() {
        println!("PASS localappdata");
    } else {
        println!("FAIL localappdata");
        failed += 1;
    }

    let _ = std::fs::remove_dir_all(&dir);
    if failed == 0 {
        println!("SELFTEST OK");
        0
    } else {
        println!("SELFTEST FAILED ({failed})");
        1
    }
}
