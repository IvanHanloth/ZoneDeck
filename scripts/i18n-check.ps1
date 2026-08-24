# ZoneDeck i18n 一致性检查
#
# 覆盖单元测试够不着的部分：测试只能校验 catalog 内部（键集 / 空值 / 占位符），
# 校验不了"代码引用的键是否存在""文档三语是否配套"这类跨文件的事情。
#
# 检查项：
#   1. 文档三语页面集一致（docs/ 为简中，docs/en/、docs/zh-tw/ 必须同名同构）
#   2. 文档站内链不跨语言（英文页链到 /guide/… 会把读者甩回中文页，VitePress 查不出来）
#   3. 前端 t("键") 引用的键在 zh-CN.js 中存在（缺失时界面直接显示原始键名）
#   4. 前端 catalog 无死键（删功能时漏删的残留文案）
#   5. 核心 Msg 枚举变体全部登记进测试的 ALL_MSGS（漏登记会让该条文案跳过跨语言校验）
#   6. 三份 catalog 的键集 / 空值 / 占位符 —— 交给既有的 vitest 用例，不重复实现
#
# 用法：
#   pwsh -File scripts/i18n-check.ps1              # 全量检查
#   pwsh -File scripts/i18n-check.ps1 -Staged      # 只在相关文件进暂存区时检查（pre-commit 钩子用）
#   pwsh -File scripts/i18n-check.ps1 -SkipVitest  # 跳过 catalog 单测（无 node_modules 的环境）

param(
    [switch]$Staged,
    [switch]$SkipVitest
)

$ErrorActionPreference = "Stop"
# 控制台按 UTF-8 输出，否则中文提示在 Git Bash / CI 里是乱码
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$root = Split-Path -Parent $PSScriptRoot
$docs = Join-Path $root "docs"
$ui = Join-Path $root "apps\config\ui"
$locales = Join-Path $ui "src\locales"

# 文档语言目录：简中在 docs/ 根下，另两种语言各占一个子目录
$docRoots = @{ "zh-CN" = $docs; "en" = (Join-Path $docs "en"); "zh-TW" = (Join-Path $docs "zh-tw") }
# 三语共有的顶层内容目录；.vitepress / public 等非内容目录不参与比对
$docSections = @("guide", "dev", "changelog")

$problems = New-Object System.Collections.Generic.List[string]

function Add-Problem([string]$Message) {
    $problems.Add($Message)
    Write-Host "::error::$Message"
}

# 统一用 .NET 读文件：Windows PowerShell 5.1 的 Get-Content 按 ANSI 处理会把中文读成乱码
function Read-TextFile([string]$Path) {
    return [System.IO.File]::ReadAllText($Path)
}

# 暂存区里的文件（相对仓库根、正斜杠）。-Staged 模式下据此决定跳过哪些检查。
function Get-StagedFiles {
    $out = git -C $root diff --cached --name-only --diff-filter=ACMR
    if ($LASTEXITCODE -ne 0) { throw "读取暂存区失败" }
    return @($out | Where-Object { $_ })
}

# 变量名不能写成 $staged：PowerShell 变量名不区分大小写，会把开关参数 $Staged 覆盖掉
$stagedFiles = if ($Staged) { Get-StagedFiles } else { @() }

# -Staged 模式下：暂存区里没有匹配 $Pattern 的文件就跳过该项检查
function Test-ShouldCheck([string]$Pattern) {
    if (-not $Staged) { return $true }
    return [bool]($stagedFiles | Where-Object { $_ -like $Pattern })
}

