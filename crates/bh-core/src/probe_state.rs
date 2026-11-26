//! 每通道的探测状态机：决定"还要不要读""多久后再试""要不要建行"。
//!
//! 与 `probe`（CLI 排障命令）无关。三件事：
//! 1. **幽灵行抑制**：`ok_once == false` 的通道不建行（面板与托盘汇总都跳过）。
//! 2. **退避**：读不到就拉开重试间隔，避免每轮空等一整个读预算。
//! 3. **立即刷新**：[`reset_all`] 解除全部退避（菜单「立即刷新」与打开面板时调用）。
//!
//! 退避口径按读取模式分开 —— 这是关键：
//! * `Mode::Feature`（发请求必答）：失败多半是"没插/休眠"，阶梯退避 1→2→5 分钟即可；
//! * `Mode::Passive`（被动等推送）：设备只在被使用时推帧，退避到 5 分钟等于每 5 分钟只有
//!   一个读预算的窗口（约 2% 命中率），实际上"永远看不到"。所以固定 1 分钟，配合默认
//!   `refresh_secs=60` 相当于每轮都试一次。

use bh_api::Mode;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 主动协议的退避阶梯（毫秒）：1min → 2min → 5min → 5min…
const FEATURE_BACKOFF_MS: [i64; 4] = [60_000, 120_000, 300_000, 300_000];
/// 被动协议的重试间隔（毫秒）：固定，不阶梯。
const PASSIVE_BACKOFF_MS: i64 = 60_000;

/// 单通道的探测状态。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Probe {
    /// 成功读到过一次 —— 只有 true 的通道才建行（幽灵行抑制）。
    pub ok_once: bool,
    /// 连续失败次数，用作退避阶梯下标。
    pub fails: u8,
    /// 下次允许尝试的时间戳；0 = 立即可试。
    pub next_at: i64,
}

static PROBES: OnceLock<Mutex<HashMap<String, Probe>>> = OnceLock::new();

fn map() -> &'static Mutex<HashMap<String, Probe>> {
    PROBES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 读一个通道的状态（没记录就是默认值）。
pub fn of(key: &str) -> Probe {
    map().lock().map(|m| m.get(key).copied()).unwrap_or_default().unwrap_or_default()
}

fn update(key: &str, f: impl FnOnce(&mut Probe)) {
    if let Ok(mut m) = map().lock() {
        f(m.entry(key.to_string()).or_default());
    }
}

/// 成功读到一次：解除退避、标记"有过读数"。
pub fn on_success(key: &str) {
    update(key, |p| {
        p.ok_once = true;
        p.fails = 0;
        p.next_at = 0;
    });
}

/// 超时一次：累计失败并按模式安排下次重试。
pub fn on_timeout(key: &str, mode: Mode, now: i64) {
    update(key, |p| {
        p.fails = (p.fails + 1).min(FEATURE_BACKOFF_MS.len() as u8 - 1);
        p.next_at = now + backoff_ms(mode, p.fails);
    });
}

/// 解除所有通道的退避（不清 `ok_once`，不丢"幽灵已确认"的历史）。
pub fn reset_all() {
    if let Ok(mut m) = map().lock() {
        for p in m.values_mut() {
            p.next_at = 0;
        }
    }
}

/// 按读取模式算下一次重试间隔。
pub fn backoff_ms(mode: Mode, fails: u8) -> i64 {
    match mode {
        Mode::Passive => PASSIVE_BACKOFF_MS,
        Mode::Feature => {
            let idx = (fails.saturating_sub(1) as usize).min(FEATURE_BACKOFF_MS.len() - 1);
            FEATURE_BACKOFF_MS[idx]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_backoff_ladder_then_caps() {
        assert_eq!(backoff_ms(Mode::Feature, 1), 60_000);
        assert_eq!(backoff_ms(Mode::Feature, 2), 120_000);
        assert_eq!(backoff_ms(Mode::Feature, 3), 300_000);
        assert_eq!(backoff_ms(Mode::Feature, 9), 300_000); // 封顶
    }

    #[test]
    fn passive_backoff_stays_one_minute() {
        // 被动设备必须每轮都试，否则 5 分钟才见一次 6 秒窗口 = 永远看不到。
        for fails in 1..=9 {
            assert_eq!(backoff_ms(Mode::Passive, fails), 60_000);
        }
    }

    #[test]
    fn success_clears_backoff_and_marks_live() {
        let key = "test:success";
        on_timeout(key, Mode::Feature, 1_000);
        let p = of(key);
        assert!(!p.ok_once);
        assert!(p.next_at > 1_000);
        on_success(key);
        let p = of(key);
        assert!(p.ok_once);
        assert_eq!(p.next_at, 0);
        assert_eq!(p.fails, 0);
    }

    #[test]
    fn reset_all_keeps_ok_once() {
        let key = "test:reset";
        on_timeout(key, Mode::Feature, 0);
        on_success(key);
        on_timeout(key, Mode::Feature, 0);
        assert!(of(key).next_at > 0);
        reset_all();
        let p = of(key);
        assert_eq!(p.next_at, 0);
        assert!(p.ok_once, "reset 不该丢掉「读到过」的历史");
    }
}
