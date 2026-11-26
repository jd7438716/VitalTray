//! 小工具：时间戳与日期换算（不引入 chrono/time 依赖）。

/// 日志时间戳（本地时间，YYYY-MM-DD HH:MM:SS）。
pub fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let t = now + local_offset_secs();
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// 当前本地时间 HH:MM:SS（面板“更新于 …”）。
pub fn clock() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let rem = (secs + local_offset_secs()).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// 清洗硬件名称："Intel(R) Core(TM) Ultra 9" → "Intel Core Ultra 9"。
pub fn tidy_name(raw: &str) -> String {
    let mut s = raw.replace("(R)", "").replace("(TM)", "").replace('®', "").replace('™', "");
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    s.trim().to_string()
}

/// 带毫秒的时间戳，供日志使用。
pub fn timestamp_ms() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let ms = now.subsec_millis();
    let t = secs + local_offset_secs();
    let days = t.div_euclid(86_400);
    let rem = t.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{ms:03}+08:00",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// M0 固定 UTC+8；M1 改用 GetTimeZoneInformation 读系统时区。
fn local_offset_secs() -> i64 {
    8 * 3600
}

/// Howard Hinnant 的 civil_from_days。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn known_day() {
        // 2026-11-23 = 20780 天（1970-01-01 起）
        assert_eq!(civil_from_days(20780), (2026, 11, 23));
    }

    #[test]
    fn tidies_hardware_names() {
        assert_eq!(tidy_name("Intel(R) Core(TM) Ultra 9 185H"), "Intel Core Ultra 9 185H");
        assert_eq!(tidy_name("  AMD Ryzen(TM)  7  "), "AMD Ryzen 7");
    }
}
