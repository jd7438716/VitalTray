# VitalTray 安装脚本：复制 exe / 建快捷方式 / 写开机自启（仅 HKCU）。
# 用法：powershell -ExecutionPolicy Bypass -File tools\install.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\vitaltray.exe'
if (-not (Test-Path $exe)) {
    Write-Host "先构建：cargo build --release" -ForegroundColor Yellow
    exit 1
}

# 升级路径：旧名 BatteryHub 的进程 / 启动项 / 快捷方式 / 安装目录先收掉，
# 否则托盘会出现两个图标，开机还会多一条指向旧 exe 的自启项。
Get-Process batteryhub -ErrorAction SilentlyContinue | Stop-Process -Force
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Remove-ItemProperty -Path $run -Name 'BatteryHub' -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\BatteryHub.lnk') -ErrorAction SilentlyContinue
$legacyDest = Join-Path $env:LOCALAPPDATA 'Programs\BatteryHub'
if (Test-Path $legacyDest) { Remove-Item $legacyDest -Recurse -Force -ErrorAction SilentlyContinue }

$dest = Join-Path $env:LOCALAPPDATA 'Programs\VitalTray'
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item $exe (Join-Path $dest 'vitaltray.exe') -Force

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$lnk = Join-Path $startMenu 'VitalTray.lnk'
$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut($lnk)
$sc.TargetPath = Join-Path $dest 'vitaltray.exe'
$sc.WorkingDirectory = $dest
$sc.Description = 'VitalTray — 托盘无线外设电量与整机温度面板'
$sc.Save()

Set-ItemProperty -Path $run -Name 'VitalTray' -Value ('"' + (Join-Path $dest 'vitaltray.exe') + '" --minimized')

Start-Process -FilePath (Join-Path $dest 'vitaltray.exe')
Write-Host "已安装到 $dest，并已设置开机自启。" -ForegroundColor Green
Write-Host "旧版 %LOCALAPPDATA%\BatteryHub 的配置会在首次启动时自动复制到新目录（旧目录不删）。" -ForegroundColor DarkGray
