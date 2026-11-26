# 修复细节（detail）

## 1. 根因

`tools/gen-notices.ps1` 有 4 处排序参与输出字节：

| 位置（修复前） | 作用 |
|---|---|
| `Get-LicenseFileList` → `Sort-Object Name` | 每个 crate 内 LICENSE 文件的顺序 → 决定 `Sources[0]` |
| `$rows = $rows \| Sort-Object Name, Version` | 依赖表顺序 + `Sources` 追加顺序 |
| `$ordered = $ordered \| Sort-Object SortKey` | 许可全文小节顺序（`SortKey` 含中文标签） |
| `$texts.Keys` 遍历 | 由上面的 `SortKey` 排序覆盖 |

`Sort-Object` 在 PowerShell 里是**文化敏感**的字符串比较。`SortKey` 形如
`Apache License 2.0（全文）|cc 1.11.3（LICENSE-APACHE）`，其中「全」U+5168、"简" U+7B80
的排序权重由当前区域设置决定：zh-CN 把「简」排在「全」前，en-US 按码位把「全」排在前。
本机是 zh-CN，CI runner 是 en-US → 生成物顺序不同 → `-Check` 失败。

这解释了为什么本地 `-Check` 一直绿：本地生成与本地校验用的是同一套文化，
**门禁的"确定性"从未真正被跨环境验证过**。

## 2. 改动

文件：`tools/gen-notices.ps1`（唯一逻辑改动文件；`THIRD-PARTY-NOTICES.txt` 因顺序变化重新生成）

1. **新增 `Sort-OrdinalBy`**：把待排对象投影成 `Key`，用
   `[string]::CompareOrdinal` 做 `List.Sort(Comparison)`，返回重排后的对象数组。
   脚本顶部注释写明"绝不使用 Sort-Object 默认文化排序"及本次踩坑原因。
2. **`Get-LicenseInfo` 取代 `Get-LicenseLabel`**：返回 `{ Code = 'apache2-full' | 'apache2-short' | 'bsd3' | 'mit' | 'unknown'; Label = '中文展示名' }`。
   排序键用 `Code`，中文只进正文。
3. **`Get-LicenseFileList`**：`Sort-Object Name` → `Sort-OrdinalBy 'Name'`。
4. **`$rows`**：新增 `SortKey = "<name>|<version>"`，`Sort-OrdinalBy` 排序（原 `Sort-Object Name, Version`）。
5. **`$ordered`**：`SortKey = "<Code>|<Sources[0]>"`，`Sort-OrdinalBy` 排序（原 `Sort-Object SortKey`）。

未改：去重逻辑（SHA256）、依赖图遍历（normal 边闭包）、HIDAPI 选举抄录、
`-Check` 的严格逐字节比较、输出模板文字。

## 3. 行为变化（对外可见）

`THIRD-PARTY-NOTICES.txt` 第四节小节编号变化（内容与字节总量不变）：

| 小节 | 修复前（zh-CN） | 修复后（两种 locale 一致） |
|---|---|---|
| 【1】 | Apache 2.0（简式声明） | Apache 2.0（全文） |
| 【2】【3】 | Apache 2.0（全文） | Apache 2.0（全文） |
| 【4】 | Apache 2.0（全文） | Apache 2.0（简式声明） |
| 【5】–【10】 | BSD-3（HIDAPI）、MIT ×5 | 同左 |

`857 行 / 44,371 B`、`10 份许可全文`、GPL 全文 0 份 —— 三项均与修复前一致。

## 4. 验证记录

```
# 1) zh-CN 生成
& .\tools\gen-notices.ps1
  → WROTE （9 个依赖 / 10 份许可全文 / 43 KB）
  → SHA256 AE1FE7D61C940DCC78FAB1D56E9BEA4123ECDE2F55E4AC0CE00A6CBDCC12681A

# 2) 切到 en-US 文化再生成
[Threading.Thread]::CurrentThread.CurrentCulture = (New-Object Globalization.CultureInfo('en-US'))
& .\tools\gen-notices.ps1
  → SHA256 AE1FE7D6…CC12681A   （逐字节相同）

# 3) 在 en-US 文化下跑门禁（最接近 CI 的本地场景）
& .\tools\gen-notices.ps1 -Check
  → OK THIRD-PARTY-NOTICES.txt 与依赖一致（9 个依赖 / 10 份许可全文）  exit=0
```

## 5. 未做 / 边界

- 未在本机跑 PowerShell 7（无 `pwsh`）：修复的结构性理由是"Ordinal 比较不吃 locale 也不吃 PS 版本"，
  但**最终判据是 CI 真机**，本次推送后观察 run 结果。
- 未给"文化敏感排序"加独立回归测试：门禁本身（严格逐字节 `-Check`）在跨环境 CI 上就是回归测试，
  再加一层单测属重复建设。
- 上一 logid `20261126-license-compliance` 的 `detail.md:48` 记录的是**修复前**的函数行号区间
  （42–50），现已不在该位置；按审计只读纪律不改写原记录，仅在其 `decision-chain.md` 末尾追加指引。
