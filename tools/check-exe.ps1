# exe 资源门禁：图标 / 版本信息 / 应用清单，缺一项就失败。
#
# 起因：v1.0/v1.1 发出去的 exe 里一个资源都没有——资源管理器显示通用白窗图标、
# 属性页一片空白、原生菜单走经典样式，发布流水线却毫无察觉。这个脚本就是那次漏检的补丁。
#
# 用法：./tools/check-exe.ps1 [exe 路径，默认 target/release/batteryhub.exe]
param([string]$Path = "target/release/batteryhub.exe")
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
if (-not [System.IO.Path]::IsPathRooted($Path)) { $Path = Join-Path $root $Path }
if (-not (Test-Path -LiteralPath $Path)) { throw "找不到 exe：$Path" }

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Path).Path)
$file = [System.IO.Path]::GetFileName($Path)
$typeNames = @{
    3 = 'ICON'; 6 = 'STRING'; 10 = 'RCDATA'; 14 = 'GROUP_ICON'; 16 = 'VERSION'; 24 = 'MANIFEST'
}

function Read-U16([int]$o) { [BitConverter]::ToUInt16($bytes, $o) }
function Read-U32([int]$o) { [BitConverter]::ToUInt32($bytes, $o) }

# ---- PE 头 → 资源目录 RVA ----
$e = [int](Read-U32 0x3C)
if ($bytes[$e] -ne 0x50 -or $bytes[$e + 1] -ne 0x45) { throw "$file 不是 PE 文件" }
$coff = $e + 4
$sections = Read-U16 ($coff + 2)
$optSize = Read-U16 ($coff + 16)
$opt = $coff + 20
$magic = Read-U16 $opt
$dd = $opt + $(if ($magic -eq 0x20B) { 112 } else { 96 })
$rsrcRva = Read-U32 ($dd + 16)
$rsrcSize = Read-U32 ($dd + 20)

# ---- RVA → 文件偏移 ----
$secTable = $opt + $optSize
function Convert-Rva([uint32]$rva) {
    for ($i = 0; $i -lt $sections; $i++) {
        $s = $secTable + $i * 40
        $va = Read-U32 ($s + 12)
        $raw = Read-U32 ($s + 20)
        $rawSize = Read-U32 ($s + 8)
        if ($rva -ge $va -and $rva -lt ($va + [Math]::Max($rawSize, 1))) { return [int]($raw + ($rva - $va)) }
    }
    throw "RVA $rva 不在任何节里"
}

# ---- 列资源类型 ----
$types = @{}
if ($rsrcSize -gt 0 -and $rsrcRva -ne 0) {
    $base = Convert-Rva $rsrcRva
    $count = (Read-U16 ($base + 12)) + (Read-U16 ($base + 14))
    for ($i = 0; $i -lt $count; $i++) {
        $name = Read-U32 ($base + 16 + $i * 8)
        if ($name -band 0x80000000) {
            $no = $base + [int]($name -band 0x7FFFFFFF)
            $len = Read-U16 $no
            $text = [System.Text.Encoding]::Unicode.GetString($bytes, $no + 2, $len * 2)
            $types[$text] = $true
        } else {
            $key = if ($typeNames.ContainsKey([int]$name)) { $typeNames[[int]$name] } else { "TYPE#$name" }
            $types[$key] = $true
        }
    }
}

$found = if ($types.Count) { ($types.Keys | Sort-Object) -join ', ' } else { '（无）' }
Write-Host "exe            : $file"
Write-Host "资源目录       : rva=$('0x{0:X}' -f $rsrcRva) size=$rsrcSize"
Write-Host "资源类型       : $found"

# ---- 断言 ----
$checks = @(
    @{ Name = '分组图标 ICON'; Ok = ($types.ContainsKey('GROUP_ICON') -or $types.ContainsKey('ICON')); Hint = '资源管理器/任务栏会显示通用白窗图标' },
    @{ Name = '版本信息 VERSION'; Ok = $types.ContainsKey('VERSION'); Hint = '属性页无版本/产品名，进程列表里只有文件名' },
    @{ Name = '应用清单 MANIFEST'; Ok = $types.ContainsKey('MANIFEST'); Hint = '菜单与对话框退回经典样式，且没有 DPI/OS 声明' }
)
$vi = (Get-Item -LiteralPath $Path).VersionInfo
$checks += @{ Name = 'FileVersion'; Ok = -not [string]::IsNullOrWhiteSpace($vi.FileVersion); Hint = '版本号空' }
$checks += @{ Name = 'ProductName'; Ok = -not [string]::IsNullOrWhiteSpace($vi.ProductName); Hint = '产品名空' }

Write-Host ""
$failed = 0
foreach ($c in $checks) {
    if ($c.Ok) {
        Write-Host ("PASS  {0}" -f $c.Name) -ForegroundColor Green
    } else {
        Write-Host ("FAIL  {0}  —— {1}" -f $c.Name, $c.Hint) -ForegroundColor Red
        $failed++
    }
}

Write-Host ""
Write-Host ("{0}：{1} 项失败" -f $file, $failed) -ForegroundColor $(if ($failed) { 'Red' } else { 'Green' })
if ($failed -gt 0) { exit 1 }
