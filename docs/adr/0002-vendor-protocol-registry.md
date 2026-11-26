# ADR-0002：厂商协议注册表（硬件检测统一下发）

- 状态：已采纳（v1.1 实现）
- 日期：2026-11-26
- 关联：ADR-0001（按设备域切分插件）

## 背景

ADR-0001 把"设备域"做成了插件（mouse / keyboard / laptop / bluetooth），但**厂商协议没有插件化**：
`bh-plugin-mouse::matches()` 里写死了 `0x046D`（罗技）和 `0x24AE`（雷柏），`read()` 里还有按 vendor 的 if-else。
后果：

1. 加一个新品牌（雷蛇 / 双飞燕 / 赛睿 …）= 去改鼠标插件和键盘插件，而不是加插件；
2. 每个域插件都要重复实现同一套 vendor 分支；
3. 匹配成本随"域 × 设备"增长，且插件各自枚举 HID（一个轮次枚举 2 次以上，单次 30ms）。

用户诉求原话：「检测到雷蛇的时候增加雷蛇，检测到双飞燕的时候增加双飞燕 …… 或者硬件检测统一下发给插件。」

## 决策

引入第二层扩展点：**厂商协议注册表**（`bh-core::registry`），宿主统一检测、协议只回答"怎么读"。

三层职责：

| 层 | 职责 | 扩展方式 |
|---|---|---|
| 宿主 `bh-core` | **一次**枚举 → `DeviceTable` 快照；句柄生命周期、超时预算、退避、UI 行 | 不改 |
| 域插件 `bh-plugin-*` | 设备分类（图标/菜单），不碰 HID | 极少数情况 |
| 协议 `bh-proto-*` | 按 VID 认领电量通道 `Endpoint`，实现 `read(io, budget)` | **加一个 crate + 注册表一行** |

核心契约（`bh-api::protocol`）：

```rust
pub trait VendorProtocol: Send + Sync {
    fn id(&self) -> &'static str;
    fn vid(&self) -> u16;                                    // 索引键
    fn endpoint(&self, d: &DeviceInfo) -> Option<Endpoint>;  // 纯计算，禁止 IO
    fn read(&self, d: &DeviceInfo, io: &mut dyn HidIo, budget: Duration) -> Result<Reading, DriverError>;
}
```

注册只有一处：

```rust
static PROTOCOLS: &[&'static dyn VendorProtocol] =
    &[&bh_proto_logitech::PROTO, &bh_proto_rapoo::PROTO, &bh_proto_razer::PROTO];
```

启动时建成 `VID → Vec<&dyn VendorProtocol>`，之后分发就是一次 `HashMap` 查询。

## 为什么不做动态加载

用户说的"检测到才增加"要的是**效果**，不是运行时加载：

- 静态表 + VID 索引下，没插的厂商在热路径上就是一次 `HashMap` miss，零成本；
- 真动态加载（Rust dylib / WASM / sidecar）在 ADR-0001 里已经否决：ABI 不稳、要额外运行时、单 exe 与"零第三方运行时"的约束都不保。
- 代价对比：加品牌 = 编译期多一个百行 crate（类型安全、可单测），而不是运行时多一份加载器与沙箱。

## 配套的性能约束（与本次改动绑定）

1. **一轮只枚举一次**：`collect_once()` 产出快照后全轮复用（旧实现每个设备再 `enumerate()`，单次 30ms）。
2. **并行读**：`std::thread::scope`，一轮耗时 = `max(budget)` 而非 `sum`（雷柏双通道 8.3s → 6.6s，且与设备数无关）。
3. **ProbeState 退避**：读不到的设备按 1min → 2min → 5min 退避，不再每轮空等；`request_refresh()` 解除全部退避。
4. **幽灵行抑制**：判定依据是"成功读到过一次"，不是"枚举到了"。接收器常驻但从未上报的通道不建行
   （`DeviceReading::ghost`，面板与托盘汇总都跳过它）。

## 后果

- 好处：加品牌只碰"新 crate + 一行"；域插件里再无 vendor 分支；轮询成本可预测；面板不再出现幽灵设备。
- 代价：`bh-api` 多了一组 trait（`VendorProtocol` / `Endpoint` / `Mode` / `DeviceTable`）；`DeviceReading` 多一个 `ghost` 字段。
- 约束：`endpoint()` 必须是纯计算（每轮对每条 collection 都会跑），任何 IO 放到 `read()` 里。
- 遗留：罗技接收器的一条 HID++ 通道同时服务鼠标与键盘，`Endpoint.glyph` 目前固定给 Mouse（沿用旧行为），
  要区分得靠 product string 或配对槽位信息。

## 实测（2026-11-26）

- 雷蛇 Viper Ultimate（1532:007a 有线）：MI_02 控制接口被厂商驱动占用（用户态 feature report `0x1 函数不正确`），
  鼠标集合能写 90 字节命令但设备一律回 `0x1F`。有线模式下电量本身无意义 → 协议只认无线 dongle（1532:007b）。
  复核工具：`cargo run -p bh-proto-razer --example razer_probe`。
- 雷柏接收器（24ae:185a）：鼠标 `ff00:0002` 与键盘 `ff00:000e` 两条通道都静态存在，两条都无帧 → 都是幽灵行，面板果然不再显示。
