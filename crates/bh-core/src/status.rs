//! 状态分级与状态色：托盘图标和面板共用同一套阈值、优先级和颜色。
//!
//! 一条原则：**颜色只表达“要不要管”，不表达“这是哪台设备”**。
//!
//! | 状态 | 判定 | 深色 | 浅色 | 含义 |
//! |------|------|------|------|------|
//! | 充足 Good | 电量 > 80% | 绿 | 绿 | 都够了，不用管 |
//! | 正常 Normal | 20–80%，未充电 | 中性 | 中性 | 一切正常，保持安静 |
//! | 低电 Low | 电量 <= 低电阈值（默认 20%） | 红 | 红 | 该充电了 |
//! | 充电 Charging | 20–80%，正在充电 | 蓝 | 蓝 | 正在补电 |
//! | 上次 Stale | 只有上次读数 | 灰 | 灰 | 数据过时 |
//! | 无数据 Unknown | 没有可读设备 | 灰 | 灰 | 还没读到 |
//!
//! 托盘只能画一个图标，汇总按“最需要关注”的顺序取：
//! **低电 > 全部充足 > 充电中 > 正常**；没有新鲜读数时才退到“上次 / 无数据”。
//! 这样“所有设备都 >80%”一定是一个颜色（绿），“任何一台 <= 阈值”一定是另一个颜色（红）。

use crate::poller::DeviceReading;

/// 充足线：严格大于 80% 才算“都够了”。
pub const GOOD_ABOVE: u8 = 80;

/// 低电默认阈值；实际使用配置里的 `low_battery`（默认 20），保证颜色和低电提醒同一口径。
pub const DEFAULT_LOW: u8 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Good,
    Normal,
    Low,
    Charging,
    Stale,
    Unknown,
}

impl State {
    /// 日志/文档用的短名。
    pub fn name(self) -> &'static str {
        match self {
            State::Good => "good",
            State::Normal => "normal",
            State::Low => "low",
            State::Charging => "charging",
            State::Stale => "stale",
            State::Unknown => "unknown",
        }
    }

    /// 该状态在指定主题下的强调色。
    pub fn color(self, dark: bool) -> u32 {
        state_color(self, dark)
    }
}

/// 托盘汇总结果：一个状态 + 要显示的电量（没有可读设备时为空）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aggregate {
    pub state: State,
    pub percent: Option<u8>,
}

/// 单个设备的状态（阈值与汇总完全一致）。
pub fn state_of(soc: u8, charging: bool, stale: bool, low: u8) -> State {
    if stale {
        return State::Stale;
    }
    if soc <= low {
        State::Low
    } else if soc > GOOD_ABOVE {
        State::Good
    } else if charging {
        State::Charging
    } else {
        State::Normal
    }
}

/// 把一轮读数汇总成托盘图标要画的状态。
///
/// 只拿“新鲜读数”参与汇总（`error` 为空的旧读数不算）；
/// 一台设备都没有新鲜读数时，才退回到上次读数或“无数据”。
pub fn aggregate(readings: &[DeviceReading], low: u8) -> Aggregate {
    // 幽灵行（接收器常驻但从未上报过的通道）不参与汇总。
    let live = || readings.iter().filter(|r| !r.ghost);
    let fresh = live().filter(|r| r.error.is_none() && !r.stale && !r.pending);

    let mut min_fresh: Option<u8> = None;
    let mut any_charging = false;
    let mut all_good = true;
    let mut any_fresh = false;
    for r in fresh {
        any_fresh = true;
        min_fresh = Some(min_fresh.map_or(r.soc, |m| m.min(r.soc)));
        any_charging |= r.charging;
        all_good &= r.soc > GOOD_ABOVE;
    }

    if any_fresh {
        let percent = min_fresh;
        let soc = percent.unwrap_or(0);
        // 低电优先：哪怕同时在充电，电量也还是低的（充到阈值以上再变蓝）。
        let state = if soc <= low {
            State::Low
        } else if all_good {
            State::Good
        } else if any_charging {
            State::Charging
        } else {
            State::Normal
        };
        return Aggregate { state, percent };
    }

    let mut min_stale: Option<u8> = None;
    for r in live().filter(|r| r.error.is_none() && !r.pending) {
        min_stale = Some(min_stale.map_or(r.soc, |m| m.min(r.soc)));
    }
    Aggregate {
        state: if min_stale.is_some() { State::Stale } else { State::Unknown },
        percent: min_stale,
    }
}

/// COLORREF：0x00BBGGRR（GDI 的字节序），用函数组色避免手写错字节。
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((b as u32) << 16) | ((g as u32) << 8) | r as u32
}

