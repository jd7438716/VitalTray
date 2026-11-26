//! 蓝牙设备电量：走 SetupAPI + DEVPKEY_Bluetooth_Battery（Windows 10 1809+）。
//!
//! 优点：不需要知道任何厂商私有协议，Windows 自己维护该属性（蓝牙低功耗设备、
//! 支持电量上报的 BLE HID 都有）。缺点：只有"Windows 认为有电池"的设备才有值。

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW, GUID_DEVCLASS_BLUETOOTH, DIGCF_PRESENT,
    SP_DEVINFO_DATA,
};
use windows_sys::Win32::Devices::Properties::DEVPROP_TYPE_BYTE;
use windows_sys::Win32::Foundation::DEVPROPKEY;

const DEVPKEY_BT_BATTERY: DEVPROPKEY = DEVPROPKEY {
    fmtid: windows_sys::core::GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5),
    pid: 2,
};
const SPDRP_FRIENDLYNAME: u32 = 0x0000_000C;

#[derive(Debug, Clone)]
pub struct BtDevice {
    pub key: String,
    pub name: String,
    pub percent: u8,
}

fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// 列出所有"Windows 报得出电量"的蓝牙设备。
pub fn list() -> Vec<BtDevice> {
    let mut out = Vec::new();
    unsafe {
        let hdev = SetupDiGetClassDevsW(&GUID_DEVCLASS_BLUETOOTH, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT);
        if hdev == 0 || hdev == -1 {
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut data: SP_DEVINFO_DATA = std::mem::zeroed();
            data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiEnumDeviceInfo(hdev, index, &mut data) == 0 {
                break;
            }
            index += 1;

            let mut instance = [0u16; 512];
            let ok = SetupDiGetDeviceInstanceIdW(hdev, &data, instance.as_mut_ptr(), instance.len() as u32, std::ptr::null_mut());
            if ok == 0 {
                continue;
            }
            let instance_id = utf16_to_string(&instance);
            if !(instance_id.starts_with("BTHENUM\\") || instance_id.starts_with("BTHLE\\")) {
                continue;
            }

            let mut battery: u8 = 0;
            let mut ty: u32 = 0;
            let mut needed: u32 = 0;
            let got = SetupDiGetDevicePropertyW(
                hdev,
                &data,
                &DEVPKEY_BT_BATTERY,
                &mut ty,
                &mut battery as *mut u8,
                std::mem::size_of::<u8>() as u32,
                &mut needed,
                0,
            );
            if got == 0 || ty != DEVPROP_TYPE_BYTE || battery > 100 {
                continue;
            }

            let mut name_buf = [0u16; 512];
            let mut reg_type = 0u32;
            let mut name_need = 0u32;
            let name_ok = SetupDiGetDeviceRegistryPropertyW(
                hdev,
                &data,
                SPDRP_FRIENDLYNAME,
                &mut reg_type,
                name_buf.as_mut_ptr() as *mut u8,
                (name_buf.len() * 2) as u32,
                &mut name_need,
            );
            let name = if name_ok != 0 { utf16_to_string(&name_buf) } else { instance_id.clone() };

            out.push(BtDevice { key: format!("bt:{instance_id}"), name, percent: battery });
        }
        SetupDiDestroyDeviceInfoList(hdev);
    }
    out
}

/// 按 key 查电量。
pub fn lookup(key: &str) -> Option<u8> {
    list().into_iter().find(|d| d.key == key).map(|d| d.percent)
}

/// 调试：列出所有蓝牙设备节点及其电量属性读取结果。
pub fn debug_lines() -> Vec<String> {
    let mut out = Vec::new();
    for d in list() {
        out.push(format!("{} = {}%", d.key, d.percent));
    }
    unsafe {
        let hdev = SetupDiGetClassDevsW(&GUID_DEVCLASS_BLUETOOTH, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT);
        if hdev == 0 || hdev == -1 {
            out.push("SetupDiGetClassDevsW failed".into());
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut data: SP_DEVINFO_DATA = std::mem::zeroed();
            data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiEnumDeviceInfo(hdev, index, &mut data) == 0 {
                break;
            }
            index += 1;
            let mut instance = [0u16; 512];
            let ok = SetupDiGetDeviceInstanceIdW(hdev, &data, instance.as_mut_ptr(), instance.len() as u32, std::ptr::null_mut());
            let id = if ok != 0 { utf16_to_string(&instance) } else { "?".into() };
            let mut battery: u8 = 0;
            let mut ty: u32 = 0;
            let mut needed: u32 = 0;
            let got = SetupDiGetDevicePropertyW(hdev, &data, &DEVPKEY_BT_BATTERY, &mut ty, &mut battery as *mut u8, 1, &mut needed, 0);
            // 对照：读一个确定存在的属性（FriendlyName），验证调用方式本身没问题
            const DEVPKEY_DEVICE_FRIENDLYNAME: DEVPROPKEY = DEVPROPKEY {
                fmtid: windows_sys::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
                pid: 14,
            };
            let mut fname = [0u16; 256];
            let mut fty: u32 = 0;
            let mut fneed: u32 = 0;
            let fgot = SetupDiGetDevicePropertyW(
                hdev,
                &data,
                &DEVPKEY_DEVICE_FRIENDLYNAME,
                &mut fty,
                fname.as_mut_ptr() as *mut u8,
                (fname.len() * 2) as u32,
                &mut fneed,
                0,
            );
            let fname_s = if fgot != 0 { utf16_to_string(&fname) } else { "-".into() };
            out.push(format!("node {} prop={} ty={} val={} | friendly={}", id, got, ty, battery, fname_s));
        }
        SetupDiDestroyDeviceInfoList(hdev);
    }
    out
}
