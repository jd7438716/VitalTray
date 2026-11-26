# README 双语门禁：中文 README.md 与英文 README.en.md 的结构必须对齐。
#
# 不做逐句翻译一致性检查（做不到也不值得），只查两件最容易漂移的事：
#   1) 标题层级序列一致（## / ### 的数量与顺序）—— 少一节、多一节立刻暴露；
#   2) 站内相对链接集合一致（docs/*、ADR、PLAN 等）—— 中文版新加了文档链接，英文版必须跟上。
# 图片（.png 等）与语言切换互链不做比较：它们本来就允许左右不同。
#
# 用法：pwsh ./tools/check-readme.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$zhPath = Join-Path $root 'README.md'
$enPath = Join-Path $root 'README.en.md'

foreach ($p in @($zhPath, $enPath)) {
    if (-not (Test-Path -LiteralPath $p)) {
        Write-Host ("FAIL 缺少 {0}：双语 README 是一体的，缺一份就失败。" -f (Split-Path $p -Leaf)) -ForegroundColor Red
        exit 1
    }
}

function Read-Text {
    param([string]$Path)
    return ([IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8) -replace "`r`n", "`n")
}

function Get-HeadingDepths {
    param([string]$Text)
    $depths = @()
    foreach ($line in ($Text -split "`n")) {
        if ($line -match '^(#{1,6})\s+\S') { $depths += $Matches[1].Length }
    }
    return $depths
}

function Get-RelativeLinks {
    param([string]$Text)
    $set = New-Object 'System.Collections.Generic.HashSet[string]'
    foreach ($m in [regex]::Matches($Text, '\]\(([^)\s]+)\)')) {
        $t = $m.Groups[1].Value
        if ($t -match '^(https?://|mailto:|#)') { continue }
        if ($t -match '^(README\.md|README\.en\.md)(#.*)?$') { continue }
        if ($t -match '\.(png|jpe?g|gif|svg|ico)$') { continue }
        [void]$set.Add(($t -replace '#.*$', ''))
    }
    return $set
}

$zh = Read-Text $zhPath
$en = Read-Text $enPath
$fail = 0

# 1) 标题结构
$zhH = (Get-HeadingDepths $zh) -join ','
$enH = (Get-HeadingDepths $en) -join ','
if ($zhH -cne $enH) {
    Write-Host "FAIL 标题结构不一致（按 # 数量表示层级）：" -ForegroundColor Red
    Write-Host ("     中文: {0}" -f $zhH) -ForegroundColor Yellow
    Write-Host ("     英文: {0}" -f $enH) -ForegroundColor Yellow
    Write-Host "     改了一边的小节，另一边要同步加/删同样的标题（文字内容可以语言不同）。" -ForegroundColor Yellow
    $fail++
} else {
    Write-Host ("OK   标题结构一致（{0} 个标题）" -f (($zhH -split ',').Count)) -ForegroundColor Green
}

# 2) 站内相对链接
$zhL = Get-RelativeLinks $zh
$enL = Get-RelativeLinks $en
$missingInEn = @($zhL | Where-Object { -not $enL.Contains($_) } | Sort-Object)
$missingInZh = @($enL | Where-Object { -not $zhL.Contains($_) } | Sort-Object)
if ($missingInEn.Count -gt 0 -or $missingInZh.Count -gt 0) {
    if ($missingInEn.Count -gt 0) {
        Write-Host "FAIL 英文版缺少这些链接：" -ForegroundColor Red
        $missingInEn | ForEach-Object { Write-Host "       - $_" -ForegroundColor Yellow }
    }
    if ($missingInZh.Count -gt 0) {
        Write-Host "FAIL 中文版缺少这些链接：" -ForegroundColor Red
        $missingInZh | ForEach-Object { Write-Host "       - $_" -ForegroundColor Yellow }
    }
    $fail++
} else {
    Write-Host ("OK   站内链接一致（{0} 个）" -f $zhL.Count) -ForegroundColor Green
}

# 3) 语言切换互链（最容易在改版时被删掉）
if ($zh -notmatch 'README\.en\.md') {
    Write-Host "FAIL 中文版顶部缺少 [English](README.en.md) 切换链接" -ForegroundColor Red; $fail++
}
if ($en -notmatch 'README\.md') {
    Write-Host "FAIL 英文版顶部缺少 [中文](README.md) 切换链接" -ForegroundColor Red; $fail++
}

if ($fail -gt 0) { Write-Host "FAIL README 双语检查未通过" -ForegroundColor Red; exit 1 }
Write-Host "OK   README 双语检查通过" -ForegroundColor Green
