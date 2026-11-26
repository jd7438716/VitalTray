# 第三方来源与许可合规 · 决策链（decision-chain）

- logid: `20261126-license-compliance`
- 时间: 2026-11-26 16:04 (+08:00)
- 决策人: butler-git（agent-2），提交许可由主 agent / 用户批准

## D1 为什么自己写 248 行生成器，而不是引入 cargo-about / cargo-deny / cargo-license

- **上下文**：项目纪律是「零第三方运行期依赖以外的东西尽量不进」，`cargo-deny` 会引入一棵
  不小的依赖树，且它的 license 检查是**策略校验**，不产出「可随 exe 分发的许可全文」——
  而 MIT/Apache/BSD 要的恰恰是**原文**。
- **替代方案**：① `cargo-about`（Rust 生态事实标准，能生成 HTML/JSON，但需要 `cargo install`、
  引入 `askalono` 等归属判断依赖）；② `cargo-deny` + 手写 notices；③ 自己用 `cargo metadata` 拼。
- **决策**：选 ③。理由：`cargo metadata --locked` 已在 CI 用到的能力范围内，脚本只做「读依赖图 →
  找 LICENSE 文件 → 去重 → 拼文本」，逻辑可 100% 本地复现，`-Check` 与生成共用同一段拼装代码
  （`tools/gen-notices.ps1:131-213`），因此「生成」和「校验」不可能漂移。
- **后果**：多维护 248 行 PowerShell；换来 zero-install、结果逐字节可复现、CI 里 5 秒内跑完。
  代价是许可**识别**较粗糙（`Get-LicenseLabel` 只认 4 类），遇到新许可文本会标「未识别许可文本」——
  但这一步只影响**展示标签**，不影响抄录原文，所以是安全的退化。

## D2 为什么要区分「运行期依赖」与「构建期工具」

- **上下文**：`Cargo.lock` 里有 9 个第三方 crate，其中 `cc` / `find-msvc-tools` / `shlex` / `pkg-config`
  只在编译期跑（`build-dependencies` 链），**不会出现在 exe 里**，对其没有分发告知义务。
- **替代方案**：① 全部列进 notices（宁多不少，反正不违法）；② 只列运行期。
- **决策**：**两者都列，但分段**——运行期列在「一、随 exe 分发」，构建期列在「二、无告知义务」。
  路径选②会漏；选①但不分段会让审计者误以为 exe 里有 `pkg-config`。分段同时满足「宁多不少」和「不误导」。
- **实现要点**：从 `workspace_members` 沿**普通依赖边**做 BFS（`:60-78`）。关键坑：
  `cargo metadata` 里普通依赖的 `dep_kinds[].kind` 是**空字符串**而不是 `"normal"`，
  若只判 `-eq 'normal'` 会把全部依赖误判成构建期、notices 直接变空。代码里 `:72-75` 显式把空值算作 normal，
  并写了注释防止后人"优化"掉。

## D3 为什么许可全文按内容 SHA256 去重

- **上下文**：10 份去重文本来自 9 个 crate 的多个 LICENSE 文件，同一份 Apache-2.0 全文在
  `windows-sys`、`libc`、`cc` 等处重复出现；不去重会得到 ~20 份几乎相同的长文本，notices 体积翻倍且难核。
- **替代方案**：① 按文件名去重（`LICENSE-MIT` 全叫一个名，会误合并不同版权声明）；② 按 crate 去重（漏抄）；
  ③ 按内容哈希去重。
- **决策**：③（`:98-113`）。文件名/路径不可靠——不同 crate 的 `LICENSE` 是不同文本；
  内容哈希则精确表达「这段法律文本只需要出现一次」。
- **后果**：notices 里每份文本都能列出**全部**适用组件（`适用于：` 列表），审计时可反向追踪；
  同一份文本在不同 crate 里有细微空白差异时会各存一份（数量略多，但不会误合并——偏安全侧）。

## D4 HIDAPI C 库：为什么选举 BSD-3-Clause，且只抄这一份

