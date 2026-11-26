# BatteryHub 安装脚本：复制 exe / 建快捷方式 / 写开机自启（仅 HKCU）。
# 用法：powershell -ExecutionPolicy Bypass -File tools\install.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\batteryhub.exe'
if (-not (Test-Path $exe)) {
    Write-Host "先构建：cargo build --release" -ForegroundColor Yellow
    exit 1
}
$dest = Join-Path $env:LOCALAPPDATA 'Programs\BatteryHub'
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item $exe (Join-Path $dest 'batteryhub.exe') -Force

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$lnk = Join-Path $startMenu 'BatteryHub.lnk'
$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut($lnk)
$sc.TargetPath = Join-Path $dest 'batteryhub.exe'
$sc.WorkingDirectory = $dest
$sc.Description = 'BatteryHub — 任务栏无线设备电量'
$sc.Save()

$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Set-ItemProperty -Path $run -Name 'BatteryHub' -Value ('"' + (Join-Path $dest 'batteryhub.exe') + '" --minimized')

Start-Process -FilePath (Join-Path $dest 'batteryhub.exe')
Write-Host "已安装到 $dest，并已设置开机自启。" -ForegroundColor Green
