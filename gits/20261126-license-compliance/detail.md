# 第三方来源与许可合规 · 改动细节（detail）

- logid: `20261126-license-compliance`
- 基线: `2877b86`，分支 `main`（写文档时与 `origin/main` 同步：`git rev-list --left-right --count origin/main...HEAD` = `0 0`）
- 复核时间: 2026-11-26 16:04 (+08:00)

## 1. 新增文件（4 个）

### 1.1 `docs/licenses.md`（112 行）— 合规主张的唯一入口

| 节 | 行 | 内容 |
|---|---|---|
| 0 一句话结论 | 9–14 | MIT 发布、仓库不含第三方源码、`research/` 已整目录忽略；价值在「可自证」而非合规本身 |
| 1.1 只读思路/协议事实 | 16–43 | 14 行来源表（项目/上游链接/许可/参考内容/落地位置）+ 同名项目警示块 |
| 1.2 进入二进制的依赖 | 45–57 | 6 行：windows-sys / windows-link / hidapi / HIDAPI C 库 / libc / cfg-if |
| 1.3 明确禁止参考 | 59–64 | ZGMFX01A（All rights reserved）、BatteryMode（EULA） |
| 2 许可选举 | 66–75 | HIDAPI 三选一 → **BSD-3-Clause**；双许可 crate → MIT；选举单向、notices 只抄选中的那份 |
| 3 什么算抄 | 77–87 | 三分法（协议事实 / 架构思路 / 代码表达）+ 「换语言重写仍算抄」判据 |
| 4 红线 | 89–94 | 4 条（含「依赖改动必须重跑 gen-notices」） |
| 5 复核记录 | 96–105 | 4 行带日期的实测记录 |
| 6 维护方式 | 107–112 | 依赖变化 / 新增来源 / PR 三种场景的操作 |

关键修正（`docs/licenses.md:37-43`）：明确 `krisgfx9/Wireless-Device-Battery-Monitor`（C++/Qt，v1.1.0 起改名 BatteryHub）
= **GPL-3.0**，`battery-hub/battery-hub`（Electron）= **MIT**，「两者同名、许可不同，早期 PLAN 写成一行且 fork 数记错」。

### 1.2 `THIRD-PARTY-NOTICES.txt`（857 行 / 44,371 B）— 生成物，不手改

| 段 | 起始行 | 内容 |
|---|---|---|
| 头部 | 1–17 | 「请勿手工编辑」+ 两个命令入口 + 依赖总数（9 个：运行期 5 / 构建期 4；去重后许可全文 10 份） |
| 一、运行期依赖 | 18 | cfg-if 1.0.5 / hidapi 2.6.7 / libc 0.2.191 / windows-link 0.2.1 / windows-sys 0.61.2 |
| 二、构建期工具 | 31 | cc 1.11.3 / find-msvc-tools 0.1.15 / pkg-config 0.3.34 / shlex 2.0.1 |
| 三、HIDAPI 内嵌 C 库 | 40 | 三选一说明 + 本项目选举 (2) BSD-Style 及其理由 |
| 四、许可全文 | 53 | 10 份去重文本，顺序由「许可名 + 首个适用组件」排序保证可复现 |

许可标签分布：Apache-2.0 全文 ×3、Apache-2.0 简式声明 ×1、BSD-3-Clause（HIDAPI C 库）×1、MIT ×5。
**GPL 全文 0 份**：唯一一处 `GNU GENERAL PUBLIC LICENSE` 字样在 `THIRD-PARTY-NOTICES.txt:45-48`，
是「三选一」选项枚举里的 `1. GNU General Public License v3.0（LICENSE-gpl3.txt）`，属于如实说明而非选入。

### 1.3 `tools/gen-notices.ps1`（248 行）— 生成器兼门禁

