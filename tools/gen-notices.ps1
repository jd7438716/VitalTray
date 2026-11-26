# 第三方许可清单生成器：生成 / 校验 THIRD-PARTY-NOTICES.txt。
#
# 为什么需要：vitaltray.exe 静态链了 windows-sys、hidapi（含其内嵌的 HIDAPI C 库）等第三方组件。
# MIT / Apache-2.0 / BSD-3-Clause 都要求「分发二进制时附带版权与许可声明」，这份文件就是履行该义务。
#
# 用法：
#   pwsh ./tools/gen-notices.ps1           # 重新生成 THIRD-PARTY-NOTICES.txt
#   pwsh ./tools/gen-notices.ps1 -Check    # 只校验，与仓库内文件不一致则 exit 1（CI 用）
#
# 数据来源：cargo metadata（--locked）+ 本地 cargo registry 里各 crate 的 LICENSE* 文件。
# 许可全文按内容 SHA256 去重，同一份文本只出现一次。
# 许可选举（双许可 / 多许可组件怎么选）写在 docs/licenses.md，本脚本只如实抄录原文。
param([switch]$Check)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$outPath = Join-Path $root 'THIRD-PARTY-NOTICES.txt'

function Convert-ToLf {
    param([string]$Text)
    return ($Text -replace "`r`n", "`n" -replace "`r", "`n")
}

function Get-LicenseFileList {
    param([string]$Dir)
    $files = @(Get-ChildItem -LiteralPath $Dir -File |
        Where-Object { $_.Name -match '^(?i)(licen[cs]e|notice|copying)' })
    return @(Sort-OrdinalBy $files 'Name')
}

