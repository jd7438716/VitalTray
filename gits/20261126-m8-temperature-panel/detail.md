# 变更明细（detail）

## A. 本会话：面板改温度 + 温度链路性能

### 面板

| 文件 | 改动 |
|---|---|
| `crates/bh-core/src/panel/model.rs` | 系统段三行只留温度：CPU/GPU 用 `temp_text`、系统盘显示硬盘温度；删除内存行；`Icon::Memory` 移除；`metric`/`metric_tone`/`usage_tone` 换成 `temp_text`/`temp_tone_of` |
| `crates/bh-core/src/panel/icons.rs` | 删掉 `Icon::Memory` 的图标绘制（避免 dead_code） |
| `docs/screenshot-panel.png` | 新面板截图（per-monitor aware 截窗口矩形，608×820） |

### 采样

| 文件 | 改动 |
|---|---|
| `crates/bh-core/src/sysmon/smart.rs`（新） | NVMe 温度：`IOCTL_STORAGE_QUERY_PROPERTY` + `StorageDeviceProtocolSpecificProperty(50)` + SMART/Health log page；解析开尔文复合温度；20 秒缓存；`ProtocolDataOffset` 两种参照点都试 |
| `crates/bh-core/src/sysmon/disk.rs` | `DiskInfo` 加 `temp_c`；盘空间 30 秒缓存（单次 41–57 µs，没必要每 2 秒问一次卷） |
| `crates/bh-core/src/sysmon/pdh.rs` | `Pdh::new(gpu_usage: bool)`：可选是否挂 `\GPU Engine(*)` |
| `crates/bh-core/src/sysmon/mod.rs` | `spawn()` 后台不采占用；新增 `spawn_with_usage()` 供 `--sysmon` |
| `crates/bh-core/src/probe.rs` | `--sysmon` 打印硬盘温度 + 读不到时的提示；改用 `spawn_with_usage()` |
| `crates/bh-core/Cargo.toml` | `windows-sys` 加 `Win32_System_IO`（`DeviceIoControl`） |

### 文档

`README.md`（显示内容表改成温度口径、「关于温度」分三路说明）、`CHANGELOG.md`（`[未发布]`）、`DEVLOG.md`（M8 两条：面板改温度 / 温度链路性能账）、`PLAN.md`（M8 里程碑）。

### 实测

- 单次采样：`GetSystemTimes` 4–12 µs、`GlobalMemoryStatusEx` 0.3 µs、GPU 温度 ~1 µs、
  PDH `collect`（只温度）87 µs、`\GPU Engine(*)` 取值 595 µs（523 实例）、`GetDiskFreeSpaceExW` 41–57 µs、
  NVMe SMART 2.8 ms（20 秒一次）。
- 每帧 ~1.0 ms → ~0.2 ms；整进程 CPU 0.16% → 0.05% 一个核。
- 常驻进程吃满一个核的真凶是抓帧线程（与温度无关），已由并行线修复。

## B. 并行线（同一工作树，一并提交）

| 范围 | 内容 |
|---|---|
| 状态色 | `crates/bh-core/src/status.rs`（新）、`theme.rs`、`panel/theme.rs`、`tray/icons.rs`、`panel/paint.rs`、`docs/status-colors.md`、`docs/status-icons.png` |
| 厂商协议注册表 | `crates/bh-api/src/protocol.rs`（新）、`crates/bh-core/src/registry.rs`（新）、`plugins.rs`、`docs/adr/0002-vendor-protocol-registry.md`、`bh-proto-logitech` / `bh-proto-rapoo` 适配 |
| Razer | `crates/bh-proto-razer/`（新 crate）、`Cargo.toml` / `Cargo.lock` |
| 抓帧 | `capture.rs`：VID 白名单、失败摘句柄 + 60 秒隔离、循环必睡；`app.rs` 默认不抓 |
| 轮询 | `poller.rs`：一轮只枚举一次、多设备并行、失败退避；幽灵设备抑制（`DeviceReading.ghost`） |
| 其它 | `crates/bh-core/examples/icons.rs` 图标预览、插件 crate 去重依赖 |

> 以上按工作树快照整体提交；并行线的后续改动从本提交之后继续。
