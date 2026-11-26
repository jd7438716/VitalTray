//! 显卡枚举与温度：D3DKMT（`gdi32.dll`）的只读查询。
//!
//! 用途：
//! * 枚举真实显卡（适配器）并拿到它的名字与 LUID —— LUID 用来和 PDH 的
//!   `\GPU Engine(*)\Utilization Percentage` 实例名对上；
//! * 查询 `KMTQAITYPE_ADAPTERPERFDATA` 拿温度（核显/部分驱动可能返回 0）。
//!
//! 全程用户态、无需管理员；调用失败时返回空值，由上层显示 “—”。

use std::ffi::c_void;

type NtStatus = i32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Luid {
    low: u32,
    high: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EnumAdapters2 {
    num: u32,
    adapters: *mut AdapterInfo,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AdapterInfo {
    handle: u32,
    luid: Luid,
    sources: u32,
    /// 头文件里叫 bPrecisePresentRegionsPreferred，与“是否在用”无关。
    precise: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OpenFromLuid {
    luid: Luid,
    handle: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct QueryAdapterInfo {
    handle: u32,
    kind: u32,
    data: *mut c_void,
    size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CloseAdapter {
    handle: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RegistryInfo {
    adapter: [u16; 260],
    bios: [u16; 260],
    dac: [u16; 260],
    chip: [u16; 260],
}

impl Default for RegistryInfo {
    fn default() -> Self {
        Self { adapter: [0; 260], bios: [0; 260], dac: [0; 260], chip: [0; 260] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AdapterPerfData {
    physical: u32,
    mem_freq: u64,
    max_mem_freq: u64,
    max_mem_freq_oc: u64,
    mem_bandwidth: u64,
    pcie_bandwidth: u64,
    fan_rpm: u32,
    /// 功耗占额定上限的千分比（0 = 不支持）。
    power: u32,
    /// 摄氏度 ×10（0 = 不支持）。
    temperature: u32,
    power_state_override: u8,
}

const KMTQAITYPE_ADAPTERREGISTRYINFO: u32 = 8;
const KMTQAITYPE_ADAPTERPERFDATA: u32 = 62;

#[link(name = "gdi32")]
unsafe extern "system" {
    fn D3DKMTEnumAdapters2(desc: *mut EnumAdapters2) -> NtStatus;
    fn D3DKMTOpenAdapterFromLuid(desc: *mut OpenFromLuid) -> NtStatus;
    fn D3DKMTQueryAdapterInfo(desc: *mut QueryAdapterInfo) -> NtStatus;
    fn D3DKMTCloseAdapter(desc: *mut CloseAdapter) -> NtStatus;
}

/// 一块真实显卡（驱动可回答注册表信息才会被收录）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub name: String,
    pub luid: (u32, u32),
}

/// 枚举显卡：名字来自 `KMTQAITYPE_ADAPTERREGISTRYINFO`，虚拟/基础适配器会被过滤掉。
pub fn adapters() -> Vec<Adapter> {
    let mut desc = EnumAdapters2 { num: 0, adapters: std::ptr::null_mut() };
    let rc = unsafe { D3DKMTEnumAdapters2(&mut desc) };
    if rc != 0 || desc.num == 0 {
        return Vec::new();
    }
    let mut buf: Vec<AdapterInfo> = vec![AdapterInfo::default(); desc.num as usize];
    desc.adapters = buf.as_mut_ptr();
    let rc = unsafe { D3DKMTEnumAdapters2(&mut desc) };
    if rc != 0 {
        return Vec::new();
    }
    let count = (desc.num as usize).min(buf.len());
    let mut out = Vec::new();
    for info in buf.iter().take(count) {
        if let Some(name) = registry_name(info.luid) {
            out.push(Adapter { name: crate::util::tidy_name(&name), luid: (info.luid.high as u32, info.luid.low) });
        }
    }
    out
}

/// 读一次显卡温度（摄氏度）。0 或越界视为“该驱动不提供”。
pub fn temperature(luid: (u32, u32)) -> Option<u8> {
    let handle = open(luid)?;
    let mut perf = AdapterPerfData::default();
    let mut query = QueryAdapterInfo {
        handle,
        kind: KMTQAITYPE_ADAPTERPERFDATA,
        data: &mut perf as *mut AdapterPerfData as *mut c_void,
        size: std::mem::size_of::<AdapterPerfData>() as u32,
    };
    let rc = unsafe { D3DKMTQueryAdapterInfo(&mut query) };
    close(handle);
    if rc != 0 {
        return None;
    }
    // 温度字段是 0.1°C；0 表示不支持，>150°C 视为无效。
    if perf.temperature == 0 || perf.temperature > 1500 {
        return None;
    }
    Some(((perf.temperature + 5) / 10) as u8)
}

fn registry_name(luid: Luid) -> Option<String> {
    let handle = open((luid.high as u32, luid.low))?;
    let mut info = RegistryInfo::default();
    let mut query = QueryAdapterInfo {
        handle,
        kind: KMTQAITYPE_ADAPTERREGISTRYINFO,
        data: &mut info as *mut RegistryInfo as *mut c_void,
        size: std::mem::size_of::<RegistryInfo>() as u32,
    };
    let rc = unsafe { D3DKMTQueryAdapterInfo(&mut query) };
    close(handle);
    if rc != 0 {
        return None;
    }
    let name = utf16(&info.adapter);
    if name.is_empty() { None } else { Some(name) }
}

fn open(luid: (u32, u32)) -> Option<u32> {
    let mut desc = OpenFromLuid {
        luid: Luid { low: luid.1, high: luid.0 as i32 },
        handle: 0,
    };
    let rc = unsafe { D3DKMTOpenAdapterFromLuid(&mut desc) };
    if rc != 0 || desc.handle == 0 { None } else { Some(desc.handle) }
}

fn close(handle: u32) {
    unsafe {
        D3DKMTCloseAdapter(&mut CloseAdapter { handle });
    }
}

fn utf16(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_sizes_match_windows_headers() {
        assert_eq!(std::mem::size_of::<AdapterInfo>(), 20);
        assert_eq!(std::mem::size_of::<OpenFromLuid>(), 12);
        assert_eq!(std::mem::size_of::<AdapterPerfData>(), 64);
        assert_eq!(std::mem::size_of::<RegistryInfo>(), 4 * 260 * 2);
    }

    #[test]
    fn utf16_stops_at_nul() {
        let mut buf = [0u16; 8];
        buf[..3].copy_from_slice(&[b'G' as u16, b'P' as u16, b'U' as u16]);
        assert_eq!(utf16(&buf), "GPU");
    }
}
