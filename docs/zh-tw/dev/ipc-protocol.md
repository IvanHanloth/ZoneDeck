---
title: IPC 協定
---

# IPC 協定

設定介面（`config.exe`）與常駐核心（`ZoneDeck.exe`）透過**具名管道**通訊。協定定義在 `crates/common/src/ipc.rs`。

## 傳輸

- 管道名稱：`\\.\pipe\zonedeck`。
- 編碼：**一行一條 JSON**，以 `\n` 分隔。用戶端傳送一條 `Command`，伺服端回一條 `Response`。
- 用戶端封裝為 `PipeClient`：預設帶重試（25 次、40ms 間隔）；`.fast()` 為**快速失敗**模式（只嘗試一次），用於狀態輪詢等不希望阻塞的情境。

## Command（設定介面 → 核心）

序列化使用 `#[serde(tag = "cmd", rename_all = "snake_case")]`。

| 命令 | JSON | 作用 |
| --- | --- | --- |
| `ReloadConfig` | `{"cmd":"reload_config"}` | 重新讀取設定並熱重新載入（重新註冊快速鍵／掛鉤／計時器） |
| `GetState` | `{"cmd":"get_state"}` | 查詢隱藏狀態 |
| `GetElevation` | `{"cmd":"get_elevation"}` | 查詢是否為系統管理員 |
| `GetStatus` | `{"cmd":"get_status"}` | **一次來回取回全部狀態**（隱藏 + 權限 + 監控），替代前兩者連發 |
| `Hide` | `{"cmd":"hide"}` | 隱藏 |
| `Show` | `{"cmd":"show"}` | 顯示 |
| `Toggle` | `{"cmd":"toggle"}` | 切換隱藏／顯示 |
| `SetAutostart` | `{"cmd":"set_autostart","enabled":true}` | 設定開機自動啟動 |
| `SetHotkeys` | `{"cmd":"set_hotkeys","enabled":false}` | 暫時停用／復原快速鍵與滑鼠監控 |
| `ReleaseWindows` | `{"cmd":"release_windows","hwnds":[..]}` | 視窗復原工具：復原顯示指定控制代碼。在核心紀錄裡的視窗按整個處理程序釋放（連同解除凍結／取消靜音）；紀錄外的控制代碼直接顯示 |
| `AdoptWindows` | `{"cmd":"adopt_windows","hwnds":[..]}` | 視窗復原工具：隱藏指定控制代碼並納入核心紀錄（享有當機復原保護），不施加靜音／凍結 |
| `Quit` | `{"cmd":"quit"}` | 結束核心 |

## Response（核心 → 設定介面）

序列化使用 `#[serde(tag = "type", rename_all = "snake_case")]`。

| 回應 | JSON | 說明 |
| --- | --- | --- |
| `Ok` | `{"type":"ok"}` | 命令成功、無額外資料 |
| `State` | `{"type":"state","hidden":true}` | 目前隱藏狀態 |
| `Elevated` | `{"type":"elevated","elevated":true}` | 是否為系統管理員 |
| `Status` | `{"type":"status","hidden":..,"elevated":..,"monitoring":..}` | 彙整狀態 |
| `Error` | `{"type":"error","message":".."}` | 錯誤訊息 |

`Status.monitoring`：核心是否正在監聽快速鍵與滑鼠（被 `SetHotkeys` 停用時為 `false`）。

::: info 錯誤訊息隨介面語言變化
`Error.message` 的文案取自核心的文案 catalog，隨 `setting.language` 變化。它是顯示給使用者的文字，**不要**當作穩定識別碼去做條件判斷。
:::

## 監控停用與心跳

`SetHotkeys { enabled: false }` 用於設定介面在**錄製／偵錯快速鍵**時暫時停用核心監控，避免誤觸發。該停用**有狀態**，需持續心跳續期：

| 常數 | 值 | 含義 |
| --- | --- | --- |
| `SUSPEND_TIMEOUT_MS` | `15000` | 監控程式時長：超過這麼久沒收到心跳，核心自動復原監控 |
| `SUSPEND_HEARTBEAT_MS` | `4000` | 設定介面重發心跳的建議間隔（須顯著小於逾時） |

::: info 為什麼要監控程式
若設定介面在停用監控期間當機／被強制結束，來不及傳送復原命令，核心也會在逾時後**自動復原監控**，不會讓使用者的快速鍵永久失效。
:::

## 典型互動時序

```
設定介面儲存設定 ──▶ reload_config ──▶ 核心熱重新載入 ──▶ ok
狀態輪詢（每 2s） ──▶ get_status(fast) ──▶ status{hidden,elevated,monitoring}
進入快速鍵設定區 ──▶ set_hotkeys{false} + 定時心跳 ──▶ 核心暫停監控
離開／失去焦點 ──▶ set_hotkeys{true} ──▶ 核心復原監控
```
