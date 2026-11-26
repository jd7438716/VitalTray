//! 调试/排障命令：--once 读一次，--dump 列 HID 设备。
//!
//! 输出跟着界面语言走（`i18n`）：设备名、VID/PID、字节值这些事实保持原样，只有标签被翻译。

use crate::i18n::{t, tf};
use crate::poller;

pub fn once() -> i32 {
    let readings = poller::collect_once();
    if readings.is_empty() {
        println!("{}", t("cli.once.empty"));
        return 0;
    }
    for r in &readings {
        // 幽灵 = 从未成功上报过的通道（接收器常驻），面板不展示，排障时仍列出来。
        let tag = if r.ghost { t("cli.once.ghost") } else { "" };
        if r.pending {
            println!("{:<24} {:<8} {:<6} {}", r.name, r.domain, tag, t("panel.waiting"));
        } else if let Some(e) = &r.error {
            let line = tf("cli.once.failed", &[("error", e)]);
            println!("{:<24} {:<8} {:<6} {line}", r.name, r.domain, tag);
        } else {
            let charging = if r.charging { t("cli.once.charging") } else { "" };
            println!("{:<24} {:<8} {:<6} {}% {}", r.name, r.domain, tag, r.soc, charging);
        }
    }
    0
}

pub fn dump_hid() -> i32 {
    match bh_hid::enumerate() {
        Ok(list) => {
            let count = list.len().to_string();
            println!("{}", tf("cli.dump.header", &[("count", &count)]));
            for d in &list {
                let i = &d.info;
                println!(
                    "{:04x}:{:04x} usage={:04x}:{:04x} {:?} {}",
                    i.vendor_id, i.product_id, i.usage_page, i.usage, i.transport, i.product
                );
            }
            println!("\n{}", t("cli.dump.bluetooth"));
            for line in crate::bt::debug_lines() {
                println!("  {line}");
            }
            0
        }
        Err(e) => {
            println!("{}", tf("cli.dump.failed", &[("error", &e)]));
            1
        }
    }
}

/// 打印一次系统状态采样（排障：温度/GPU 是否可用）。
pub fn sysmon_once() -> i32 {
    crate::sysmon::spawn_with_usage();
    std::thread::sleep(std::time::Duration::from_millis(1600));
    let s = crate::sysmon::latest();
    let m = crate::machine::info();
    println!("{}", tf("cli.sysmon.machine", &[("vendor", &m.vendor), ("model", &m.model)]));
    println!("{}", tf("cli.sysmon.cpu", &[("model", &m.cpu)]));
    let cpu_use = pct(s.cpu_use);
    let cpu_temp = temp(s.cpu_temp);
    println!("{}", tf("cli.sysmon.cpu_line", &[("use", &cpu_use), ("temp", &cpu_temp)]));
    if let Some(mem) = &s.mem {
        let args = [
            ("percent", mem.percent.to_string()),
            ("used", mem.used_mb.to_string()),
            ("total", mem.total_mb.to_string()),
        ];
        println!("{}", tf("cli.sysmon.mem", &pairs(&args)));
    }
    if let Some(d) = &s.disk {
        let temp_c = temp(d.temp_c);
        let args = [
            ("letter", d.letter.to_string()),
            ("percent", d.percent.to_string()),
            ("free", (d.free_mb / 1024).to_string()),
            ("total", (d.total_mb / 1024).to_string()),
            ("temp", temp_c),
        ];
        println!("{}", tf("cli.sysmon.disk", &pairs(&args)));
    }
    if s.gpus.is_empty() {
        println!("{}", tf("cli.sysmon.gpu_none", &[("use", &pct(s.gpu_use))]));
    }
    for g in &s.gpus {
        let args = [("name", g.name.clone()), ("use", pct(g.use_percent)), ("temp", temp(g.temp_c))];
        println!("{}", tf("cli.sysmon.gpu", &pairs(&args)));
    }
    if s.cpu_temp.is_none() {
        println!("{}", t("cli.sysmon.hint_cpu"));
    }
    if s.disk.as_ref().is_some_and(|d| d.temp_c.is_none()) {
        println!("{}", t("cli.sysmon.hint_disk"));
    }
    0
}

fn pct(v: Option<u8>) -> String {
    v.map(|v| format!("{v}%")).unwrap_or_else(|| "—".to_string())
}

fn temp(v: Option<u8>) -> String {
    v.map(|v| format!("{v}°C")).unwrap_or_else(|| "—".to_string())
}

