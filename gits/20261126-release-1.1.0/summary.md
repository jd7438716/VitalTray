# 提交「发版 1.1.0」变更（summary）

- logid: `20261126-release-1.1.0`
- 时间: 2026-11-26 14:35 (Asia/Shanghai)
- 基线: `7f63325 feat(m8): 面板改温度（去占用、加硬盘温度）+ 状态色与厂商协议注册表`
- 目标: 把已经验收的改动收口成 1.1.0 并发布（tag → CI 自动构建并发布 GitHub Release）

## 结论

代码改动已全部在 `7f63325` 里（两条并行开发线合并提交，理由见
`gits/20261126-m8-temperature-panel/decision-chain.md` D1）。本次提交**只做发版收口**：

1. `Cargo.toml` 的 workspace 版本 `1.0.0 → 1.1.0`；
2. `Cargo.lock` 随之更新（`cargo build --release` 产出，CI 用 `--locked` 构建，必须一致）；
3. `CHANGELOG.md` 把 `[未发布]` 收口成 `[1.1.0] - 2026-11-26`，并留空新的 `[未发布]`。

## 关键事实

| 项 | 值 |
|---|---|
| 待提交文件 | 4（Cargo.toml / Cargo.lock / CHANGELOG.md / gits/20261126-release-1.1.0/） |
| `cargo build --locked --release` | ✅ 通过（CI 同口径） |
| `--self-test` | ✅ SELFTEST OK（plugins 版本显示 1.1.0） |
| `--version` | ✅ BatteryHub 1.1.0 |
| 单测 | ✅ 67 项（bh-core 49 + 其它 17 + self-test 1） |
| 行数门禁 | ✅ 64 文件 0 超限 |
| tag 一致性 | ✅ release.yml 校验 `tag` 去 `v` 后 == `bh-app` 版本 == `1.1.0` |
| push | 待执行（用户已授权「提交。发布」） |

## 发布方式

`git push origin main` + `git push origin v1.1.0` → `.github/workflows/release.yml` 在 windows-latest 上
构建、跑 `--self-test`、生成 `SHA256SUMS.txt`、发布到 GitHub Releases。发布后再用
`gh release edit v1.1.0 --notes-file <文件>` 把手写说明覆盖掉自动生成的 commit 列表。
