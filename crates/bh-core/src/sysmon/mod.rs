//! 系统状态采样线程：CPU / 内存 / 硬盘 / GPU + 温度。
//!
//! 单独线程每 2 秒采一次，UI 线程只读最新快照（不阻塞面板弹出、也不和 HID 轮询线程抢资源）。
//! 采样失败一律返回 None，面板显示 “—”，绝不编造数字。

mod cpu;
mod disk;
mod gpu;
mod kmt;
mod mem;
mod pdh;
mod smart;

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub use disk::DiskInfo;
pub use mem::MemInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInfo {
    pub name: String,
    pub use_percent: Option<u8>,
    pub temp_c: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub cpu_use: Option<u8>,
    pub cpu_temp: Option<u8>,
    pub mem: Option<MemInfo>,
    pub disk: Option<DiskInfo>,
    pub gpus: Vec<GpuInfo>,
    /// 所有适配器里的最高占用；适配器名缺失时用来兜底。
    pub gpu_use: Option<u8>,
}

static LATEST: OnceLock<Mutex<Snapshot>> = OnceLock::new();

fn latest_slot() -> &'static Mutex<Snapshot> {
    LATEST.get_or_init(|| Mutex::new(Snapshot::default()))
}

/// UI 线程读一份快照（拷贝很小）。
pub fn latest() -> Snapshot {
    latest_slot().lock().map(|s| s.clone()).unwrap_or_default()
}

pub fn spawn() -> std::thread::JoinHandle<()> {
    spawn_sampler(false)
}

/// 排障用（`--sysmon`）：除温度外也采 GPU 占用。
pub fn spawn_with_usage() -> std::thread::JoinHandle<()> {
    spawn_sampler(true)
}

fn spawn_sampler(gpu_usage: bool) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("bh-sysmon".into())
        .spawn(move || {
            let mut sampler = Sampler::new(gpu_usage);
            // 给速率型计数器一个采样间隔，首帧才有意义（见 pdh.rs）。
            std::thread::sleep(Duration::from_millis(600));
            loop {
                let snap = sampler.sample();
                if let Ok(mut slot) = latest_slot().lock() {
                    *slot = snap;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("spawn sysmon")
}

struct Sampler {
    cpu: cpu::CpuUsage,
    pdh: Option<pdh::Pdh>,
    gpus: gpu::Adapters,
}

impl Sampler {
    fn new(gpu_usage: bool) -> Self {
        let mut cpu = cpu::CpuUsage::default();
        // 先采一帧当基准，首次展示就有占用值。
        let _ = cpu.sample();
        Self { cpu, pdh: pdh::Pdh::new(gpu_usage), gpus: gpu::Adapters::default() }
    }

    fn sample(&mut self) -> Snapshot {
        let now = now_ms();
        let mut snap = Snapshot::default();
        snap.cpu_use = self.cpu.sample();
        snap.mem = mem::read();
        snap.disk = disk::system_disk();

        self.gpus.refresh_if_stale(now);
        let mut usage = std::collections::HashMap::new();
        if let Some(pdh) = &self.pdh {
            pdh.collect();
            snap.cpu_temp = pdh.thermal_c();
            usage = pdh.gpu_usage();
            snap.gpu_use = usage.values().copied().max();
        }
        snap.gpus = self.gpus.snapshot(&usage);
        snap
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_default_is_empty() {
        let s = Snapshot::default();
        assert_eq!(s.cpu_use, None);
        assert!(s.gpus.is_empty());
    }
}
