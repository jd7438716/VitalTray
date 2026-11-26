# 第三方来源与许可合规 · 经验（lessons）

- logid: `20261126-license-compliance`
- 时间: 2026-11-26 16:04 (+08:00)

## L1 门禁必须做负向测试，否则你只是加了一段永远绿的命令

本次真正有价值的验证不是「`-Check` 通过」，而是「**制造漂移后它确实 exit 1**」。
不测失败的验证，等于把「我加了门禁」当成「门禁有效」——两者差一个 exit code。

可复用的负向测试套路（对 `check-exe.ps1`、`check-size.ps1` 同样适用）：

```powershell
$origHash = (Get-FileHash $target -Algorithm SHA256).Hash
Copy-Item $target "$target.baktmp" -Force
try {
    <最小化地破坏目标，如追加一行 / 删一个字段>
    & .\tools\<gate>.ps1 -Check
    "[exit=$LASTEXITCODE]"      # 期望非 0
}
finally {
    Move-Item "$target.baktmp" $target -Force
    (Get-FileHash $target -Algorithm SHA256).Hash -eq $origHash   # 期望 True
}
```

**要点**：目标文件必须是**未被 git 跟踪**的新文件（如 `THIRD-PARTY-NOTICES.txt`）时，`git checkout --`
**救不回来**（untracked 文件不在 index 里）。所以必须自己备份 + 用哈希证明复原。
本次实测：漂移 → `exit 1` 且报「首个差异在第 858 行」，恢复后 SHA256 `29C668EF…A60693` 与改动前一致，
`git status` 仍是 14 条，无残留临时文件——三条证据缺一不可。

## L2 本机没有 `pwsh`（PowerShell 7），文档里写的命令在本机会失败

`docs/licenses.md`、`THIRD-PARTY-NOTICES.txt` 头部、PR 模板里都写的是 `pwsh ./tools/gen-notices.ps1`。
本机实测 `pwsh` → `CommandNotFoundException`：只有 Windows PowerShell **5.1**（`powershell.exe`）。
CI 用的是 GitHub Actions 的 `shell: pwsh`（runner 自带 7.x），所以**管道侧写法正确、本机写法不可用**。

