---
title: 設定檔欄位
---

# 設定檔欄位參考

Boss Key 的設定儲存在 `config.json` 中，可攜版存在程式資料夾，安裝版存在 `%APPDATA%\BossKey`，詳見[資料目錄](/zh-tw/dev/architecture#資料目錄)。位置變動時舊設定會自動移轉過去。**結構與舊版完全相容**，舊使用者設定可直接沿用。首次執行若不存在則使用預設值。欄位定義見 `crates/common/src/config.rs`。

::: tip 一般不需手動修改
設定由設定介面自動讀寫並儲存，通常不需手動編輯。本頁面向需要理解欄位含義的開發者。
:::

## 頂層結構

| 欄位 | 型別 | 說明 |
| --- | --- | --- |
| `version` | string | 設定 schema 版本，結構變動時才動 |
| `app_version` | string | 上次執行過的**程式**版本；與目前程式版本不符即「更新後首次啟動」，核心據此自動開啟設定介面。預設留空 |
| `history` | number[] | 歷史記錄（時間戳記） |
| `frozen_pids` | number[] | 目前被凍結的程序 PID（用於復原） |
| `hotkey` | object | 鍵盤快速鍵，見下 |
| `setting` | object | 主要設定，見下 |
| `notifications` | object | 通知開關，見下 |
| `verhub` | object | 更新／公告相關，見下 |
| `window_rules` | object[] | 視窗規則（細粒度） |
| `process_rules` | object[] | 程序規則（粗粒度） |
| `hide_binding` | object[] | *v2 版*扁平綁定，僅用於移轉，移轉後清空、不再寫回 |

## `hotkey`

| 欄位 | 預設 | 說明 |
| --- | --- | --- |
| `hide_hotkey` | `"Ctrl+Q"` | 隱藏／顯示視窗 |
| `close_hotkey` | `"Win+Esc"` | 關閉核心 |
| `hide_only_hotkey` | `""` | [僅隱藏視窗](/zh-tw/guide/hotkeys#單向快速鍵與隱藏前景視窗)，留空為關閉 |
| `show_only_hotkey` | `""` | [僅顯示視窗](/zh-tw/guide/hotkeys#單向快速鍵與隱藏前景視窗)，留空為關閉 |
| `hide_foreground_hotkey` | `""` | [隱藏前景視窗](/zh-tw/guide/hotkeys#單向快速鍵與隱藏前景視窗)，留空為關閉 |
| `hide_intercept` | `false` | [隱藏快速鍵不傳遞](/zh-tw/guide/hotkeys#快速鍵不傳遞)（鍵盤掛鉤攔截） |
| `close_intercept` | `false` | [關閉快速鍵不傳遞](/zh-tw/guide/hotkeys#快速鍵不傳遞)（鍵盤掛鉤攔截） |
| `hide_only_intercept` | `false` | 僅隱藏快速鍵不傳遞 |
| `show_only_intercept` | `false` | 僅顯示快速鍵不傳遞 |
| `hide_foreground_intercept` | `false` | 隱藏前景視窗快速鍵不傳遞 |

## `setting`

| 欄位 | 型別 | 預設 | 對應功能 |
| --- | --- | --- | --- |
| `mute_after_hide` | bool | `true` | [隱藏後靜音](/zh-tw/guide/options) |
| `send_before_hide` | bool | `false` | [隱藏前傳送暫停鍵](/zh-tw/guide/options) |
| `hide_current` | bool | `true` | [同時隱藏目前使用中的視窗](/zh-tw/guide/options) |
| `click_to_hide` | bool | `true` | [按一下通知區域圖示切換隱藏](/zh-tw/guide/options) |
| `hide_icon_after_hide` | bool | `false` | [一併隱藏 Boss Key 通知區域圖示](/zh-tw/guide/options) |
| `tray_badges` | object | 見下 | [圖示狀態提示](/zh-tw/guide/notifications#圖示狀態提示) |
| `tray_show_tooltip` | bool | `true` | [顯示圖示懸浮名稱](/zh-tw/guide/notifications#顯示圖示懸浮名稱) |
| `freeze_after_hide` | bool | `false` | [程序凍結總開關](/zh-tw/guide/freeze) |
| `enhanced_freeze` | bool | `false` | [增強凍結](/zh-tw/guide/freeze) |
| `freeze_whole_tree` | bool | `false` | [凍結完整程序](/zh-tw/guide/freeze) |
| `show_float_window` | bool | `false` | 浮動視窗（開發中） |
| `mouse` | object | 見下 | [滑鼠按鍵隱藏](/zh-tw/guide/hotkeys) |
| `auto_hide_enabled` | bool | `false` | [閒置自動隱藏](/zh-tw/guide/hotkeys) |
| `auto_hide_time` | number | `5` | 閒置時間（分鐘，1–120） |
| `top_left_hide` 等四角 | bool | `false` | [四角隱藏](/zh-tw/guide/hotkeys) |
| `corner_fast_only` | bool | `true` | 僅快速移動觸發 |
| `allow_move_restore` | bool | `false` | 角落復原 |
| `log_retention_days` | number | `7` | [記錄檔保留天數](/zh-tw/guide/options)（0 = 關閉） |
| `log_level` | string | `"warn"` | [記錄輸出等級](/zh-tw/guide/options)：`debug`｜`info`｜`warn`｜`error` |
| `autostart_admin` | bool | `false` | [以系統管理員身分自動啟動](/zh-tw/guide/autostart)（僅排程工作方式生效） |
| `language` | string | `"auto"` | [介面語言](/zh-tw/guide/options)：`auto`｜`zh-CN`｜`en`｜`zh-TW` |

::: details 舊版扁平滑鼠開關（已淘汰）
`middle_button_hide`／`side_button1_hide`／`side_button2_hide` 僅用於還原序列化移轉，移轉後歸零、不再寫回檔案。請使用 `mouse` 結構。
:::

### `setting.mouse`

每顆按鍵為一個 `MouseButton`：`{ enabled: bool, clicks: 1..=3, modifiers: string }`。

| 欄位 | 預設 | 說明 |
| --- | --- | --- |
| `left`／`middle`／`right`／`side1`／`side2` | 見下 | 五顆按鍵各自的觸發條件 |
| `multi_click_ms` | `350` | 連按判定時間（毫秒，150–1000） |
| `allow_click_restore` | `true` | 允許再按一次復原 |

::: info 全新安裝預設
全新安裝預設開啟**中鍵按一下**（`middle.enabled = true`，`clicks = 1`），其餘四顆關閉。設定檔缺 `mouse` 一節的舊設定讀進來則**全關**。
:::

### `setting.tray_badges`

[圖示狀態提示](/zh-tw/guide/notifications#圖示狀態提示)：四種顏色的圓點角標各自繫結一個狀態來源，多個狀態同時活躍時依**紅 > 綠 > 黃 > 藍**的優先順序僅顯示一個圓點。

| 欄位 | 預設 | 預設含義 |
| --- | --- | --- |
| `red` | `"hidden"` | 存在隱藏中的視窗 |
| `green` | `"auto_hide"` | 已啟用自動隱藏 |
| `yellow` | `"hide_current"` | 已啟用同時隱藏目前視窗 |
| `blue` | `"freeze"` | 已啟用程序凍結 |

每項取值：`hidden`（存在隱藏中的視窗）｜`auto_hide`（已啟用自動隱藏）｜`hide_current`（已啟用同時隱藏目前視窗）｜`freeze`（已啟用程序凍結）｜`elevated`（以系統管理員身分執行）｜`monitor_paused`（快速鍵監控已暫停）｜`""`（留空 = 不顯示該顏色）；未知取值讀取時正規化為留空。

## `notifications`

| 欄位 | 預設 | 說明 |
| --- | --- | --- |
| `on_start` | `true` | 核心啟動通知 |
| `on_quit` | `true` | 核心結束通知 |
| `on_autostart` | `true` | 開機自動啟動狀態變更通知 |
| `on_hide` | `false` | 每次隱藏通知 |
| `on_show` | `false` | 每次顯示通知 |

## `verhub`

| 欄位 | 預設 | 說明 |
| --- | --- | --- |
| `include_preview` | `false` | 更新檢查是否納入預覽版 |
| `seen_announcement_id` | `""` | 已讀的最新公告 id |

## `window_rules`（視窗規則）

細粒度規則，按控制代碼＋標題鎖定單一視窗；`regex` 為 `Some` 時按標題正規表示式命中。

| 欄位 | 說明 |
| --- | --- |
| `title` | 視窗標題 |
| `hwnd` | 視窗控制代碼 |
| `process` | 程序名稱 |
| `PID` | 程序 ID（**大寫** key，相容舊設定） |
| `path` | 執行檔路徑 |
| `regex` | 標題正規表示式（進階模式；省略表示精確規則） |
| `include_untitled` | 正規表示式是否納入無標題視窗 |
| `include_background` | 正規表示式是否納入背景視窗 |

## `process_rules`（程序規則）

粗粒度規則，按執行檔隱藏該程式的所有視窗。

| 欄位 | 預設 | 說明 |
| --- | --- | --- |
| `process` | | 程序名稱 |
| `path` | | 執行檔路徑 |
| `regex` | | 正規表示式（作用於路徑或檔名） |
| `by_name` | `false` | 只按檔名比對，忽略路徑 |
| `include_untitled` | `true` | 是否納入無標題視窗（程序規則預設納入） |
| `include_background` | `false` | 是否納入背景視窗 |

## 相容與移轉

- **未知欄位被忽略**：未來新增欄位不會導致舊核心解析失敗。
- **缺少欄位用預設值**：任意欄位缺少都回落到預設值。
- **舊綁定自動移轉**：`hide_binding` → `window_rules`；舊滑鼠開關 → `mouse` 連按觸發。移轉是**冪等**的。
- **`PID` 大寫**：序列化輸出使用大寫 `PID`，與舊 Python 版本相容。
