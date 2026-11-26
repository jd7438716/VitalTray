//! 自动抓帧：对**白名单厂商**的厂商 HID 集合持续只读监听，写日志供逆向。
//!
//! 默认关闭：只有配置 `[capture] vids=` 里列了 VID 才启动（例如 `vids=09da,24ae`）。
//! 目的：用户平时正常使用设备时，报文会自动落盘，不必手动跑 CLI。
//!
//! 三条硬约束（v1.0.0 曾因违反第 1 条吃满一个 CPU 核）：
//! 1. 循环体无论如何都要睡 —— 没有可读数据时退避，绝不空转。
//! 2. 读失败的句柄立即摘掉并隔离，错误必须进日志（不吞）。
//! 3. 只抓显式白名单里的 VID，不对全系统的厂商集合"顺便"开句柄。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_FILE: u64 = 1024 * 1024;
const LOG_EVERY: u32 = 20; // 每 20 条帧写一行 app.log，提示"抓到了"
const RESCAN: Duration = Duration::from_secs(5);
const IDLE_SLEEP: Duration = Duration::from_millis(100);
const QUARANTINE: Duration = Duration::from_secs(60);
const READ_TIMEOUT_MS: u32 = 200;

/// 解析 VID 白名单：`24ae,09da` 或 `0x24ae 0x09da`；空串 = 不抓。
pub fn parse_watch(spec: &str) -> Vec<u16> {
    let mut out: Vec<u16> = spec
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .map(|s| s.trim().trim_start_matches("0x").trim_start_matches("0X"))
        .filter(|s| !s.is_empty())
        .filter_map(|s| u16::from_str_radix(s, 16).ok())
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

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

struct Slot {
    handle: bh_hid::HidHandle,
    name: String,
    file: String,
}

/// 启动抓帧线程。`vids` 为空时直接不启动（默认行为）。
pub fn spawn(dir: PathBuf, vids: Vec<u16>) {
    if vids.is_empty() {
        return;
    }
    std::thread::Builder::new()
        .name("bh-capture".into())
        .spawn(move || run(dir, vids))
        .ok();
}

fn run(dir: PathBuf, vids: Vec<u16>) {
    let _ = std::fs::create_dir_all(&dir);
    let mut slots: Vec<Slot> = Vec::new();
    let mut quarantine: HashMap<String, Instant> = HashMap::new();
    let mut buf = [0u8; 64];
    let mut counter: u32 = 0;
    // 首轮立刻扫，之后每 RESCAN 一次（顺带回捞隔离到期的集合）。
    let mut last_scan = Instant::now() - RESCAN;

    loop {
        if last_scan.elapsed() >= RESCAN {
            last_scan = Instant::now();
            rescan(&vids, &mut slots, &mut quarantine);
        }

        let mut got = false;
        let mut i = 0;
        while i < slots.len() {
            match bh_api::HidIo::read_timeout(&mut slots[i].handle, &mut buf, READ_TIMEOUT_MS) {
                Ok(n) if n > 0 => {
                    got = true;
                    counter += 1;
                    let hex: Vec<String> = buf[..n].iter().map(|b| format!("{b:02x}")).collect();
                    append(&dir, &format!("{}.log", slots[i].file), &format!("{} {}", n, hex.join(" ")));
                    if counter % LOG_EVERY == 0 {
                        crate::log::debug("capture", &format!("{counter} frames from {}", slots[i].name));
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    // 读不了的集合（I2C-HID / 被独占）直接摘掉并隔离，绝不重试打成死循环。
                    let slot = slots.swap_remove(i);
                    crate::log::debug("capture", &format!("drop {}: {e:?}", slot.name));
                    quarantine.insert(slot.name, Instant::now());
                    continue;
                }
            }
            i += 1;
        }

        // 这一轮什么都没读到 —— 必须睡，否则就是 v1.0.0 的空转事故。
        if !got {
            std::thread::sleep(IDLE_SLEEP);
        }
    }
}

fn rescan(vids: &[u16], slots: &mut Vec<Slot>, quarantine: &mut HashMap<String, Instant>) {
    let Ok(list) = bh_hid::enumerate() else { return };
    let mut have: std::collections::HashSet<String> = slots.iter().map(|s| s.name.clone()).collect();
    for d in list {
        let info = &d.info;
        // 只抓白名单厂商的厂商页集合；标准用法页不参与逆向。
        if !vids.contains(&info.vendor_id) || info.usage_page < 0xFF00 {
            continue;
        }
        let name = format!(
            "{:04x}_{:04x}_{:04x}_{:04x}",
            info.vendor_id, info.product_id, info.usage_page, info.usage
        );
        if have.contains(&name) {
            continue;
        }
        if let Some(at) = quarantine.get(&name) {
            if at.elapsed() < QUARANTINE {
                continue;
            }
            quarantine.remove(&name);
        }
        if let Ok(h) = bh_hid::open(&d) {
            have.insert(name.clone());
            slots.push(Slot { handle: h, name: name.clone(), file: safe(&name) });
            crate::log::info("capture", &format!("watching {name}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_watch_accepts_common_forms() {
        assert_eq!(parse_watch(""), Vec::<u16>::new());
        assert_eq!(parse_watch("24ae"), vec![0x24ae]);
        assert_eq!(parse_watch("0x24AE, 09da"), vec![0x09da, 0x24ae]);
        assert_eq!(parse_watch("09da;09da  zz"), vec![0x09da]);
    }
}
