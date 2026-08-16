# 确保本机/CI 具备编译安装包所需的 Inno Setup 环境。
#
# 为什么需要这个脚本：
#   ZoneDeck.iss 声明了简繁中文，而这两个语言包从 Inno Setup 7.0 起才随官方安装包分发
#   （7.0 更新日志：“Added official Lithuanian, Simplified Chinese and Traditional
#   Chinese translations.”）。GitHub runner 预装的却是 Inno Setup 6.x，编译会因缺少 .isl
#   失败，所以这里统一装 7.x，顺带保证本地和 CI 用的是同一套环境，不随 runner 镜像漂移。
#
# 装法：优先 winget，装不了就回退到直接下载官方安装包。
#   两条路取到的是同一个文件 —— winget 清单里 JRSoftware.InnoSetup.7 的 InstallerUrl
#   就是下面这个 GitHub release 链接。回退分支是为 CI 准备的：runner 镜像里没有 winget
#   （actions/runner-images#8584 至今未合），只有 winget 一条路的话发布流程会直接挂掉。
#
# 幂等：已装好可用的 Inno 就跳过。
#
# 用法：
#   powershell -File scripts/install-inno.ps1          # 安装 / 校验
#   powershell -File scripts/install-inno.ps1 -Quiet   # 静默，只输出 ISCC 路径
#
# 脚本最后一行输出 ISCC.exe 的完整路径，供调用方（scripts/package.ps1）取用。

param(
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue" # 进度条会让 Invoke-WebRequest 在 CI 里慢一个数量级

# 钉死版本：7 系最新稳定版
$innoVersion = "7.0.2"
$wingetId = "JRSoftware.InnoSetup.7"
$innoUrl = "https://github.com/jrsoftware/issrc/releases/download/is-7_0_2/innosetup-$innoVersion-x64.exe"
$minVersion = [version]"7.0"

# 与 ZoneDeck.iss 的 [Languages] 保持一致
$requiredLanguages = @("ChineseSimplified.isl", "ChineseTraditional.isl")

function Write-Info([string]$Message) {
    if (-not $Quiet) { Write-Host $Message -ForegroundColor Cyan }
}

function Get-VersionOrZero($Raw) {
    if ("$Raw" -match '(\d+(\.\d+)+)') { return [version]$Matches[1] }
    return [version]"0.0"
}
function Test-IsccUsable([string]$Path) {
    if (-not (Test-Path $Path)) { return $false }
    $langDir = Join-Path (Split-Path -Parent $Path) "Languages"
    foreach ($lang in $requiredLanguages) {
        if (-not (Test-Path (Join-Path $langDir $lang))) { return $false }
    }
    return $true
}

# 只认注册表里 >= 7.0 的安装。不扫 PATH、不认低版本目录里手工补的 .isl：
function Find-Iscc {
    $uninstallKeys = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )

    $candidates = @(
        Get-ItemProperty $uninstallKeys -ErrorAction SilentlyContinue |
            Where-Object {
                $_.DisplayName -like "Inno Setup*" -and $_.InstallLocation -and
                (Get-VersionOrZero $_.DisplayVersion) -ge $minVersion
            } |
            Sort-Object -Descending { Get-VersionOrZero $_.DisplayVersion } |
            ForEach-Object { Join-Path $_.InstallLocation "ISCC.exe" }
    )
    $candidates += "$env:ProgramFiles\Inno Setup 7\ISCC.exe"
    $candidates += "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe"

    foreach ($path in $candidates) {
        if (Test-IsccUsable $path) { return $path }
    }
    return $null
}

function Install-ViaWinget {
    if (-not (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
        Write-Info "    没有 winget，改用直接下载"
        return $false
    }
    Write-Info "    winget install --id $wingetId ..."
    # winget 失败只当作“这条路走不通”
    $ErrorActionPreference = "Continue"

    $log = winget install --id $wingetId -e -s winget -h `
        --accept-package-agreements --accept-source-agreements 2>&1
    $code = $LASTEXITCODE
    $ErrorActionPreference = "Stop"
    if ($code -ne 0) {
        Write-Info "    winget 退出码 $code，改用直接下载："
        if (-not $Quiet) { $log | ForEach-Object { Write-Host "      $_" -ForegroundColor DarkGray } }
        return $false
    }
    return $true
}

function Install-ViaDownload {
    Write-Info "    下载 $innoUrl ..."
    $installer = Join-Path ([System.IO.Path]::GetTempPath()) "innosetup-$innoVersion.exe"
    Invoke-WebRequest -Uri $innoUrl -OutFile $installer
    Start-Process $installer -Wait -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"
    Remove-Item $installer -Force -ErrorAction SilentlyContinue
}

$iscc = Find-Iscc
if ($iscc) {
    Write-Info "==> 已有可用的 Inno Setup：$iscc"
} else {
    Write-Info "==> 安装 Inno Setup $innoVersion..."
    if (-not (Install-ViaWinget)) { Install-ViaDownload }

    $iscc = Find-Iscc
    if (-not $iscc) {
        throw "Inno Setup $innoVersion 安装完成后仍找不到带简繁中文语言包的 ISCC.exe"
    }
    Write-Info "    已安装：$iscc"
}

Write-Output $iscc
