# 提交「notices 排序 locale 无关化」修复（summary）

- logid: `20261126-notices-ordinal-fix`
- 时间: 2026-11-26 16:35 (Asia/Shanghai)
- 基线: `d5578b1 docs(licenses): 第三方来源与许可成文 + notices 生成器与 CI/release 门禁`
- 触发: CI run [45780896975](https://github.com/jd7438716/battery-hub/actions/runs/45780896975) 在 `d5578b1` 上 **失败**

## 结论

失败**不是** PowerShell 7 兼容性问题（我提交时唯一的疑虑），而是 `gen-notices.ps1` 里的
**排序对区域设置敏感**：`Sort-Object` 用文化相关的字符串比较，中文标签「全」与「简」的
先后顺序在 zh-CN 与 en-US 下相反，于是同一份依赖在 CI 机器上生成出另一种小节顺序，
`-Check` 把自己判红。

CI 报错与我本地文件的差异行**逐字一致**：

```
首个差异在第 56 行（仓库内 / 期望）：
 - 【1】Apache License 2.0（简式声明，全文见另一节）      ← 本地 zh-CN 生成
 + 【1】Apache License 2.0（全文）                        ← CI en-US 生成
```

本机复现对照（同一批标签，仅切换 `CurrentThread.CurrentCulture`）：

| 环境 | 排序结果 |
|---|---|
| zh-CN | 简式声明 → 全文 → BSD 3-Clause → MIT |
| en-US | **全文 → 简式声明** → BSD 3-Clause → MIT |

## 修复

`tools/gen-notices.ps1`：排序一律走 `[string]::CompareOrdinal`（按码位比较，与 locale /
PowerShell 版本无关），并把排序键改成 ASCII（许可代号 `apache2-full` / `mit` …），
中文标签只做展示、不参与比较。

## 验证（实测，非推断）

| 断言 | 命令 | 结果 |
|---|---|---|
| 生成器在两种文化下产出逐字节相同 | zh-CN 生成 → SHA256；切 en-US 再生成 → SHA256 | 均为 `AE1FE7D6…CC12681A`，相同 |
| 在最能复现 CI 的文化下门禁通过 | 切 en-US 后 `& ./tools/gen-notices.ps1 -Check` | `OK 9 个依赖 / 10 份许可全文`，exit 0 |
| 文件体量未变（仅顺序变化） | `Get-Item` 长度 / 行数 | 44,371 B / 857 行，与上一版一致 |
| 许可分布未变 | `Select-String '^【\d+】'` | Apache 全文 ×3、简式 ×1、BSD-3（HIDAPI）×1、MIT ×5 |
| 行数门禁 / 测试未受影响 | `check-size.ps1`、`cargo test` | 66 文件 0 超限；exit 0 |

## 已知边界

本机仍无 PowerShell 7（`pwsh` 不存于 PATH），该修复的最终判据是 **CI 真机复跑**——
本次提交后会观察 `windows-latest` + `shell: pwsh` 的结果，避免再次用"本地能跑"外推。
