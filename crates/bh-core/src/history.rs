//! 采样历史：每设备一个 JSONL 文件，只存"变化或心跳"，保留 7 天。

use std::io::Write;
use std::path::{Path, PathBuf};

pub const KEEP_MS: i64 = 7 * 24 * 3600 * 1000;
pub const HEARTBEAT_MS: i64 = 15 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub ts: i64,
    pub soc: u8,
    pub charging: bool,
}

pub struct Store {
    dir: PathBuf,
}

fn safe_name(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        Self { dir: dir.to_path_buf() }
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{}.jsonl", safe_name(key)))
    }

    /// 记录一条；与上一条相比"没变化且未到心跳"就跳过。
    pub fn record(&self, key: &str, sample: Sample) -> std::io::Result<()> {
        let last = self.load(key).last().copied();
        if let Some(p) = last {
            let changed = p.soc != sample.soc || p.charging != sample.charging;
            if !changed && sample.ts - p.ts < HEARTBEAT_MS {
                return Ok(());
            }
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(self.path(key))?;
        writeln!(f, "{{\"ts\":{},\"soc\":{},\"ch\":{}}}", sample.ts, sample.soc, sample.charging)?;
        Ok(())
    }

    /// 读取并做 7 天裁剪（裁剪结果不写回，写入侧按需覆盖）。
    pub fn load(&self, key: &str) -> Vec<Sample> {
        let Ok(text) = std::fs::read_to_string(self.path(key)) else { return Vec::new() };
        let mut out = Vec::new();
        let mut newest = 0i64;
        for line in text.lines() {
            if let Some(s) = parse_line(line) {
                newest = newest.max(s.ts);
                out.push(s);
            }
        }
        let cutoff = newest - KEEP_MS;
        out.retain(|s| s.ts >= cutoff);
        out
    }

    /// 压缩：只保留 7 天内的数据。
    pub fn compact(&self, key: &str) {
        let samples = self.load(key);
        if samples.is_empty() {
            return;
        }
        let mut buf = String::new();
        for s in &samples {
            buf.push_str(&format!("{{\"ts\":{},\"soc\":{},\"ch\":{}}}\n", s.ts, s.soc, s.charging));
        }
        let _ = std::fs::write(self.path(key), buf);
    }
}

fn parse_line(line: &str) -> Option<Sample> {
    let ts = number(line, "\"ts\":")?;
    let soc = number(line, "\"soc\":")?;
    let charging = line.contains("\"ch\":true");
    Some(Sample { ts, soc: soc.clamp(0, 100) as u8, charging })
}

fn number(line: &str, key: &str) -> Option<i64> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        let s = parse_line("{\"ts\":1700000000000,\"soc\":73,\"ch\":false}").unwrap();
        assert_eq!(s.ts, 1_700_000_000_000);
        assert_eq!(s.soc, 73);
        assert!(!s.charging);
    }

    #[test]
    fn record_and_load() {
        let dir = std::env::temp_dir().join("bh-history-test");
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::new(&dir);
        store.record("bt:aa", Sample { ts: 1000, soc: 90, charging: false }).unwrap();
        store.record("bt:aa", Sample { ts: 2000, soc: 90, charging: false }).unwrap(); // 心跳内，忽略
        store.record("bt:aa", Sample { ts: 3000, soc: 88, charging: false }).unwrap();
        let all = store.load("bt:aa");
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].soc, 88);
    }
}
