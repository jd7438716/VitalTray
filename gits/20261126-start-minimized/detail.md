# 打开只驻留托盘 · 改动细节（detail）

- logid: `20261126-start-minimized`
- 基线: `b6afb01`（HEAD，main 分支）
- 本文件由 butler-git 在暂存前补充：`summary.md` 与 `decision-chain.md` 为前序步骤产出，未改写。
- 审计方式：`git -C D:/work/batteryhub status -uall` + `git diff` 逐文件核对；行号指向**工作区当前内容**。

## 1. 代码改动（2 个源文件）

### `crates/bh-core/src/cli.rs`（+32 行）

| 位置 | 改动 |
|---|---|
| cli.rs:17 | `Args` 新增字段 `pub minimized: bool` |
| cli.rs:46 | `Args::parse` 新增分支 `"--minimized" => a.minimized = true` |
| cli.rs:77 | `--help` 选项键表插入 `"cli.opt.minimized"`（i18n 完整性测试要求 10 语言对齐） |
| cli.rs:97–121 | 新增 `mod tests`：`minimized_and_panel_are_independent_flags`（--minimized / --panel 互不串味、同给都读进来）、`unknown_args_are_ignored_not_fatal`（未知参数不致命且不污染 minimized） |

### `crates/bh-core/src/app.rs`（+19 / −3 行，`git diff --stat` 一栏显示 22 = 19+3 行变动）

| 位置 | 改动 |
|---|---|
| app.rs:132–137 | 新增启动决策（含 4 行注释）：`start_minimized = args.minimized \|\| cfg.start_minimized`；`show_panel_at_start = args.panel \|\| !start_minimized`（优先级 `--panel` > `--minimized` > `[system] start_minimized`） |
| app.rs:167–171 | 原 `if args.panel { panel::show() }` → `if show_panel_at_start { … } else { log "start minimized: tray only, no panel" }` |
| app.rs:191–200 | `WAKE_MESSAGE`（第二次启动的唤醒）不再无条件 `panel::show()`：`in_tray = state::with(\|st\| st.and_then(\|s\| s.tray.as_ref()).is_some())`，在托盘则只记日志 `already in tray, stay minimized`，仅在无托盘图标时弹面板（唯一入口兜底）；201 行是未改动的 `0` 返回值 |

引用闭合性静态核对（本 agent 实查，非推断）：
- `cfg.start_minimized` 字段确实存在：`config.rs:14`（定义）、`config.rs:34`（默认 `true`）、`config.rs:55-56`（`system.start_minimized` 解析）、`config.rs:81/87`（写回）。该文件本次**未修改**——字段早已在库。
- `state::with` / `State.tray` 确实存在：`state.rs:21`（`pub tray: Option<TrayIcon>`）、`state.rs:34`（`pub fn with<R>(f: impl FnOnce(Option<&State>) -> R) -> R`）。`state.rs` 本次**未修改**。

### 未改动但被引用的既有事实
`autostart.rs`（自启项写 `"--minimized"`）与 `tools/install.ps1` 本次**未修改**——它们一直在传这个参数，只是以前没人解析。

## 2. i18n 改动（10 个文件，各 +1 行 `cli.opt.minimized`）

`crates/bh-core/src/i18n/cli/` 下 de / en / es / fr / ja / ko / pt_br / ru / zh_cn / zh_tw，新增键统一落在 **第 16 行**（`cli.opt.panel` 之后、`cli.opt.watch` 之前）。

**需要如实记录的副作用**：其中 6 个文件（de.rs、en.rs、fr.rs、pt_br.rs、zh_cn.rs、zh_tw.rs）首行 UTF-8 BOM（`\ufeff`）被移除，diff 里表现为首行注释的 −1/+1。这是编辑工具写回时的规范化，不影响 Rust 编译（BOM 在源码首行本就无意义）；es.rs / ja.rs / ko.rs / ru.rs 原本无 BOM，故只有 +1 行。

## 3. 文档改动（4 个文件）

