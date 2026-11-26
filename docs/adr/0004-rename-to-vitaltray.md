# ADR 0004：产品更名 BatteryHub → VitalTray

- 状态：已接受（Accepted），2026-11-26 执行
- 相关：[ADR 0003 许可策略](0003-license-strategy.md) ｜ [licenses.md](../licenses.md) ｜ [README 名称与商标](../../README.md)

## 背景

产品从 2026-11-23 起叫 **BatteryHub**。这个名字的问题是：

1. **上游撞名**：本项目重点参考的 C++/Qt 项目在 v1.1.0 **正式把自己改名为 BatteryHub**
   （其 README 第 1–3 行："# BatteryHub … Version 1.1.0 renamed the app to BatteryHub"）。
   我们参考了它的架构思路，却用同一个名字，容易被当成它的衍生版。
2. **第三方撞名**：另有一个 Electron 项目 **Battery Hub**（`battery-hub/battery-hub`）。
3. **检索基数大**：GitHub `battery-hub in:name` 命中 **152** 个同名仓库（2026-11-26 检索），
   而本项目自己的仓库名也叫 `battery-hub`。
4. **许可不解决命名**：MIT / GPL 都只给著作权许可，**不授予商标权**；名字冲突要靠改名解决，
   不是靠许可证声明解决（详见 ADR 0003 §边界）。

## 决策

改名为 **VitalTray**，并做**全链路**更名（展示名 + exe + 资源 + 配置目录 + 注册表 + 仓库），
同时提供**自动迁移**保证老用户无感升级。

## 候选对比（2026-11-26，GitHub `in:name` 检索 + Web 粗查）

| 候选 | GitHub 同名仓库 | Web / 其他占用 | 评价 |
|---|---|---|---|
| **VitalTray** | **0** | 无同名产品 | **选中**：vitals（生命体征）正好覆盖"电量 + 温度"，10 种语言都好念 |
| WattVigil | 0 | 无 | 备选：语义偏抽象（vigil=守夜） |
| CellPerch | 0 | 无 | 备选：含义需解释 |
| AmperTray | 0 | 无 | 备选：读起来像 Ampere 的错拼 |
| TrayWatt | 0 | 已有 **TrayBatt**（Windows 笔记本电池托盘工具） | 淘汰：行业里已近似，易混 |
| BattScope | 0 | App Store 有 **BattScope**（iPhone 电池健康 App） | 淘汰：电池域同名产品已存在 |
| CellDeck | 2 | — | 淘汰：非零冲突 |
| WattWard / WattNest / CellNest | 5 / 5 / 9 | — | 淘汰：非零冲突 |
| Voltiq | 162 | — | 淘汰：重度撞名 |

> 检索口径（可复跑）：`gh api "search/repositories?q=<name>+in:name&per_page=1"` 看 `total_count`；
> Web 用 `<name>` + `battery` / `tray` 组合检索，排除同名产品或商标。
> **注意**：这只是 GitHub + 通用 Web 的粗查，**不是** USPTO / CNIPA 的正式商标检索。
> 若将来上架应用商店或收费，需另做正式检索。

## 命名映射（全链路）

| 面 | 旧 | 新 |
|---|---|---|
| 展示名 / 关于框 / 面板标题 | BatteryHub | VitalTray |
| 可执行文件 | `batteryhub.exe` | `vitaltray.exe` |
| 图标资源 | `assets/batteryhub.ico` | `assets/vitaltray.ico` |
| Windows 版本资源 | ProductName/InternalName/OriginalFilename/CompanyName | 全部 VitalTray / vitaltray.exe |
| 应用清单标识 | `BatteryHub.app` | `VitalTray.app` |
| 配置与日志目录 | `%LOCALAPPDATA%\BatteryHub\` | `%LOCALAPPDATA%\VitalTray\` |
| 开机自启值 | `HKCU\...\Run\BatteryHub` | `HKCU\...\Run\VitalTray` |
| 单实例互斥体 | `Local\BatteryHub.SingleInstance` | `Local\VitalTray.SingleInstance` |
| 安装目录 / 快捷方式 | `Programs\BatteryHub` / `BatteryHub.lnk` | `Programs\VitalTray` / `VitalTray.lnk` |
| GitHub 仓库 | `jd7438716/battery-hub` | `jd7438716/vital-tray`（GitHub 自动保留旧 URL 重定向） |
| CLI 用法文案 / i18n 全部 10 种语言 | batteryhub.exe / BatteryHub | vitaltray.exe / VitalTray |

**刻意不改**：内部 crate 名（`bh-api` / `bh-core` / `bh-app` / `bh-proto-*` …）。
理由：用户不可见；一次性重命名 11 个 crate 会制造巨大 diff 与冲突面，收益为零。若哪天要改，单独做机械提交。

## 迁移方案（老用户无感）

| 项 | 做法 | 代码位置 |
|---|---|---|
| 配置 | 首次启动若无新 `config.ini` 且旧目录有 → **复制**一份；**旧目录不删** | `crates/bh-core/src/paths.rs` `migrate_legacy_config()` |
| 自启 | 启动时删除旧 `Run\BatteryHub`；若配置要求自启，按新名重写 `Run\VitalTray` | `crates/bh-core/src/autostart.rs` `remove_legacy()` + `sync()` |
| 安装/卸载脚本 | 顺带停旧进程、删旧 Run 值/快捷方式/安装目录 | `tools/install.ps1`、`tools/uninstall.ps1` |
| 便携模式 | 不做跨目录迁移（用户自管数据），README 说明手动复制 | README「从 BatteryHub 升级」 |

设计取舍：**复制而不移动**旧配置。旧版 exe 可能还在用户机器上，移动等于替用户做破坏性决定；
复制带来的唯一代价是旧目录残留一份 `config.ini`，卸载脚本会问一次是否删除。

## 后果

- 发版资产名从 `batteryhub.exe` 变为 `vitaltray.exe`：README 的下载/校验命令、`SHA256SUMS.txt`
  内容、CI 产物名（`vitaltray-exe`）已同步；历史 Releases 里的旧资产保持原样（历史事实）。
- GitHub 仓库改名后旧链接（`.../battery-hub`）由 GitHub 重定向，CI badge 与 Releases 链接已改为新路径。
- 已发布的 v1.0.0 / v1.1.1 二进制仍叫 `batteryhub.exe`：它们**不是** VitalTray，README 的升级节已说明。
- 版本号未因改名单独提升：改名随下一次发布进入 CHANGELOG「未发布」，由发版决定 tag。

## 验证（执行时实测）

| 断言 | 命令 | 结果 |
|---|---|---|
| 编译与测试 | `cargo build --locked` / `cargo test --locked --all-targets` | 0 error / 80 passed |
| 资源正确 | `tools/check-exe.ps1` | PASS（GROUP_ICON / VERSION / MANIFEST / ProductName） |
| 二进制无线索残留 | 扫 exe 的 UTF-16 与 ASCII 串 | `VitalTray` ×30、`BatteryHub` ×0、`batteryhub` ×0 |
| 名称残留（源码/脚本/文档） | 改名脚本自带的残留检查 | 无 |
| 许可清单未过期 | `tools/gen-notices.ps1 -Check` | OK（9 依赖 / 10 份全文） |
| CI 真机 | GitHub Actions on windows-latest + pwsh 7 | 见提交后 run 结果 |

## 复审触发条件

发现 VitalTray 与既有产品/商标冲突时：先按本 ADR 的检索口径复核，再决定（可选方案：
加限定词、换名、换图标）。正式商标检索与商业化评估不在本项目当前范围。
