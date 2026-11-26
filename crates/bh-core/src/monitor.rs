//! 把"读数"变成"历史 + 估算 + 低电提醒"。

use crate::estimate::{self, Estimate};
use crate::history::{Sample, Store};
use crate::poller::DeviceReading;
use std::collections::HashMap;
use std::path::Path;

pub struct Alert {
    pub name: String,
    pub soc: u8,
}

pub struct Monitor {
    store: Store,
    estimates: HashMap<String, Option<Estimate>>,
    notified: HashMap<String, (i64, u8)>,
}

impl Monitor {
    pub fn new(dir: &Path) -> Self {
        Self {
            store: Store::new(dir),
            estimates: HashMap::new(),
            notified: HashMap::new(),
        }
    }

    /// 记录读数、更新估算，返回需要提醒的低电设备。
    pub fn observe(&mut self, readings: &[DeviceReading], threshold: u8, now: i64) -> Vec<Alert> {
        let mut alerts = Vec::new();
        for r in readings {
            if r.error.is_some() || r.stale {
                continue;
            }
            let _ = self.store.record(&r.key, Sample { ts: now, soc: r.soc, charging: r.charging });
            let samples = self.store.load(&r.key);
            self.estimates.insert(r.key.clone(), estimate::estimate(&samples, r.soc));

            if r.charging || r.soc > threshold {
                self.notified.remove(&r.key);
                continue;
            }
            // 低电提醒：同一设备 6 小时内只提醒一次，除非电量又掉了 5%
            if let Some((ts, soc)) = self.notified.get(&r.key) {
                if now - ts < 6 * 3600 * 1000 && soc.saturating_sub(r.soc) < 5 {
                    continue;
                }
            }
            self.notified.insert(r.key.clone(), (now, r.soc));
            alerts.push(Alert { name: r.name.clone(), soc: r.soc });
        }
        alerts
    }

    pub fn estimate_for(&self, key: &str) -> Option<&Estimate> {
        self.estimates.get(key).and_then(|e| e.as_ref())
    }

    pub fn tooltip_suffix(&self, key: &str) -> String {
        match self.estimate_for(key) {
            Some(e) => format!(" · {}", estimate::format_hours(e.hours_left)),
            None => String::new(),
        }
    }
}
