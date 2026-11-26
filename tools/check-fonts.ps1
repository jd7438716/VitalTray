# 字体门禁：仓库里不得出现字体文件（.ttf/.ttc/.otf/.woff/.woff2/.eot）。
#
# 为什么：界面只把「字体家族名」交给 GDI，字形由 Windows 从 %WINDIR%\Fonts 提供，程序不复制、
# 不分发字体文件（见 docs/licenses.md §6）。系统字体与绝大多数商业字体都禁止再分发，
# 一旦把 .ttf/.ttc 塞进仓库或安装包，就从"合法使用"变成"分发受版权保护的字体软件"。
#
# 要自带字体时（例如将来做非 Windows 构建）：只选 SIL OFL / Apache-2.0 等允许再分发的字体，
# 许可全文进 THIRD-PARTY-NOTICES.txt，并同步更新 docs/licenses.md §6。
#
# 用法：pwsh ./tools/check-fonts.ps1（CI 会跑）
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$pattern = '\.(ttf|ttc|otf|woff2?|eot)$'

# 只看 git 里的文件：research/ 等本地素材目录不入库，也不该被本门禁打扰。
$files = @(git -C $root ls-files --cached | Where-Object { $_ -match $pattern })

if ($files.Count -gt 0) {
    Write-Host "FAIL 仓库里有字体文件（不允许入库）：" -ForegroundColor Red
    $files | ForEach-Object { Write-Host "  $_" }
    Write-Host ""
    Write-Host "界面只用系统字体（只引用名字）。确需自带字体：先确认许可（OFL / Apache-2.0），" -ForegroundColor Yellow
    Write-Host "更新 docs/licenses.md §6 与 THIRD-PARTY-NOTICES.txt，再把文件提交。" -ForegroundColor Yellow
    exit 1
}

Write-Host "OK  仓库无字体文件（界面只用系统字体）" -ForegroundColor Green
