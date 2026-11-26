//! 自动抓帧：对"没有插件认领"的厂商 HID 集合持续只读监听，写日志供逆向。
//!
//! 默认常开（可在配置里关闭）：每 200ms 读一次，文件 1 MB 滚动。
//! 目的：用户平时正常使用设备时，报文会自动落盘，不必手动跑 CLI。

use crate::plugins;
use std::path::{Path, PathBuf};

const MAX_FILE: u64 = 1024 * 1024;
const LOG_EVERY: u32 = 20; // 每 20 条帧写一行 app.log，提示"抓到了"

fn safe(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn append(dir: &Path, name: &str, line: &str) {
    let path: PathBuf = dir.join(name);
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_FILE {
            let _ = std::fs::remove_file(&path);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

fn unsupported(dev: &bh_api::DeviceInfo) -> bool {
    if dev.usage_page < 0xFF00 {
        return false;
    }
    plugins::domains().iter().all(|d| d.matches(dev) as u8 == 0)
}

pub fn spawn(dir: PathBuf) {
    std::thread::Builder::new()
        .name("bh-capture".into())
        .spawn(move || {
            let _ = std::fs::create_dir_all(&dir);
            let mut handles: Vec<(bh_hid::HidHandle, String, String)> = Vec::new();
            let mut buf = [0u8; 64];
            let mut counter: u32 = 0;
            let mut last_scan = std::time::Instant::now();

            loop {
                if last_scan.elapsed().as_secs() >= 10 {
                    last_scan = std::time::Instant::now();
                    if let Ok(list) = bh_hid::enumerate() {
                        let mut have: std::collections::HashSet<String> =
                            handles.iter().map(|(_, name, _)| name.clone()).collect();
                        for d in list {
                            if !unsupported(&d.info) {
                                continue;
                            }
                            let name = format!(
                                "{:04x}_{:04x}_{:04x}_{:04x}",
                                d.info.vendor_id, d.info.product_id, d.info.usage_page, d.info.usage
                            );
                            if have.contains(&name) {
                                continue;
                            }
                            if let Ok(h) = bh_hid::open(&d) {
                                have.insert(name.clone());
                                handles.push((h, name.clone(), safe(&name)));
                                crate::log::info("capture", &format!("watching {name}"));
                            }
                        }
                    }
                }

                for (h, name, file) in handles.iter_mut() {
                    if let Ok(n) = bh_api::HidIo::read_timeout(h, &mut buf, 200) {
                        if n > 0 {
                            counter += 1;
                            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{b:02x}")).collect();
                            append(&dir, &format!("{file}.log"), &format!("{} {}", n, hex.join(" ")));
                            if counter % LOG_EVERY == 0 {
                                crate::log::debug("capture", &format!("{counter} frames from {name}"));
                            }
                        }
                    }
                }
            }
        })
        .ok();
}
