# 提交「打开只驻留托盘」变更（summary）

- logid: `20261126-start-minimized`
- 时间: 2026-11-26 17:35 (Asia/Shanghai)
- 基线: `b6afb01 docs: 仓库链接统一为 jd7438716/VitalTray（仓库名改大小写后同步）`
- 目标: 修掉「打开的时候跳出个菜单，挡住整个页面」（用户反馈触发）

## 结论

用户反馈的「打开」会弹出一块面板，复查发现**两条独立路径**都这样：`--panel` 启动，
以及第二次双击 exe 的唤醒路径（`WAKE_MESSAGE` → `panel::show()`）。本次统一改成
「打开 = 只驻留托盘」，并让两个早就存在但**只写不读**的开关真正生效：
命令行 `--minimized`（自启项与安装脚本一直在传，解析器却不认）与 `[system] start_minimized`
（`Config` 有字段、`save` 会写，但没有任何代码用它做决定）。

## 改动

| 文件 | 改动 |
|---|---|
| `crates/bh-core/src/cli.rs` | 新增 `minimized` 字段 + 解析 `--minimized` + `--help` 选项表加一行 + 2 个解析单测 |
| `crates/bh-core/src/app.rs` | 启动：`--panel` 或（非 `--minimized` 且 `start_minimized=0`）才弹面板；唤醒：托盘图标在就不弹 |
| `crates/bh-core/src/i18n/cli/*.rs` ×10 | 每个语言补 `cli.opt.minimized` 一行（i18n 完整性测试要求 10 种语言对齐） |
| `README.md` / `README.en.md` | 「打开 = 只在托盘里」+ 启动行为 + `--minimized` 说明（双语同步） |
| `CHANGELOG.md` | `[未发布] → 修复` |
| `DEVLOG.md` | 当日记录 + 实测表 |

## 实测（本机，新二进制 SHA256 `7AA347D1AF009EBC3799857FC891D5CFB1EB18563DABC993E2F44C85877A1C93`）

| 场景 | 结果 |
|---|---|
| 无参数启动 | 日志 `start minimized: tray only, no panel`；进程下没有 `VitalTrayPanel` 窗口 |
| 第二次启动（已在跑） | 日志 `wake from second instance: already in tray, stay minimized`；仍是 1 个进程 |
| `--panel` | 面板照常弹出（调试开关未退化） |
| `start_minimized=0` | 面板照常弹出（配置生效） |
| `start_minimized=0` + `--minimized` | 不弹（命令行压过配置） |
| `cargo test --workspace --all-targets` | ✅ 全绿（含 2 个新增 CLI 单测与 i18n 完整性） |
| `check-size` / `check-readme` / `check-cleanroom` / `check-fonts` / `check-exe` | ✅ 全部通过（130 个 `.rs` 0 超限 0 警告） |

## 部署

`tools/install.ps1` 重装同版本二进制：`%LOCALAPPDATA%\Programs\VitalTray\vitaltray.exe` 与
`target\release\vitaltray.exe` SHA256 一致；自启项仍是 `"...vitaltray.exe" --minimized`（现在这个参数有用了）。
版本号与 tag 均未动（用户当时没有要求发版）。

**补记（2026-11-26 17:47）**：随后用户要求发布，本修复已收口成 **v1.2.1** 并发到 GitHub Releases
（`1a650f5` → `0d65783 chore(release): v1.2.1`，tag `v1.2.1`），详见
[`gits/20261126-release-1.2.1/summary.md`](../20261126-release-1.2.1/summary.md)。
