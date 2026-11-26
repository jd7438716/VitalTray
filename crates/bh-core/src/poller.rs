//! 轮询：一次枚举 → 协议认领 → 并行读 → 汇总（后台线程）。
//!
//! 三条硬约束（v1.0.0 的教训）：
//! 1. **每轮只枚举一次**。旧实现给每个设备又枚举一遍，白花 30ms/设备。
//! 2. **并行读**，一轮总耗时 = max(budget) 而不是 sum。两个雷柏通道不再互相排队。
//! 3. **静默设备指数退避**，且从未成功上报过的通道（接收器常驻的幽灵通道）不建行。

use bh_api::{
    DeviceDomain, DeviceInfo, DriverError, Glyph, Mode, ReadContext, Reading, SystemInfo, Transport,
    VendorProtocol,
};
use crate::brand;
use crate::plugins;
use crate::registry;
use crate::system::HostSystem;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

static HID_HANDLES: OnceLock<Mutex<HashMap<String, bh_hid::HidHandle>>> = OnceLock::new();
static LAST_GOOD: OnceLock<Mutex<HashMap<String, (u8, bool, i64)>>> = OnceLock::new();
static PROBES: OnceLock<Mutex<HashMap<String, Probe>>> = OnceLock::new();
static WAKE: OnceLock<(Mutex<bool>, Condvar)> = OnceLock::new();

const STALE_MAX_MS: i64 = 30 * 60 * 1000;
const PASSIVE_BUDGET_MS: u64 = 6000;
const FEATURE_BUDGET_MS: u64 = 2000;
/// 静默设备的退避阶梯（毫秒）：1min → 2min → 5min → 5min…
const BACKOFF_MS: [i64; 4] = [60_000, 120_000, 300_000, 300_000];

fn wake() -> &'static (Mutex<bool>, Condvar) {
    WAKE.get_or_init(|| (Mutex::new(false), Condvar::new()))
}

fn last_good() -> &'static Mutex<HashMap<String, (u8, bool, i64)>> {
    LAST_GOOD.get_or_init(|| Mutex::new(HashMap::new()))
}

fn handles() -> &'static Mutex<HashMap<String, bh_hid::HidHandle>> {
    HID_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn probes() -> &'static Mutex<HashMap<String, Probe>> {
    PROBES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 单通道的探测状态。
#[derive(Debug, Clone, Copy, Default)]
struct Probe {
    /// 成功读到过一次 —— 只有 true 的通道才建行（幽灵行抑制）。
    ok_once: bool,
    /// 连续失败次数，用作退避阶梯下标。
    fails: u8,
    /// 下次允许尝试的时间戳；0 = 立即可试。
    next_at: i64,
}

fn probe_of(key: &str) -> Probe {
    probes().lock().map(|m| m.get(key).copied()).unwrap_or_default().unwrap_or_default()
}

fn set_probe(key: &str, f: impl FnOnce(&mut Probe)) {
    if let Ok(mut m) = probes().lock() {
        f(m.entry(key.to_string()).or_default());
    }
}

/// 请求轮询线程立刻跑一轮（菜单/面板的“立即刷新”），并解除所有退避。
pub fn request_refresh() {
    if let Ok(mut m) = probes().lock() {
        for p in m.values_mut() {
            p.next_at = 0;
        }
    }
    let (lock, cv) = wake();
    if let Ok(mut pending) = lock.lock() {
        *pending = true;
        cv.notify_all();
    }
}

fn sleep_or_wake(secs: u64) {
    let (lock, cv) = wake();
    let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    if *guard {
        *guard = false;
        return;
    }
    let (mut guard, _) = cv.wait_timeout(guard, Duration::from_secs(secs)).unwrap_or_else(|e| e.into_inner());
    *guard = false;
}

#[derive(Debug, Clone)]
pub struct DeviceReading {
    pub key: String,
    pub name: String,
    pub domain: String,
    pub glyph: Glyph,
    /// 品牌短名（VID 映射或整机厂商）；只用于展示。
    pub brand: Option<String>,
    pub soc: u8,
    pub charging: bool,
    pub error: Option<String>,
    pub stale: bool,
    /// 从未成功上报过该通道（接收器常驻的幽灵通道）：面板不展示，排障命令仍可见。
    pub ghost: bool,
}

impl DeviceReading {
    pub fn summary(&self) -> String {
        let icon = match self.glyph {
            Glyph::Mouse => "鼠标",
            Glyph::Keyboard => "键盘",
            Glyph::Headset => "耳机",
            Glyph::Gamepad => "手柄",
            Glyph::Laptop => "本机",
            Glyph::Phone => "手机",
            Glyph::Unknown => "设备",
        };
        match &self.error {
            Some(_) => format!("{icon} —"),
            None if self.stale => format!("{icon} {}%（上次）", self.soc),
            None if self.charging => format!("{icon} {}%⚡", self.soc),
            None => format!("{icon} {}%", self.soc),
        }
    }
}

