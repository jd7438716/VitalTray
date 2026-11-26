# 变更日志

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [1.0.0] - 2026-11-26

首个公开版本，M0–M7 全部落地。

### 新增

- **托盘常驻**：左键弹只读状态面板，右键弹原生菜单；单实例，重复启动只会唤出已有进程。
- **状态面板**（左键）：品牌 / 型号 + CPU 占用与温度、GPU 型号/占用/温度、内存、系统盘、外设电量；
  2× 超采样 GDI 绘制、明暗主题跟随系统、按鼠标所在显示器的 DPI 与工作区摆位、自动避开屏幕边缘。
- **右键菜单**：立即刷新 / 打开日志目录 / 开机自启 / 托盘图标开关 / 图标样式 / 调试日志 / 关于 / 退出。
- **设备域插件**：`mouse` / `keyboard` / `laptop` / `bluetooth` 四个插件 crate；
  加一个域 = 新建 crate + 在 `plugins.rs` 的 `domains()` 里加一行。
- **品牌协议**：Logitech HID++（feature 索引 + 电池状态）、Rapoo VT 系列（无线接收器 / 有线）。
- **历史与续航估算**：按设备 JSONL 滚动存储，RLS + Theil-Sen 双估计，样本不足时明确标「数据不足」。
- **低电提醒**：托盘气泡通知（阈值可配）。
- **分级开发日志**：`%LOCALAPPDATA%\BatteryHub\app.log`，`error < warn < info < debug < trace`，
  1 MB × 3 滚动，菜单「打开日志目录」一键定位。
- **i18n**：中 / 英切换。
- **便携模式** `--portable`：配置与日志留在 exe 同目录，不写注册表。
- **调试 CLI**：`--once`、`--dump`、`--watch vid:pid`、`--sysmon`、`--panel`、`--self-test`、`--log-level`、`--config`。
- **工程门禁**：单文件 ≤ 400 行（`tools/check-size.ps1`）；CI 跑 clippy / 单测 / release 构建 / `--self-test`；
  推 `v*` tag 自动构建并把 exe + `SHA256SUMS.txt` 发到 GitHub Releases。

### 变更

- 托盘悬停提示只显示标题 `BatteryHub`，电量与续航全部进左键面板，提示里不再出现数字。
- 任务栏文本 widget 下线（M7 决策）：信息全部进面板，不再往任务栏塞文字。
- 主程序改为 GUI 子系统，从开始菜单 / 开机自启启动不再弹终端窗口。

### 已知限制

- exe 未签名，首次运行可能有 SmartScreen 提示（发布页 `SHA256SUMS.txt` 可自行核对）。
- 不做自动更新（v1 明确不做）。
- 只挂主显示器；多显示器任务栏未做。
- Logitech 接收器多设备配对表、校准向导 UI 未做（当前只能用 CLI 抓帧）。
- 温度 / 占用拿不到时显示 “—”，不猜数字（设计如此，不是缺陷）。
- 72 小时常驻稳定性验收尚未执行，留待 1.0.1 前补。
- 不支持 Linux / macOS。