- **上下文**：`hidapi` crate 内嵌的 HIDAPI C 库**不是单许可**，`etc/hidapi/` 下同时有
  `LICENSE-gpl3.txt` / `LICENSE-bsd.txt` / `LICENSE-orig.txt`——由使用者三选一。
  exe 通过 `hidapi` 静态链接了它的 Windows 后端（`etc/hidapi/windows/hid.c` 等），因此**必须选一份并遵守**。
- **替代方案**：① 选 GPLv3；② 选 BSD-3-Clause；③ 选原始 HIDAPI 许可（与 BSD 极接近但措辞不同）；
  ④ 换掉 hidapi 自己调 Win32 HID API。
- **决策**：**② BSD-3-Clause**。理由：与项目 MIT 兼容、可分发闭源二进制、不触发 GPL 的源码开放义务；
  ③与②内容几乎一致但不如 BSD 广为人知，审计成本略高；④要重写 HID 枚举与报文读写，收益不成比例。
  **选举是单向的**（选了 GPL 就无法退回），且必须可追溯，所以：
  - 选举结论写进 `docs/licenses.md:66-75`（含理由）；
  - 生成器**只抄 `LICENSE-bsd.txt`**（`gen-notices.ps1:115-129`），明确注释「不把 LICENSE-gpl3.txt 收进来，
    避免读者误以为本项目选了 GPL」；
  - notices 第三节同时**如实披露**三选一的存在（`THIRD-PARTY-NOTICES.txt:40-51`）——只抄一份不等于隐瞒另外两份。
- **后果**：notices 里 GPL 全文 0 份（实测仅 1 处选项枚举文字）。若将来有人把选举改成 GPL，
  必须同时改文档、脚本、notices 三处——脚本里 `throw` 的兜底会在 hidapi 消失时强制复查这段逻辑。

## D5 为什么门禁用 `-Check`（比较模式），并放在 clippy 之后 / release 之前

- **上下文**：需要拦住「改了依赖忘了重生成 notices」。两种实现方式：
  ① CI 里跑生成，然后 `git diff --exit-code` 看有没有变化；② 生成器自带 `-Check` 比较模式。
- **替代方案**：① 简单，但会在 CI 里**写入文件**（污染工作区、失败信息只有「文件又变了」）；
  ② 需要多写 30 行，但能在失败时**直接打印首个差异行**并提示重生成命令。
- **决策**：②。`gen-notices.ps1:215-243` 读盘 → 归一化行尾 → `-cne` 严格比较 → 打印
  「首个差异在第 N 行（仓库内 / 期望）」→ `exit 1`。
- **位置**：CI 放在 clippy(`ci.yml:42-43`) 之后、单测(`:51-52`) 之前——此时依赖已 `--locked` 解析完，
  报错最快，且不会让测试白跑；release 放在 SHA256 之后、上传之前(`release.yml:66-69`)——
  发布物一旦带上缺声明的 exe 就撤不回来，必须在上传前拦。
- **后果**：CI 失败信息可直接照抄修复命令；代价是脚本必须自己实现 diff 展示（已做）。

## D6 为什么先修 PLAN 里的两处事实错误（而不是「以后再说」）

- **上下文**：`PLAN.md` §2.3 原先把参考项目写成一行「原版 BatteryHub / 4 个 C++ fork | GPL-3.0」。
  实测 `research/` 有 7 个 clone：`battery-hub` 是 **MIT**（Electron 项目），其余 6 个才是 GPL-3.0；
  且 C++ 系的派生仓库是 **5** 个不是 4 个。
- **风险**：这不是笔误级问题——若将来有人据表误以为「所有同名项目都是 GPL」，可能做出
  过度保守或错误的许可判断；反过来若真把 MIT 项目当 GPL 而不敢参考，也是实证损失。
- **决策**：拆成两行（`PLAN.md:54-55`）并在 `docs/licenses.md:37-43` 用警示块说明「两者同名、许可不同」，
  同时把派生仓库数改成 5。**合规文档的第一价值是可自证，而错误的事实会让整份文档失去证据力**，
  所以修事实的优先级高于扩写新内容。
- **后果**：`PLAN.md:60` 原有的「hidapi | BSD-3」那一行现在有了选举依据（`docs/licenses.md` §2），前后自洽。