fn system_device() -> DeviceInfo {
    DeviceInfo {
        vendor_id: 0,
        product_id: 0,
        product: "本机电池".to_string(),
        serial: None,
        usage_page: 0,
        usage: 0,
        transport: Transport::System,
        key: "sys:main".to_string(),
    }
}

/// 非 HID 候选（本机电源 + 蓝牙），由域插件直接读，不需要 HID 句柄。
fn non_hid_candidates() -> Vec<DeviceInfo> {
    let mut out = Vec::new();
    if HostSystem.battery().is_some() {
        out.push(system_device());
    }
    for d in crate::bt::list() {
        out.push(DeviceInfo {
            vendor_id: 0,
            product_id: 0,
            product: d.name.clone(),
            serial: None,
            usage_page: 0,
            usage: 0,
            transport: Transport::Bluetooth,
            key: d.key.clone(),
        });
    }
    out
}

fn best_domain(dev: &DeviceInfo) -> Option<&'static dyn DeviceDomain> {
    let mut best: Option<(&'static dyn DeviceDomain, u8)> = None;
    for d in plugins::domains().iter().copied() {
        let score = d.matches(dev) as u8;
        if score > 0 && best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((d, score));
        }
    }
    // 只有 Strong（>=2）才尝试读取，Weak 只作为"识别到但不读"的提示。
    best.filter(|(_, s)| *s >= 2).map(|(d, _)| d)
}

fn budget_for(mode: Mode) -> Duration {
    match mode {
        Mode::Passive => Duration::from_millis(PASSIVE_BUDGET_MS),
        Mode::Feature => Duration::from_millis(FEATURE_BUDGET_MS),
    }
}

fn domain_name(glyph: Glyph) -> String {
    match glyph {
        Glyph::Mouse => "mouse",
        Glyph::Keyboard => "keyboard",
        Glyph::Laptop => "laptop",
        _ => "device",
    }
    .to_string()
}

struct Job {
    device: bh_hid::Device,
    info: DeviceInfo,
    proto: &'static dyn VendorProtocol,
    budget: Duration,
    row: DeviceReading,
}

/// 跑一轮：返回每个设备的状态。
pub fn collect_once() -> Vec<DeviceReading> {
    let now = bh_hid::now_ms();
    let sys = HostSystem;
    let mut out: Vec<DeviceReading> = Vec::new();

    // 一、本机 / 蓝牙：域插件直接读。
    for dev in non_hid_candidates() {
        let Some(domain) = best_domain(&dev) else { continue };
        let brand = if dev.transport == Transport::System {
            let m = crate::machine::info();
            if m.vendor.is_empty() { None } else { Some(m.vendor.clone()) }
        } else {
            brand::vendor_name(dev.vendor_id).map(|s| s.to_string())
        };
        let mut row = DeviceReading {
            key: dev.key.clone(),
            name: dev.product.clone(),
            domain: domain.meta().id.to_string(),
            glyph: domain.glyph(),
            brand,
            soc: 0,
            charging: false,
            error: None,
            stale: false,
            ghost: false,
        };
        let mut ctx = ReadContext { hid: None, system: Some(&sys), now_ms: now };
        match domain.read(&dev, &mut ctx) {
            Ok(r) => {
                row.soc = r.soc;
                row.charging = r.charging.unwrap_or(false);
            }
            Err(e) => {
                row.error = Some(format!("{e:?}"));
                row.ghost = true;
            }
        }
        out.push(row);
    }

    // 二、HID：只枚举一次，交给协议注册表认领。
    let mut seen: HashSet<String> = HashSet::new();
    let mut jobs: Vec<Job> = Vec::new();
    for device in bh_hid::enumerate().unwrap_or_default() {
        let Some((proto, ep)) = registry::claim(&device.info) else { continue };
        // 同一通道可能有多条 collection（接收器常见），按 page:usage 去重。
        let key = format!("{}:{:04x}:{:04x}", device.info.key, device.info.usage_page, device.info.usage);
        if !seen.insert(key.clone()) {
            continue;
        }
        let mut info = device.info.clone();
        info.key = key.clone();
        let probe = probe_of(&key);
        let mut row = DeviceReading {
            key: key.clone(),
            name: info.product.clone(),
            domain: domain_name(ep.glyph),
            glyph: ep.glyph,
            brand: brand::vendor_name(info.vendor_id).map(|s| s.to_string()),
            soc: 0,
            charging: false,
            error: None,
            stale: false,
            ghost: !probe.ok_once,
        };
        // 退避中：不碰设备，直接沿用上次读数或标为待上报。
        if probe.next_at > now {
            if probe.ok_once {
                if let Ok(m) = last_good().lock() {
                    if let Some(&(soc, charging, at)) = m.get(&key) {
                        row.soc = soc;
                        row.charging = charging;
                        row.stale = now - at < STALE_MAX_MS;
                    }
                }
                if row.soc == 0 && row.error.is_none() {
                    row.error = Some("等待设备上报（用一下设备）".into());
                }
            } else {
                row.error = Some("等待设备上报（用一下设备）".into());
            }
            out.push(row);
            continue;
        }
        jobs.push(Job { device, info, proto, budget: budget_for(ep.mode), row });
    }

    out.extend(run_jobs(jobs));
    out
}

