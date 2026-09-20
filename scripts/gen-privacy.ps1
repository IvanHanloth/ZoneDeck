# 从文档站的隐私声明生成安装向导用的纯文本版
#
# 安装包在「信息」页（InfoBeforeFile）展示隐私声明全文，该页只吃纯文本，
# 而政策正文以 docs 下的 Markdown 为唯一真源——手抄一份迟早和网站对不上，
# 故由本脚本转换，产物不入库，每次打包时重新生成。
#
# 用法：powershell -File scripts/gen-privacy.ps1
# 由 scripts/package.ps1 在编译安装包前自动调用。

param([string]$OutDir)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $OutDir)
{
    $OutDir = Join-Path $root ".github\inno-script\privacy"
}

# Inno Setup 的语言名 → 文档站里对应的那份 Markdown
$sources = [ordered]@{
    "chinesesimplified" = "docs\privacy.md"
    "chinesetraditional" = "docs\zh-tw\privacy.md"
    "english" = "docs\en\privacy.md"
}
# 表格转成条目时的「字段名 值」分隔符，英文版用半角
$separators = @{
    "chinesesimplified" = "："
    "chinesetraditional" = "："
    "english" = ": "
}

# CJK 文字与全角标点；软换行合并后据此判断要不要留空格。
$CJK = '\u3000-\u303F\u4E00-\u9FFF\u3400-\u4DBF\uFF00-\uFFEF'
# 全角标点。它的两侧不留空格，Markdown 标记与软换行带进来的都去掉。
$CJKPunct = '\u3000-\u303F\uFF00-\uFF20\uFF3B-\uFF40\uFF5B-\uFF65'

# Markdown 的软换行只是源码折行，渲染时仍属同一段。纯文本得先合回一行，否则
# 安装向导里会出现参差的硬换行，跨行的 **粗体** 也配不成对。
# 按 CommonMark 一律用空格相接，CJK 之间多出来的那个由 Convert-Inline 去掉。
function Join-SoftWraps([string[]]$lines)
{
    $out = [System.Collections.Generic.List[string]]::new()
    $buf = ''
    $inCode = $false
    foreach ($line in $lines)
    {
        $trimmed = $line.Trim()
        # 围栏代码块内原样保留，换行是内容的一部分
        if ( $trimmed.StartsWith('```'))
        {
            if ($buf -ne '')
            {
                $out.Add($buf); $buf = ''
            }
            $inCode = -not $inCode
            $out.Add($line)
            continue
        }
        if ($inCode)
        {
            $out.Add($line); continue
        }

        # 自成一行的结构：空行、标题、表格、frontmatter 分隔线
        $standalone = $trimmed -eq '' -or $trimmed -match '^#{1,6} |^\|' -or $trimmed -eq '---'
        # 列表项起个头，后面的续行要并进来
        $listStart = $trimmed -match '^([-*]|\d+\.) '
        if ($standalone -or $listStart)
        {
            if ($buf -ne '')
            {
                $out.Add($buf); $buf = ''
            }
            if ($standalone)
            {
                $out.Add($line)
            }
            else
            {
                $buf = $line
            }
            continue
        }

        if ($buf -eq '')
        {
            $buf = $line
            continue
        }
        # 接缝要不要空格：中文之间不该因源码折行多出一个，中英之间该有的那个要留住；
        # 折在开括号或开引号之后、闭标点之前的，两边也得贴紧。
        $openEnd = $buf -match '([(\[（【「《]|[\s(\[]["''])$'
        $closeStart = $trimmed -match '^["'']?[)\]）】」》,.;:!?]'
        $bothCjk = ($buf -match "[$CJK]$") -and ($trimmed -match "^[$CJK]")
        # 折行把 ** 拆成两半时接缝不能塞空格，否则粗体配不成对（列表项已在上面单独处理）
        $splitMarker = ($buf -match '[*_]$') -or ($trimmed -match '^[*_]')
        $joint = if ($openEnd -or $closeStart -or $bothCjk -or $splitMarker)
        {
            ''
        }
        else
        {
            ' '
        }
        $buf = "$buf$joint$trimmed"
    }
    if ($buf -ne '')
    {
        $out.Add($buf)
    }
    return , $out.ToArray()
}