## D7 为什么给 3 个 proto crate 和 2 份协议笔记各补一行来源

- **上下文**：洁净室纪律要求「协议事实必须能追到来源」。此前来源只活在 `PLAN.md` 和人的记忆里，
  直接读 `crates/bh-proto-razer/src/lib.rs` 的人看不到任何许可线索。
- **决策**：在每个 crate 的模块文档头（`lib.rs:4` 起）与每份协议笔记（`:4-5`）各加 1–2 行：
  写明参考项目名、其许可、以及**「仅取协议事实、未复制代码」**这一关键限定，并链到 `docs/licenses.md`。
- **后果**：成本是一行注释，收益是「看见文件头就知道该文件为什么合法」——将来被质疑时，
  不需要翻提交历史就能自证。选择写在 `lib.rs` 而非 `Cargo.toml`/独立文件，是为了让读者**无法回避**。

## D8 决策文档是否随本次提交一起入库

- **事实**：本仓库 `gits/` **未**被 `.gitignore` 忽略（`git check-ignore -v gits/` exit 1），
  且已有 14 条 `gits/**/*.md` 被跟踪；`5fc69f1`、`7f63325`、`ba69318` 三个提交都是
  「代码改动 + 同 logid 的 gits 文档」一起提交。
- **决策**：跟惯例，一起入库（本次共 18 个文件）。若不 add，文档会作为 untracked 留在工作区，
  提交后 `git status` 不干净，「验证」步无法给出干净结论。
- **状态**：**已确认**（2026-11-26 16:08 主 agent 授权步骤 7–8，指令即「一次 commit + 推送 `origin main`」，
  并逐一点名 18 个路径）→ 按惯例一次提交，不做二次 `docs(gits)` 提交。

## 风险与回滚

| 风险 | 缓解 |
|---|---|
| CI 新增门禁误报（如 cargo registry 缓存未命中导致 `cargo metadata` 失败） | 脚本 `:84-86` 对缺源码给出「先运行 cargo fetch」明确提示；CI 前面步骤已 `--locked` 构建过依赖 |
| Windows PowerShell 5.1 与 pwsh 7 对 `Get-Content -Encoding UTF8` 行为差异 | `-Check` 与写盘路径都显式指定编码（`:220-223`、`:245-246`），本地 5.1 实测通过 |
| 将来新增依赖带来新许可类型 | `Get-LicenseLabel` 退化为「未识别许可文本」但仍抄原文；CI 不会静默放过（`-Check` 仍会比较内容） |
| 门禁变「永远绿」 | 已做负向测试（`detail.md` §V2），确认 exit 1 与差异行提示有效 |
| 回滚 | 纯文档/脚本/工作流，`git revert` 即可；无数据迁移、无产物依赖 |

## 状态

- 变更审查（步骤 1–2）：✅ 完成（10 改 + 4 增，`git status --short` + 逐文件 diff 通读）
- 决策文档（步骤 3–5）：✅ 本目录 4 篇
- 暂存 / 提交 / 推送（步骤 6–8）：✅ **主 agent 于 2026-11-26 16:08 授权**（`add` / `commit` / `push` 属非读操作，
  按 butler-shared §4 合规红线必须先有明确指令）。按 18 个路径精确暂存 → `git diff --cached` 复审 →
  `docs(licenses): 第三方来源与许可成文 + notices 生成器与 CI/release 门禁` → `git push origin main`。
  本文件随之入库，提交后的 `git status` 应为干净（本文档为唯一可能残留项，故与提交同批）。

## 附：提交后复核（步骤 8 验收口径）

| 断言 | 验收命令 | 期望 |
|---|---|---|
| 提交已落地 | `git log -1 --format=%s` | 以 `docs(licenses):` 开头 |
| 18 个文件全部入库 | `git show --stat --oneline HEAD` | 18 files changed |
| 工作区干净 | `git status --short` | 空 |
| 与远端同步 | `git rev-list --left-right --count origin/main...HEAD` | `0 0` |
| notices 未过期 | `./tools/gen-notices.ps1 -Check` | `exit 0` |