| 段 | 行 | 做法 |
|---|---|---|
| 参数 | 13 | `param([switch]$Check)`；`$Check` 只校验不写盘 |
| 路径推导 | 16–17 | `$root = Split-Path $PSScriptRoot -Parent`，不依赖当前目录 |
| 许可文件匹配 | 24–29 | 文件名正则 `^(licen[cs]e|notice|copying)`，按名排序保证稳定 |
| 内容去重 | 31–40 | UTF-8 字节 SHA256 作为字典键 |
| 许可识别 | 42–50 | 依次判 Apache 全文/简式、BSD、MIT、GPL；识别不出标「未识别」 |
| **运行期闭包** | 60–78 | 从 `workspace_members` 出发只沿 `dep_kinds.kind` 为空或 `normal` 的边 BFS |
| 只列第三方 | 82 | `$p.source -notlike 'registry+*'` 跳过 workspace 自身 |
| 缺源码兜底 | 84–86 | 目录不存在即 `throw`「先运行 cargo fetch」（不让生成器产出残缺清单） |
| HIDAPI 选举落地 | 115–129 | 只抄 `etc/hidapi/LICENSE-bsd.txt`；**不**收 `LICENSE-gpl3.txt`；hidapi 不存在则 `throw` 提示重写说明 |
| 稳定性 | 192–199 | 输出顺序 = 许可标签 + 首个适用组件，避免字典遍历顺序造成假 diff |
| `-Check` | 215–243 | 读盘 → 归一化行尾 → `-cne` 严格比较（区分大小写）；不等则打印首个差异行并 `exit 1` |
| 写盘 | 245–248 | `UTF8Encoding($false)` 显式无 BOM |

两处已写进代码的「踩坑防御」：`:70-71` 普通依赖的 `kind` 是空值而非 `normal`（当成构建期会把所有依赖误判）；
`:220-223` Windows PowerShell 5.1 对无 BOM UTF-8 按 ANSI 解码，`-Check` 必须显式 `-Encoding UTF8`，否则永远失败。

### 1.4 `.github/pull_request_template.md`（19 行）— 过程强制

6 个必勾项：洁净室（含「换语言重写也算抄」）、许可（不带 GPL/MPL/`research/`）、
依赖（改 `Cargo.toml`/`Cargo.lock` 须重生成 notices 并登记 `docs/licenses.md`）、
行数（`tools/check-size.ps1`，该脚本存在）、验证（test/clippy + UI 改动要实机）、
读数纪律（拿不到就显示「—」，不编造）。

## 2. 修改文件（10 个）

| 文件 | 位置 | 改动 |
|---|---|---|
| `.github/workflows/ci.yml` | +45–50 | 在 clippy(`:42-43`) 与测试(`:51-52`) 之间插入「第三方许可清单未过期」步：`shell: pwsh` + `./tools/gen-notices.ps1 -Check` |
| `.github/workflows/release.yml` | +66–69 | 打 Release 前同一道校验（SHA256 步之后、action 之前） |
| `.github/workflows/release.yml` | +81–82 | `files:` 追加 `THIRD-PARTY-NOTICES.txt` 与 `LICENSE`，发布物自带告知义务 |
| `PLAN.md` | `:49` | §2.2 工作流第 4 步改写成「notices 由 gen-notices.ps1 从 Cargo.lock 生成，CI 用 `-Check` 校验未过期」 |
| `PLAN.md` | `:54-55` | §2.3 许可表一行拆两行：`BatteryHub（C++/Qt，含 5 个派生仓库）\| GPL-3.0`、`Battery Hub（Electron）\| MIT`（原为「原版 BatteryHub / 4 个 C++ fork \| GPL-3.0」） |
| `README.md` | `:174-176` | 许可段加两条入口：`docs/licenses.md`、`THIRD-PARTY-NOTICES.txt`（注明自动生成 + CI 校验） |
| `CHANGELOG.md` | `:8-18` | 「未发布」下新增「文档 / 合规」段：成文、两处事实修正（同名项目 + HIDAPI 选举）、PR 模板 |
| `crates/bh-proto-logitech/src/lib.rs` | `:4` | `//! 来源清单与许可（Solaar 为 GPL-2.0，仅取协议事实、未复制代码）见 docs/licenses.md。` |
| `crates/bh-proto-rapoo/src/lib.rs` | `:4-5` | 同上口径 + 「仅取帧格式与观测数据，未复制任何第三方代码」 |
| `crates/bh-proto-razer/src/lib.rs` | `:4-5` | 同上口径 + 注明 OpenRazer 为 GPL-2.0-or-later |
| `docs/protocol-notes/logitech-hidpp.md` | `:5` | 来源行补 `[docs/licenses.md](../licenses.md)` 链接 |
| `docs/protocol-notes/rapoo.md` | `:4-5` | 补来源（本机只读抓包 `24AE:185A` + 公开逆向资料）+ 未复制声明 + 许可链接 |

