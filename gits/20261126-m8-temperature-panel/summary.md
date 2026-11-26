# 提交「面板改温度 + 并行线」变更（summary）

- logid: `20261126-m8-temperature-panel`
- 时间: 2026-11-26 14:30 (Asia/Shanghai)
- 基线: `d6330f8 chore(release): v1.0.0`
- 目标: 把用户验收过的「面板从资源占用改成温度」这一轮改动提交

## 结论

工作树里同时存在**两条并行开发线**，且改动在 hunk 级交叉，无法只提交其一还保持可编译：

1. **本会话（温度面板 + 温度链路性能）**：`panel/model.rs`、`panel/icons.rs`、`sysmon/{smart,disk,pdh,mod}.rs`、`probe.rs`、`bh-core/Cargo.toml`、文档与面板截图。
2. **另一会话（状态色 / 厂商协议注册表 / 抓帧修复 / 轮询性能）**：`theme*`、`tray/icons.rs`、`status.rs`、`registry.rs`、`bh-api/protocol.rs`、`bh-proto-*`、`capture.rs`、`poller.rs`、`app.rs`、`docs/adr/0002*` 等。

因此本次为**整树一个提交**（两条线一并入库）。理由与备选见 `decision-chain.md` D1。

## 关键事实

| 项 | 值 |
|---|---|
| 变更文件 | 33 改 + 6 新增（含 `crates/bh-proto-razer/` 新 crate） |
| `cargo build --release` | ✅ |
| 单测 | ✅ 67 个（bh-core 49 + 其它域 17 + self-test 1） |
| `--self-test` | ✅ SELFTEST OK |
| 行数门禁 | ✅ 64 文件 0 超限（1 warning：`poller.rs` 337 行） |
| 提交前动作 | 停掉常驻 `batteryhub.exe`（否则 exe 被占用，无法链接验证） |
| push | ❌ 未做（用户只要求提交） |

## 温度面板实测（本机 ASUS Zenbook 14 / Ultra 9 185H / Intel Arc / ZHITAI 4TB）

| 项 | 结果 |
|---|---|
| CPU 温度 | 82–100 °C（ACPI 热区 `\_TZ.THRM`，负载一来就顶 TjMax） |
| 硬盘温度 | 47–50 °C（NVMe SMART 直读，免管理员） |
| GPU 温度 | “—”（Intel 核显不上报；用户态无解，见 decision-chain D5） |
| 内存温度 | 无（板载 LPDDR5 无传感器，行已删除） |
| 每帧采样开销 | ~1.0 ms → ~0.2 ms；整进程 CPU 0.16% → 0.05% 一个核 |

见 `detail.md`（逐文件）与 `decision-chain.md`（取舍与后果）。
