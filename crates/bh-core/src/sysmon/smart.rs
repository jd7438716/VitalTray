//! NVMe 硬盘温度：`IOCTL_STORAGE_QUERY_PROPERTY` 的协议专有查询（NVMe SMART/Health log page）。
//!
//! 走的是「直接向设备要一个 NVMe 日志页」，不是 `Get-StorageReliabilityCounter`（那条路要管理员，
//! 普通用户会拿到 `Access denied`）。所以这里提权也不需要、也不该提：
//! * 句柄用卷（`\\.\C:`）或物理盘（`\\.\PhysicalDrive0`）都行，访问权限只要 0；
//! * SATA/老驱动/非 NVMe 盘会失败，一律返回 `None`，面板显示 “—”，不编造数字。
//!
//! 复合温度是开尔文，`0` 表示该盘不上报。

use std::ffi::c_void;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
/// `STORAGE_PROPERTY_ID::StorageDeviceProtocolSpecificProperty`
const DEVICE_PROTOCOL_SPECIFIC: u32 = 50;
/// `STORAGE_QUERY_TYPE::PropertyStandardQuery`
const QUERY_STANDARD: u32 = 0;
/// `STORAGE_PROTOCOL_TYPE::ProtocolTypeNvme`
const PROTOCOL_NVME: u32 = 3;
/// `NVMeDataType::NVMeDataTypeLogPage`
const NVME_DATA_TYPE_LOG_PAGE: u32 = 2;
/// NVMe SMART / Health Information log page id。
const NVME_LOG_SMART_HEALTH: u32 = 0x02;
/// SMART/Health log page 固定 512 字节。
const LOG_SIZE: u32 = 512;
/// 输入缓冲：8 字节 `STORAGE_PROPERTY_QUERY` 头 + 40 字节 `STORAGE_PROTOCOL_SPECIFIC_DATA`。
const INPUT_SIZE: usize = 8 + 40;
const OUTPUT_SIZE: usize = 1024;
/// 温度缓存：SMART 是给排障用的，20 秒刷新一次足够，也不用每 2 秒去敲一次盘。
const CACHE_TTL_MS: i64 = 20_000;

static CACHE: Mutex<Option<(char, u8, i64)>> = Mutex::new(None);

/// 系统盘（卷）的温度，单位 °C；盘不支持就 `None`。
pub fn volume_temp(letter: char) -> Option<u8> {
    let now = now_ms();
    if let Ok(cache) = CACHE.lock() {
        if let Some((l, temp, at)) = *cache {
            if l.eq_ignore_ascii_case(&letter) && now.saturating_sub(at) < CACHE_TTL_MS {
                return Some(temp);
            }
        }
    }
    let temp = query(&format!("\\\\.\\{letter}:"));
    if let (Some(temp), Ok(mut cache)) = (temp, CACHE.lock()) {
        *cache = Some((letter, temp, now));
    }
    temp
}

fn query(path: &str) -> Option<u8> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateFileW(wide.as_ptr(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut()) };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut input = [0u8; INPUT_SIZE];
    put_u32(&mut input, 0, DEVICE_PROTOCOL_SPECIFIC);
    put_u32(&mut input, 4, QUERY_STANDARD);
    put_u32(&mut input, 8, PROTOCOL_NVME);
    put_u32(&mut input, 12, NVME_DATA_TYPE_LOG_PAGE);
    put_u32(&mut input, 16, NVME_LOG_SMART_HEALTH);
    // 相对 `STORAGE_PROTOCOL_SPECIFIC_DATA` 的起点：8 + 40 = 48。
    put_u32(&mut input, 24, INPUT_SIZE as u32);
    put_u32(&mut input, 28, LOG_SIZE);
    let mut output = [0u8; OUTPUT_SIZE];
    let mut returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            input.as_ptr() as *const c_void,
            INPUT_SIZE as u32,
            output.as_mut_ptr() as *mut c_void,
            OUTPUT_SIZE as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }
    // 描述符里回填的偏移量，各家驱动参照点不同：有的从描述符起点算，有的从 ProtocolSpecificData 起点算。
    let offset = get_u32(&output, 24) as usize;
    [8 + offset, offset]
        .into_iter()
        .find_map(|base| output.get(base..).and_then(log_temp))
}

/// 从 NVMe SMART/Health log page 里取复合温度（°C）。
pub fn log_temp(log: &[u8]) -> Option<u8> {
    let kelvin = u16::from_le_bytes([*log.get(1)?, *log.get(2)?]);
    // 0 = 该盘不报；273 K（0 °C）以下或 400 K（127 °C）以上都当脏数据。
    if !(273..=400).contains(&kelvin) {
        return None;
    }
    Some((kelvin - 273) as u8)
}

fn put_u32(buf: &mut [u8], at: usize, value: u32) {
    buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn get_u32(buf: &[u8], at: usize) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&buf[at..at + 4]);
    u32::from_le_bytes(bytes)
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
    fn parses_composite_temperature() {
        // 0x0142 K = 322 K = 49 °C，与其他字段（备件 100%、磨损 4%）一起照实解码。
        assert_eq!(log_temp(&[0x00, 0x42, 0x01, 0x64, 0x01, 0x04]), Some(49));
        assert_eq!(log_temp(&[0x00, 0x36, 0x01]), Some(37)); // 0x0136 = 310 K
    }

    #[test]
    fn rejects_missing_or_insane_temperature() {
        assert_eq!(log_temp(&[0x00, 0x00, 0x00]), None);
        assert_eq!(log_temp(&[0x00, 0x10, 0x00]), None); // 16 K
        assert_eq!(log_temp(&[0x00, 0xd0, 0x07]), None); // 2000 K
        assert_eq!(log_temp(&[0x00]), None);
    }
}