/// 取出句柄 → 并行读 → 归还句柄并落库。
fn run_jobs(jobs: Vec<Job>) -> Vec<DeviceReading> {
    if jobs.is_empty() {
        return Vec::new();
    }
    // 句柄只在准备阶段进锁，读取阶段完全并行。
    let mut prepared: Vec<(Job, Option<bh_hid::HidHandle>)> = Vec::with_capacity(jobs.len());
    {
        let mut guard = handles().lock().unwrap_or_else(|e| e.into_inner());
        for job in jobs {
            let hit = guard.remove(&job.info.key);
            let h = hit.or_else(|| bh_hid::open(&job.device).ok());
            prepared.push((job, h));
        }
    }

    let done: Vec<(Job, Option<bh_hid::HidHandle>, Result<Reading, DriverError>)> = std::thread::scope(|s| {
        let mut join = Vec::with_capacity(prepared.len());
        for (job, h) in prepared {
            join.push(s.spawn(move || {
                let mut h = h;
                let res = match h.as_mut() {
                    Some(io) => job.proto.read(&job.info, io, job.budget),
                    None => Err(DriverError::Io("打开失败".into())),
                };
                (job, h, res)
            }));
        }
        join.into_iter().filter_map(|t| t.join().ok()).collect()
    });

    let now = bh_hid::now_ms();
    let mut out = Vec::with_capacity(done.len());
    for (job, h, res) in done {
        let key = job.row.key.clone();
        let mut row = job.row;
        match res {
            Ok(r) => {
                row.soc = r.soc;
                row.charging = r.charging.unwrap_or(false);
                row.ghost = false;
                if let Ok(mut m) = last_good().lock() {
                    m.insert(key.clone(), (r.soc, row.charging, now));
                }
                set_probe(&key, |p| {
                    p.ok_once = true;
                    p.fails = 0;
                    p.next_at = 0;
                });
                if let (Some(h), Ok(mut g)) = (h, handles().lock()) {
                    g.insert(key, h);
                }
            }
            Err(DriverError::Timeout) => {
                let prev = last_good().lock().ok().and_then(|m| m.get(&key).copied());
                let ok_once = probe_of(&key).ok_once;
                set_probe(&key, |p| {
                    p.fails = (p.fails + 1).min(BACKOFF_MS.len() as u8 - 1);
                    p.next_at = now + BACKOFF_MS[p.fails.saturating_sub(1) as usize];
                });
                match prev {
                    Some((soc, charging, at)) if ok_once && now - at < STALE_MAX_MS => {
                        row.soc = soc;
                        row.charging = charging;
                        row.stale = true;
                        if let (Some(h), Ok(mut g)) = (h, handles().lock()) {
                            g.insert(key, h);
                        }
                    }
                    _ => row.error = Some("等待设备上报（用一下设备）".into()),
                }
            }
            Err(e) => {
                // 非超时错误多半是句柄失效：丢掉，下轮重开。
                row.error = Some(format!("{e:?}"));
            }
        }
        out.push(row);
    }
    out
}

/// 后台轮询线程：每隔 interval 秒跑一轮，结果通过 channel 交给 UI。
/// 期间可被 `request_refresh()` 提前唤醒（菜单“立即刷新”）。
pub fn spawn(interval_secs: u32, tx: Sender<Vec<DeviceReading>>) -> std::thread::JoinHandle<()> {
    let secs = interval_secs.clamp(10, 3600) as u64;
    std::thread::Builder::new()
        .name("bh-poller".into())
        .spawn(move || loop {
            let readings = collect_once();
            if tx.send(readings).is_err() {
                break;
            }
            sleep_or_wake(secs);
        })
        .expect("spawn poller")
}