> 3 个 crate 的改动**纯 `//!` 注释**，无任何 Rust 逻辑/签名/依赖变动；
> `git diff crates/` 的每一行都以 `//!` 开头，可逐行核对。

## 3. 一致性核对（文档里的数字 vs 仓库事实）

| 文档断言 | 实测 | 结论 |
|---|---|---|
| notices：9 个依赖（运行期 5 / 构建期 4） | `-Check` 实跑输出「9 个依赖 / 10 份许可全文」 | ✅ |
| `research/` 下 7 个 clone，6 GPL-3.0 + 1 MIT | 逐目录读 LICENSE：battery-hub=MIT，cpp-monitor/fork-ferloom/fork-fuyue/fork-hassani/fork-taskbarvitals/fork-yunluo=GPL-3.0（另有 `bin/` 非 clone） | ✅ |
| `git ls-files research` 为 0 条 | 实测 0 条 | ✅ |
| `.gitignore` 整目录忽略 `research/` | `.gitignore:18` = `research/` | ✅ |
| notices 不含 GPL 全文 | `GNU GENERAL PUBLIC LICENSE` 命中 1 行，位于三选一选项枚举 | ✅ |
| PR 模板引用的 `tools/check-size.ps1` 存在 | `Test-Path` = True | ✅ |
| release 附带的 `LICENSE` 存在且为 MIT | 首行 `MIT License` | ✅ |
| PLAN 提到的 hidapi = BSD-3 | `PLAN.md:60` 既有行「hidapi \| BSD-3」与选举一致（原本就写过，现在有了选举依据） | ✅ |

## 4. 验证（可复现）

### V1 notices 门禁正向

```powershell
Set-Location D:\work\batteryhub
& .\tools\gen-notices.ps1 -Check
```
```
OK   THIRD-PARTY-NOTICES.txt 与依赖一致（9 个依赖 / 10 份许可全文）
[exit=0]
```

### V2 notices 门禁负向（证明门禁能真的拦住）

「永远不会失败的门禁等于没有门禁」，所以做了漂移测试，并在 `try/finally` 里保证复原：

1. `$origHash = Get-FileHash THIRD-PARTY-NOTICES.txt`，复制备份；
2. `Add-Content` 追加一行 `DRIFT-TEST-LINE`；
3. 跑 `-Check`，**期望 exit 1**；
4. `finally` 恢复备份，再比 SHA256。

实测输出：
```
FAIL THIRD-PARTY-NOTICES.txt 与当前依赖不一致。
     首个差异在第 858 行（仓库内 / 期望）：
       - DRIFT-TEST-LINE
       +
[drift-run exit=1]
[restore verified: True]   # 29C668EF…A60693 与改动前一致
[bak gone: True]
```
复原后 `git status --porcelain` 仍是原来的 14 条，无残留临时文件。

### V3 暂存安全性预演

```powershell
git check-ignore -v docs/licenses.md THIRD-PARTY-NOTICES.txt tools/gen-notices.ps1 .github/pull_request_template.md
# exit=1 → 4 个新文件都未被忽略，能正常入库
git add --dry-run <14 个文件>
# 逐个 add，无误伤
```

### V4 构建与测试（已跑，非推断）

