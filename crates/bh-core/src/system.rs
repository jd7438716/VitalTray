//! 本机电源信息（GetSystemPowerStatus），供 laptop 插件使用。

use bh_api::{SystemBattery, SystemInfo};
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Default)]
pub struct HostSystem;

impl SystemInfo for HostSystem {
    fn battery(&self) -> Option<SystemBattery> {
        let mut st: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetSystemPowerStatus(&mut st) };
        if ok == 0 {
            return None;
        }
        // BatteryFlag: 128 = 无电池, 255 = 未知
        if st.BatteryFlag == 128 || st.BatteryFlag == 255 {
            return None;
        }
        let percent = if st.BatteryLifePercent == 255 { return None } else { st.BatteryLifePercent };
        let seconds_left = if st.BatteryLifeTime == u32::MAX { None } else { Some(st.BatteryLifeTime) };
        let ac_online = st.ACLineStatus == 1;
        Some(SystemBattery {
            percent,
            charging: ac_online,
            ac_online,
            seconds_left,
        })
    }

    fn bluetooth_battery(&self, key: &str) -> Option<u8> {
        crate::bt::lookup(key)
    }
}
