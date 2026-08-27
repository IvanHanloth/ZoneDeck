# ZoneDeck 一键生产打包脚本
# 流程：编译前端（Vite + Svelte）→ 生产编译 Rust workspace → 组装便携文件夹 dist\ZoneDeck
#      → 可选生成 InnoSetup 安装包 dist\installer（-Installer）
#
# 用法：
#   powershell -File scripts/package.ps1               # 便携文件夹
#   powershell -File scripts/package.ps1 -Installer    # 便携文件夹 + 安装包
#   powershell -File scripts/package.ps1 -SkipFrontend # 复用已有 dist（前端没改时提速）
#
# 版本号默认取自 Cargo.toml（唯一真源，见 scripts/version.ps1），无需手动传 -Version。
param(
    [switch]$Installer,
    [switch]$SkipFrontend,
    [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $Version) {
    $cargoToml = Get-Content (Join-Path $root "Cargo.toml") -Raw
    if ($cargoToml -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
        throw "无法从 Cargo.toml 读取版本号，请显式传入 -Version"
    }
    $Version = $Matches[1]
}
# 安装包资源信息要求纯数字四段号：3.1.0-rc.1 → 3.1.0.0
$Version4 = "$(($Version -split '-')[0]).0"

# 1. 前端
$uiDir = Join-Path $root "apps\config\ui"
if (-not $SkipFrontend) {
    Write-Host "==> 编译前端（Vite + Svelte）..." -ForegroundColor Cyan
    if (-not (Test-Path (Join-Path $uiDir "node_modules"))) {
        Write-Host "    首次构建，安装依赖（pnpm install）..."
        pnpm --dir $uiDir install
        if ($LASTEXITCODE -ne 0) { throw "pnpm install 失败" }
    }
    pnpm --dir $uiDir run build
    if ($LASTEXITCODE -ne 0) { throw "前端构建失败" }
}
if (-not (Test-Path (Join-Path $root "apps\config\dist\index.html"))) {
    throw "缺少前端产物 apps/config/dist（可去掉 -SkipFrontend 重新构建）"
}

# 2. Rust 生产编译（zonedeck-config 的 tauri 构建脚本会内嵌 dist）
Write-Host "==> 生产编译（cargo build --release）..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo 编译失败" }

# 3. 组装便携文件夹（dist\ZoneDeck）
# 发布时整个文件夹直接压成 zip，所以解压后就是一个 ZoneDeck 目录，不会散落到当前目录。
$portableDir = Join-Path $root "dist\ZoneDeck"
if (Test-Path $portableDir) { Remove-Item $portableDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $portableDir | Out-Null

Copy-Item "target\release\core.exe" (Join-Path $portableDir "ZoneDeck.exe")
Copy-Item "target\release\zonedeck-config.exe" (Join-Path $portableDir "config.exe")
# LICENSE 带上 .txt 后缀：Windows 上双击才有默认打开方式；安装包的许可协议页也复用这份
Copy-Item "LICENSE" (Join-Path $portableDir "LICENSE.txt")
# 三语 README 全带上：便携版没有安装向导，README 是唯一的随包说明，
# 其中「清理残留数据」一节交代了程序在用户目录下留了什么。
Copy-Item "README.md" (Join-Path $portableDir "README.md")
Copy-Item "README.en.md" (Join-Path $portableDir "README.en.md")
Copy-Item "README.zh-TW.md" (Join-Path $portableDir "README.zh-TW.md")
# 便携版没有卸载程序，用户目录下的数据得靠它清
Copy-Item "scripts\cleanup.ps1" (Join-Path $portableDir "cleanup.ps1")

Write-Host "==> 便携版组装完成：$portableDir" -ForegroundColor Green
Get-ChildItem $portableDir | Select-Object Name, @{Name = "Size"; Expression = { "{0:N0} KB" -f ($_.Length / 1KB) } } | Format-Table -AutoSize

# 4. 安装包（可选，输出到 dist\installer，与便携版分开）
if ($Installer) {
    Write-Host "==> 生成 InnoSetup 安装包..." -ForegroundColor Cyan
    # 按需安装 Inno Setup 7（简繁中文语言包自 7.0 起随官方安装包分发），返回 ISCC.exe 路径
    $iscc = & (Join-Path $PSScriptRoot "install-inno.ps1") | Select-Object -Last 1
    if (-not $iscc) { throw "Inno Setup 环境准备失败" }

    $installerDir = Join-Path $root "dist\installer"
    if (Test-Path $installerDir) { Remove-Item $installerDir -Recurse -Force }

    & $iscc "/DMyAppVersion=$Version" "/DMyAppVersion4=$Version4" ".github\inno-script\ZoneDeck.iss"
    if ($LASTEXITCODE -ne 0) { throw "InnoSetup 编译失败" }
    Write-Host "==> 安装包输出：$installerDir" -ForegroundColor Green
    Get-ChildItem $installerDir
}
