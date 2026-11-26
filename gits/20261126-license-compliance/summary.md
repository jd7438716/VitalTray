# 提交「第三方来源与许可合规」变更（summary）

- logid: `20261126-license-compliance`
- 时间: 2026-11-26 16:04 (Asia/Shanghai)
- 基线: `2877b86 chore(release): v1.1.1`
- 模式: commit（一次性提交 + 推送 `origin main`）
- 目标: 把「本项目读过哪些开源项目 / 各自许可 / 我们怎么履行义务 / 怎么自证没抄」从**口头与散落注释**
  变成**入库的可审计产物**，并让「改依赖忘了重新生成 notices」在 CI 就失败。

## 结论

本项目以 MIT 发布，仓库内不含第三方源码（`research/` 整目录忽略，`git ls-files research` = 0 条）。
真正被触发告知义务的只有**随 exe 静态链接的 5 个运行期 crate + hidapi 内嵌的 HIDAPI C 库**。
此前这些义务**没有任何落盘证据**：没有 notices 文件、发布物不带许可、没人校验依赖变了之后清单是否过期。
本次改动把这条义务变成「文档 + 生成器 + 双门禁 + PR 必勾项」四件套，并顺手修正了 PLAN 许可表里
两处**事实错误**（两个同名 Battery Hub 的许可混写、fork 计数记错）。

本次改动共 **14 个文件**（10 改 + 4 增），另附本目录 4 篇决策文档。

## 关键事实（均已实测，非推断）

| 项 | 值 | 证据 |
|---|---|---|
| 变更文件数 | 14（10 M + 4 ??） | `git status --porcelain` |
| 新增依赖 | **0**（未动 `Cargo.toml` / `Cargo.lock`） | `git status` 无 Cargo 相关改动 |
| Rust 逻辑改动 | **0**：3 个 crate 只加 `//!` 文档注释 | `git diff crates/` 只有 `//!` 行 |
| notices 一致性门禁 | ✅ `-Check` exit 0，`9 个依赖 / 10 份许可全文` | `gen-notices.ps1 -Check` 实跑输出 |
| 门禁**确实会失败** | ✅ 人为加一行漂移 → exit 1，报「首个差异在第 858 行」 | 负向测试，详见 `detail.md` §4 |
| 负向测试后复原 | ✅ SHA256 逐字节一致（`29c668ef…a60693`） | 备份/恢复 + 哈希比对 |
| 参考 clone 事实核对 | ✅ `research/` 7 个 clone = 6 GPL-3.0 + 1 MIT，与 `docs/licenses.md` 一致 | 逐个读 LICENSE 文件 |
| notices 内 GPL 全文 | ✅ **0 份**（仅 1 处文字枚举「三选一」选项） | `Select-String 'GNU GENERAL PUBLIC LICENSE'` = 1 行，位于三选一说明 |
| `research/` 入库检查 | ✅ 0 条 | `git ls-files research` |
| 4 个新增文件被 gitignore 误伤 | ✅ 否（`git check-ignore` 全部未命中） | `git check-ignore -v` exit 1 |
| 暂存预演 | ✅ 14 个文件全部可 add | `git add --dry-run` |
| `cargo test --locked --all-targets` | ✅ **71 passed / 0 failed**，exit 0 | 实跑汇总各 target 的 `test result:` |
| `cargo clippy --locked --all-targets` | ✅ 0 error / 19 条 warning，**全部既存**（bh-core 18 + razer:96 1 条） | 实跑；razer 告警距本次改动行很远 |
| 绕过模式扫描（DA01–DA07） | ✅ **零命中**；DA08 命中 32 条经逐条核对**全部误报** | `butler_diff_analyzer` + 人工定性，见 `detail.md` §5 |

## 本次改什么（按用途分组）

1. **成文（新）**：`docs/licenses.md` 回答「读过谁、许可要求什么、我们怎么满足、什么算抄」；
   `THIRD-PARTY-NOTICES.txt` 是可随 exe 分发的许可全文汇编。
2. **生成器（新）**：`tools/gen-notices.ps1` 从 `cargo metadata --locked` 推运行期/构建期闭包，
   按内容 SHA256 去重抄录 LICENSE 全文；`-Check` 只校验不改文件。
3. **门禁（改）**：`ci.yml` 在 clippy 之后、测试之前插入 notices 校验；`release.yml` 在打 Release 前
   校验，并把 `THIRD-PARTY-NOTICES.txt` 与 `LICENSE` 一起放进发布物。
4. **修正事实（改）**：`PLAN.md` §2.3 把「原版 BatteryHub / 4 个 C++ fork」拆成两行（C++/Qt = GPL-3.0、
   含 5 个派生仓库；Electron 版 = MIT）；`docs/licenses.md` 明确 HIDAPI C 库三选一、本项目**选举 BSD-3-Clause**。
5. **来源行（改）**：3 个 proto crate 文件头 + 2 份协议笔记各补一行「来源与许可见 docs/licenses.md」。
6. **过程强制（新）**：`.github/pull_request_template.md` 6 项必勾，含洁净室、许可、依赖重生成 notices。
7. **导出（改）**：`README.md` 许可段 + `CHANGELOG.md` 「未发布」段各加指向上面产物的入口。

## 建议 commit message

```
docs(licenses): 第三方来源与许可成文 + notices 生成器与 CI/release 门禁

- 新增 docs/licenses.md：参考来源清单、许可选举（HIDAPI 选 BSD-3-Clause）、洁净室判据
- 新增 THIRD-PARTY-NOTICES.txt 与 tools/gen-notices.ps1（-Check 供 CI 使用）
- 修正 PLAN 许可表：两个同名 Battery Hub 分别 GPL-3.0 / MIT，fork 数 4 → 5
- 3 个 proto crate 与 2 份协议笔记补来源与许可行
- ci/release 加 notices 门禁；release 附带 THIRD-PARTY-NOTICES.txt 与 LICENSE
- 新增 PR 模板：未复制第三方代码 / 依赖改动须重生成 notices 等必勾项

无 Cargo 依赖变更，Rust 逻辑零改动（仅文档注释）。
```

## 风险与遗留

| 项 | 状态 |
|---|---|
| XiuxiTimer（Gitee）许可未核实 | 已登记为待办（`docs/licenses.md` §5 末行），非本次门禁范围 |
| BlueGauge 上游链接缺失 | 同文档 §1.1 标注「待补」 |
| 本地无 `pwsh`（PS7） | 仅 Windows PowerShell 5.1 可用；脚本已验证在 5.1 下正常工作（见 `lessons.md` L2） |
| 任务描述称 13 个文件 | 实测 14 个（多出 `CHANGELOG.md`、`README.md`），见 `lessons.md` L4；两者均属本次改动，建议一并提交 |

## 回滚

纯文档/脚本/工作流改动，无编译产物与数据迁移：`git revert <sha>` 即完全回退；
即使 CI 门禁生效后发现瑕疵，也只是让「清单过期」的 PR 失败，不会阻塞已有构建。
