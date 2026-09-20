# Markdown 强调标记的完整性检查。
#
# 编辑器按行宽折行时不保护 ** 的完整性，会把它拆到两行：
#     ……不发送你的击键内容或鼠标轨迹。
#     **
# CommonMark 规定强调的闭合标记前不能有空白，于是整段强调失效，网页上直接渲染出裸的
# **。这种破绽肉眼可见却容易在 review 里滑过去，故由 i18n-check.ps1（PR 与文档部署）
# 和 gen-privacy.ps1（发版打包）两条路径分别调用本脚本把关。根治办法见仓库根 .editorconfig。
#
# 用法：$problems = & scripts/check-md-markers.ps1 -Path a.md, b.md
# 返回：问题描述数组；没有问题时为空。调用方自行决定是收集还是直接中断。

param([Parameter(Mandatory)][string[]]$Path)

$ErrorActionPreference = "Stop"

foreach ($file in $Path)
{
    # LiteralPath：VitePress 的动态路由文件名带方括号（changelog/[version].md），
    # 走 -Path 会被当成通配符而找不到文件
    $lines = @(Get-Content -LiteralPath $file -Encoding UTF8)
    $inCode = $false
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i].Trim()
        $lineNo = $i + 1
        # 围栏代码块里的 * 是内容（正则示例就常以 .* 收尾），不是 Markdown 标记
        if ( $line.StartsWith('```'))
        {
            $inCode = -not $inCode
            continue
        }
        if ($inCode)
        {
            continue
        }

        if ($line -eq '**' -or $line -eq '*')
        {
            "{0} 第 {1} 行只剩强调标记：折行把 ** 拆开了，请把它接回上一行末尾" -f $file, $lineNo
        }
        # 行尾孤零零一个 *（列表项的 "* " 不算）：另一半在下一行
        elseif (($line -match '(?<!\*)\*$') -and ($line -notmatch '^[-*] '))
        {
            "{0} 第 {1} 行以单个 * 结尾：折行把 ** 拆开了，请与下一行合并" -f $file, $lineNo
        }
    }
}
