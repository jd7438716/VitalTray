# 发版 1.2.1 · 变更(summary)

- logid: `20261126-release-1.2.1`
- 时间: 2026-11-26 17:47 (Asia/Shanghai)
- 基线: `1a650f5 fix(startup): 打开只驻留托盘，不再弹面板`
- 目标: 把「打开只驻留托盘」收口成 1.2.1 并发布（用户反馈触发）

## 结论

本次 release commit 只做收口，代码与文档在 `1a650f5`：

1. `Cargo.toml` workspace 版本 `1.2.0 → 1.2.1`（`Cargo.lock` 11 个成员随之更新，CI 用 `--locked`）；
2. `CHANGELOG.md` 把 `[未发布]` 收口成 `[1.2.1] - 2026-11-26`；
3. `README.md` / `README.en.md` 的「当前版本」与路线同步到 1.2.1；
4. `gits/20261126-start-minimized/detail.md` 补记 BOM 回填与 `--amend`（首版 `9199b4b` → 最终 `1a650f5`）。

## 关键事实

| 项 | 值 |
|---|---|
| 待提交文件 | 6（Cargo.toml / Cargo.lock / CHANGELOG.md / README.md / README.en.md / detail.md） |
| `cargo test --workspace --locked --all-targets` | ✅ 82 项全绿、0 failed |
| `cargo clippy --locked --all-targets` | ✅ 0 error（沿用仓库既有 warning 口径，未加 `-D warnings`） |
| 行数门禁 | ✅ 130 文件 0 超限 **0 警告** |
| `check-readme.ps1` | ✅ 双语（15 个标题 / 11 个站内链接一致） |
| `check-cleanroom.ps1` | ✅ 浅层 96 文件 0 命中；深层 9553 条比对 0 重合 |
| `check-fonts.ps1` | ✅ 0 命中 |
| `gen-notices.ps1 -Check` | ✅ 9 个依赖 / 10 份许可全文未过期 |
| `check-exe.ps1` | ✅ 5 项全 PASS（图标 / 版本信息 / 清单 / FileVersion / ProductName） |
| `--version` / `--self-test` | ✅ `VitalTray 1.2.1` / `SELFTEST OK` |
| 本地 release exe | 581120 B，SHA256 `1b44ccbaf0ec8bcb1634692c90775be3a5ae64df904249bfd7092c02793dfda2` |
| 发布物 exe（CI 构建） | 581120 B，SHA256 `b2281a70202e8e74550af54213547a1520a8bb3fbe1c2ab6985f224a7c925304` |
| tag 一致性 | ✅ release.yml 校验 `v1.2.1` 去 `v` == `bh-app` 版本 == `1.2.1`（run 45794587264 success） |

## 发布方式

`git push origin main` + `git push origin v1.2.1` → `.github/workflows/release.yml` 在 windows-latest
构建、跑门禁与 `--self-test`、生成 `SHA256SUMS.txt`、发布到 GitHub Releases：

- Release：<https://github.com/jd7438716/VitalTray/releases/tag/v1.2.1>（非 draft、非 prerelease）
- 资产 4 件：`vitaltray.exe` / `SHA256SUMS.txt` / `THIRD-PARTY-NOTICES.txt` / `LICENSE`
- CI（main `0d65783`）与 Release 两个 run 均 **success**

发布后做了两件收尾：

1. **下载发布物回验**：`gh release download v1.2.1` → exe 实算 SHA256 与 `SHA256SUMS.txt` 一致（MATCH），
   实跑 `--version` = `VitalTray 1.2.1`、`--self-test` = `SELFTEST OK`（本地构建与 CI 构建哈希不同属正常，
   以发布页 `SHA256SUMS.txt` 为准）；
2. **手写发布说明**：自动生成的 notes 只有 commit 标题，用户最关心的「打开不再弹面板」在标题里看不出来，
   用 `gh release edit v1.2.1 --notes-file <文件>` 覆盖为手写说明（含行为变化表、三个开关含义、校验方式）。
