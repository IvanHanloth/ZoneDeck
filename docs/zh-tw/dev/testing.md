---
title: 測試策略
---

# 測試策略

ZoneDeck 涵蓋了從純邏輯單元測試到系統層級整合測試的多層測試。CI 會在每次 PR／推送時執行全部測試。

## 執行測試

```bash
# 全部 Rust 測試
cargo test --workspace

# 前端單元測試
npm --prefix apps/config/ui test
```

::: warning 核心並行測試可能因 COM 當機
`zonedeck-core` 的部分測試涉及 COM（如靜音鏈路）初始化，多執行緒並行執行可能當機。遇到此類問題時改用單執行緒：

```bash
cargo test -p zonedeck-core -- --test-threads=1
```
:::

## Rust 測試涵蓋範圍

### `zonedeck-common`

模型／設定／比對／協定的**純邏輯單元測試**，以及設定檔讀寫的整合測試。例如：

- 設定解析與預設值、舊設定移轉（扁平綁定 → 視窗規則、舊滑鼠開關 → 連按觸發）；
- `PID` 大寫欄位相容、正規表示式規則來回序列化；
- 連按次數／連按時間的範圍鉗制；
- 協定 `Command`／`Response` 的 round-trip 與 snake_case 標籤；
- 語言標籤解析與偏好正規化（`zh-Hant` → `zh-TW`、無翻譯的語言回落 `auto` 等）；
- 資料目錄定位與移轉（可攜版就地／安裝版走使用者資料夾／不可寫入時回退並報明原因／標記檔案與解除安裝程式的辨識／舊設定搬過去、原檔案刪不掉、目標已有設定不覆蓋）；
- 設定寫入的原子性（寫入失敗不截斷原檔案、不留臨時檔案，錯誤訊息帶路徑）。

### `zonedeck-core`

系統行為的整合測試（多為建立真實資源來回驗證）：

| 領域 | 測試內容 |
| --- | --- |
| 視窗列舉／隱藏／顯示 | 建立真實視窗來回 |
| 快速鍵解析 | 字串 → RegisterHotKey 參數 |
| 快速鍵攔截 | 鍵盤掛鉤判定的純邏輯：修飾鍵完全吻合、長按只觸發一次、按下被吞則放開也吞 |
| 單一執行個體互斥鎖 | 具名互斥鎖 |
| 具名管道 | 伺服端收發；連開連關的重連競態（用戶端搶在 `ConnectNamedPipe` 之前連上仍須正常應答） |
| 程序凍結 | 真實子程序暫停／復原 |
| 靜音 | Core Audio COM 鏈路 |
| HideController | mock 注入驗證靜音／凍結／暫停鍵編排 |
| 開機自動啟動 | 一般登錄檔機碼邏輯 + 真實 `schtasks` 接受工作 XML |
| 當機記錄 | 寫入／輪換／panic 掛鉤 |
| 當機復原 | 快照寫入磁碟來回 + mock 復原編排 |
| 圖示編碼 | CRC32／Adler-32／base64 已知向量 + PNG 結構解析 + 真實 explorer.exe 擷取 |
| 端對端 IPC | 經 `PipeClient` 驅動真實 agent 的 IPC／當機復原測試 |
| 文案 catalog | 三種語言逐條非空、英文與中文不重複、佔位符跨語言一致 |

::: info 受限環境說明
`schtasks` 相關的開機自動啟動整合測試在受限／無權限的 CI 或沙箱環境中可能失敗，這屬於環境限制而非功能迴歸。本機具備權限時應能通過。
:::

## 前端測試（vitest）

前端把純邏輯抽到 `ui/src/lib/`，並配套 `vitest` 單元測試：

- `hotkey.test.js` —— 快速鍵解析／格式化；
- `pointer.test.js` —— 滑鼠連按／連按時間；
- `grouping.test.js` —— 視窗／程序規則增刪與篩選；
- `theme.test.js` —— 佈景主題偏好邏輯；
- `i18n.test.js` —— 語言解析，以及三份 catalog 的鍵集／佔位符對齊。

UI 元件（`components/`）保持薄邏輯，主要複雜度都下沉到可測試的 `lib/`。

## 撰寫測試的約定

- **純邏輯優先單元測試**：任何可抽成純函式的邏輯都應放入 `common` 或前端 `lib/` 並做單元測試。
- **系統行為整合測試**：涉及視窗／COM／管道等，建立真實資源來回驗證，必要時用 mock（`Effects`／`WindowManager` trait）隔離副作用。
- 提交前請確保本機測試通過，具體見 [貢獻指南](/zh-tw/dev/contributing)。
