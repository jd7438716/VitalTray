# 洁净室门禁：确保"只参考协议事实、不复制 GPL 代码"这条公开声明是可执行的。
#
# 两道检查：
#   1) 浅层（CI 也跑）：扫源码里是否混入 GPL/MPL 的许可证正文或 SPDX 标记 —— 复制代码最容易留下的指纹。
#   2) 深层（仅本地，需 research/ 下有上游 clone）：把上游的注释与字符串字面量逐条与我们源码做
#      「去空白后整串包含」比对；命中即失败。CI 里 research/ 不入库，这道会自动跳过。
#
# 用法：pwsh ./tools/check-cleanroom.ps1
# 背景：docs/adr/0003-license-strategy.md
param([int]$MinLen = 25)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

# ---- 1) 浅层：许可证正文 / SPDX 标记不应出现在我们自己的代码里 ----
$licenseMarkers = @(
    'GNU General Public License',
    'GNU Lesser General Public License',
    'GNU Affero General Public License',
    'This program is free software; you can redistribute it',
    'Mozilla Public License',
    'SPDX-License-Identifier: GPL',
    'SPDX-License-Identifier: AGPL',
    'SPDX-License-Identifier: MPL',
    'Copyright (C) 20'   # 间接指纹：整段搬来的版权头
)
$codeFiles = @()
$codeFiles += @(Get-ChildItem "$root\crates", "$root\plugins", "$root\tools" -Recurse -File -Include *.rs, *.ps1 -ErrorAction SilentlyContinue)
$codeFiles = @($codeFiles | Where-Object { $_.FullName -notlike '*\target\*' -and $_.Name -ne 'check-cleanroom.ps1' })

$bad = 0
# 允许的"提到许可名"：只引流必要的引用处，且不得夹带正文。每条都要写理由，新增项须在 PR 里说明。
# 键 = 相对仓库根的路径。
$allow = @{
    'tools\gen-notices.ps1' = 'HIDAPI 三选一许可的选项枚举：只引用许可名，未收录 GPL 正文'
}
foreach ($f in $codeFiles) {
    $rel = $f.FullName.Substring($root.Length + 1)
    $text = [IO.File]::ReadAllText($f.FullName, [Text.Encoding]::UTF8)
    foreach ($m in $licenseMarkers) {
        if ($text.IndexOf($m, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            if ($allow.ContainsKey($rel)) {
                Write-Host ("ALLOW {0} ← 「{1}」：{2}" -f $rel, $m, $allow[$rel]) -ForegroundColor DarkGray
                continue
            }
            Write-Host ("FAIL 疑似许可证正文/版权头：{0} ← 「{1}」" -f $rel, $m) -ForegroundColor Red
            Write-Host "     若确属引用（例如注释里的来源说明），加进本脚本的 allow 白名单并写理由；真复制了 GPL 代码则必须删除。" -ForegroundColor Yellow
            $bad++
        }
    }
}
Write-Host ("浅层检查：扫描 {0} 个源码文件，{1} 处命中" -f $codeFiles.Count, $bad)
if ($bad -gt 0) { Write-Host "FAIL 洁净室检查未通过（浅层）" -ForegroundColor Red; exit 1 }

# ---- 2) 深层：与 research/ 下上游 clone 做逐字重合比对（仅本地） ----
$research = Join-Path $root 'research'
if (-not (Test-Path -LiteralPath $research)) {
    Write-Host "深层检查：无 research/ 目录（CI 正常情况），跳过逐字比对。" -ForegroundColor DarkGray
    Write-Host "OK   洁净室检查通过（仅浅层）" -ForegroundColor Green
    exit 0
}

$rustSquash = ([string]::Concat((Get-ChildItem "$root\crates", "$root\plugins" -Recurse -File -Include *.rs | ForEach-Object { [IO.File]::ReadAllText($_.FullName, [Text.Encoding]::UTF8) })) -replace '\s+', '')
$upstreamFiles = @(Get-ChildItem $research -Recurse -File -Include *.cpp, *.h, *.hpp, *.cs, *.js, *.ts -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -notlike '*\node_modules\*' })
$cands = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($f in $upstreamFiles) {
    $raw = [IO.File]::ReadAllText($f.FullName, [Text.Encoding]::UTF8)
    foreach ($m in [regex]::Matches($raw, '"(?:[^"\\]|\\.){12,}"')) { [void]$cands.Add(($m.Value.Trim('"') -replace '\\n', ' ')) }
    foreach ($m in [regex]::Matches($raw, '(?m)//[^\n]{12,}')) { [void]$cands.Add($m.Value.Substring(2).Trim()) }
}
$tested = 0
$skippedApi = 0
$hits = @()
foreach ($c in $cands) {
    # 平台 API 常量（全大写标识符，如 KMTQAITYPE_ADAPTERPERFDATA / IOCTL_* / DEVPKEY_*）：
    # 是 Windows SDK 的公开符号，不是可版权的表达，凡是调用同一 API 的项目必然重合 —— 跳过。
    if ($c.Trim() -cmatch '^[A-Z][A-Z0-9_]{11,}$') { $skippedApi++; continue }
    $sq = ($c -replace '\s+', '')
    if ($sq.Length -lt $MinLen) { continue }
    $tested++
    if ($rustSquash.Contains($sq)) { $hits += $c }
}
Write-Host ("深层检查：上游 {0} 个文件 / 比对 {1} 条注释与字符串（≥{2} 字符），跳过平台 API 常量 {3} 条" -f $upstreamFiles.Count, $tested, $MinLen, $skippedApi)
if ($hits.Count -gt 0) {
    foreach ($h in ($hits | Select-Object -First 10)) { Write-Host ("FAIL 与上游逐字重合：{0}" -f $h) -ForegroundColor Red }
    Write-Host "FAIL 发现逐字重合：这不再是「只参考思路」，必须重写该段实现。" -ForegroundColor Red
    exit 1
}
Write-Host "OK   洁净室检查通过（浅层 + 深层，0 处逐字重合）" -ForegroundColor Green
