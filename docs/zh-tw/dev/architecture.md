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
│  │ • 通知區域圖示／通知       │        │ config.json（資料目錄）    │
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
│   │   └── src/{model,config,matching,ipc,i18n,paths}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule（serde 相容舊 config.json，PID 大寫）
│   │       config    Config/Setting/Hotkey（相容讀取舊設定 + 移轉；儲存走 tmp + rename 原子取代）
│   │       matching  視窗比對邏輯
│   │       paths     資料目錄定位（安裝版走 %APPDATA%，可攜版就地，見下）
│   │       ipc       Command/Response 協定 + PipeClient 用戶端
│   │       i18n      介面語言標籤（Lang）與語言偏好解析，核心與設定程式共用
│   └── core/                       常駐核心（lib + bin）
│       └── src/
│           platform/win32.rs  視窗列舉/隱藏/顯示（WindowManager trait）
│           agent.rs      訊息迴圈，彙整快速鍵/通知區域/IPC/滑鼠/計時器
│           hotkey.rs     快速鍵字串 → RegisterHotKey 解析
│           hide.rs       隱藏選擇邏輯 + HideController（plan/commit 兩段式編排，
│                         含死控制代碼剪枝與復原前的視窗/處理程序身分校驗）
│           effects.rs    Effects trait（靜音/凍結/暫停鍵，可注入 mock）
│           effects_worker.rs  副作用專職執行緒（FIFO 佇列；訊息迴圈只做 SW_HIDE）
│           audio.rs      Core Audio 工作階段靜音
│           freeze.rs     NtSuspend/Resume + pssuspend64 增強凍結
│           input_hooks.rs 輸入掛鉤專職執行緒（承載兩個低階掛鉤，優先權 above normal）
│           mouse_hook.rs WH_MOUSE_LL（中鍵/側鍵/四角）
│           keyboard_hook.rs WH_KEYBOARD_LL（「不傳遞」快速鍵攔截）
│           idle.rs       GetLastInputInfo 閒置 + 自動隱藏判定
│           tray.rs       Shell_NotifyIcon 通知區域 + 通知
│           ipc_server.rs 具名管道伺服端（建立失敗退避重試，不結束）
│           autostart.rs  開機自動啟動（排程工作 XML 含失敗自動重新啟動 + 登錄檔回落）
│           elevation.rs  系統管理員偵測 + UAC 提升權限重新啟動
│           i18n.rs       核心使用者可見文案 catalog（通知區域選單／通知／IPC 錯誤；記錄檔不走它）
│           logging.rs    分級檔案記錄（logs/BossKey-YYYY-MM-DD.log 按日切割 + panic 掛鉤）
│           recovery.rs   當機復原（意圖先行寫入 + 原子寫；快照帶開機時刻與
│                         處理程序建立時刻，跨重新開機的快照會被丟棄）
│           icon.rs       程序圖示擷取（HICON → 手寫 PNG/base64 編碼）
│           single_instance.rs  具名互斥鎖單一執行個體
└── apps/config/                    設定介面（Tauri 2 + Svelte 5）
    ├── src-tauri/  Rust 後端命令 + tauri.conf.json + capabilities
    │   └── src/verhub.rs  Verhub 用戶端（版本／公告／回饋／日誌／專案連結，基於 verhub-sdk；
    │                      回饋可選轉為 GitHub Issue，由 Verhub 機器人建立，此時須填 GitHub 帳號；
    │                      專案連結帶快取：記憶體 + 資料目錄下的 verhub_cache.json，有效期一天）
    ├── ui/         前端原始碼（Vite + Svelte 5）
    │   └── src/    lib/（純邏輯 + vitest 測試）+ components/（Svelte 元件）
    │                + locales/（三語文案 catalog，以 zh-CN.js 為基準）
    └── dist/       前端建置產物（gitignore；由 ui/ 經 vite build 產生）
