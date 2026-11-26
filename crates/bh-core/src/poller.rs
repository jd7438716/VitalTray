//! 轮询：枚举设备 → 匹配插件 → 读取 → 汇总（后台线程）。

use bh_api::{DeviceDomain, DeviceInfo, Glyph, ReadContext, SystemInfo, Transport};
use crate::plugins;
use crate::system::HostSystem;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

static HID_HANDLES: OnceLock<Mutex<HashMap<String, bh_hid::HidHandle>>> = OnceLock::new();
static LAST_GOOD: OnceLock<Mutex<HashMap<String, (u8, bool, i64)>>> = OnceLock::new();
static WAKE: OnceLock<(Mutex<bool>, Condvar)> = OnceLock::new();

const STALE_MAX_MS: i64 = 30 * 60 * 1000;

fn wake() -> &'static (Mutex<bool>, Condvar) {
    WAKE.get_or_init(|| (Mutex::new(false), Condvar::new()))
}

/// 请求轮询线程立刻跑一轮（菜单/面板的“立即刷新”）。
pub fn request_refresh() {
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

fn last_good() -> &'static Mutex<HashMap<String, (u8, bool, i64)>> {
    LAST_GOOD.get_or_init(|| Mutex::new(HashMap::new()))
}

fn handles() -> &'static Mutex<HashMap<String, bh_hid::HidHandle>> {
    HID_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
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

/// 枚举 HID / 蓝牙 / 本机电源，生成候选设备（同一物理设备的多个 HID 集合会被合并）。
pub fn candidates() -> Vec<DeviceInfo> {
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
    // HID：同一 key 有多条 collection，保留"插件匹配度最高"的那条（其次看 vendor 页）
    let mut hid: std::collections::BTreeMap<String, (u8, u16, DeviceInfo)> = std::collections::BTreeMap::new();
    if let Ok(devs) = bh_hid::enumerate() {
        for d in devs {
            let score = plugins::domains()
                .iter()
                .map(|p| p.matches(&d.info) as u8)
                .max()
                .unwrap_or(0);
            let page = d.info.usage_page;
            let entry_key = format!("{}:{:04x}:{:04x}", d.info.key, d.info.usage_page, d.info.usage);
            let mut info = d.info.clone();
            info.key = entry_key.clone();
            hid.entry(entry_key)
                .and_modify(|cur| {
                    if score > cur.0 || (score == cur.0 && page > cur.1) {
                        *cur = (score, page, info.clone());
                    }
                })
                .or_insert((score, page, info));
        }
    }
    out.extend(hid.into_values().map(|(_, _, info)| info));
    out
}

fn best_domain(dev: &DeviceInfo) -> Option<&'static dyn DeviceDomain> {
    let mut best: Option<(&'static dyn DeviceDomain, u8)> = None;
    for d in plugins::domains() {
        let score = d.matches(dev) as u8;
        if score > 0 && best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((d, score));
        }
    }
    // 只有 Strong（>=2）才尝试读取，Weak 只作为"识别到但不读"的提示，避免无谓打开句柄
    best.filter(|(_, s)| *s >= 2).map(|(d, _)| d)
}

/// 跑一轮：返回每个设备的状态。
pub fn collect_once() -> Vec<DeviceReading> {
    let sys = HostSystem;
    let mut out = Vec::new();
    for dev in candidates() {
        let Some(domain) = best_domain(&dev) else { continue };
        let meta = domain.meta();
        let brand = if dev.transport == Transport::System {
            let m = crate::machine::info();
            if m.vendor.is_empty() { None } else { Some(m.vendor.clone()) }
        } else {
            crate::brand::vendor_name(dev.vendor_id).map(|s| s.to_string())
        };
        let mut result = DeviceReading {
            key: dev.key.clone(),
            name: dev.product.clone(),
            domain: meta.id.to_string(),
            glyph: domain.glyph(),
            brand,
            soc: 0,
            charging: false,
            error: None,
            stale: false,
        };
        if dev.transport == Transport::System {
            let mut ctx = ReadContext { hid: None, system: Some(&sys), now_ms: bh_hid::now_ms() };
            match domain.read(&dev, &mut ctx) {
                Ok(r) => { result.soc = r.soc; result.charging = r.charging.unwrap_or(false); }
                Err(e) => result.error = Some(format!("{e:?}")),
            }
            out.push(result);
            continue;
        }
        // 按 VID:PID + usage 精确找当前系统里的那个 collection
        let handle = bh_hid::enumerate().ok().and_then(|list| {
            list.into_iter()
                .filter(|d| {
                    d.info.vendor_id == dev.vendor_id
                        && d.info.product_id == dev.product_id
                        && d.info.usage_page == dev.usage_page
                        && d.info.usage == dev.usage
                })
                .max_by_key(|d| domain.matches(&d.info) as u8)
        });
        let Some(dev_obj) = handle else {
            result.error = Some("打开失败".into());
            out.push(result);
            continue;
        };
        // 句柄保持打开：Windows 会缓存打开期间收到的输入报告，下次轮询能读到
        let mut guard = handles().lock().unwrap_or_else(|e| e.into_inner());
        if !guard.contains_key(&dev.key) {
            match bh_hid::open(&dev_obj) {
                Ok(h) => {
                    guard.insert(dev.key.clone(), h);
                }
                Err(e) => {
                    result.error = Some(e);
                    out.push(result);
                    continue;
                }
            }
        }
        let h = guard.get_mut(&dev.key).expect("inserted");
        let mut ctx = ReadContext { hid: Some(h as &mut dyn bh_api::HidIo), system: None, now_ms: bh_hid::now_ms() };
        match domain.read(&dev, &mut ctx) {
            Ok(r) => {
                result.soc = r.soc;
                result.charging = r.charging.unwrap_or(false);
                last_good().lock().map(|mut m| m.insert(dev.key.clone(), (r.soc, result.charging, bh_hid::now_ms()))).ok();
            }
            Err(bh_api::DriverError::Timeout) => {
                let prev = last_good().lock().ok().and_then(|m| m.get(&dev.key).copied());
                match prev {
                    Some((soc, charging, at)) if bh_hid::now_ms() - at < STALE_MAX_MS => {
                        result.soc = soc;
                        result.charging = charging;
                        result.stale = true;
                    }
                    _ => result.error = Some("等待设备上报（用一下设备）".into()),
                }
            }
            Err(e) => {
                result.error = Some(format!("{e:?}"));
                guard.remove(&dev.key);
            }
        }
        drop(guard);
        out.push(result);
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
