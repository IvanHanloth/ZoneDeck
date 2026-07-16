# 确保本机/CI 具备编译安装包所需的 Inno Setup 环境。
#
# 为什么需要这个脚本：
#   1. Inno Setup 的官方安装包里没有中文语言包，而 Boss-Key.iss 声明了简繁中文，
#      缺了 .isl 文件 ISCC 会直接编译失败（GitHub runner 预装的 Inno 同样没有）；
#   2. 顺带保证本地和 CI 用的是同一套 Inno 环境，不随 runner 镜像漂移。
#
# 两件事都是幂等的：装好了就跳过，只补缺的部分。
#
# 用法：
#   powershell -File scripts/install-inno.ps1          # 安装 / 补齐
#   powershell -File scripts/install-inno.ps1 -Quiet   # 静默，只输出 ISCC 路径
#
# 脚本最后一行输出 ISCC.exe 的完整路径，供调用方（scripts/package.ps1）取用。

param(
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue" # 进度条会让 Invoke-WebRequest 在 CI 里慢一个数量级

# 钉死版本：6 系最新稳定版。注意别升到 7.x —— Boss-Key.iss 用的是 IS6 语法。
$innoVersion = "6.7.3"
$innoUrl = "https://github.com/jrsoftware/issrc/releases/download/is-6_7_3/innosetup-$innoVersion.exe"

# 中文 .isl 不随安装包分发，从源码仓库取。两个文件都声明兼容 Inno Setup 6.5.0+，
# 因此可以配 6.7.3 使用；这里固定到一个不可变 tag，避免 main 分支改动影响构建。
$langRef = "is-7_0_2"
$langBaseUrl = "https://raw.githubusercontent.com/jrsoftware/issrc/$langRef/Files/Languages"
$languages = @("ChineseSimplified.isl", "ChineseTraditional.isl")

# 低于这个版本的 Inno 认不了上面的 .isl，得换成钉死的版本
$minVersion = [version]"6.5.0"

function Write-Info([string]$Message) {
    if (-not $Quiet) { Write-Host $Message -ForegroundColor Cyan }
}

function Find-Iscc {
    $cmd = Get-Command iscc.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $candidates = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    return $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}

function Get-IsccVersion([string]$Path) {
    $raw = (Get-Item $Path).VersionInfo.ProductVersion
    if ($raw -match '(\d+\.\d+(\.\d+)?)') { return [version]$Matches[1] }
    return [version]"0.0"
}

# 1. Inno Setup 本体
$iscc = Find-Iscc
if ($iscc) {
    $existing = Get-IsccVersion $iscc
    if ($existing -lt $minVersion) {
        Write-Info "==> 已装的 Inno Setup $existing 过旧（中文语言包要求 $minVersion+），改装 $innoVersion"
        $iscc = $null
    } else {
        Write-Info "==> 已有 Inno Setup $existing：$iscc"
    }
}

if (-not $iscc) {
    Write-Info "==> 下载并安装 Inno Setup $innoVersion..."
    $installer = Join-Path ([System.IO.Path]::GetTempPath()) "innosetup-$innoVersion.exe"
    Invoke-WebRequest -Uri $innoUrl -OutFile $installer
    Start-Process $installer -Wait -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"
    Remove-Item $installer -Force -ErrorAction SilentlyContinue

    $iscc = Find-Iscc
    if (-not $iscc) { throw "Inno Setup 安装完成后仍找不到 ISCC.exe" }
    Write-Info "    已安装：$iscc"
}

# 2. 中文语言包
$langDir = Join-Path (Split-Path -Parent $iscc) "Languages"
if (-not (Test-Path $langDir)) { New-Item -ItemType Directory -Force -Path $langDir | Out-Null }

foreach ($lang in $languages) {
    $target = Join-Path $langDir $lang
    if (Test-Path $target) {
        Write-Info "    语言包已就位：$lang"
        continue
    }
    Write-Info "    下载语言包：$lang"
    Invoke-WebRequest -Uri "$langBaseUrl/$lang" -OutFile $target
}

Write-Output $iscc
