---
title: 前端與設定介面
---

# 前端與設定介面

設定介面（`config.exe`）由 **Tauri 2（Rust 後端）+ Svelte 5（前端）** 構成。本章介紹其技術選型、結構與關鍵設計。

## 為什麼選擇 Svelte

設定介面是一個**表單密集型的小應用程式**，經過選型比較後採用 **Svelte 5 + Vite**：

- **雙向繫結是核心訴求**：Svelte 的 `bind:` 原生支援雙向繫結（React 無雙向繫結，需手寫受控元件；Vue 的 `v-model` 可用但執行階段更大）。
- **編譯期反應式、無虛擬 DOM**：建置產物僅約 80 KB（gzip 約 28 KB），契合本專案「體積極小」的目標。
- **Tauri 官方一等支援**。

## 前端工程結構

```
apps/config/ui/
├── src/
│   ├── App.svelte           頂層：標籤頁 + 自動儲存 + 啟動編排
│   ├── main.js              進入點（掛載前先套用一次佈景主題，避免閃爍）
│   ├── components/          UI 元件（TitleBar、各設定面板、各對話方塊…）
│   └── lib/                 純邏輯（可單元測試）
│       ├── state.svelte.js  全域狀態（$state）+ 各類動作
│       ├── ipc.js           呼叫 Tauri 命令／監聽事件
│       ├── hotkey.js        快速鍵解析/格式化（配 hotkey.test.js）
│       ├── pointer.js       滑鼠連按/連按時間常數（配 pointer.test.js）
│       ├── grouping.js      視窗/程序規則的增刪與篩選（配 grouping.test.js）
│       ├── theme.js         佈景主題切換（配 theme.test.js）
│       ├── i18n.svelte.js   介面語言：catalog 查表 + 語言解析（配 i18n.test.js）
│       └── verhub.js        檢查更新/公告/開啟外部連結
├── locales/                 三語文案 catalog（zh-CN.js / en.js / zh-TW.js）
├── vite.config.js
└── svelte.config.js
```

設定介面以標籤頁組織，對應五個面板元件：

| 標籤 | 元件 | 內容 |
| --- | --- | --- |
| 視窗綁定 | `BindingPanel` | 視窗清單 + 視窗/程序規則 |
| 快速鍵與滑鼠 | `HotkeysPanel` | 鍵盤快速鍵、滑鼠連按、四角、閒置自動隱藏 |
| 通知設定 | `NotificationsPanel` | 逐事件通知開關 |
| 其他選項 | `OptionsPanel` | 靜音/暫停/凍結/權限/記錄檔/工具 |
| 關於與意見回饋 | `AboutPanel` | 版本、更新、公告、意見回饋 |

## 無邊框自繪視窗

Tauri 設定 `decorations: false`，標題列由前端自繪：

- `data-tauri-drag-region` 實現拖曳、連按兩下最大化；
- 自繪最小化／最大化／關閉按鈕；
- 八向邊緣 `startResizeDragging` 縮放熱區（最大化時自動停用）；
- 淺色／深色／跟隨系統三態佈景主題（`localStorage` 保存 + `prefers-color-scheme` 監聽）。

## 設定自動儲存

`App.svelte` 中用 `$effect` 深度追蹤設定物件（含 `window_rules`／`process_rules`）的任何變化，停頓後經 `scheduleSave`（內部 debounce）自動寫入磁碟。載入階段不觸發儲存，`loadAll` 完成後才「啟用」自動儲存。

儲存後前端透過 IPC 通知核心 `reload_config`，核心熱重新載入設定。

## 介面語言

文案集中在 `src/locales/`，每個語言一個扁平鍵值表；`lib/i18n.svelte.js` 負責查表與語言解析。

```js
import { t } from "../lib/i18n.svelte.js";

t("common.close");                    // → 關閉
t("restore.frozen", { n: 3 });        // → 已凍結 3 個程序
```

- `t()` 讀取 `$state` 裡的目前語言，因此在範本中呼叫它即可在切換語言時自動重新算繪，不需手動訂閱。
- **`zh-CN.js` 是文案基準**：新增文案先寫它，再同步 `en.js` 與 `zh-TW.js`。`i18n.test.js` 會驗證三份 catalog 的鍵集與佔位符完全一致，漏譯即測試失敗。
- 缺少的鍵回落到簡體中文，仍缺少則傳回鍵本身，便於開發期發現漏譯。
- 目前語言取自 `setting.language`；`auto` 時按 `navigator.language` 推斷。`App.svelte` 用 `$effect` 追蹤該欄位，變更即時換文案，並同步寫 `<html lang>`（影響字型回落與斷行）。
- `lib/` 裡的純邏輯模組（`pointer.js`、`grouping.js`、`theme.js`）也經由 `t()` 取文案；因預設語言為簡體中文，它們的既有單元測試不需改動。

::: warning NO_TITLE 不是文案
`grouping.js` 的 `NO_TITLE`（`"无标题窗口"`）與核心 `bosskey_common::NO_TITLE` 一致，是寫進 `config.json` 的跨程序哨兵值，**不可翻譯**；僅在顯示時用 `t("common.noTitleWindow")` 換成目前語言。
:::

## 狀態輪詢不卡介面

- 狀態取得合併為**單一 `GetStatus`** 管道命令，並使用**連線快速失敗**（不重試）。
- Tauri 命令用 `spawn_blocking` 非同步化，避免阻塞。
- 前端每 2 秒輪詢一次狀態（頁面不可見時暫停）。

## 前端不相依於 dev server

最終產物中，前端在**編譯期被內嵌**進 `bosskey-config.exe`，靜態執行，**不需要任何本機伺服器**。

- 開發前端 UI：`npm run dev` 在瀏覽器裡預覽（mock 資料、熱重新載入）。
- 驗證 Tauri 整合：`npm run build` 後 `cargo run -p bosskey-config`。

## 與核心的連動範例

- 核心通知區域的「視窗復原工具」／「關於」會帶參數開啟設定介面：冷啟動從啟動參數讀取，已執行時由單一執行個體外掛程式傳來事件。
- 首次啟動若核心未執行，設定介面會自動 `startCore` 啟動核心。
