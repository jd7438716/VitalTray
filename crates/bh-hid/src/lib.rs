//! HID 枚举与读写（hidapi 封装），实现 bh_api::HidIo。

use bh_api::{DeviceInfo, DriverError, HidIo, Reading, Transport};

pub struct Device {
    pub info: DeviceInfo,
}

/// 枚举当前系统上的 HID 设备。
pub fn enumerate() -> Result<Vec<Device>, String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for d in api.device_list() {
        let product = d.product_string().unwrap_or("HID").to_string();
        let serial = d.serial_number().map(|s| s.to_string());
        let vid = d.vendor_id();
        let pid = d.product_id();
        let key = match &serial {
            Some(s) if !s.is_empty() => format!("{vid:04x}:{pid:04x}:{s}"),
            _ => format!("{vid:04x}:{pid:04x}"),
        };
        out.push(Device {
            info: DeviceInfo {
                vendor_id: vid,
                product_id: pid,
                product,
                serial,
                usage_page: d.usage_page(),
                usage: d.usage(),
                transport: classify(d.bus_type()),
                key,
            },
        });
    }
    Ok(out)
}

fn classify(b: hidapi::BusType) -> Transport {
    match b {
        hidapi::BusType::Bluetooth => Transport::Bluetooth,
        hidapi::BusType::Usb => Transport::Usb,
        _ => Transport::Usb,
    }
}

/// 打开一个设备并返回读写句柄。
pub fn open(dev: &Device) -> Result<HidHandle, String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    // hidapi 没有按 key 打开：用 VID/PID + serial 优先匹配
    let mut fallback = None;
    for d in api.device_list() {
        if d.vendor_id() == dev.info.vendor_id && d.product_id() == dev.info.product_id {
            // 同一物理设备有多个 collection：优先匹配 usage page/usage，否则会打开错误的接口
            if dev.info.usage_page != 0
                && (d.usage_page() != dev.info.usage_page || d.usage() != dev.info.usage)
            {
                continue;
            }
            let serial_match = match (&dev.info.serial, d.serial_number()) {
                (Some(a), Some(b)) => a == b,
                (None, None) => true,
                _ => false,
            };
            if serial_match {
                if let Ok(h) = d.open_device(&api) {
                    return Ok(HidHandle { inner: h });
                }
            } else if fallback.is_none() {
                if let Ok(h) = d.open_device(&api) {
                    fallback = Some(h);
                }
            }
        }
    }
    if let Some(inner) = fallback {
        return Ok(HidHandle { inner });
    }
    Err(format!("cannot open {} ({:04x}:{:04x})", dev.info.product, dev.info.vendor_id, dev.info.product_id))
}

pub struct HidHandle {
    inner: hidapi::HidDevice,
}

impl HidIo for HidHandle {
    fn feature_get(&mut self, report_id: u8, buf: &mut [u8]) -> Result<usize, DriverError> {
        if buf.is_empty() {
            return Err(DriverError::Malformed);
        }
        buf[0] = report_id;
        self.inner.get_feature_report(buf).map_err(|e| DriverError::Io(e.to_string()))
    }

    fn feature_set(&mut self, report_id: u8, buf: &[u8]) -> Result<(), DriverError> {
        let mut frame = Vec::with_capacity(buf.len() + 1);
        frame.push(report_id);
        frame.extend_from_slice(buf);
        self.inner.send_feature_report(&frame).map_err(|e| DriverError::Io(e.to_string()))
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), DriverError> {
        self.inner.write(buf).map(|_| ()).map_err(|e| DriverError::Io(e.to_string()))
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, DriverError> {
        self.inner.read_timeout(buf, (timeout_ms as i32).max(0)).map_err(|e| DriverError::Io(e.to_string()))
    }
}

/// 读取一次电量（供插件调用前的便利函数）。
pub fn read_with(domain: &dyn bh_api::DeviceDomain, dev: &Device) -> Result<Reading, DriverError> {
    let mut h = open(dev).map_err(DriverError::Io)?;
    let mut ctx = bh_api::ReadContext { hid: Some(&mut h), system: None, now_ms: now_ms() };
    domain.read(&dev.info, &mut ctx)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
