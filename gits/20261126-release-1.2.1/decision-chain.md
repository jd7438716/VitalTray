# 发版 1.2.1 · 决策链（decision-chain）

## D1 版本号取 patch（1.2.0 → 1.2.1），而不是 minor

改动只修行为：启动与二次打开不再弹面板，没有新增/删除 CLI 参数语义以外的能力，也没有配置格式变更
（`start_minimized`、`--minimized` 都是**已有的**键与参数，本次只是让它们生效）。
按语义化版本，向后兼容的缺陷修复取 patch。

## D2 拆两笔提交（fix + chore(release)），不 squash

与仓库历史一致（`feat!: ...` 与 `chore(release): v1.2.0` 本来就分开）：
- `1a650f5 fix(startup): ...` 是可回退的代码+文档单元，带 gits/ 决策文档；
- `0d65783 chore(release): v1.2.1` 只动版本号/CHANGELOG/README，便于 `git tag` 指向"纯收口"提交，
  也让 CHANGELOG 的 `[1.2.1]` 与 tag 一一对应。

## D3 为什么回填 6 个 i18n 文件的 BOM 并 `--amend`

首版提交 `9199b4b` 里，6 个 `i18n/cli/*.rs` 的首行 UTF-8 BOM 被编辑工具顺带移除，diff 多出 6 处
首行 −1/+1 噪声（与本次修复毫无关系）。仓库里 89 个 `.rs` 只有 5 个带 BOM，多数无 BOM，
但**不该在一次行为修复里顺手改编码**。处理：按父版本原样补回 BOM → `git commit --amend --no-edit`，
最终 diff 10 个文件各 +1 行；该处理与哈希变化记录在 `gits/20261126-start-minimized/detail.md`。

## D4 为什么手写发布说明覆盖自动生成

`generate_release_notes: true` 只会列出 commit 标题。用户反馈的原话是「打开时跳出个菜单，挡住整个页面」，
标题里读不出来。手写说明给出：行为变化（启动/二次打开/托盘关掉时）、三个开关的优先级表、SHA256 与校验命令、
覆盖升级方式。取舍：多一次 `gh release edit`，但下载页第一屏就能自解释。

## D5 tag 用 annotated

与 `v1.1.1` / `v1.2.0` 一致（`git cat-file -t` = tag），tag 消息里写清本版主题
（「打开只驻留托盘：启动与二次打开不再弹面板」），`git tag -n` 一眼可读。

## 待观察风险（诚实记录）

- **本地构建与 CI 构建的哈希不一致**（`1b44ccba…` vs `b2281a70…`）：同一份源码、同样 `--locked`，
  差异来自构建环境（rustc 版本 1.88 与 runner 工具链细节、PE 时间戳）。发布物以 `SHA256SUMS.txt` 为准，
  已下载回验；但这意味着「本地能复现发布物哈希」这件事**做不到**，将来的可复现构建是另一个议题。
- **双击 exe 现在确实"没反应"**：这是本版按用户要求换来的静默。若再被反馈像坏了，
  下一步给托盘图标加一次短暂高亮/气泡（仍不弹面板），已记在 `start-minimized/decision-chain.md`。
