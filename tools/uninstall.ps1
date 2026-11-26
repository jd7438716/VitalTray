# BatteryHub 卸载脚本：退出进程 / 删启动项 / 删快捷方式与安装目录。
# 用法：powershell -ExecutionPolicy Bypass -File tools\uninstall.ps1
$dest = Join-Path $env:LOCALAPPDATA 'Programs\BatteryHub'
Get-Process batteryhub -ErrorAction SilentlyContinue | Stop-Process -Force
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Remove-ItemProperty -Path $run -Name 'BatteryHub' -ErrorAction SilentlyContinue
$lnk = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\BatteryHub.lnk'
Remove-Item $lnk -ErrorAction SilentlyContinue
Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue
$cfg = Join-Path $env:LOCALAPPDATA 'BatteryHub'
if (Test-Path $cfg) {
    $answer = Read-Host "删除配置与历史记录 ($cfg)? (y/N)"
    if ($answer -eq 'y') { Remove-Item $cfg -Recurse -Force }
}
Write-Host "已卸载。" -ForegroundColor Green