function Get-Sha256Of {
    param([string]$Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace '-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

# 排序一律用 Ordinal（按码位比较），**绝不**用 Sort-Object 的默认文化排序：
# 本机是 zh-CN + Windows PowerShell 5.1，CI 是 en-US + PowerShell 7，两者对含中文的
# 标签排序结果不同 —— 曾导致 CI 上同一依赖生成出不同顺序的文件，门禁把自己判红。
function Sort-OrdinalBy {
    param([object[]]$Items, [string]$KeyProperty)
    $arr = @($Items)
    if ($arr.Count -eq 0) { return @() }
    $pairs = New-Object 'System.Collections.Generic.List[object]'
    foreach ($it in $arr) {
        $pairs.Add([pscustomobject]@{ Key = [string]$it.$KeyProperty; Item = $it })
    }
    $cmp = [System.Comparison[object]]{ param($a, $b) [string]::CompareOrdinal($a.Key, $b.Key) }
    $pairs.Sort($cmp)
    return @($pairs | ForEach-Object { $_.Item })
}

# 许可标签拆成「ASCII 代号（只用于排序）+ 中文展示名」，中文不参与排序比较。
function Get-LicenseInfo {
    param([string]$Text)
    if ($Text -match 'Apache License\s*\n\s*Version 2\.0') {
        return [pscustomobject]@{ Code = 'apache2-full'; Label = 'Apache License 2.0（全文）' }
    }
    if ($Text -match 'Apache License, Version 2\.0') {
        return [pscustomobject]@{ Code = 'apache2-short'; Label = 'Apache License 2.0（简式声明，全文见另一节）' }
    }
    if ($Text -match 'Redistribution and use in source and binary forms') {
        return [pscustomobject]@{ Code = 'bsd3'; Label = 'BSD 3-Clause' }
    }
    if ($Text -match 'Permission is hereby granted, free of charge') {
        return [pscustomobject]@{ Code = 'mit'; Label = 'MIT' }
    }
    if ($Text -match 'GNU GENERAL PUBLIC LICENSE') {
        return [pscustomobject]@{ Code = 'gpl'; Label = 'GPL（不应出现在本清单）' }
    }
    return [pscustomobject]@{ Code = 'unknown'; Label = '未识别许可文本' }
}

# ---- 1. 读依赖图：区分「链接进二进制」与「仅构建期使用」 ----
$raw = & cargo metadata --locked --format-version 1 2>$null
if ($LASTEXITCODE -ne 0) { throw "cargo metadata 失败（退出码 $LASTEXITCODE）" }
$meta = $raw | ConvertFrom-Json

$nodes = @{}
foreach ($n in $meta.resolve.nodes) { $nodes[$n.id] = $n }

# 从 workspace 成员出发，只沿 normal（非 build/dev）依赖边遍历 = 运行期闭包。
$runtimeIds = New-Object 'System.Collections.Generic.HashSet[string]'
$queue = New-Object 'System.Collections.Generic.Queue[string]'
foreach ($m in $meta.workspace_members) {
    if ($runtimeIds.Add($m)) { $queue.Enqueue($m) }
}
while ($queue.Count -gt 0) {
    $node = $nodes[$queue.Dequeue()]
    if (-not $node) { continue }
    foreach ($dep in $node.deps) {
        # 注意：cargo metadata 里普通依赖的 kind 是空值（只有 dev/build 才显式标注），
        # 所以“空”必须算作 normal，否则会把全部依赖误判成构建期工具。
        $isNormal = $false
        foreach ($k in $dep.dep_kinds) {
            if ([string]::IsNullOrEmpty($k.kind) -or $k.kind -eq 'normal') { $isNormal = $true }
        }
        if ($isNormal -and $runtimeIds.Add($dep.pkg)) { $queue.Enqueue($dep.pkg) }
    }
}

$rows = @()
foreach ($p in $meta.packages) {
    if ($p.source -notlike 'registry+*') { continue }   # 只列第三方；workspace 自身跳过
    $dir = Split-Path -Parent $p.manifest_path
    if (-not (Test-Path -LiteralPath $dir)) {
        throw "缺少 crate 源码：$($p.name) $($p.version) —— 先运行 cargo fetch"
    }
    $rows += [pscustomobject]@{
        Name    = $p.name
        Version = $p.version
        License = if ($p.license) { $p.license } else { '(未声明)' }
        Dir     = $dir
        Runtime = $runtimeIds.Contains($p.id)
        SortKey = ("{0}|{1}" -f $p.name, $p.version)
    }
}
$rows = @(Sort-OrdinalBy $rows 'SortKey')
if ($rows.Count -eq 0) { throw '没有找到任何第三方依赖，检查 cargo metadata 输出' }

# ---- 2. 收集许可全文并按内容去重 ----
$texts = @{}
foreach ($row in $rows) {
    foreach ($f in Get-LicenseFileList $row.Dir) {
        $text = Convert-ToLf (Get-Content -LiteralPath $f.FullName -Raw -Encoding UTF8)
        $sha = Get-Sha256Of $text
        if (-not $texts.ContainsKey($sha)) {
            $info = Get-LicenseInfo $text
            $texts[$sha] = [pscustomobject]@{
                Code    = $info.Code
                Label   = $info.Label
                Text    = $text.TrimEnd("`n")
                Sources = New-Object 'System.Collections.Generic.List[string]'
            }
        }
        $texts[$sha].Sources.Add(("{0} {1}（{2}）" -f $row.Name, $row.Version, $f.Name))
    }
}

# ---- 3. hidapi 内嵌 C 库：单独抄录「本项目选定的那份」 ----
# 上游 HIDAPI 是三选一（GPLv3 / BSD-3 / 原始许可），由使用者选择；本项目选 BSD-3-Clause。
# 只抄 LICENSE-bsd.txt，不把 LICENSE-gpl3.txt 收进来，避免读者误以为本项目选了 GPL。
$hidapiRow = $rows | Where-Object { $_.Name -eq 'hidapi' } | Select-Object -First 1
if (-not $hidapiRow) { throw 'Cargo.lock 里没有 hidapi，脚本里的 HIDAPI 选举说明需要重写' }
$hidapiBundled = Join-Path $hidapiRow.Dir 'etc\hidapi'
$bsdPath = Join-Path $hidapiBundled 'LICENSE-bsd.txt'
if (-not (Test-Path -LiteralPath $bsdPath)) { throw "找不到 HIDAPI 内嵌 C 库的 BSD 许可：$bsdPath" }
$bsdText = Convert-ToLf (Get-Content -LiteralPath $bsdPath -Raw -Encoding UTF8)
$texts[(Get-Sha256Of $bsdText)] = [pscustomobject]@{
    Code    = 'bsd3-hidapi'
    Label   = 'BSD 3-Clause（HIDAPI C 库）'
    Text    = $bsdText.TrimEnd("`n")
    Sources = (New-Object 'System.Collections.Generic.List[string]')
}
$texts[(Get-Sha256Of $bsdText)].Sources.Add('HIDAPI 内嵌 C 库（etc/hidapi/windows/hid.c 等）')

# ---- 4. 拼装输出 ----
$gen = @()
$gen += 'VitalTray 第三方许可声明（THIRD-PARTY-NOTICES）'
$gen += '================================================='
$gen += ''
$gen += '本文件由 tools/gen-notices.ps1 自动生成，请勿手工编辑。'
$gen += '  重新生成：pwsh ./tools/gen-notices.ps1'
$gen += '  一致性校验：pwsh ./tools/gen-notices.ps1 -Check（CI 会跑）'
$gen += ''
$gen += 'VitalTray 自身以 MIT 许可发布（见 LICENSE）。本文件覆盖随 vitaltray.exe'
$gen += '一起分发的第三方组件及其许可全文，用于履行 MIT / Apache-2.0 / BSD-3-Clause'
$gen += '「分发二进制时须附带版权与许可声明」的义务。'
$gen += ''
$gen += '对于 MIT OR Apache-2.0 的双许可组件，本项目选举 MIT；'
$gen += '对于 HIDAPI C 库的三选一许可，本项目选举 BSD-3-Clause。'
$gen += '选举理由与全部来源清单见 docs/licenses.md。'
$gen += ''
$gen += ("依赖总数：{0} 个（运行期 {1} 个，构建期 {2} 个）；去重后许可全文 {3} 份。" -f `
    $rows.Count, @($rows | Where-Object { $_.Runtime }).Count, @($rows | Where-Object { -not $_.Runtime }).Count, $texts.Count)
$gen += ''

$gen += '一、运行期依赖（参与链接，随 exe 分发）'
$gen += '--------------------------------------'
$gen += '| 组件 | 版本 | 许可（Cargo.toml 声明） |'
$gen += '|---|---|---|'
foreach ($row in ($rows | Where-Object { $_.Runtime })) {
    $gen += ("| {0} | {1} | {2} |" -f $row.Name, $row.Version, $row.License)
}
$gen += ''
$gen += '说明：Windows 上部分依赖（如 libc）可能被条件编译排除、最终不产生机器码；'
$gen += '本清单按 Cargo.lock 的正常依赖闭包保守列出（宁多不少）。'
$gen += ''

$buildOnly = @($rows | Where-Object { -not $_.Runtime })
if ($buildOnly.Count -gt 0) {
    $gen += '二、构建期工具（只在编译时运行，不随 exe 分发，无分发告知义务）'
    $gen += '--------------------------------------------------------------'
    $gen += '| 组件 | 版本 | 许可（Cargo.toml 声明） |'
    $gen += '|---|---|---|'
    foreach ($row in $buildOnly) {
        $gen += ("| {0} | {1} | {2} |" -f $row.Name, $row.Version, $row.License)
    }
    $gen += ''
}

$gen += '三、HIDAPI 内嵌 C 库（特别说明）'
$gen += '------------------------------'
$gen += 'vitaltray.exe 通过 hidapi crate 静态编译了 HIDAPI C 库的 Windows 后端'
$gen += '（etc/hidapi/windows/hid.c、hidapi_descriptor_reconstruct.c 及头文件）。'
$gen += ''
$gen += '上游 HIDAPI 采用三选一许可，由使用者选择：'
$gen += '  1. GNU General Public License v3.0（LICENSE-gpl3.txt）'
$gen += '  2. BSD-Style License（LICENSE-bsd.txt）'
$gen += '  3. 原始 HIDAPI 许可（LICENSE-orig.txt）'
$gen += ''
$gen += '本项目明确选举 (2) BSD-Style License：与 MIT 兼容、可分发闭源二进制、'
$gen += '不引入 GPL 的源码开放义务。原文见本文件第四节「BSD 3-Clause（HIDAPI C 库）」。'
$gen += ''

$gen += '四、许可全文（按内容去重，每份只出现一次）'
$gen += '------------------------------------------'
# 输出顺序按「许可代号 + 首个适用组件」排序，保证每次生成结果一致（见 Sort-OrdinalBy 注释）。
$ordered = @()
foreach ($sha in $texts.Keys) {
    $entry = $texts[$sha]
    $ordered += [pscustomobject]@{ SortKey = ("{0}|{1}" -f $entry.Code, $entry.Sources[0]); Entry = $entry }
}
$ordered = @(Sort-OrdinalBy $ordered 'SortKey')
$order = 0
foreach ($item in $ordered) {
    $entry = $item.Entry
    $order++
    $gen += ''
    $gen += ("【{0}】{1}" -f $order, $entry.Label)
    $gen += '适用于：'
    foreach ($s in $entry.Sources) { $gen += ("  - {0}" -f $s) }
    $gen += ''
    $gen += '--------------------------------------------------------------'
    $gen += $entry.Text
    $gen += '--------------------------------------------------------------'
}

$content = ($gen -join "`n") + "`n"

if ($Check) {
    if (-not (Test-Path -LiteralPath $outPath)) {
        Write-Host "FAIL 缺少 THIRD-PARTY-NOTICES.txt，先运行：pwsh ./tools/gen-notices.ps1" -ForegroundColor Red
        exit 1
    }
    # 必须显式 -Encoding UTF8：Windows PowerShell 5.1 对无 BOM 的 UTF-8 会按 ANSI 解码，
    # 读回来的中文全是乱码，-Check 会永远失败。
    $existing = Convert-ToLf (Get-Content -LiteralPath $outPath -Raw -Encoding UTF8)
    $existing = $existing.TrimEnd("`n") + "`n"
    if ($existing -cne $content) {
        Write-Host 'FAIL THIRD-PARTY-NOTICES.txt 与当前依赖不一致。' -ForegroundColor Red
        Write-Host '     改动过依赖（Cargo.toml / Cargo.lock）后必须重新生成并一起提交：' -ForegroundColor Yellow
        Write-Host '       pwsh ./tools/gen-notices.ps1' -ForegroundColor Yellow
        $a = $existing -split "`n"
        $b = $content -split "`n"
        $max = [Math]::Min($a.Count, $b.Count)
        for ($i = 0; $i -lt $max; $i++) {
            if ($a[$i] -cne $b[$i]) {
                Write-Host ("     首个差异在第 {0} 行（仓库内 / 期望）：" -f ($i + 1)) -ForegroundColor Yellow
                Write-Host ("       - {0}" -f $a[$i]) -ForegroundColor Yellow
                Write-Host ("       + {0}" -f $b[$i]) -ForegroundColor Yellow
                break
            }
        }
        exit 1
    }
    Write-Host ("OK   THIRD-PARTY-NOTICES.txt 与依赖一致（{0} 个依赖 / {1} 份许可全文）" -f $rows.Count, $texts.Count) -ForegroundColor Green
    exit 0
}

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($outPath, $content, $utf8NoBom)
Write-Host ("WROTE {0}（{1} 个依赖 / {2} 份许可全文 / {3} KB）" -f `
    (Split-Path $outPath -Leaf), $rows.Count, $texts.Count, [Math]::Round((Get-Item $outPath).Length / 1KB)) -ForegroundColor Green
