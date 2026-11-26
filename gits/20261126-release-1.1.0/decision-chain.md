# 发版 1.1.0 · 决策链（decision-chain）

## D1 为什么这次提交只包含版本号，而代码在 7f63325

`7f63325` 已经把两条并行开发线的代码一次性提交（用户验收过的改动 + 另一会话的改动）。
对已提交的历史再拆分需要 rebase/改写历史并 force-push，收益（更好看的 history）远小于风险
（改写公共分支、可能丢改动）。因此本次只追加一个纯粹的 release chore commit，保证
「代码 = 7f63325」「版本 = 1.1.0」两件事在历史里各自可查。

备选：
- 用 `git reset --soft` 重新拆成 4 个提交 → 需 force-push，且会与仍在运行的并发会话冲突。否决。
- 直接把版本号也算进 7f63325 → 历史已推不动（未 push，但改写代价同上）。否决。

## D2 为什么选 1.1.0 而不是 1.0.1 或 2.0.0

- 有新增能力（厂商协议注册表、雷蛇协议 crate、抓帧白名单配置项 `[capture] vids`）→ 不是 patch。
- 无破坏性变更：配置文件向后兼容（缺 `[capture]` 段即默认不抓）、命令与 CLI 参数未变。→ 不是 major。
- 因此 minor：`1.1.0`。

## D3 为什么必须同时提交 Cargo.lock

`release.yml` 用 `cargo build --locked`，lock 与 `Cargo.toml` 不一致会直接构建失败。
本次新增了 `bh-proto-razer` crate、移除了两个插件对协议库的依赖，lock 必然变化；
必须在 tag 之前把 lock 提交进去，否则 CI 在 tag 上构建会失败。

## D4 为什么手写 Release 说明而不是用自动生成

`generate_release_notes: true` 只按 commit / PR 生成。而 1.1.0 最重要的变化
（抓帧线程 99% 单核的修复、幽灵行抑制）在 commit 标题里没有体现
（合并提交标题只写了「面板改温度 + 状态色与厂商协议注册表」），
自动说明会漏掉用户最关心的那条。所以发布后用 `gh release edit --notes-file` 覆盖为手写说明。

## 风险与回滚

- 风险：CI 在 tag 上失败（版本不一致 / lock 不一致 / self-test 失败）。缓解：本地已按 CI 同口径
  全部预跑（`--locked` 构建、`--self-test`、行数门禁、67 项单测）。
- 回滚：删除远端 tag 与 Release（`gh release delete v1.1.0 --yes` + `git push --delete origin v1.1.0`），
  代码本身不受影响。
