# BatteryHub（自研版）

[![CI](https://github.com/jd7438716/battery-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/jd7438716/battery-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Windows 10/11 托盘工具：**无线外设电量 + 整机状态面板**（CPU / 内存 / GPU / 磁盘 / 温度）。
Rust + 原生 Win32，零第三方运行时，MIT。

> 状态：**M0–M7 完成**。左键托盘看面板（系统状态 + 设备电量），右键托盘做设置（原生菜单）。
> 计划书：[PLAN.md](PLAN.md) ｜ 插件架构决策：[docs/adr/0001-plugin-architecture.md](docs/adr/0001-plugin-architecture.md)

![状态面板](docs/screenshot-panel.png)

> 本机实拍：ASUS Zenbook 14 UX3405MA。CPU 温度、GPU 占用、内存、系统盘、外设电量与品牌/型号一屏可见；
> 驱动不提供的温度显示 “—”，不编造数字。

## 快速开始

```powershell
cargo build --release
.\target\release\batteryhub.exe             # 启动（托盘出现图标）
.\target\release\batteryhub.exe --panel     # 启动后直接在鼠标位置弹出面板（调试/截图）
.\target\release\batteryhub.exe --sysmon    # 采样一次 CPU/内存/磁盘/GPU/温度并打印
.\target\release\batteryhub.exe --self-test # 无界面自检
```

常用命令：`--once`（读一次电量）、`--dump`（列 HID 设备）、`--watch vid:pid`（只读抓报文）、
`--sysmon`（系统状态采样）、`--panel`（直接弹面板）。

## 交互

- **左键托盘图标** → 只读状态面板：品牌型号 + 系统状态（CPU/GPU/内存/磁盘）+ 设备电量，
  贴着鼠标弹出、自动避开屏幕边缘。
- **右键托盘图标** → 原生菜单做功能：立即刷新 / 打开日志目录 / 开机自启 / 托盘图标开关 /
  图标样式 / 调试日志 / 关于 / 退出。
- **鼠标悬停托盘图标** → 提示只显示标题 `BatteryHub`；电量、续航都在左键面板里，提示不重复数字。
- 面板里**只有信息、没有按钮**：点任意处、按 `Esc`、再点一次托盘 → 收起；
  在面板上按右键也是收起面板、把菜单交出来（左键=看，右键=做，两边不混）。
- **任务栏文本已移除**：按用户反馈，不再往任务栏里塞文字；所有信息进面板。

## 显示内容与数据来源

| 项目 | 内容 | 来源（全部用户态、无需管理员） |
|---|---|---|
| CPU | 占用 % + 温度 °C | `GetSystemTimes`；PDH `Thermal Zone Information(*)\High Precision Temperature`（ACPI 热区，0.1 K） |
| GPU | 型号 + 占用 % + 温度 °C | D3DKMT 枚举适配器（`KMTQAITYPE_ADAPTERREGISTRYINFO` / `ADAPTERPERFDATA`）；PDH `GPU Engine(*)\Utilization Percentage` |
| 内存 | 占用 % + 已用/总量 | `GlobalMemoryStatusEx` |
| 系统盘 | 占用 % + 可用/总量 | `GetWindowsDirectoryW` + `GetDiskFreeSpaceExW` |
| 品牌 / 型号 | OEM 品牌、整机型号、CPU 型号 | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` / `CentralProcessor\0` |
| 外设电量 | 鼠标 / 键盘 / 耳机… + 品牌型号 | 插件（本机电源、蓝牙 DEVPKEY、HID++、Rapoo…） |

**关于温度**：CPU 温度依赖固件暴露的 ACPI 热区；GPU 温度依赖显卡驱动是否在
`KMTQAITYPE_ADAPTERPERFDATA` 里上报（NVIDIA/AMD 常见，部分 Intel 核显为 0）。
拿不到就显示 “—”，并在 `--sysmon` 里给出提示。**不做也不建议**为了读数装内核驱动或提权。

## 目录

```text
crates/bh-api      插件 API（不依赖平台，插件只依赖它）
crates/bh-core     宿主：配置 / 日志 / 单实例 / 自启 / 托盘 / 面板 / 系统采样 / 消息循环
crates/bh-app      可执行入口
plugins/           设备域插件：mouse / keyboard / laptop / bluetooth
docs/adr/          架构决策记录
tools/check-size.ps1   文件大小门禁（>400 行失败）
```

面板代码结构（`crates/bh-core/src/panel/`）：

```text
mod.rs      窗口生命周期、显隐、消息、命中与键盘
window.rs   建窗、DPI、贴鼠标摆位、夹回工作区
model.rs    内容模型（文字 + 语义色），不含像素
layout.rs   竖排布局与命中矩形
paint.rs    2× 超采样 GDI 绘制 + 逐像素 alpha
icons.rs    线段/圆/多边形画的所有图标
gdi.rs      纯绘制小工具
theme.rs    明暗主题配色
```

系统采样在 `crates/bh-core/src/sysmon/`（独立线程每 2 秒一次，UI 只读快照）。

## 插件模型

插件 = **设备域**（鼠标 / 键盘 / 笔记本 / 以后的耳机、手柄、手机），一个域一个 crate：

```rust
impl DeviceDomain for Domain {
    fn glyph(&self) -> Glyph { Glyph::Mouse }
    fn matches(&self, d: &DeviceInfo) -> Match { /* ... */ }
    fn read(&self, d: &DeviceInfo, io: &mut dyn HidIo) -> Result<Reading, DriverError> { /* ... */ }
}
```

加一个域 = 新建 crate + 在 `crates/bh-core/src/plugins.rs` 的 `domains()` 里加一行。
品牌协议不单独做插件，放在协议库 crate 里给多个域复用。

## 配置与自启

- 配置：`%LOCALAPPDATA%\BatteryHub\config.ini`（`--portable` 时放 exe 同目录）。
- 自启：默认写 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，不碰 HKLM、不要管理员；
  菜单里可随时开关；`--no-autostart` 表示本次不改注册表。
- 面板自动读取当前系统明暗主题；多显示器时按鼠标所在显示器的 DPI 与工作区摆位。
- 命令行：`--no-tray`、`--no-capture`、`--portable`、`--config <path>`、`--debug`、`--self-test`、`--sysmon`、`--panel`。

## 日志（开发日志功能）

- 位置：`%LOCALAPPDATA%\BatteryHub\app.log`（便携模式在 exe 同目录）。
- 格式：`2026-11-23T22:47:49.283+08:00 [INFO ] [poll] -> [laptop:78%, mouse:86%]`
- 级别：`error < warn < info < debug < trace`，默认 `info`。
- 切换：`--log-level debug` 或「右键托盘 → 调试日志（详细）」，会写回配置。
- 滚动：单文件 1 MB，保留 3 个历史文件。
- 排障：菜单「打开日志目录」一秒定位。
- 开发过程记录见 [DEVLOG.md](DEVLOG.md)。

## 纪律

- 单文件 ≤ 400 行（`tools/check-size.ps1`，CI 会跑）。
- 洁净室：只参考协议事实，不复制任何 GPL 代码（见 PLAN.md 第 2 节）。
- 温度/占用拿不到就显示 “—”，不猜数字。

## 路线

- **M0 ✅** 骨架 / 托盘 / 菜单 / 自启 / 插件框架
- **M1 ✅** HID 枚举 / 本机电池 / 蓝牙 / Logitech HID++ / 轮询
- **M2 ✅** 任务栏文本 widget（**M7 已下线**）
- **M3 ✅** 历史 / 续航估算 / 低电通知
- **M4 ✅** i18n / 安装包 / 更多驱动
- **M5 ✅** Rapoo 私有协议实测接入
- **M6 ✅** 分级开发日志
- **M7 ✅** 竖排状态面板：CPU/GPU/内存/磁盘/温度、品牌型号、图标、菜单式交互
- **待办**：Logitech 接收器多设备配对表；校准向导 UI；磁盘 SMART 温度（需要管理员，暂不做）。

## 许可

MIT。
