---
title: 系統架構
---

# 系統架構

Boss Key v3 採用 **核心＋設定分離** 的**雙程序架構**，兩者透過**具名管道**通訊。本章介紹整體設計與工程結構。

## 雙程序總覽

```
┌────────────────────────────────────────────────────────────────┐
│ 使用者互動工作階段（Session 1+）                                 │
│                                                                  │
│  ┌──────────────────────────┐        ┌────────────────────────┐ │
│  │ Boss Key.exe（常駐）      │        │ config.exe             │ │
│  │ 純 Rust 原生，~350KB      │◀─IPC──▶│ Tauri（Rust + WebView）│ │
│  │ 隱藏的訊息視窗 + wndproc  │ 具名   │ 依需求開啟，關閉即結束 │ │
│  │                          │ 管道   │                        │ │
│  │ • RegisterHotKey 全域快速鍵        │ • 視窗綁定／快速鍵錄製 │ │
│  │ • WH_MOUSE_LL 滑鼠/四角   │        │ • 選項／關於／檢查更新 │ │
│  │ • GetLastInputInfo 閒置   │        │ • 提升權限／視窗復原   │ │
│  │ • 列舉/隱藏/顯示視窗       │        └────────────┬───────────┘ │
│  │ • Core Audio 靜音         │                     │ 讀寫         │
│  │ • NtSuspend 程序凍結       │        ┌────────────▼───────────┐ │
│  │ • 通知區域圖示／通知       │        │ config.json（與 exe 同資料夾）│
│  │ • 開機自動啟動（排程/登錄檔）        │ 核心收到 reload 後熱重新載入 │
│  └──────────────────────────┘        └────────────────────────┘ │
│         ▲ 隨登入自動啟動                                         │
└─────────┼────────────────────────────────────────────────────────┘
          │ 排程工作(登入觸發,最高權限) / 登錄檔 Run 回落
```

## 關鍵設計決策

### 核心必須執行在使用者互動工作階段

核心**不能**做成 Session 0 的 Windows 服務，否則無法列舉／隱藏使用者視窗、無法安裝掛鉤。因此核心以隨登入自動啟動的一般程式形式執行在使用者工作階段中。

### 程序間通訊（IPC）

- 使用具名管道 `\\.\pipe\bosskey`，**一行一條 JSON**（`Command`／`Response`）。
- 設定儲存後，設定介面傳送 `reload_config`，核心**熱重新載入**並重新註冊快速鍵／掛鉤／計時器，**不需重新啟動核心**。
- 協定細節見 [IPC 協定](/zh-tw/dev/ipc-protocol)。

### 全部使用者模式監聽，不使用核心驅動程式

| 能力 | 使用的 API | 說明 |
| --- | --- | --- |
| 全域快速鍵 | `RegisterHotKey` | 最規範、完善的觸發方式 |
| 快速鍵不傳遞 | `WH_KEYBOARD_LL` | 僅在有快速鍵開啟「不傳遞」時才安裝 |
| 滑鼠／四角 | `WH_MOUSE_LL` | 僅在啟用滑鼠／四角時才安裝 |
| 閒置偵測 | `GetLastInputInfo` | 不需常駐監聽鍵盤 |

不相依於任何核心驅動程式，降低誤判與安全風險。

### 權限模型

核心預設 `asInvoker`，**不強制 UAC**。僅以下兩項需要系統管理員，透過設定介面的「以系統管理員身分重新啟動核心」**依需求提升權限**：

- 增強凍結（`pssuspend64.exe`）；
- 排程工作最高權限自動啟動。

## 工程結構（Cargo workspace）

