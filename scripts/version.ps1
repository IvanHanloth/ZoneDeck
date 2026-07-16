# Boss Key 版本号工具
#
# 版本号的唯一真源是 Cargo.toml 的 [workspace.package] version，
# 另外三处（tauri.conf.json、ui/package.json、Cargo.lock）必须与之一致。
# 发版流程（.github/workflows/tag.yml）先 apply 写入并提交，
# 构建流程（release.yml）再 check 校验，防止 tag 与代码里的版本号对不上。
#
# 用法：
#   powershell -File scripts/version.ps1 apply v3.0.1      # 写入四处文件
#   powershell -File scripts/version.ps1 check v3.0.1      # 校验四处与该 tag 一致，不一致则失败
#   powershell -File scripts/version.ps1 check             # 不给 tag 时以 Cargo.toml 为基准校验
#   powershell -File scripts/version.ps1 show              # 打印当前版本号
#
# 在 GitHub Actions 中运行时，会把 version / version4 / is_prerelease
# 追加写入 $GITHUB_OUTPUT，供后续步骤使用。

param(
    [Parameter(Mandatory)]
    [ValidateSet('apply', 'check', 'show')]
    [string]$Action,

    [string]$Tag
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$cargoToml = Join-Path $root "Cargo.toml"
$tauriConf = Join-Path $root "apps\config\src-tauri\tauri.conf.json"
$uiPackage = Join-Path $root "apps\config\ui\package.json"

# 统一用 .NET 读写：Windows PowerShell 5.1 的 Get-Content/Set-Content 按 ANSI 处理，
# 会把文件里的中文写成乱码。写回一律 UTF-8 无 BOM —— BOM 会让 cargo / node 解析失败。
function Read-TextFile([string]$Path) {
    return [System.IO.File]::ReadAllText($Path)
}

function Write-TextFile([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText($Path, $Content, (New-Object System.Text.UTF8Encoding($false)))
}

# 归一化标签：去掉前导 v，允许旧式四段号（末段必须为 0），产出标准 semver
function Get-NormalizedVersion([string]$RawTag) {
    if (-not $RawTag) { throw "缺少版本号参数（例如 v3.0.1 或 3.1.0-rc.1）" }

    $v = $RawTag.Trim().TrimStart('v', 'V')

    # 旧式四段号 3.0.1.0 → 3.0.1；末段非 0 时无法表示为 semver，直接报错
    if ($v -match '^(\d+\.\d+\.\d+)\.(\d+)$') {
        if ($Matches[2] -ne '0') {
            throw "不支持四段版本号 $v（末段非 0）。请使用 semver，例如 3.0.1 或 3.1.0-rc.1"
        }
        $v = $Matches[1]
    }

    if ($v -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
        throw "版本号格式非法：$RawTag（应形如 3.0.1 或 3.1.0-rc.1）"
    }

    return $v
}

# Windows 安装包要求纯数字四段号：3.1.0-rc.1 → 3.1.0.0
function Get-InstallerVersion([string]$Version) {
    $core = ($Version -split '-')[0]
    return "$core.0"
}

# 从 Cargo.toml 的 [workspace.package] 读取当前版本号
function Get-CurrentVersion {
    $content = Read-TextFile $cargoToml
    if ($content -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
        throw "无法从 Cargo.toml 的 [workspace.package] 读取 version"
    }
    return $Matches[1]
}

function Set-CargoVersion([string]$Version) {
    $content = Read-TextFile $cargoToml
    $updated = [regex]::Replace(
        $content,
        '(?ms)(^\[workspace\.package\].*?^version\s*=\s*")[^"]+(")',
        { param($m) "$($m.Groups[1].Value)$Version$($m.Groups[2].Value)" },
        1)
    Write-TextFile $cargoToml $updated
}

function Set-JsonVersion([string]$Path, [string]$Version) {
    $content = Read-TextFile $Path
    # 只改顶层第一处 "version": "..."，避免动到依赖项里的版本约束
    $updated = [regex]::Replace(
        $content,
        '(?m)^(\s*"version"\s*:\s*")[^"]+(")',
        { param($m) "$($m.Groups[1].Value)$Version$($m.Groups[2].Value)" },
        1)
    Write-TextFile $Path $updated
}

function Get-JsonVersion([string]$Path) {
    return (Read-TextFile $Path | ConvertFrom-Json).version
}

# 把版本号回填到 Cargo.lock（workspace 成员），保持 lock 与 Cargo.toml 同步
function Update-CargoLock {
    cargo update --workspace --quiet
    if ($LASTEXITCODE -ne 0) { throw "cargo update --workspace 失败" }
}

function Export-Outputs([string]$Version) {
    $version4 = Get-InstallerVersion $Version
    $isPrerelease = if ($Version -match '-') { 'true' } else { 'false' }

    Write-Host "版本号：$Version（安装包 $version4，预发布 $isPrerelease）"

    if ($env:GITHUB_OUTPUT) {
        @(
            "version=$Version",
            "version4=$version4",
            "is_prerelease=$isPrerelease"
        ) | Out-File -FilePath $env:GITHUB_OUTPUT -Append -Encoding utf8
    }
}

switch ($Action) {
    'show' {
        Export-Outputs (Get-CurrentVersion)
    }

    'apply' {
        $version = Get-NormalizedVersion $Tag
        Set-CargoVersion $version
        Set-JsonVersion $tauriConf $version
        Set-JsonVersion $uiPackage $version
        Update-CargoLock
        Write-Host "已写入 Cargo.toml / tauri.conf.json / package.json / Cargo.lock"
        Export-Outputs $version
    }

    'check' {
        # 不给 tag 时，以 Cargo.toml 为基准校验其余文件是否跟得上
        $version = if ($Tag) { Get-NormalizedVersion $Tag } else { Get-CurrentVersion }
        $actual = @{
            'Cargo.toml'      = Get-CurrentVersion
            'tauri.conf.json' = Get-JsonVersion $tauriConf
            'package.json'    = Get-JsonVersion $uiPackage
        }

        $bad = $actual.GetEnumerator() | Where-Object { $_.Value -ne $version }
        if ($bad) {
            foreach ($entry in $bad) {
                Write-Host "::error::$($entry.Key) 的版本号是 $($entry.Value)，与目标 $version 不符"
            }
            throw "版本号校验失败：请先运行 scripts/version.ps1 apply $version 并提交"
        }

        Write-Host "版本号校验通过：三处文件均为 $version"
        Export-Outputs $version
    }
}