```

::: tip common 為什麼無平台相依
`crates/common` 刻意不相依於 Windows API，因此可以跨平台編譯，其純邏輯（設定解析、比對、協定）也更易做單元測試。平台相關程式碼集中在 `crates/core`。
:::

## 資料目錄

設定 `config.json`、記錄檔 `logs/`、復原檔 `recovery.json`、快取 `verhub_cache.json` 共處一個**資料目錄**，由 `crates/common/src/paths.rs` 定位。安裝版與可攜版分開對待：

| 情形 | 資料目錄 | `DataDirKind` |
| --- | --- | --- |
| 安裝版 | `%APPDATA%\BossKey` | `Installed` |
| 可攜版，程式資料夾可寫入 | 程式資料夾 | `Portable` |
| 可攜版，程式資料夾寫不進去 | `%APPDATA%\BossKey` | `PortableFallback` |

可攜版把資料留在程式資料夾，複製走整個資料夾就帶走了全部設定；安裝版則不能這麼做——安裝程式可以裝進 `Program Files`，那裡一般權限程序不可寫入，設定程式每次儲存都會得到 `os error 5`。

### 怎麼分辨是哪一種

看程式資料夾裡有沒有安裝痕跡（`paths::is_installed`）：

1. 安裝程式放的標記檔案 `installed.marker`（`[Files]` 裡裝，解除安裝時隨之移除）；
2. 解除安裝程式 `unins*.exe` —— 兜底，標記檔案被誤刪時仍認得出是安裝版，不至於把資料寫回 `Program Files`。序號隨重複安裝遞增，故按前綴比對。

::: warning 判據必須是檔案，不能是程序權限
核心可能以系統管理員身分執行、設定程式不會：核心在 `Program Files` 下寫得進去，設定程式寫不進去。若兩邊各按自己能否寫入來選資料夾，就會各讀一份設定，使用者改了設定卻不生效。看檔案則兩邊必然一致。也因此，安裝版根本不做可寫性探測——結果一樣是使用者資料夾。
:::

### 回退與移轉

可攜版探測到程式資料夾不可寫入時退回使用者資料夾，`kind` 記為 `PortableFallback`。核心把它寫進記錄檔，設定程式透過 `data_location` 命令讀到後彈出提示，說明這是權限問題以及怎麼改（見 `DataNoticeModal.svelte`）。程式功能不受影響。

用到使用者資料夾時，程式資料夾裡的 `config.json` 會搬過來：先複製，再盡力刪掉原檔案。目標已有設定就不動它——那是目前在用的一份，舊檔案不得覆蓋，也不去刪。刪不掉（沒有寫入權限、檔案被占用）就留在原處，反正不會再被讀到。

::: tip 設定介面的瀏覽器資料另有一處
Tauri 按 `tauri.conf.json` 裡的 identifier 把 WebView2 使用者資料放在 `%LOCALAPPDATA%\cn.hanloth.bosskey.config`，不在資料目錄裡，也不由 `paths.rs` 管。安裝程式的解除安裝程式與可攜版隨附的 `scripts/cleanup.ps1` 都會清理它。
:::

每次啟動的實際資料目錄與判定結果會寫進記錄檔開頭，排查讀寫失敗先看它。

## 核心內部：Agent 訊息迴圈

`agent.rs` 是核心的中樞：它建立一個**隱藏的訊息視窗**並執行 Windows 訊息迴圈，彙整以下事件來源：

- 全域快速鍵（`WM_HOTKEY`）；
- 滑鼠掛鉤（中鍵／側鍵／四角）；
- 具名管道伺服端（來自設定介面的命令）；
- 計時器（閒置偵測、狀態維護等）；
- 視窗事件（`SetWinEventHook`：頂層視窗銷毀／顯示／改標題）；
- 通知區域圖示互動。

訊息迴圈狀態由 `RefCell` 承載：通知區域／懸浮窗選單的強制回應迴圈（`TrackPopupMenu`）會重入 `wndproc`，重入期間的事件借用失敗即被安全丟棄，避免出現兩個可變參考的別名。IPC 執行緒建立具名管道失敗時按退避（1s → 5s → 30s）重試，不會結束。

### 低階輸入掛鉤不與訊息迴圈同執行緒

`WH_MOUSE_LL`／`WH_KEYBOARD_LL` 的回呼由**安裝執行緒的訊息幫浦**派送，且系統的輸入執行緒要等掛鉤鏈返回才繼續投遞事件。若與 agent 同執行緒，列舉視窗、寫入復原檔案、處理全系統視窗事件這類操作會直接拖慢全域滑鼠與鍵盤輸入，單次超過 `LowLevelHooksTimeout`（預設 300ms）時系統還會丟棄該事件。

故 `input_hooks.rs` 單獨啟動一條只跑訊息幫浦的執行緒承載這兩個掛鉤，執行緒優先權提到 above normal，回呼裡只做純記憶體判定與 `PostMessageW`（滑鼠移動這條最熱的路徑上不加鎖，取樣存在不可分割變數裡）。agent 執行緒透過一個僅訊息視窗向它同步下達裝卸請求，並依返回值決定是否回退（鍵盤掛鉤裝不上時「不傳遞」快速鍵退化為 `RegisterHotKey`）。

agent 執行緒本身**不**提優先權：它做的是列舉／凍結／寫入磁碟這類重活，抬高只會從前景程式手裡搶 CPU。

視窗事件驅動隱藏紀錄的即時維護：被隱藏的視窗自行銷毀或被外部復原顯示時，紀錄即刻移除並寫入磁碟；標題變化同步進隱藏紀錄與精確視窗規則（僅記憶體，隨下次正常寫入時落盤），使「標題 + 程序路徑」的追溯與找回始終基於最新資訊。復原時若控制代碼已失效，還會按「程序路徑 + 標題」在目前不可見視窗中嘗試重新找回。

當觸發隱藏／顯示時，交由 `HideController` 編排，流程為「意圖先行」兩段式：`plan_hide` 算出執行計畫（剪掉失效紀錄、補齊 PID）→ 把計畫後的快照寫入 `recovery.json`（先寫入再動手，隱藏中途當機不丟紀錄）→ `commit_hide` 同步隱藏視窗（`SW_HIDE`），並把靜音／凍結／暫停鍵交給副作用專職執行緒（`effects_worker.rs`）按 FIFO 非同步執行——訊息迴圈不被慢操作（音訊列舉、pssuspend 等待）阻塞，快速鍵與介面保持回應。

佇列內的先後有講究：暫停鍵→靜音→靜置→凍結。凍結讓程序徹底停止回應訊息，隱藏若還沒在螢幕上畫完就凍結，被凍結的視窗會留下殘影；發出去的暫停鍵同樣要有時間被目標程式處理掉。故凍結前統一靜置一次（`FREEZE_SETTLE_DELAY`，整批只等一次，沒有要凍結的程序就不等）。靜音不排在這道等待之後——它走音訊工作階段，與目標程序是否在跑無關。

復原（顯示）時逐條校驗紀錄的有效性：控制代碼須仍存在且仍屬於當初的處理程序（`IsWindow` + PID 比對），凍結／靜音紀錄須符合處理程序建立時刻——控制代碼與 PID 都會被系統回收重複使用，校驗不過的紀錄跳過並如實計入日誌。

::: info 可測試性設計
`Effects` 被抽象為 trait，測試時可注入 mock，從而在不真正靜音／凍結系統的情況下驗證隱藏編排邏輯。同理 `WindowManager` 也是 trait。
:::

## 穩定性設計（當機自癒三層防線）

1. **當機記錄**：關鍵事件與 panic 寫入[資料目錄](#資料目錄)下的 `logs/BossKey-YYYY-MM-DD.log`（按日切割，依 `log_retention_days` 保留，0 表示不記錄；release 建置丟棄 DEBUG 級）。
2. **當機復原**：隱藏動作執行前先把「將要隱藏／凍結／靜音什麼」寫入 `recovery.json`（tmp + rename 原子替換），異常結束後重新啟動自動找回；快照帶開機時刻與處理程序建立時刻，跨重新開機的過期快照直接丟棄，不會對無關視窗／處理程序做復原動作。
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