```
Boss-Key/
├── Cargo.toml                      workspace（含 release profile 調校）
├── crates/
│   ├── common/                     共用程式庫（無平台相依，可跨平台編譯）
│   │   └── src/{model,config,matching,ipc,verhub,i18n}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule（serde 相容舊 config.json，PID 大寫）
│   │       config    Config/Setting/Hotkey（相容讀取舊設定 + 移轉）
│   │       matching  視窗比對邏輯
│   │       ipc       Command/Response 協定 + PipeClient 用戶端
│   │       verhub    版本／公告／更新檢查相關模型
│   │       i18n      介面語言標籤（Lang）與語言偏好解析，核心與設定程式共用
│   └── core/                       常駐核心（lib + bin）
│       └── src/
│           platform/win32.rs  視窗列舉/隱藏/顯示（WindowManager trait）
│           agent.rs      訊息迴圈，彙整快速鍵/通知區域/IPC/滑鼠/計時器
│           hotkey.rs     快速鍵字串 → RegisterHotKey 解析
│           hide.rs       隱藏選擇邏輯 + HideController（隱藏/顯示編排）
│           effects.rs    Effects trait（靜音/凍結/暫停鍵，可注入 mock）
│           audio.rs      Core Audio 工作階段靜音
│           freeze.rs     NtSuspend/Resume + pssuspend64 增強凍結
│           mouse_hook.rs WH_MOUSE_LL（中鍵/側鍵/四角）
│           keyboard_hook.rs WH_KEYBOARD_LL（「不傳遞」快速鍵攔截）
│           idle.rs       GetLastInputInfo 閒置 + 自動隱藏判定
│           tray.rs       Shell_NotifyIcon 通知區域 + 通知
│           ipc_server.rs 具名管道伺服端
│           autostart.rs  開機自動啟動（排程工作 XML 含失敗自動重新啟動 + 登錄檔回落）
│           elevation.rs  系統管理員偵測 + UAC 提升權限重新啟動
│           i18n.rs       核心使用者可見文案 catalog（通知區域選單／通知／IPC 錯誤；記錄檔不走它）
│           logging.rs    分級檔案記錄（logs/BossKey-YYYY-MM-DD.log 按日切割 + panic 掛鉤）
│           recovery.rs   當機復原（隱藏狀態寫入磁碟，異常結束後找回視窗）
│           icon.rs       程序圖示擷取（HICON → 手寫 PNG/base64 編碼）
│           single_instance.rs  具名互斥鎖單一執行個體
└── apps/config/                    設定介面（Tauri 2 + Svelte 5）
    ├── src-tauri/  Rust 後端命令 + tauri.conf.json + capabilities
    ├── ui/         前端原始碼（Vite + Svelte 5）
    │   └── src/    lib/（純邏輯 + vitest 測試）+ components/（Svelte 元件）
    │                + locales/（三語文案 catalog，以 zh-CN.js 為基準）
    └── dist/       前端建置產物（gitignore；由 ui/ 經 vite build 產生）
```

::: tip common 為什麼無平台相依
`crates/common` 刻意不相依於 Windows API，因此可以跨平台編譯，其純邏輯（設定解析、比對、協定）也更易做單元測試。平台相關程式碼集中在 `crates/core`。
:::

## 核心內部：Agent 訊息迴圈

`agent.rs` 是核心的中樞：它建立一個**隱藏的訊息視窗**並執行 Windows 訊息迴圈，彙整以下事件來源：

- 全域快速鍵（`WM_HOTKEY`）；
- 滑鼠掛鉤（中鍵／側鍵／四角）；
- 具名管道伺服端（來自設定介面的命令）；
- 計時器（閒置偵測、狀態維護等）；
- 通知區域圖示互動。

當觸發隱藏／顯示時，交由 `HideController` 編排：選擇命中的視窗 → 套用 `Effects`（靜音／凍結／暫停鍵）→ 隱藏／顯示視窗，並把狀態寫入 `recovery.json`。

::: info 可測試性設計
`Effects` 被抽象為 trait，測試時可注入 mock，從而在不真正靜音／凍結系統的情況下驗證隱藏編排邏輯。同理 `WindowManager` 也是 trait。
:::

## 穩定性設計（當機自癒三層防線）

1. **當機記錄**：關鍵事件與 panic 寫入 exe 同資料夾的 `logs/BossKey-YYYY-MM-DD.log`（按日切割，依 `log_retention_days` 保留，0 表示不記錄；release 建置丟棄 DEBUG 級）。
2. **當機復原**：隱藏時把「隱藏／凍結／靜音了什麼」寫入 `recovery.json`，異常結束後重新啟動自動找回。
3. **監控程式**：排程工作 `RestartOnFailure`（當機後 1 分鐘內重新啟動，最多 3 次）。release 建置 `panic = "abort"`，panic 掛鉤寫完記錄後以非零碼結束，正好觸發排程工作重新啟動。

使用者視角的說明見 [視窗復原與當機自癒](/zh-tw/guide/recovery)。

## 介面語言

核心與設定程式共用 `crates/common` 的 `Lang`（`zh-CN`／`en`／`zh-TW`）與語言偏好解析，文案則各自維護：

| 位置 | 文案載體 | 說明 |
| --- | --- | --- |
| `crates/core/src/i18n.rs` | `Msg` 列舉 + 三語 `match` | 通知區域選單、通知、IPC 錯誤；`tf()` 負責 `{名字}` 佔位符替換 |
| `apps/config/ui/src/locales/*.js` | 扁平鍵值表 | 設定介面全部文案；`t(key, params)` 查表並替換佔位符 |

- 生效語言由 `setting.language` 決定：`auto` 時按系統顯示語言推斷（核心用 `GetUserDefaultLocaleName`，前端用 `navigator.language`），推斷不出回落到簡體中文。
- 設定介面儲存設定後會傳送 `reload_config`，核心據此同步語言，因此切換語言不需重新啟動任一程序。
- **記錄檔不參與 i18n**，一律使用簡體中文，以便跨語言排查問題。
- `NO_TITLE`（`"无标题窗口"`）是跨程序、寫進 `config.json` 的哨兵值，**不隨語言變化**，僅在顯示時翻譯。

## 前端架構

設定介面前端使用 Svelte 5 + Vite，採用無邊框自繪視窗、淺色深色佈景主題、設定自動儲存等設計。詳見 [前端與設定介面](/zh-tw/dev/frontend)。
