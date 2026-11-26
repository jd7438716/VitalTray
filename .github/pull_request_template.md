## 这个 PR 做了什么

<!-- 一两句话说明改动目的。修 bug 请附复现步骤。 -->

## 检查项（全部勾选后 CI 才放行）

- [ ] **洁净室**：本 PR 未复制任何第三方代码（含"换语言重写一遍""改名/改注释"这类改写）。
      协议来源已写进 `docs/protocol-notes/`，实现见 notes 而非参考源码；
      `pwsh ./tools/check-cleanroom.ps1` 通过（本地有 `research/` 时会做逐字重合比对）。
- [ ] **许可**：没有把 GPL / MPL 代码、大段摘抄或 `research/` 下的文件带进仓库。
- [ ] **字体**：没有把 `.ttf` / `.ttc` / `.otf` 等字体文件带进仓库；界面只按名字引用系统字体
      （见 `docs/licenses.md` §6；本地跑 `pwsh ./tools/check-fonts.ps1`）。
- [ ] **依赖**：若改过 `Cargo.toml` / `Cargo.lock`，已运行 `pwsh ./tools/gen-notices.ps1`
      并把 `THIRD-PARTY-NOTICES.txt` 一起提交；新增参考来源已登记到 `docs/licenses.md`。
- [ ] **行数**：`pwsh ./tools/check-size.ps1` 通过（单文件 ≤ 400 行）。
- [ ] **验证**：`cargo test --locked --all-targets` 与 `cargo clippy --locked --all-targets` 通过；
      涉及 UI/托盘/面板的改动，已实机跑过并说明验证方式。
- [ ] **读数纪律**：拿不到的温度/电量仍然显示 “—”，没有为了好看编造数值或兜底成 0。

## 备注

<!-- 需要 reviewer 特别注意的点、已知取舍、后续待办。 -->