# 去掉行内的 Markdown 标记：粗体、斜体、行内代码保留文字，链接保留文字并把地址附在后面
function Convert-Inline([string]$text)
{
    $text = $text -replace '\*\*([^*]+)\*\*', '$1'
    $text = $text -replace '(?<!\*)\*([^*\n]+)\*(?!\*)', '$1'
    $text = $text -replace '`([^`]+)`', '$1'
    # 链接文本就是地址本身时（含 mailto:）不再把地址重复附一遍
    $text = $text -replace '\[([^\]]+)\]\((?:mailto:)?\1\)', '$1'
    $text = $text -replace '\[([^\]]+)\]\(([^)]+)\)', '$1 ($2)'
    $text = $text -replace '<(https?://[^>]+)>', '$1'
    # 合并软换行时一律补了空格，Markdown 标记两侧也带着空格；
    # CJK 文字与全角标点之间不该有空格，按中文排版去掉（中英之间的保留）。
    $text = $text -replace "(?<=[$CJKPunct])[ ]+", ''
    $text = $text -replace "[ ]+(?=[$CJKPunct])", ''
    return $text.TrimEnd()
}

# 拆一行表格：| a | b | → @("a", "b")
function Split-Row([string]$line)
{
    return ($line.Trim().Trim('|') -split '\|') | ForEach-Object { Convert-Inline $_.Trim() }
}

function Convert-Document([string]$markdown, [string]$separator, [string]$src)
{
    $out = [System.Collections.Generic.List[string]]::new()
    # 表格没有结束标记，靠「上一行是不是表格」来判断，故表头要跨行记住
    $headers = $null
    $inFrontMatter = $false
    $raw = $markdown -split "\r?\n"
    $broken = @(& (Join-Path $PSScriptRoot "check-md-markers.ps1") -Path $src)
    if ($broken) { throw ($broken -join [Environment]::NewLine) }
    $lines = Join-SoftWraps $raw

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]

        # frontmatter：首行的 --- 开启，下一个 --- 关闭
        if ($line.Trim() -eq '---' -and $i -eq 0)
        {
            $inFrontMatter = $true; continue
        }
        if ($inFrontMatter)
        {
            if ($line.Trim() -eq '---')
            {
                $inFrontMatter = $false
            }
            continue
        }

        # 表格：表头行、分隔行、数据行
        if ( $line.TrimStart().StartsWith('|'))
        {
            $cells = Split-Row $line
            if ($null -eq $headers)
            {
                $headers = $cells; continue
            }
            # | --- | --- | 这样的分隔行
            if (($cells | Where-Object { $_ -notmatch '^:?-+:?$' }).Count -eq 0)
            {
                continue
            }
            # 首列作条目标题，其余列冠以表头
            $out.Add("  - $( $cells[0] )")
            for ($c = 1; $c -lt $cells.Count; $c++) {
                $name = if ($c -lt $headers.Count)
                {
                    $headers[$c]
                }
                else
                {
                    ""
                }
                $out.Add("    $name$separator$( $cells[$c] )")
            }
            continue
        }
        $headers = $null

        $trimmed = $line.TrimStart()
        # 嵌套列表靠缩进表达层级，转换后要原样带着
        $indent = ' ' * ($line.Length - $trimmed.Length)
        switch -regex ($trimmed)
        {
            '^# (.+)$' {
                $title = Convert-Inline $Matches[1]
                $out.Add($title)
                $out.Add('=' * 60)
                break
            }
            '^## (.+)$' {
                $out.Add("")
                $out.Add((Convert-Inline $Matches[1]))
                $out.Add('-' * 60)
                break
            }
            '^#{3,} (.+)$' {
                $out.Add("")
                $out.Add((Convert-Inline $Matches[1]))
                $out.Add('-' * 40)
                break
            }
            '^[-*] (.+)$' {
                $out.Add("$indent  - $( Convert-Inline $Matches[1] )")
                break
            }
            '^(\d+)\. (.+)$' {
                $out.Add("$indent  $( $Matches[1] ). $( Convert-Inline $Matches[2] )")
                break
            }
            default {
                $out.Add((Convert-Inline $line))
            }
        }
    }

    # 连续空行压成一行，行尾空行去掉
    $text = ($out -join "`r`n") -replace '(\r\n){3,}', "`r`n`r`n"
    return $text.Trim() + "`r`n"
}

if (-not (Test-Path $OutDir))
{
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}

# Inno Setup 按 ANSI 读无 BOM 的 txt，中文会乱码，必须带 BOM
$utf8Bom = New-Object System.Text.UTF8Encoding $true

foreach ($lang in $sources.Keys)
{
    $src = Join-Path $root $sources[$lang]
    if (-not (Test-Path $src))
    {
        throw "缺少隐私声明原文：$src"
    }
    $text = Convert-Document (Get-Content $src -Raw -Encoding UTF8) $separators[$lang] $src
    $dest = Join-Path $OutDir "PRIVACY.$lang.txt"
    [System.IO.File]::WriteAllText($dest, $text, $utf8Bom)
    Write-Host "    $lang -> $dest"
}
