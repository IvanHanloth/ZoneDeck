# Boss Key 残留数据清理脚本
#
# 便携版的设置、日志、恢复文件就放在程序文件夹里，删掉文件夹即可。但另有两样东西在
# 用户目录下，删文件夹清不掉：配置界面的浏览器数据，以及程序文件夹不可写时改存到
# %APPDATA%\BossKey 的那份设置。开机自启还会留下计划任务与注册表项。本脚本负责这些。
#
# 用法（在本文件所在目录打开 PowerShell）：
#   powershell -ExecutionPolicy Bypass -File cleanup.ps1
#   powershell -ExecutionPolicy Bypass -File cleanup.ps1 -Force   # 不询问，直接清理
#
# 本脚本不会删除程序本身：程序文件夹请在脚本跑完后自行删除。
# 安装版无需用它，卸载程序已经做了同样的事。

param([switch]$Force)

$ErrorActionPreference = "Stop"

# 界面语言跟随系统，与程序保持一致（zh-CN 为基准）。
$lang = if ($PSUICulture -eq 'zh-CN' -or $PSUICulture -like 'zh-Hans*') { 'zh-CN' }
elseif ($PSUICulture -like 'zh-*') { 'zh-TW' }
else { 'en' }

$catalog = @{
    'zh-CN' = @{
        Title     = 'Boss Key 残留数据清理'
        Nothing   = '没有发现残留数据，无需清理。'
        Found     = '将删除以下内容：'
        Confirm   = '确认删除？输入 y 继续，其他任意键取消'
        Cancelled = '已取消，未做任何改动。'
        Killing   = '正在结束仍在运行的 Boss Key 进程...'
        Removed   = '已删除：{0}'
        Failed    = '删除失败：{0}（{1}）'
        Done      = '清理完成。程序文件夹请自行删除。'
        DataDir   = '用户目录下的数据（程序目录不可写时存到这里）'
        WebView   = '配置界面的浏览器数据'
        Task      = '开机自启计划任务'
        RegRun    = '开机自启注册表项'
    }
    'zh-TW' = @{
        Title     = 'Boss Key 殘留資料清理'
        Nothing   = '沒有發現殘留資料，不需清理。'
        Found     = '將刪除以下內容：'
        Confirm   = '確認刪除？輸入 y 繼續，其他任意鍵取消'
        Cancelled = '已取消，未做任何變更。'
        Killing   = '正在結束仍在執行的 Boss Key 處理程序...'
        Removed   = '已刪除：{0}'
        Failed    = '刪除失敗：{0}（{1}）'
        Done      = '清理完成。程式資料夾請自行刪除。'
        DataDir   = '使用者資料夾下的資料（程式資料夾不可寫入時存到這裡）'
        WebView   = '設定介面的瀏覽器資料'
        Task      = '開機自動啟動排程工作'
        RegRun    = '開機自動啟動登錄項目'
    }
    'en'    = @{
        Title     = 'Boss Key leftover data cleanup'
        Nothing   = 'No leftover data found; nothing to clean up.'
        Found     = 'The following will be deleted:'
        Confirm   = 'Delete these? Type y to continue, anything else to cancel'
        Cancelled = 'Cancelled; nothing was changed.'
        Killing   = 'Stopping running Boss Key processes...'
        Removed   = 'Deleted: {0}'
        Failed    = 'Could not delete {0} ({1})'
        Done      = 'Cleanup finished. Delete the program folder yourself.'
        DataDir   = 'Data in the user folder (used when the program folder is not writable)'
        WebView   = "Settings window's browser data"
        Task      = 'Autostart scheduled task'
        RegRun    = 'Autostart registry entry'
    }
}
$t = $catalog[$lang]

# 以下四项须与程序保持一致：
#   用户数据目录    crates/common/src/paths.rs（USER_DIR_NAME）
#   浏览器数据目录  apps/config/src-tauri/tauri.conf.json（identifier）
#   自启任务与注册表项  crates/core/src/autostart.rs（TASK_NAME / REG_VALUE_NAME）
$dataDir = Join-Path $env:APPDATA 'BossKey'
$webViewDir = Join-Path $env:LOCALAPPDATA 'cn.hanloth.bosskey.config'
$taskName = 'BossKeyAutostart'
$runSubkey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$runValueName = 'Boss Key Application'

$targets = @()
if (Test-Path $dataDir) {
    $targets += [pscustomobject]@{ Kind = 'Dir'; Label = $t.DataDir; Detail = $dataDir }
}
if (Test-Path $webViewDir) {
    $targets += [pscustomobject]@{ Kind = 'Dir'; Label = $t.WebView; Detail = $webViewDir }
}
# schtasks 找不到任务时返回非零码，用它判断存在性；输出丢弃，此处只关心结果。
& schtasks.exe /Query /TN $taskName *> $null
if ($LASTEXITCODE -eq 0) {
    $targets += [pscustomobject]@{ Kind = 'Task'; Label = $t.Task; Detail = $taskName }
}
$runEntry = Get-ItemProperty -Path $runSubkey -Name $runValueName -ErrorAction SilentlyContinue
if ($runEntry) {
    $targets += [pscustomobject]@{ Kind = 'Reg'; Label = $t.RegRun; Detail = "$runSubkey\$runValueName" }
}

Write-Host $t.Title -ForegroundColor Cyan
if ($targets.Count -eq 0) {
    Write-Host $t.Nothing -ForegroundColor Green
    return
}

Write-Host ""
Write-Host $t.Found
foreach ($target in $targets) {
    Write-Host ("  - {0}`n    {1}" -f $target.Label, $target.Detail)
}
Write-Host ""

if (-not $Force) {
    $answer = Read-Host $t.Confirm
    if ($answer -ne 'y' -and $answer -ne 'Y') {
        Write-Host $t.Cancelled -ForegroundColor Yellow
        return
    }
}

# 核心是常驻进程，不结束它的话数据目录里的日志正被占用，删不干净；
# 它还会在退出前回写配置，把刚删掉的目录重新建出来。
Write-Host $t.Killing
foreach ($name in @('Boss Key', 'config')) {
    Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}

foreach ($target in $targets) {
    try {
        switch ($target.Kind) {
            'Dir' { Remove-Item -LiteralPath $target.Detail -Recurse -Force -ErrorAction Stop }
            'Task' {
                & schtasks.exe /Delete /F /TN $target.Detail *> $null
                if ($LASTEXITCODE -ne 0) { throw "schtasks exit $LASTEXITCODE" }
            }
            'Reg' { Remove-ItemProperty -Path $runSubkey -Name $runValueName -Force -ErrorAction Stop }
        }
        Write-Host ($t.Removed -f $target.Detail) -ForegroundColor Green
    }
    catch {
        Write-Host ($t.Failed -f $target.Detail, $_.Exception.Message) -ForegroundColor Red
    }
}

Write-Host ""
Write-Host $t.Done -ForegroundColor Green