| 文件 | 行 | 改动 |
|---|---|---|
| README.md | 60、67 | 快速开始加 `--minimized` 一行；常用命令补充说明 |
| README.md | 80-81 | 新增「打开 = 只在托盘里」，并把旧的「第二次双击把已经跑的叫出来」条目删掉/重写 |
| README.md | 172-173、176 | 新增「启动行为」小节（`start_minimized=1` 默认 / `--minimized` / `--panel`）；命令行清单补 `--minimized` |
| README.en.md | 65、72、87-90、191-193、198 | 与中文一一对应（双语同步门禁 `check-readme` 要求） |
| CHANGELOG.md | 6-18 | 新增 `## [未发布] → ### 修复` 条目（含用户反馈原话、`--minimized` 空转旧账、`start_minimized` 生效、唤醒例外、`--panel` 优先级） |
| DEVLOG.md | 347-375 | 当日记录：根因（两条弹面板路径）、关键事实、决策、5 场景实测表、部署说明（不 bump 版本、不打 tag） |

## 4. 本提交**不含**的内容（守门）

- `Cargo.toml` / `Cargo.lock` 的版本号：本次 diff 中未出现（`git diff --stat` 16 个文件无 Cargo.*），版本 bump 由主 agent 下一步单独提交。
- `gits/20261126-start-minimized/summary.md`、`decision-chain.md`：由前序步骤写好，本步**原样暂存、不改写**；只新增本文件 `detail.md`。
- 未跟踪垃圾：`git status -uall --porcelain` 的未跟踪项**恰好 3 个**，全在本 logid 目录内（`summary.md` / `detail.md` / `decision-chain.md`），没有别的未跟踪文件；`tmp/`、`target/`、`research/` 经 `git check-ignore` 确认均在 `.gitignore` 内（`gits/` 本身未被忽略，故 3 份文档会入库）。
- 明确**不进**暂存：`tmp/`、`target/`、临时备份、`.env` 类敏感文件（本仓库本次均不存在这类未跟踪项）。

## 5. 暂存清单（供下一步精确 `git add`，共 19 个路径）

16 个已修改 + 3 个新增文档，逐条列举，禁止用 `git add -A` / `git add .`：

```
crates/bh-core/src/cli.rs
crates/bh-core/src/app.rs
crates/bh-core/src/i18n/cli/de.rs
crates/bh-core/src/i18n/cli/en.rs
crates/bh-core/src/i18n/cli/es.rs
crates/bh-core/src/i18n/cli/fr.rs
crates/bh-core/src/i18n/cli/ja.rs
crates/bh-core/src/i18n/cli/ko.rs
crates/bh-core/src/i18n/cli/pt_br.rs
crates/bh-core/src/i18n/cli/ru.rs
crates/bh-core/src/i18n/cli/zh_cn.rs
crates/bh-core/src/i18n/cli/zh_tw.rs
README.md
README.en.md
CHANGELOG.md
DEVLOG.md
gits/20261126-start-minimized/summary.md
gits/20261126-start-minimized/detail.md
gits/20261126-start-minimized/decision-chain.md
```

## 6. 验证归属（CFACT）

构建与测试证据来自前序会话（`summary.md#27-37`）：新二进制 SHA256 `7AA347D1…C93`、`cargo test --workspace --all-targets` 全绿、`check-size`/`check-readme`/`check-cleanroom`/`check-fonts`/`check-exe` 通过、5 个场景实测日志与面板状态表。
**butler-git 本步只做只读核对**（status/diff/行号/引用闭合），未重跑 cargo —— 属如实标注，不冒充自证。

本步实际核对过的锚点（`git -C D:/work/batteryhub` 只读，2026-11-26 17:4x）：
`HEAD = b6afb01`、`git diff --numstat` cli.rs `32/0` / app.rs `19/3`、`git status -uall --porcelain` 16 M + 3 ??、`Cargo.toml`/`Cargo.lock` 无变更；`config.rs:14/34/55-56/81/87`、`state.rs:21/34`、10 个 i18n 文件 `cli.opt.minimized` 均在 **16 行**（其中 6 个 BOM 文件 de/en/fr/pt_br/zh_cn/zh_tw 在 diff 里表现为首行 −1/+1）；README/README.en/CHANGELOG/DEVLOG 引用行号逐一 `Select-String` 实查。
