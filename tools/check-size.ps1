# 文件大小门禁：任何 .rs 超过 400 行即失败；超过 300 行给警告。
# 排除 target/ 与 research/（研究用的第三方源码）。
param([int]$MaxLines = 400, [int]$WarnLines = 300)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$files = Get-ChildItem -Path $root -Recurse -Filter *.rs -File |
    Where-Object { $_.FullName -notlike '*\target\*' -and $_.FullName -notlike '*\research\*' }
$failed = 0
$warned = 0
foreach ($f in $files) {
    $lines = (Get-Content -LiteralPath $f.FullName | Measure-Object -Line).Lines
    $rel = $f.FullName.Substring($root.Length).TrimStart('\')
    if ($lines -gt $MaxLines) {
        Write-Host ("FAIL {0,5} lines  {1}" -f $lines, $rel) -ForegroundColor Red
        $failed++
    } elseif ($lines -gt $WarnLines) {
        Write-Host ("WARN {0,5} lines  {1}" -f $lines, $rel) -ForegroundColor Yellow
        $warned++
    }
}
Write-Host ""
Write-Host ("checked {0} files: {1} over limit, {2} warnings" -f $files.Count, $failed, $warned)
if ($failed -gt 0) { exit 1 }