```powershell
cargo test --locked --all-targets      # => 71 passed; 0 failed; [exit=0]
cargo clippy --locked --all-targets    # => 0 error; 19 条 warning（全部既存）; [exit=0]
```
- 测试：汇总各 target 的 `test result: ok.` 得 **71 passed / 0 failed**，与 `5fc69f1` 记录一致。
- clippy：`bh-core (lib) generated 18 warnings` + `bh-proto-razer (lib) generated 1 warning`
  —— 与基线「18 项既有风格 warning」吻合；razer 那条位于 `crates/bh-proto-razer/src/lib.rs:96`
  （`unnecessary >= y + 1`），距本次新增的 `:3-5` 文档注释很远，**不是本次引入**。
- 结论：注释级改动未新增编译告警，测试全绿。

### V5 未做（明确声明，避免过度声称）

- **未跑** GitHub Actions 真机：`pwsh` 门禁的 YAML 接线只在本地按等价命令验证过
  （本机无 `pwsh`，见 `lessons.md` L2）。
- **未跑** `cargo build --release`：本次改动不触及构建配置与链接，release 产物行为不变。

## 5. 绕过模式扫描（butler_diff_analyzer，附加独立审查）

对**真实** `git diff`（`-U0`）跑确定性绕过检测：

- 命中 **32 条 DA08**（「疑似中文硬编码/日志当实现（伪实现）」），**全部为误报**：
  逐个核对后，命中行要么是 YAML 的 `- name:` 步骤名、要么是 `# 注释`、要么是 `CHANGELOG`/`PLAN`/`README`
  的 Markdown 正文、要么是 3 个 crate 的 `//!` 文档注释。本 diff 没有任何可执行逻辑新增，
  自然不存在「日志当实现」。
- **DA01–DA07 零命中**：无吞错、无注释掉的可执行代码、无删测试、无硬编码绕过、无 lint 豁免。
- 已知局限（如实记录）：`path` 模式只递归读 `.ts/.js` 类文件，对 `tools/*.ps1` 返回 0 条
  **不代表** PowerShell 已被扫描；`gen-notices.ps1` 的检查结论来自人工逐行通读（`detail.md` §1.3）
  与 `-Check` 正/负向实跑，而非该扫描器。

> 误报的教训记在 `lessons.md` L8：文档类 diff 会让 DA08 大量误报，扫描结果必须人工逐条定性，
> 不能直接当「P0 缺陷」上报。

## 6. 提交范围（待主 agent 许可后执行）

```
git -C D:\work\batteryhub add `
  .github/workflows/ci.yml .github/workflows/release.yml `
  CHANGELOG.md PLAN.md README.md `
  crates/bh-proto-logitech/src/lib.rs crates/bh-proto-rapoo/src/lib.rs crates/bh-proto-razer/src/lib.rs `
  docs/protocol-notes/logitech-hidpp.md docs/protocol-notes/rapoo.md `
  .github/pull_request_template.md THIRD-PARTY-NOTICES.txt docs/licenses.md tools/gen-notices.ps1 `
  gits/20261126-license-compliance/summary.md gits/20261126-license-compliance/detail.md `
  gits/20261126-license-compliance/decision-chain.md gits/20261126-license-compliance/lessons.md
```

本仓库**把 `gits/` 文档一并入库**（`git ls-files gits` = 14 条既有文档；`5fc69f1`/`7f63325`/`ba69318`
三个提交都同时提交了 `gits/<logid>/*.md`），故本次共 **18 个文件**（14 代码文档 + 4 决策文档）。
若主 agent 决定只提交 14 个文件，则决策文档需另起 `docs(gits)` 提交——但会偏离项目既有惯例。

然后 `git commit`（message 见 `summary.md`「建议 commit message」）→ `git push origin main`。
`gits/` 未被 `.gitignore` 忽略（`git check-ignore -v gits/` exit 1），因此**不 add 就会留在 untracked**，
会让提交后 `git status` 不干净——这也是建议一并入库的操作性理由。
