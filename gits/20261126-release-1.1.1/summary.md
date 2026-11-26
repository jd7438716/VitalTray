# 发版 1.1.1 · 变更(summary)

- logid: `20261126-release-1.1.1`
- 时间: 2026-11-26 15:50 (Asia/Shanghai)
- 基线: `5fc69f1 fix(identity): exe 补图标/版本信息/应用清单 + 托盘 TaskbarCreated 与第二实例唤醒`
- 目标: 把「exe 身份补齐 + 托盘健壮性」和上一笔未推送的「幽灵行可发现性」一起收口成 1.1.1 并发版

## 结论

本次 release commit 只做收口，代码在 `5fc69f1`（身份/托盘）与 `c7a1023`（幽灵行）：

1. `Cargo.toml` workspace 版本 `1.1.0 → 1.1.1`；
2. `Cargo.lock` 随之更新（CI 用 `--locked`，不一致会直接失败）；
3. `CHANGELOG.md` 把 `[未发布]` 收口成 `[1.1.1] - 2026-11-26`，并留空新的 `[未发布]`；
4. `README.md` 的「当前版本」跟上（该项在 1.1.0 时就漂成了 1.0.0，上一笔先修正到 1.1.0，这里再到 1.1.1）。

## 关键事实

| 项 | 值 |
|---|---|
| 待提交文件 | 5（Cargo.toml / Cargo.lock / CHANGELOG.md / README.md / 本目录） |
| `cargo build --locked --release` | ✅ 通过（CI 同口径） |
| `--version` | ✅ BatteryHub 1.1.1 |
| exe 版本资源 | ✅ FileVersion / ProductVersion = 1.1.1 |
| `--self-test` | ✅ SELFTEST OK |
| 单测 | ✅ 71 项（`cargo test --locked --all-targets`） |
| 行数门禁 | ✅ 66 文件 0 超限 0 警告 |
| `check-exe.ps1` | ✅ 5 项全 PASS |
| tag 一致性 | ✅ release.yml 校验 `tag` 去 `v` == `bh-app` 版本 == `1.1.1` |

## 发布方式

`git push origin main` + `git push origin v1.1.1` → `.github/workflows/release.yml` 在 windows-latest 上
构建、跑资源门禁与 `--self-test`、生成 `SHA256SUMS.txt`、发布到 GitHub Releases。

发布后用 `gh release edit v1.1.1 --notes-file <文件>` 把手写说明覆盖上去——自动生成只会列 commit 标题，
而用户最关心的「exe 终于有图标了」在标题里不明显。
