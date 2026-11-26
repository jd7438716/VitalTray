//! 排障工具：找雷蛇控制接口，并打印各 HID 集合对 90 字节 feature report 的真实反应。
//!
//! 用法：`cargo run -p bh-proto-razer --example razer_probe`
//!
//! 结论会写回 `bh-proto-razer` 的 `endpoint()` 与 `read_battery()`。

fn main() {
    let list = match bh_hid::enumerate() {
        Ok(l) => l,
        Err(e) => {
            println!("枚举失败: {e}");
            return;
        }
    };
    let cands: Vec<_> = list.into_iter().filter(|d| d.info.vendor_id == 0x1532).collect();
    if cands.is_empty() {
        println!("没有发现雷蛇设备（VID_1532）。");
        return;
    }
    println!("雷蛇设备 {} 条 collection：\n", cands.len());
    for d in cands {
        let i = &d.info;
        print!(
            "{:04x}:{:04x} usage={:04x}:{:04x}  {:<26}  ",
            i.vendor_id, i.product_id, i.usage_page, i.usage, i.product
        );
        match bh_hid::open(&d) {
            Ok(mut h) => match bh_proto_razer::read_battery(&mut h, std::time::Duration::from_millis(400)) {
                Ok(r) => println!("应答 soc={} charging={:?}", r.soc, r.charging),
                Err(e) => println!("无应答 {e:?}"),
            },
            Err(e) => println!("打不开 {e}"),
        }
    }
}
