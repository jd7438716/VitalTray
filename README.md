# BatteryHub（自研版）

[![CI](https://github.com/jd7438716/battery-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/jd7438716/battery-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Windows 10/11 托盘工具：**无线外设电量 + 整机温度面板**（CPU / GPU / 硬盘温度）。
Rust + 原生 Win32，零第三方运行时，MIT。

> 状态：**M0–M8 完成**。左键托盘看面板（温度 + 设备电量），右键托盘做设置（原生菜单）。
> 计划书：[PLAN.md](PLAN.md) ｜ 插件架构决策：[docs/adr/0001-plugin-architecture.md](docs/adr/0001-plugin-architecture.md)

![状态面板](docs/screenshot-panel.png)

> 本机实拍：ASUS Zenbook 14 UX3405MA。CPU 温度、GPU 型号、硬盘温度、外设电量与品牌/型号一屏可见；
> 驱动不提供的温度显示 “—”，不编造数字。**面板只报温度不报占用**——占用百分比归任务管理器。

## 下载与安装

到 [Releases](https://github.com/jd7438716/battery-hub/releases) 下载 `batteryhub.exe`，双击即可运行
（GUI 子系统，不会弹黑窗）。

- **校验下载**：发布页 `SHA256SUMS.txt` 里有哈希，
  ```powershell
  Get-FileHash .\batteryhub.exe -Algorithm SHA256
  ```
- **便携模式**：`batteryhub.exe --portable` —— 配置与日志放 exe 同目录，不写注册表。
- **开机自启**：默认写用户级 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，不需要管理员；
  右键托盘可随时关掉。
- **卸载**：右键托盘 → 退出；删掉 exe；再删 `%LOCALAPPDATA%\BatteryHub` 与注册表值
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\BatteryHub`。
- **SmartScreen**：exe 未签名，首次运行可能提示「Windows 已保护你的电脑」→ 更多信息 → 仍要运行；
  介意的话用上面的 SHA256 自行核对。

当前版本：**1.1.0**（变更见 [CHANGELOG.md](CHANGELOG.md)）。

## 从源码构建

```powershell
cargo build --release
.\target\release\batteryhub.exe             # 启动（托盘出现图标）
.\target\release\batteryhub.exe --panel     # 启动后直接在鼠标位置弹出面板（调试/截图）
.\target\release\batteryhub.exe --sysmon    # 采样一次温度（顺带打印占用/空间，排障用）并打印
.\target\release\batteryhub.exe --self-test # 无界面自检
```

常用命令：`--once`（读一次电量）、`--dump`（列 HID 设备）、`--watch vid:pid`（只读抓报文）、
`--sysmon`（系统状态采样）、`--panel`（直接弹面板）。

## 交互

- **左键托盘图标** → 只读温度面板：品牌型号 + CPU/GPU/硬盘温度 + 设备电量，
  贴着鼠标弹出、自动避开屏幕边缘。
- **状态色**：绿 = 所有设备 >80%；中性 = 20–80%；红 = 任一设备 ≤ 低电阈值（默认 20%）；
  蓝 = 充电中；灰 = 上次读数 / 无数据。细则见 [docs/status-colors.md](docs/status-colors.md)。
- **右键托盘图标** → 原生菜单做功能：立即刷新 / 打开日志目录 / 开机自启 / 托盘图标开关 /
  图标样式 / 调试日志 / 关于 / 退出。
- **鼠标悬停托盘图标** → 提示只显示标题 `BatteryHub`；电量、续航都在左键面板里，提示不重复数字。
- 面板里**只有信息、没有按钮**：点任意处、按 `Esc`、再点一次托盘 → 收起；
  在面板上按右键也是收起面板、把菜单交出来（左键=看，右键=做，两边不混）。
- **托盘图标是唯一的菜单入口**：菜单里关掉它之后，再运行一次 `batteryhub.exe` 就会弹出面板，
  在面板上右键即可把菜单拿回来（不用去改配置文件）。
- 第二次双击 exe **不会**再起一个进程，而是把已经在跑的那个叫出来（以前是静默退出、看起来像没反应）。
- **任务栏文本已移除**：按用户反馈，不再往任务栏里塞文字；所有信息进面板。

## 显示内容与数据来源

面板只回答“**热不热**”，不回答“忙不忙”——占用率（CPU/GPU/内存/磁盘 I/O）交给任务管理器。

| 项目 | 内容 | 来源（全部用户态、无需管理员） |
|---|---|---|
| CPU | 温度 °C | PDH `Thermal Zone Information(*)\High Precision Temperature`（ACPI 热区，0.1 K） |
| GPU | 型号 + 温度 °C | D3DKMT 枚举适配器（`KMTQAITYPE_ADAPTERREGISTRYINFO` / `_ADAPTERPERFDATA`） |
| 硬盘 | 温度 °C + 可用/总量 | `IOCTL_STORAGE_QUERY_PROPERTY` 读 NVMe SMART/Health log page（复合温度）；`GetWindowsDirectoryW` + `GetDiskFreeSpaceExW` |
| 品牌 / 型号 | OEM 品牌、整机型号、CPU 型号 | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` / `CentralProcessor\0` |
| 外设电量 | 鼠标 / 键盘 / 耳机… + 品牌型号 | 插件（本机电源、蓝牙 DEVPKEY、HID++、Rapoo…） |

**关于温度**：三路读数各自依赖硬件/驱动，拿不到就显示 “—”，绝不编造。

- **CPU**：固件暴露的 ACPI 热区。口径由固件决定，有的机型常年偏高（本机就常在 85–100 °C）。
- **GPU**：看驱动是否在 `KMTQAITYPE_ADAPTERPERFDATA` 里上报温度（NVIDIA/AMD 常见；**Intel 核显多为 0**，
  本机 Intel Arc 就是 “—”）。核显没有别的用户态温度源。
- **硬盘**：NVMe 的 SMART 复合温度，走协议专有查询即可读到，**不需要管理员**。
  注意 `Get-StorageReliabilityCounter` 那条路是要管理员的，别混为一谈；SATA/机械盘或老驱动读不到就是 “—”。
- **内存**：不显示——板载内存没有温度传感器，内存占用也不归这个面板。

**不做也不建议**为了读数装内核驱动或提权。

## 目录

```text
crates/bh-api      插件 API（不依赖平台，插件只依赖它）
crates/bh-core     宿主：配置 / 日志 / 单实例 / 自启 / 托盘 / 面板 / 系统采样 / 消息循环
crates/bh-app      可执行入口（build.rs 把图标 / 版本信息 / 应用清单编进 exe）
plugins/           设备域插件：mouse / keyboard / laptop / bluetooth
assets/            batteryhub.ico（16–256 九档，exe 图标与托盘兜底图标同源）
docs/adr/          架构决策记录
tools/check-size.ps1   文件大小门禁（>400 行失败）
tools/check-exe.ps1    资源门禁（缺图标 / 版本信息 / 清单即失败）
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
- exe 必须带图标 / 版本信息 / 应用清单（`tools/check-exe.ps1`，CI 与 release 都会跑）。
  曾经把三样全缺的 exe 发上了 Releases——资源管理器里是通用白窗图标，属性页一片空白。
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
- **v1.0.0 ✅** 首个公开版本（2026-11-26），见 [CHANGELOG.md](CHANGELOG.md)
- **待办**：Logitech 接收器多设备配对表；校准向导 UI；磁盘 SMART 温度（需要管理员，暂不做）。

## 许可

MIT。
