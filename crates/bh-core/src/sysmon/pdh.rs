//! PDH 性能计数器：CPU 温度与 GPU 占用。
//!
//! 只读、无需管理员。两个计数器都按 `(*)` 通配实例读取：
//! * `\Thermal Zone Information(*)\High Precision Temperature` —— ACPI 热区，单位 0.1 K；
//! * `\GPU Engine(*)\Utilization Percentage` —— 每个引擎/进程一条，按 LUID + 引擎类型汇总。
//!
//! 注意 PDH 的速率型计数器需要至少两次 `PdhCollectQueryData`，第一次采到的值不可信；
//! 这里的 `Pdh::new` 先采一次，宿主线程再等 600ms 采第二次。

use std::collections::HashMap;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

pub struct Pdh {
    query: PDH_HQUERY,
    thermal: Option<PDH_HCOUNTER>,
    /// 温度计数器的单位：10 = 0.1 K，1 = K。
    thermal_div: f64,
    gpu: Option<PDH_HCOUNTER>,
}

impl Pdh {
    /// 打开查询并挂上计数器。`gpu_usage` = 是否要 `\GPU Engine(*)` 那一路：
    /// 它有几百个实例，是这套采样里最贵的一段（`collect` + 取值约 0.8 ms/帧），
    /// 面板只显示温度时没必要为它付钱。
    pub fn new(gpu_usage: bool) -> Option<Self> {
        let mut query: PDH_HQUERY = std::ptr::null_mut();
        if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) } != ERROR_SUCCESS {
            return None;
        }
        let mut thermal_div = 10.0;
        let mut thermal = add(query, r"\Thermal Zone Information(*)\High Precision Temperature");
        if thermal.is_none() {
            thermal = add(query, r"\Thermal Zone Information(*)\Temperature");
            thermal_div = 1.0;
        }
        let gpu = if gpu_usage { add(query, r"\GPU Engine(*)\Utilization Percentage") } else { None };
        if thermal.is_none() && gpu.is_none() {
            unsafe { PdhCloseQuery(query) };
            return None;
        }
        let pdh = Self { query, thermal, thermal_div, gpu };
        pdh.collect();
        Some(pdh)
    }

    /// 采一帧。速率型计数器从第二帧起才可信。
    pub fn collect(&self) {
        unsafe { PdhCollectQueryData(self.query) };
    }

    /// 所有热区里的最高温度（摄氏度）。
    pub fn thermal_c(&self) -> Option<u8> {
        let counter = self.thermal?;
        array(counter)
            .into_iter()
            .filter_map(|(_, v)| kelvin_tenths_to_c(v, self.thermal_div))
            .max()
    }

    /// 每块显卡（按 LUID）的最高引擎占用百分比。
    pub fn gpu_usage(&self) -> HashMap<(u32, u32), u8> {
        let Some(counter) = self.gpu else { return HashMap::new() };
        let mut grouped: HashMap<(u32, u32, String), f64> = HashMap::new();
        for (name, value) in array(counter) {
            let Some(luid) = parse_luid(&name) else { continue };
            let engine = engine_type(&name);
            if engine.is_empty() {
                continue;
            }
            *grouped.entry((luid.0, luid.1, engine.to_string())).or_insert(0.0) += value.max(0.0);
        }
        let mut per_luid: HashMap<(u32, u32), f64> = HashMap::new();
        for ((high, low, _), value) in grouped {
            let entry = per_luid.entry((high, low)).or_insert(0.0);
            *entry = entry.max(value);
        }
        per_luid
            .into_iter()
            .map(|(k, v)| (k, (v.round() as u32).min(100) as u8))
            .collect()
    }
}

impl Drop for Pdh {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

fn add(query: PDH_HQUERY, path: &str) -> Option<PDH_HCOUNTER> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut counter: PDH_HCOUNTER = std::ptr::null_mut();
    let rc = unsafe { PdhAddEnglishCounterW(query, wide.as_ptr(), 0, &mut counter) };
    if rc == ERROR_SUCCESS && !counter.is_null() { Some(counter) } else { None }
}

fn array(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
    unsafe {
        let mut size = 0u32;
        let mut count = 0u32;
        let rc = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut());
        if rc != PDH_MORE_DATA || size == 0 || count == 0 {
            return Vec::new();
        }
        // 缓冲区里除了结构体还有实例名字符串，多留两个元素余量。
        let items = size as usize / std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() + 2;
        let mut buf: Vec<PDH_FMT_COUNTERVALUE_ITEM_W> = vec![Default::default(); items];
        let rc = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, buf.as_mut_ptr());
        if rc != ERROR_SUCCESS {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(count as usize);
        for item in buf.iter().take(count as usize) {
            let status = item.FmtValue.CStatus;
            if status != PDH_CSTATUS_VALID_DATA && status != PDH_CSTATUS_NEW_DATA {
                continue;
            }
            let name = if item.szName.is_null() { String::new() } else { pwstr(item.szName) };
            out.push((name, item.FmtValue.Anonymous.doubleValue));
        }
        out
    }
}

unsafe fn pwstr(p: *const u16) -> String {
    let mut len = 0usize;
    while len < 512 && unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

/// 热区温度 → 摄氏度。`div` = 10 表示 0.1K，1 表示 K。
pub fn kelvin_tenths_to_c(value: f64, div: f64) -> Option<u8> {
    if value <= 0.0 || div <= 0.0 {
        return None;
    }
    let c = value / div - 273.15;
    if !(15.0..=120.0).contains(&c) {
        return None;
    }
    Some(c.round() as u8)
}

/// `pid_4_luid_0x00000000_0x0000d5fb_...` → (high, low)。
pub fn parse_luid(instance: &str) -> Option<(u32, u32)> {
    let at = instance.find("luid_0x")? + "luid_0x".len();
    let mut parts = instance[at..].split('_');
    let high = u32::from_str_radix(parts.next()?, 16).ok()?;
    let low = u32::from_str_radix(parts.next()?.trim_start_matches("0x"), 16).ok()?;
    Some((high, low))
}

/// `..._engtype_3D` → "3D"；没有引擎类型时返回空串。
pub fn engine_type(instance: &str) -> &str {
    instance.split("engtype_").nth(1).unwrap_or("").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gpu_instance_names() {
        let name = "pid_19188_luid_0x00000000_0x0000d5fb_phys_0_eng_0_engtype_3D";
        assert_eq!(parse_luid(name), Some((0, 0xd5fb)));
        assert_eq!(engine_type(name), "3D");
        assert_eq!(parse_luid("pid_4_phys_0_eng_0_engtype_"), None);
    }

    #[test]
    fn converts_thermal_values() {
        assert_eq!(kelvin_tenths_to_c(3512.0, 10.0), Some(78));
        assert_eq!(kelvin_tenths_to_c(3000.0, 10.0), Some(27));
        assert_eq!(kelvin_tenths_to_c(0.0, 10.0), None);
        assert_eq!(kelvin_tenths_to_c(351.2, 1.0), Some(78));
    }
}
