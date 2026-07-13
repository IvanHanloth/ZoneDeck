# Boss Key v3 生产打包脚本
# 生产编译并把可直接分发的独立文件夹组装到 package/Boss-Key
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "==> 生产编译（cargo build --release）..." -ForegroundColor Cyan
cargo build --release

$outDir = Join-Path $root "package\Boss-Key"
if (Test-Path $outDir) { Remove-Item $outDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Copy-Item "target\release\bosskey-core.exe" $outDir
Copy-Item "target\release\bosskey-config.exe" $outDir
Copy-Item "icon.ico" $outDir

@"
Boss Key v3 便携版
==================
文件说明：
- bosskey-core.exe    常驻核心（后台运行，右下角托盘图标）。双击启动。
- bosskey-config.exe  配置界面。也可从托盘图标 -> 设置 打开。
- icon.ico            托盘图标。

首次使用：
1. 双击 bosskey-core.exe，任务栏右下角出现盾牌托盘图标即表示已运行。
2. 右键托盘图标 -> 设置，或双击 bosskey-config.exe 打开配置界面。
3. 绑定要隐藏的窗口，设置热键，点击“保存设置”。

进阶：
- 增强冻结：将 Microsoft PSTools 中的 pssuspend64.exe 放入本目录，
  并在配置界面“管理员权限”中点击“以管理员身份重启核心”。
- 开机自启：配置界面右下角“开机自启”，或托盘菜单“开机自启”。

系统要求：Windows 10 及以上，且已安装 Microsoft Edge WebView2 运行时
（Windows 10/11 通常已内置；仅配置界面需要）。
"@ | Set-Content -Path (Join-Path $outDir "使用说明.txt") -Encoding UTF8

Write-Host "==> 打包完成：$outDir" -ForegroundColor Green
Get-ChildItem $outDir | Select-Object Name, @{Name = "Size"; Expression = { "{0:N0} KB" -f ($_.Length / 1KB) } } | Format-Table -AutoSize
