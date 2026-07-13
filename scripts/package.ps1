# Boss Key v3 一键生产打包脚本
# 流程：编译前端（Vite + Svelte）→ 生产编译 Rust workspace → 组装便携文件夹
#      → 可选生成 InnoSetup 安装包（-Installer）
#
# 用法：
#   powershell -File scripts/package.ps1               # 便携文件夹
#   powershell -File scripts/package.ps1 -Installer    # 便携文件夹 + 安装包
#   powershell -File scripts/package.ps1 -SkipFrontend # 复用已有 dist（前端没改时提速）
param(
    [switch]$Installer,
    [switch]$SkipFrontend,
    [string]$Version = "3.0.0.0"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# 1. 前端
$uiDir = Join-Path $root "apps\config\ui"
if (-not $SkipFrontend) {
    Write-Host "==> 编译前端（Vite + Svelte）..." -ForegroundColor Cyan
    if (-not (Test-Path (Join-Path $uiDir "node_modules"))) {
        Write-Host "    首次构建，安装依赖（npm install）..."
        npm --prefix $uiDir install --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) { throw "npm install 失败" }
    }
    npm --prefix $uiDir run build
    if ($LASTEXITCODE -ne 0) { throw "前端构建失败" }
}
if (-not (Test-Path (Join-Path $root "apps\config\dist\index.html"))) {
    throw "缺少前端产物 apps/config/dist（可去掉 -SkipFrontend 重新构建）"
}

# 2. Rust 生产编译（bosskey-config 的 tauri 构建脚本会内嵌 dist）
Write-Host "==> 生产编译（cargo build --release）..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo 编译失败" }

# 3. 组装便携文件夹
$outDir = Join-Path $root "package\Boss-Key"
if (Test-Path $outDir) { Remove-Item $outDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Copy-Item "target\release\bosskey-core.exe" $outDir
Copy-Item "target\release\bosskey-config.exe" $outDir
Copy-Item "icon.ico" $outDir

Write-Host "==> 便携版组装完成：$outDir" -ForegroundColor Green
Get-ChildItem $outDir | Select-Object Name, @{Name = "Size"; Expression = { "{0:N0} KB" -f ($_.Length / 1KB) } } | Format-Table -AutoSize

# 4. 安装包（可选）
if ($Installer) {
    Write-Host "==> 生成 InnoSetup 安装包..." -ForegroundColor Cyan
    $iscc = Get-Command iscc.exe -ErrorAction SilentlyContinue
    if (-not $iscc) {
        $candidates = @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
        )
        $iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    } else {
        $iscc = $iscc.Source
    }
    if (-not $iscc) { throw "未找到 ISCC.exe，请安装 Inno Setup 6（winget install JRSoftware.InnoSetup）" }

    & $iscc "/DMyAppVersion=$Version" ".github\inno-script\Boss-Key-v3.iss"
    if ($LASTEXITCODE -ne 0) { throw "InnoSetup 编译失败" }
    Write-Host "==> 安装包输出：package\installer" -ForegroundColor Green
    Get-ChildItem (Join-Path $root "package\installer")
}
