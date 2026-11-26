# 第三方来源与许可

> 本文回答两个问题：① 本项目读过哪些开源项目；② 各自许可要求什么、我们怎么满足。
> 纪律依据：[PLAN.md](../PLAN.md) 第 2 节「只抄思路：洁净室规则」。
> 随 exe 分发的第三方许可全文见 [THIRD-PARTY-NOTICES.txt](../THIRD-PARTY-NOTICES.txt)（自动生成）。

## 0. 一句话结论

VitalTray（原 BatteryHub）以 **MIT** 发布，仓库内**不含任何第三方源代码**：参考用的仓库 clone 全部放在 `research/`，
该目录在 [.gitignore](../.gitignore) 里整目录忽略，`git ls-files research` 结果为空。

所以当前**没有**任何 GPL / MPL 的义务被触发。本文的价值不在于"合规"本身，而在于**可自证**：
哪天被上游作者或第三方质疑"抄了 GPL 代码"，能当场拿出证据链——哪些只是读了思路、
哪些是许可允许的依赖、协议事实从哪来。

## 1. 参考来源清单

### 1.1 只读思路 / 协议事实（无代码进入仓库）

| 项目 | 上游 | 许可 | 参考内容 | 落地位置 |
|---|---|---|---|---|
| BatteryHub（C++/Qt） | [krisgfx9/Wireless-Device-Battery-Monitor](https://github.com/krisgfx9/Wireless-Device-Battery-Monitor) | **GPL-3.0** | 托盘/任务栏交互、驱动表架构、设备覆盖范围 | 思路 → [PLAN.md](../PLAN.md) §3、§6 |
| BatteryHub-Community | [Hassani0x/BatteryHub-Community](https://github.com/Hassani0x/BatteryHub-Community) | GPL-3.0（上游派生的独立 fork） | 同上，补充设备列表 | 思路 |
| Wireless Battery Center | [yunluoxincheng/Wireless-Battery-Center](https://github.com/yunluoxincheng/Wireless-Battery-Center) | GPL-3.0（上游派生） | 设备中心式 UI、分组展示 | 思路 |
| Wireless-Device-Battery-Monitor-CN | [fuyue521/Wireless-Device-Battery-Monitor-CN](https://github.com/fuyue521/Wireless-Device-Battery-Monitor-CN) | GPL-3.0（上游派生） | 中文本地化与文案口径 | 思路 |
| Wireless-Device-Battery-Monitor | [ferloom/Wireless-Device-Battery-Monitor](https://github.com/ferloom/Wireless-Device-Battery-Monitor) | GPL-3.0（上游派生） | 设备上报缺失时的降级显示 | 思路 |
| TaskbarVitals | [symNistaken/TaskbarVitals](https://github.com/symNistaken/TaskbarVitals) | GPL-3.0（未声明渊源，按 GPL 保守处理） | 任务栏文本 + 系统占用信息的组合方式（本项目**未**采用占用信息） | 思路 |
| Battery Hub（Electron） | [battery-hub/battery-hub](https://github.com/battery-hub/battery-hub) | **MIT** | 面板信息层级（电量环/续航估算）的表达方式 | 思路 |
| Solaar | [pwr-Solaar/Solaar](https://github.com/pwr-Solaar/Solaar) | **GPL-2.0** | Logitech HID++ 2.0 帧格式与特性号 | 事实 → [protocol-notes/logitech-hidpp.md](protocol-notes/logitech-hidpp.md) |
| OpenRazer | [openrazer/openrazer](https://github.com/openrazer/openrazer) | **GPL-2.0-or-later** | Razer 90 字节 feature report、命令类 0x07 | 事实 → [bh-proto-razer](../crates/bh-proto-razer/src/lib.rs) 文件头 |
| HeadsetControl | [Sapd/HeadsetControl](https://github.com/Sapd/HeadsetControl) | **GPL-2.0** | 无线耳机（Logitech/Sony 等）电量查询方式 | **计划中**，耳机域插件尚未实现 |
| LibreHardwareMonitor | [LibreHardwareMonitor/LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor) | **MPL-2.0** | 温度读数口径（哪些源可信、哪些要管理员） | 思路 → [README](../README.md)「关于温度」 |
| rivalcfg | [Sapd/rivalcfg](https://github.com/Sapd/rivalcfg) | WTFPL | 雷蛇电池查询命令 | **未使用**（协议事实改用 OpenRazer 口径） |
| BlueGauge | BlueGauge（开源电量面板，上游链接待补） | MIT | 设备面板配色与图标语义 | **未使用** |
| XiuxiTimer（休息提醒） | Gitee 同名仓库 | **未核实** | 托盘右键菜单项顺序与自启开关位置 | 思路，待补许可核实 |

> ⚠️ **两个同名项目，别混**：
> - `krisgfx9/Wireless-Device-Battery-Monitor`（C++/Qt，v1.1.0 起改名叫 BatteryHub）= **GPL-3.0**
> - `battery-hub/battery-hub`（Electron）= **MIT**
>
> 两者都叫 "Battery Hub"，许可却一个 GPL 一个 MIT。早期 [PLAN.md](../PLAN.md) 的许可表把二者写成一行
> 「原版 BatteryHub / 4 个 C++ fork | GPL-3.0」，既把 MIT 项目错标成 GPL，又数错了 fork 数量
> （实际 5 个），已于 2026-11-26 修正。

### 1.2 实际进入二进制的依赖（许可允许，需随 exe 附声明）

| 组件 | 版本 | 许可 | 说明 |
|---|---|---|---|
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | Win32 API 绑定 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | windows-sys 的链接元数据 |
| hidapi | 2.6.7 | MIT | HID 设备访问（Rust 封装） |
| HIDAPI C 库 | 内嵌于 hidapi | **BSD-3-Clause**（本项目选举，见 §2） | Windows 后端静态编译进 exe |
| libc | 0.2.191 | MIT OR Apache-2.0 | hidapi 的跨平台依赖，Windows 上可能被条件编译排除 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | 编译期宏，无残留代码 |

构建期工具（`cc` / `find-msvc-tools` / `shlex` / `pkg-config`）只在编译时运行、不随 exe 分发，
但仍一并列在 notices 里便于审计。

### 1.3 明确禁止参考

| 项目 | 原因 |
|---|---|
| ZGMFX01A（Rust 版电量工具） | 自定义许可，All rights reserved，禁止参考代码 |
| BatteryMode | EULA，非开源，禁止参考 |

## 2. 许可选举（多许可组件选哪份）

| 组件 | 可选许可 | 本项目选举 | 理由 |
|---|---|---|---|
| HIDAPI C 库 | GPLv3 / BSD-3-Clause / 原始 HIDAPI 许可（三选一，由使用者定） | **BSD-3-Clause** | 与 MIT 兼容、可分发闭源二进制、不引入 GPL 源码开放义务。原文见 hidapi 2.6.7 内嵌 `etc/hidapi/LICENSE.txt` |
| windows-sys / windows-link / libc / cfg-if 等 | MIT OR Apache-2.0 | **MIT** | 与项目主体许可一致，声明义务最轻；Apache-2.0 的专利与 NOTICE 条款对单文件小工具属额外负担 |

> 选举是**单向**的：一旦选了 GPL 就无法退回。所以本项目对所有多许可组件一律选非 GPL 分支，
> 且在 notices 里只抄录选中的那份全文（例如不把 `LICENSE-gpl3.txt` 收进 THIRD-PARTY-NOTICES.txt，
> 避免读者误解本项目选了 GPL）。

## 3. 什么算抄（洁净室判据）

| 分类 | 例子 | 处理 |
|---|---|---|
| 协议事实 | VID/PID、usage page、帧格式、字段偏移、校验算法、时序 | 可用，但必须记入 `docs/protocol-notes/<设备>.md` 并注明来源 |
| 架构思路 | 驱动表、轮询调度、菜单结构、任务栏子窗口挂载 | 可学，必须用自己的方案重新实现 |
| 代码表达 | 函数体、结构体定义、注释、命名、文件组织 | **禁止**复制 |

**"换成 Rust 再写一遍"仍然算抄**：版权法保护的是表达，不是语言。以下做法都不改变衍生性质——
换语言、改变量名、调换语句顺序、加中文注释、让 AI"改写一遍"。判定标准只有一条：
**关掉参考源码后，还能不能只凭自己写的协议笔记把它实现出来**。

## 4. 红线

1. 不把 GPL / MPL 代码带进仓库，包括测试数据、注释里的大段摘抄、示例代码。
2. 不把 `research/` 下的任何文件提交进 git（该目录已整目录忽略，别用 `git add -f` 绕过）。
3. 新增设备协议：先写 `docs/protocol-notes/`，再写实现；CRATE 文件头写一行来源。
4. 改动依赖（`Cargo.toml` / `Cargo.lock`）：必须重跑 `pwsh ./tools/gen-notices.ps1` 并一起提交。

## 5. 复核记录

| 日期 | 动作 | 结论 |
|---|---|---|
| 2026-11-26 | 逐个读取 `research/` 下 7 个第三方 clone 的 LICENSE 文件 | 6 个 GPL-3.0（C++ 系 + 派生），1 个 MIT（Electron 系） |
| 2026-11-26 | 核对 `Cargo.lock` 全部 9 个第三方 crate 的许可字段与 LICENSE 原文 | 均为 MIT / Apache-2.0，无 copyleft |
| 2026-11-26 | 读取 hidapi 内嵌 HIDAPI C 库的三份许可文件 | 三选一，本项目选举 BSD-3-Clause，非 GPL |
| 2026-11-26 | `git ls-files research` | 0 条，第三方源码从未入库 |

> 待办：XiuxiTimer（Gitee）许可未核实，确认后再补进 §1.1。

## 6. 维护方式

- **依赖变化**：`pwsh ./tools/gen-notices.ps1` → 生成 [THIRD-PARTY-NOTICES.txt](../THIRD-PARTY-NOTICES.txt)；
  CI 会跑 `-Check`，忘了重新生成会直接失败。
- **新增参考项目**：本文件加一行（项目 / 上游 / 许可 / 参考内容 / 落地位置），并在对应协议笔记写明来源。
- **PR**：模板里有「本 PR 未复制任何第三方代码」的必勾项，勾之前先自查 §3。