本地正确跑法：
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\gen-notices.ps1 -Check
# 或直接
& .\tools\gen-notices.ps1 -Check
```
**好消息**：脚本已为 5.1 的 UTF-8 坑做了防御——`gen-notices.ps1:220-223` 显式 `-Encoding UTF8`，
否则 5.1 会把无 BOM 的 UTF-8 按 ANSI 解码、中文变乱码，`-Check` 会**永远失败**。实测在 5.1 下 exit 0。
→ 建议后续在这三处文档里补一句「旧版 Windows 用 `powershell -File`」，或者干脆统一写成两种都行的形式。

## L3 合规文档里的每个数字都要能当场复算

`docs/licenses.md` 写了「`research/` 下 7 个 clone」「6 个 GPL-3.0、1 个 MIT」「Cargo.lock 9 个第三方 crate」。
这些数字是整份文档的证据力所在——错一个，被质疑时整份文档会连坐。本次逐个复算：

- 7 个 clone：`battery-hub`=MIT，`cpp-monitor`/`fork-ferloom`/`fork-fuyue`/`fork-hassani`/
  `fork-taskbarvitals`/`fork-yunluo`=GPL-3.0（`research/bin` 不是 clone，无 LICENSE）→ **与文档一致**。
- 9 个 crate：`cargo metadata` 经生成器统计（运行期 5 + 构建期 4）→ **与 notices 头部一致**。

**可复算性本身就是合规的一部分**：如果连作者自己都要"回忆"这些数字，别人无法审计。

## L4 `git status` 的实际条数必须和任务描述对账，不能只信文字

任务描述称「13 个未提交文件」，实测 `git status --porcelain` 是 **14 条**（10 M + 4 ??）。
逐条对照后发现：描述里点名的是 12 类（4 新 + PLAN + 3 crate + 2 笔记 + CI + release），
另有 `CHANGELOG.md`、`README.md` 两个同样属于本次改动的文件未被点名。

**教训**：提交前的第一步永远是「读 `git status` 并与上级给的清单逐条对齐」，差异必须显式上报（本次已上报主 agent），
而不是默认「上级说的数就是全量」。反方向的风险同样存在：多出来的可能是无关改动（如临时脚本、别的任务遗留），
那种情况要**排除在本次提交之外**——本次判断这 2 个文件属于同一主题（都在为许可产物做入口），故建议一并提交。

## L5 「同名不同许可」是真实的踩坑源，必须写成警示块

两个项目都叫 Battery Hub：`krisgfx9/Wireless-Device-Battery-Monitor`（C++/Qt，GPL-3.0）
与 `battery-hub/battery-hub`（Electron，MIT）。原 PLAN 把二者写成一行 GPL-3.0，fork 数也少记 1。
**这类错误不会被任何工具发现**（两个仓库都合法、都有 LICENSE），只能靠人工核对上游域名与仓库名。

判据：**看上游 `owner/repo` 全路径，不看项目显示名**。已在 `docs/licenses.md:37-43` 写成 ⚠️ 警示块，
并把「早期写得不对、2026-11-26 已修正」留在文档里——**保留纠错痕迹比抹掉它更有说服力**。

## L6 环境限制要在文档里如实声明，不要用「应该通过」代替证据

提交前在 `2877b86` 工作区**实际跑过** `cargo test --locked --all-targets`（**71 passed / 0 failed**，
逐 target 汇总见 `detail.md` §V4）与 `cargo clippy --locked --all-targets`（**0 error / 19 条既存 warning**），
所以 §V4 的「已跑，非推断」成立、`summary.md` 的关键事实表可对账。
真正**未做**的只有两项，已如实标注：① 真机 GitHub Actions（`shell: pwsh` 的 YAML 接线只按等价命令本地验证）；
② `cargo build --release`（本次不触及构建配置与链接）。§V5 已列明这两条「未做」及理由。
宁可标「仅静态验证」，也不要把「注释改动不影响编译」写成「测试通过」——
反过来同理：真跑过就写实测输出，不写「应该没问题」。

## L7 butler-git persona 引用的 `butler-commit` skill 在本 profile 不存在

`personas/git.md:12` 要求 `skill(butler-commit)`，但该 skill 在 `~/.dsh/skills/` 下**不存在**
（`skill('butler-commit')` 返回 unknown）。本次实际依据的是 persona 第 22–44 行的严格步骤 +
`butler-stage`（暂存纪律）+ `butler-shared`（合规红线）。
→ 建议要么补建 `butler-commit` skill（把 message 规范与步骤落到 skill 里），要么修正 persona 的引用，
避免每次执行都要人工兜这个断裂。**这是流程债，不是本次改动的缺陷。**

## L8 文档类 diff 会让 DA08 误报刷屏；扫描器对 .ps1 无能为力

对真实 diff 跑 `butler_diff_analyzer`：32 条 P0 全部是 DA08「疑似中文硬编码/日志当实现」。
逐条核对后 **0 条成立**——本 diff 新增的全是 YAML 步骤名、Markdown 正文与 `//!` 注释，
没有一行可执行逻辑。DA01–DA07（吞错 / 注释掉代码 / 删测试 / 硬编码 / 绕 lint）**零命中**。

两点教训：
1. **扫描器报 P0 ≠ 真有缺陷**。工具自己的说明也写着「需人工确认」；文档型 diff 必须逐条定性后再定级，
   否则会把一篇合规文档报成 32 个 P0。
2. **不要用 `path` 模式给 `.ps1` 背书**：`path` 只递归读 `.ts/.js` 类文件，对 `tools/gen-notices.ps1`
   返回「0 findings」并不代表它被检查过。该脚本的结论来自人工通读 + `-Check` 正/负向实跑。
   「工具没报错」和「工具有覆盖」是两件事。

## L9 别用自己手搓的 diff 喂审查工具

本次第一次跑扫描器时，我为了提高命中率在输入里手写了几行「可能有问题的代码」（其中一行
`if (-not ($rows.Count -eq 0) -or $false) { }` **文件里根本不存在**）。那次的结论毫无证据力，
已作废并用真实 `git diff -U0` 重跑。**审查工具的输入必须是可复现的真实产物**，否则等于自己造证据。
