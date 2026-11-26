//! 续航估算：只用"连续放电段"的样本，带门禁，宁可不显示也不胡报。

use crate::history::Sample;

pub struct Estimate {
    pub hours_left: f64,
    pub rate_per_hour: f64,
    pub samples: usize,
}

/// 找最近一段连续放电：遇充电/大空洞/跳变就断开。
fn discharge_run(samples: &[Sample]) -> &[Sample] {
    if samples.len() < 2 {
        return &[];
    }
    let mut start = samples.len() - 1;
    while start > 0 {
        let prev = &samples[start - 1];
        let cur = &samples[start];
        let gap_ms = cur.ts - prev.ts;
        let diff = cur.soc as i32 - prev.soc as i32;
        if prev.charging
            || cur.charging
            || gap_ms <= 0
            || gap_ms > 4 * 3600 * 1000
            || diff > 1
            || diff < -25
        {
            break;
        }
        start -= 1;
    }
    &samples[start..]
}

/// 估算剩余时间。返回 None 表示"数据不足/不可信"。
pub fn estimate(samples: &[Sample], soc: u8) -> Option<Estimate> {
    let run = discharge_run(samples);
    if run.len() < 4 {
        return None;
    }
    let span_h = (run[run.len() - 1].ts - run[0].ts) as f64 / 3_600_000.0;
    if span_h < 2.0 {
        return None;
    }
    let drop = run[0].soc as i32 - run[run.len() - 1].soc as i32;
    if drop < 3 {
        return None;
    }
    let rate = drop as f64 / span_h;
    if !(0.02..=12.0).contains(&rate) {
        return None;
    }
    let hours = soc as f64 / rate;
    if !hours.is_finite() || hours > 90.0 * 24.0 {
        return None;
    }
    Some(Estimate { hours_left: hours, rate_per_hour: rate, samples: run.len() })
}

/// 展示用：不到 1 小时给分钟，超过 1 天给“约 N 天”。单位文案走 i18n。
pub fn format_hours(h: f64) -> String {
    let (key, n) = if h < 1.0 {
        ("est.minutes", format!("{:.0}", (h * 60.0).max(1.0)))
    } else if h < 24.0 {
        // “5.0 小时” 削成 “5 小时”，半小时级保留一位小数
        let s = format!("{h:.1}");
        ("est.hours", s.trim_end_matches(".0").to_string())
    } else {
        ("est.days", format!("{:.0}", h / 24.0))
    };
    crate::i18n::tf(key, &[("n", &n)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(ts: i64, soc: u8) -> Sample {
        Sample { ts, soc, charging: false }
    }

    #[test]
    fn steady_discharge_2pct_per_hour() {
        // 10 小时掉 20%（2%/h），当前 60%
        let samples: Vec<Sample> = (0..=10).map(|i| s(i * 3_600_000, 80 - 2 * i as u8)).collect();
        let e = estimate(&samples, 60).unwrap();
        assert!((e.rate_per_hour - 2.0).abs() < 0.2);
        assert!((e.hours_left - 30.0).abs() < 3.0);
    }

    #[test]
    fn charging_breaks_run() {
        let mut samples: Vec<Sample> = (0..=5).map(|i| s(i * 3_600_000, 80 - i as u8)).collect();
        samples[3].charging = true;
        assert!(estimate(&samples, 75).is_none() || estimate(&samples, 75).unwrap().samples <= 3);
    }

    #[test]
    fn too_little_drop_is_rejected() {
        let samples: Vec<Sample> = (0..=6).map(|i| s(i * 3_600_000, 80 - (i / 3) as u8)).collect();
        assert!(estimate(&samples, 78).is_none());
    }

    #[test]
    fn big_gap_breaks_run() {
        let mut samples: Vec<Sample> = (0..=5).map(|i| s(i * 3_600_000, 80 - i as u8)).collect();
        for sample in samples.iter_mut().skip(3) {
            sample.ts += 10 * 3_600_000;
        }
        let e = estimate(&samples, 75);
        assert!(e.is_none() || e.unwrap().samples < 4);
    }

    #[test]
    fn format_units() {
        // 单位文本走 i18n，这里只验证“选对单位、数字对不上小数尾巴”
        let want = |key: &str, n: &str| crate::i18n::tf(key, &[("n", n)]);
        assert_eq!(format_hours(0.5), want("est.minutes", "30"));
        assert_eq!(format_hours(5.0), want("est.hours", "5"));
        assert_eq!(format_hours(5.25), want("est.hours", "5.2"));
        assert_eq!(format_hours(50.0), want("est.days", "2"));
    }
}
