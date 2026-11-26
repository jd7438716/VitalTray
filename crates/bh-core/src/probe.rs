//! 调试/排障命令：--once 读一次，--dump 列 HID 设备。

use crate::poller;

pub fn once() -> i32 {
    let readings = poller::collect_once();
    if readings.is_empty() {
        println!("未发现可读取的设备（插上无线鼠标/键盘，或本机没有电池）。");
        return 0;
    }
    for r in &readings {
        match &r.error {
            Some(e) => println!("{:<24} {:<8} 读取失败: {e}", r.name, r.domain),
            None => println!("{:<24} {:<8} {}%{}", r.name, r.domain, r.soc, if r.charging { " (充电中)" } else { "" }),
        }
    }
    0
}

pub fn dump_hid() -> i32 {
    match bh_hid::enumerate() {
        Ok(list) => {
            println!("共 {} 个 HID 设备：", list.len());
            for d in &list {
                let i = &d.info;
                println!(
                    "{:04x}:{:04x} usage={:04x}:{:04x} {:?} {}",
                    i.vendor_id, i.product_id, i.usage_page, i.usage, i.transport, i.product
                );
            }
            println!("\n蓝牙节点：");
            for line in crate::bt::debug_lines() {
                println!("  {line}");
            }
            0
        }
        Err(e) => {
            println!("枚举失败: {e}");
            1
        }
    }
}

/// 打印一次系统状态采样（排障：温度/GPU 是否可用）。
pub fn sysmon_once() -> i32 {
    crate::sysmon::spawn();
    std::thread::sleep(std::time::Duration::from_millis(1600));
    let s = crate::sysmon::latest();
    let m = crate::machine::info();
    println!("机器: {} {}", m.vendor, m.model);
    println!("CPU:  {}", m.cpu);
    println!(
        "CPU:  占用={} 温度={}",
        opt(s.cpu_use, |v| format!("{v}%")),
        opt(s.cpu_temp, |v| format!("{v}°C"))
    );
    if let Some(mem) = &s.mem {
        println!("内存: {}%  {}/{} MB", mem.percent, mem.used_mb, mem.total_mb);
    }
    if let Some(d) = &s.disk {
        println!("磁盘: {}: {}%  可用 {} GB / {} GB", d.letter, d.percent, d.free_mb / 1024, d.total_mb / 1024);
    }
    if s.gpus.is_empty() {
        println!("GPU:  未枚举到适配器（占用兜底={}）", opt(s.gpu_use, |v| format!("{v}%")));
    }
    for g in &s.gpus {
        println!("GPU:  {} 占用={} 温度={}", g.name, opt(g.use_percent, |v| format!("{v}%")), opt(g.temp_c, |v| format!("{v}°C")));
    }
    if s.cpu_temp.is_none() {
        println!("提示：本机 ACPI 热区未上报温度，或该机型不提供。");
    }
    0
}

fn opt<T>(v: Option<T>, f: impl FnOnce(T) -> String) -> String {
    v.map(f).unwrap_or_else(|| "—".to_string())
}

/// 只读监听某设备的输入报文：--watch 046d:c52b [--watch-for 180] [--watch-out file]
pub fn watch(target: &str, secs: u64, out: Option<&str>) -> i32 {
    let Some((v, p)) = target.split_once(':') else {
        println!("格式应为 vid:pid，例如 24ae:185a");
        return 1;
    };
    let (Ok(vid), Ok(pid)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) else {
        println!("VID/PID 需要 16 进制");
        return 1;
    };
    let mut log = String::new();
    let mut emit = |line: String| {
        println!("{line}");
        log.push_str(&line);
        log.push('\n');
    };

    let mut handles: Vec<(bh_hid::HidHandle, String)> = Vec::new();
    emit(format!("监听 {vid:04x}:{pid:04x}，{secs} 秒，只读。可开关设备/使用它来产生报文。"));

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
                        emit(format!("  + 接口 {name}"));
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
                        emit(format!("  ! {name} read error: {e:?}"));
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

    emit(format!("结束：{total_frames} 条报文，{} 个接口", handles.len()));
    if let Some(path) = out {
        if std::fs::write(path, &log).is_ok() {
            println!("已写入 {path}");
        }
    }
    if total_frames == 0 {
        println!("提示：设备空闲时不推报文。可开关设备电源，或在该设备上操作，然后重跑。");
    }
    0
}

/// 读一次 feature report：--feature-get 24ae:185a --feature-id 05
pub fn feature_get(target: &str, report_id: u8) -> i32 {
    let Some((v, p)) = target.split_once(':') else {
        println!("格式应为 vid:pid");
        return 1;
    };
    let (Ok(vid), Ok(pid)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) else {
        println!("VID/PID 需要 16 进制");
        return 1;
    };
    let Ok(list) = bh_hid::enumerate() else {
        println!("枚举失败");
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
    println!("no reply for report {report_id:#04x}");
    1
}

/// 分析自动抓帧日志：按"帧前缀"归组，找哪些字节稳定落在 0..=100（候选电量）。
pub fn capture_report(dir: &std::path::Path) -> i32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        println!("没有 capture 目录：{}", dir.display());
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
                println!("    候选电量字节: {}", candidates.join(", "));
            }
        }
    }
    println!("\n总计 {total} 帧");
    if total == 0 {
        println!("还没有抓到帧：设备空闲时不会主动上报。正常使用（移动/按键/开关电源）后会自动落盘。");
    }
    0
}