fn pairs<'a, const N: usize>(items: &'a [(&'static str, String); N]) -> Vec<(&'static str, &'a str)> {
    items.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

/// 只读监听某设备的输入报文：--watch 046d:c52b [--watch-for 180] [--watch-out file]
pub fn watch(target: &str, secs: u64, out: Option<&str>) -> i32 {
    let Some((v, p)) = target.split_once(':') else {
        println!("{}", tf("cli.watch.bad_target", &[("example", "24ae:185a")]));
        return 1;
    };
    let (Ok(vid), Ok(pid)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) else {
        println!("{}", t("cli.watch.bad_hex"));
        return 1;
    };
    let mut log = String::new();
    let mut emit = |line: String| {
        println!("{line}");
        log.push_str(&line);
        log.push('\n');
    };

    let mut handles: Vec<(bh_hid::HidHandle, String)> = Vec::new();
    emit(tf(
        "cli.watch.start",
        &[
            ("vid", &format!("{vid:04x}")),
            ("pid", &format!("{pid:04x}")),
            ("secs", &secs.to_string()),
        ],
    ));

    let start = std::time::Instant::now();
    let mut total_frames = 0usize;
    let mut buf = [0u8; 64];
    let mut last_scan = std::time::Instant::now();
    let mut errs_shown = 0;

    while start.elapsed().as_secs() < secs {
        // 每 5 秒补一次句柄（设备重连/唤醒后会重新枚举）
        if handles.is_empty() || last_scan.elapsed().as_secs() >= 5 {
            last_scan = std::time::Instant::now();
            if let Ok(list) = bh_hid::enumerate() {
                let mut have = std::collections::HashSet::new();
                for (_, name) in &handles {
                    have.insert(name.clone());
                }
                for d in list {
                    if d.info.vendor_id != vid || d.info.product_id != pid {
                        continue;
                    }
                    let name = format!("{:04x}:{:04x}", d.info.usage_page, d.info.usage);
                    if have.contains(&name) {
                        continue;
                    }
                    if let Ok(h) = bh_hid::open(&d) {
                        emit(tf("cli.watch.iface", &[("name", &name)]));
                        handles.push((h, name));
                    }
                }
            }
        }

        let mut got_any = false;
        for (h, name) in handles.iter_mut() {
            match bh_api::HidIo::read_timeout(h, &mut buf, 50) {
                Err(e) => {
                    if errs_shown < 5 {
                        errs_shown += 1;
                        let err = format!("{e:?}");
                        emit(tf("cli.watch.read_error", &[("name", name), ("error", &err)]));
                    }
                }
                Ok(n) if n > 0 => {
                    got_any = true;
                    total_frames += 1;
                    let hex: Vec<String> = buf[..n].iter().map(|b| format!("{b:02x}")).collect();
                    emit(format!("[{:>7.2}s] {name} {n:02} bytes: {}", start.elapsed().as_secs_f64(), hex.join(" ")));
                }
                _ => {}
            }
        }
        if !got_any {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    emit(tf(
        "cli.watch.end",
        &[("frames", &total_frames.to_string()), ("ifaces", &handles.len().to_string())],
    ));
    if let Some(path) = out {
        if std::fs::write(path, &log).is_ok() {
            println!("{}", tf("cli.watch.written", &[("path", path)]));
        }
    }
    if total_frames == 0 {
        println!("{}", t("cli.watch.hint"));
    }
    0
}

/// 读一次 feature report：--feature-get 24ae:185a --feature-id 05
pub fn feature_get(target: &str, report_id: u8) -> i32 {
    let Some((v, p)) = target.split_once(':') else {
        println!("{}", t("cli.feature.bad_target"));
        return 1;
    };
    let (Ok(vid), Ok(pid)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) else {
        println!("{}", t("cli.feature.bad_hex"));
        return 1;
    };
    let Ok(list) = bh_hid::enumerate() else {
        println!("{}", t("cli.feature.no_enum"));
        return 1;
    };
    let mut cands: Vec<_> = list.into_iter().filter(|d| d.info.vendor_id == vid && d.info.product_id == pid).collect();
    cands.sort_by_key(|d| std::cmp::Reverse(d.info.usage_page));
    for d in cands {
        let Ok(mut h) = bh_hid::open(&d) else { continue };
        let mut buf = [0u8; 64];
        if let Ok(n) = bh_api::HidIo::feature_get(&mut h, report_id, &mut buf) {
            if n > 0 {
                let hex: Vec<String> = buf[..n].iter().map(|b| format!("{b:02x}")).collect();
                println!("usage={:04x}:{:04x} len={n} {}", d.info.usage_page, d.info.usage, hex.join(" "));
                return 0;
            }
        }
    }
    println!("{}", tf("cli.feature.no_reply", &[("id", &format!("{report_id:#04x}"))]));
    1
}

/// 分析自动抓帧日志：按"帧前缀"归组，找哪些字节稳定落在 0..=100（候选电量）。
pub fn capture_report(dir: &std::path::Path) -> i32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        println!("{}", tf("cli.capture.no_dir", &[("dir", &dir.display().to_string())]));
        return 1;
    };
    let mut total = 0usize;
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("log") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let mut groups: std::collections::BTreeMap<String, Vec<Vec<u8>>> = std::collections::BTreeMap::new();
        for line in text.lines() {
            let mut it = line.split_whitespace();
            let Some(_len) = it.next() else { continue };
            let bytes: Vec<u8> = it.filter_map(|h| u8::from_str_radix(h, 16).ok()).collect();
            if bytes.len() < 4 {
                continue;
            }
            let key: String = bytes[..3].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            groups.entry(key).or_default().push(bytes);
        }
        if groups.is_empty() {
            continue;
        }
        println!("\n== {} ==", path.file_name().and_then(|s| s.to_str()).unwrap_or("?"));
        for (prefix, frames) in groups.iter().take(12) {
            total += frames.len();
            let len = frames[0].len();
            let mut candidates = Vec::new();
            for pos in 1..len.min(10) {
                let vals: Vec<u8> = frames.iter().map(|f| f[pos]).collect();
                let min = *vals.iter().min().unwrap();
                let max = *vals.iter().max().unwrap();
                let distinct = vals.iter().collect::<std::collections::BTreeSet<_>>().len();
                if max <= 100 && min >= 1 {
                    candidates.push(format!("byte{pos}=[{min}..{max}] n={distinct}"));
                }
            }
            let sample: Vec<String> = frames.iter().take(2).map(|f| f.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")).collect();
            println!("  prefix[{prefix}] frames={} len={len}", frames.len());
            println!("    sample: {}", sample.join(" | "));
            if !candidates.is_empty() {
                println!("    {}", tf("cli.capture.candidates", &[("list", &candidates.join(", "))]));
            }
        }
    }
    println!("\n{}", tf("cli.capture.total", &[("count", &total.to_string())]));
    if total == 0 {
        println!("{}", t("cli.capture.empty"));
    }
    0
}
