//! 显卡列表：适配器每 60 秒刷新一次，温度与占用每次采样更新。

use super::kmt;
use super::GpuInfo;
use std::collections::HashMap;

const REFRESH_MS: i64 = 60_000;

#[derive(Default)]
pub struct Adapters {
    list: Vec<kmt::Adapter>,
    at_ms: i64,
}

impl Adapters {
    pub fn refresh_if_stale(&mut self, now_ms: i64) {
        if self.list.is_empty() || now_ms.saturating_sub(self.at_ms) >= REFRESH_MS {
            self.list = kmt::adapters();
            self.at_ms = now_ms;
        }
    }

    /// 把适配器、占用（按 LUID 对齐）与温度组合成展示用数据。
    pub fn snapshot(&self, usage: &HashMap<(u32, u32), u8>) -> Vec<GpuInfo> {
        self.list
            .iter()
            .map(|a| GpuInfo {
                name: a.name.clone(),
                use_percent: usage.get(&a.luid).copied(),
                temp_c: kmt::temperature(a.luid),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_usage_by_luid() {
        let mut usage = HashMap::new();
        usage.insert((0u32, 0xd5fbu32), 37u8);
        let adapters = Adapters {
            list: vec![
                kmt::Adapter { name: "GPU A".into(), luid: (0, 0xd5fb) },
                kmt::Adapter { name: "GPU B".into(), luid: (0, 0x1234) },
            ],
            at_ms: 0,
        };
        let snap = adapters.snapshot(&usage);
        assert_eq!(snap[0].use_percent, Some(37));
        assert_eq!(snap[0].name, "GPU A");
        assert_eq!(snap[1].use_percent, None);
    }
}