/// 语义状态色：深色任务栏用亮一档的前景，浅色任务栏用深一档。
///
/// 取值参考 Windows 11 任务栏电池图标（绿/黄/红）与 Fluent 色板，
/// 并保证与任务栏底色（深 #202020 / 浅 #F3F3F3）对比度 >= 3:1（见 tests）。
pub fn state_color(state: State, dark: bool) -> u32 {
    match (state, dark) {
        (State::Good, true) => rgb(108, 203, 95),     // #6CCB5F
        (State::Good, false) => rgb(16, 124, 16),     // #107C10
        (State::Normal, true) => rgb(242, 242, 242),  // #F2F2F2
        (State::Normal, false) => rgb(26, 26, 26),    // #1A1A1A
        (State::Low, true) => rgb(255, 107, 107),     // #FF6B6B
        (State::Low, false) => rgb(196, 43, 28),      // #C42B1C
        (State::Charging, true) => rgb(76, 194, 255), // #4CC2FF
        (State::Charging, false) => rgb(0, 95, 184),  // #005FB8
        (State::Stale, true) => rgb(169, 174, 181),   // #A9AEB5
        (State::Stale, false) => rgb(95, 99, 104),    // #5F6368
        (State::Unknown, true) => rgb(143, 149, 156), // #8F959C
        (State::Unknown, false) => rgb(118, 118, 118), // #767676
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(percent: u8, charging: bool, stale: bool, low: u8) -> State {
        state_of(percent, charging, stale, low)
    }

    fn reading(soc: u8, charging: bool, stale: bool, error: Option<&str>) -> DeviceReading {
        DeviceReading {
            key: format!("k{soc}"),
            name: "dev".into(),
            domain: "test".into(),
            glyph: bh_api::Glyph::Mouse,
            brand: None,
            soc,
            charging,
            error: error.map(|e| e.to_string()),
            pending: false,
            stale,
            ghost: false,
        }
    }

    #[test]
    fn thresholds_are_80_and_the_low_setting() {
        assert_eq!(dec(81, false, false, 20), State::Good);
        assert_eq!(dec(80, false, false, 20), State::Normal);
        assert_eq!(dec(21, false, false, 20), State::Normal);
        assert_eq!(dec(20, false, false, 20), State::Low);
        assert_eq!(dec(5, false, false, 20), State::Low);
        // 阈值跟配置走：阈值 30 时 25% 也算低电
        assert_eq!(dec(25, false, false, 30), State::Low);
    }

    #[test]
    fn charging_only_colors_the_middle_band() {
        assert_eq!(dec(55, true, false, 20), State::Charging);
        // 低电优先：还在阈值下充电也显示红，充满过线才变蓝
        assert_eq!(dec(15, true, false, 20), State::Low);
        // 充足优先：>80% 充电中仍是“都够了”
        assert_eq!(dec(85, true, false, 20), State::Good);
    }

    #[test]
    fn stale_wins_over_level() {
        assert_eq!(dec(90, false, true, 20), State::Stale);
        assert_eq!(dec(5, false, true, 20), State::Stale);
    }

    #[test]
    fn aggregate_prefers_low_then_all_good_then_charging() {
        let many = [
            reading(90, false, false, None),
            reading(85, false, false, None),
        ];
        assert_eq!(aggregate(&many, 20), Aggregate { state: State::Good, percent: Some(85) });

        let one_low = [reading(90, false, false, None), reading(15, false, false, None)];
        assert_eq!(aggregate(&one_low, 20), Aggregate { state: State::Low, percent: Some(15) });

        // 充电中不算低电（阈值以上）；一台充电一台正常 -> 蓝
        let charging = [reading(55, true, false, None), reading(90, false, false, None)];
        assert_eq!(aggregate(&charging, 20), Aggregate { state: State::Charging, percent: Some(55) });

        // 都 >80 时，充电也还是绿
        let topped = [reading(85, true, false, None), reading(95, false, false, None)];
        assert_eq!(aggregate(&topped, 20), Aggregate { state: State::Good, percent: Some(85) });

        let normal = [reading(55, false, false, None), reading(70, false, false, None)];
        assert_eq!(aggregate(&normal, 20), Aggregate { state: State::Normal, percent: Some(55) });
    }

    #[test]
    fn aggregate_ignores_broken_and_stale_when_fresh_exists() {
        let mixed = [
            reading(90, false, false, None),
            reading(10, false, true, None),
            reading(0, false, false, Some("open failed")),
        ];
        assert_eq!(aggregate(&mixed, 20), Aggregate { state: State::Good, percent: Some(90) });

        let stale_only = [reading(60, false, true, None), reading(40, false, true, None)];
        assert_eq!(aggregate(&stale_only, 20), Aggregate { state: State::Stale, percent: Some(40) });

        assert_eq!(aggregate(&[], 20), Aggregate { state: State::Unknown, percent: None });
        let broken = [reading(0, false, false, Some("open failed"))];
        assert_eq!(aggregate(&broken, 20), Aggregate { state: State::Unknown, percent: None });
    }

    fn channel(color: u32) -> (u8, u8, u8) {
        ((color & 0xFF) as u8, ((color >> 8) & 0xFF) as u8, ((color >> 16) & 0xFF) as u8)
    }

    fn luminance(rgb: (u8, u8, u8)) -> f64 {
        let f = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(rgb.0) + 0.7152 * f(rgb.1) + 0.0722 * f(rgb.2)
    }

    fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn every_state_color_is_readable_on_the_taskbar() {
        let dark_bg = (32, 32, 32); // Win11 深色任务栏底色附近
        let light_bg = (243, 243, 243); // Win11 浅色任务栏底色附近
        for state in [State::Good, State::Normal, State::Low, State::Charging, State::Stale, State::Unknown] {
            for (dark, bg) in [(true, dark_bg), (false, light_bg)] {
                let c = contrast(channel(state_color(state, dark)), bg);
                assert!(c >= 3.0, "{state:?} dark={dark} 对比度只有 {c:.2}:1");
            }
        }
    }
}