# ---- 1. 文档三语页面集一致 ----------------------------------------------------
function Test-DocParity {
    $sets = @{}
    foreach ($lang in $docRoots.Keys) {
        $base = $docRoots[$lang]
        $pages = New-Object System.Collections.Generic.List[string]
        foreach ($section in $docSections) {
            $dir = Join-Path $base $section
            if (-not (Test-Path $dir)) { continue }
            Get-ChildItem $dir -Recurse -File -Include *.md, *.ts | ForEach-Object {
                $pages.Add($_.FullName.Substring($base.Length + 1).Replace('\', '/'))
            }
        }
        if (Test-Path (Join-Path $base "index.md")) { $pages.Add("index.md") }
        $sets[$lang] = $pages | Sort-Object
    }

    foreach ($lang in @("en", "zh-TW")) {
        $missing = $sets["zh-CN"] | Where-Object { $_ -notin $sets[$lang] }
        $extra = $sets[$lang] | Where-Object { $_ -notin $sets["zh-CN"] }
        foreach ($p in $missing) { Add-Problem "文档缺 $lang 版：$($docRoots[$lang] | Split-Path -Leaf)/$p（简中已有 docs/$p）" }
        foreach ($p in $extra) { Add-Problem "文档多出 $lang 版：$p 在简中没有对应页面" }
    }
}

# ---- 2. 站内链接不跨语言 ------------------------------------------------------
# 英文 / 繁中页面必须链到自己语言的路径；简中页面反之不得链到 /en/、/zh-tw/。
function Test-DocLinks {
    $expect = @{
        "en"    = @{ Dir = "en"; Bad = '\]\(/(guide|dev|changelog)/'; Hint = "应加 /en/ 前缀" }
        "zh-TW" = @{ Dir = "zh-tw"; Bad = '\]\(/(guide|dev|changelog)/'; Hint = "应加 /zh-tw/ 前缀" }
    }
    foreach ($lang in $expect.Keys) {
        $rule = $expect[$lang]
        $dir = Join-Path $docs $rule.Dir
        if (-not (Test-Path $dir)) { continue }
        Get-ChildItem $dir -Recurse -File -Filter *.md | ForEach-Object {
            $lines = (Read-TextFile $_.FullName) -split "`r?`n"
            for ($i = 0; $i -lt $lines.Count; $i++) {
                if ($lines[$i] -match $rule.Bad) {
                    $rel = $_.FullName.Substring($root.Length + 1).Replace('\', '/')
                    Add-Problem "$rel :$($i + 1) 链接指向简中页面（$($rule.Hint)）：$($Matches[0])…"
                }
            }
        }
    }

    # 简中页面链到其他语言，同样是串页
    foreach ($section in $docSections + @(".")) {
        $dir = Join-Path $docs $section
        if (-not (Test-Path $dir)) { continue }
        Get-ChildItem $dir -File -Recurse:($section -ne ".") -Filter *.md | ForEach-Object {
            $lines = (Read-TextFile $_.FullName) -split "`r?`n"
            for ($i = 0; $i -lt $lines.Count; $i++) {
                if ($lines[$i] -match '\]\(/(en|zh-tw)/') {
                    $rel = $_.FullName.Substring($root.Length + 1).Replace('\', '/')
                    Add-Problem "$rel :$($i + 1) 简中页面链到了 $($Matches[1]) 版页面"
                }
            }
        }
    }
}

# ---- 3 / 4. 前端 t() 引用与 catalog 互相覆盖 ----------------------------------
function Get-CatalogKeys {
    # 用 node 读 catalog：locale 文件是 ES module，值里有多行字符串，正则解析不可靠
    $keys = node --input-type=module -e "import c from './apps/config/ui/src/locales/zh-CN.js'; console.log(Object.keys(c).join('\n'))"
    if ($LASTEXITCODE -ne 0) { throw "读取 zh-CN.js 失败（node 是否可用？）" }
    return @($keys -split "`r?`n" | Where-Object { $_ })
}

function Test-CatalogUsage {
    $defined = Get-CatalogKeys
    # 测试文件里有故意的未知键（用于验证回落行为），不参与比对
    $sources = Get-ChildItem (Join-Path $ui "src") -Recurse -File -Include *.svelte, *.js |
        Where-Object { $_.Name -notlike "*.test.js" -and $_.Directory.Name -ne "locales" }

    # 缺键方向只认 t("字面量")：这是唯一能确定"这就是个文案键"的形式，不会误报
    $called = @{}
    # 死键方向认任意字符串字面量：键常经变量中转（labelKey: "tab.about"、
    # t(cond ? "a" : "b")），只按 t(...) 匹配会把它们全误判成无人引用
    $literals = New-Object System.Collections.Generic.HashSet[string]
    foreach ($file in $sources) {
        $text = Read-TextFile $file.FullName
        foreach ($m in [regex]::Matches($text, '\bt\(\s*"([^"]+)"')) {
            $called[$m.Groups[1].Value] = $file.Name
        }
        foreach ($m in [regex]::Matches($text, '"([^"\r\n]+)"')) {
            [void]$literals.Add($m.Groups[1].Value)
        }
    }

    foreach ($key in $called.Keys | Sort-Object) {
        if ($key -notin $defined) {
            Add-Problem "$($called[$key]) 用了未定义的文案键 `"$key`"（界面会直接显示键名）"
        }
    }
    foreach ($key in $defined | Sort-Object) {
        if (-not $literals.Contains($key)) {
            Add-Problem "文案键 `"$key`" 已无人引用，请从三份 catalog 一并删除"
        }
    }
}

# ---- 5. 核心 Msg 变体是否都登记进 ALL_MSGS ------------------------------------
function Test-RustMsgRegistration {
    $file = Join-Path $root "crates\core\src\i18n.rs"
    $text = Read-TextFile $file

    if ($text -notmatch '(?ms)pub enum Msg \{(.*?)^\}') { throw "未能在 i18n.rs 中找到 Msg 枚举" }
    $variants = [regex]::Matches($Matches[1], '(?m)^\s{4}(\w+),') | ForEach-Object { $_.Groups[1].Value }

    if ($text -notmatch '(?ms)const ALL_MSGS: \[Msg; (\d+)\] = \[(.*?)^\s*\];') { throw "未能在 i18n.rs 中找到 ALL_MSGS" }
    $declared = [int]$Matches[1]
    $registered = [regex]::Matches($Matches[2], 'Msg::(\w+)') | ForEach-Object { $_.Groups[1].Value }

    foreach ($v in $variants) {
        if ($v -notin $registered) { Add-Problem "Msg::$v 未登记进 ALL_MSGS，跨语言校验会漏掉它" }
    }
    foreach ($v in $registered) {
        if ($v -notin $variants) { Add-Problem "ALL_MSGS 里的 Msg::$v 在枚举中已不存在" }
    }
    if ($declared -ne $registered.Count) {
        Add-Problem "ALL_MSGS 声明长度 $declared 与实际条目数 $($registered.Count) 不符"
    }
}

# ---- 6. catalog 内部一致性：复用既有 vitest 用例 ------------------------------
function Test-CatalogWithVitest {
    pnpm --dir $ui test -- src/lib/i18n.test.js 2>&1 | Out-String | Write-Verbose
    if ($LASTEXITCODE -ne 0) {
        Add-Problem "catalog 单测未通过，请运行：pnpm --dir apps/config/ui test -- src/lib/i18n.test.js"
    }
}

# ---- 执行 --------------------------------------------------------------------
Push-Location $root
try {
    if (Test-ShouldCheck "docs/*") {
        Test-DocParity
        Test-DocLinks
    }
    if (Test-ShouldCheck "apps/config/ui/*") {
        Test-CatalogUsage
        if (-not $SkipVitest) { Test-CatalogWithVitest }
    }
    if (Test-ShouldCheck "crates/*") {
        Test-RustMsgRegistration
    }
}
finally {
    Pop-Location
}

if ($problems.Count -gt 0) {
    Write-Host ""
    Write-Host "i18n 检查未通过：$($problems.Count) 处问题（见上）" -ForegroundColor Red
    exit 1
}

Write-Host "i18n 检查通过" -ForegroundColor Green
