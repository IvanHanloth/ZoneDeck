---
title: 常見問題
---

# 常見問題（FAQ）

## 為什麼我的電腦執行不了設定程式？

v3 的設定程式（`config.exe`）使用 Tauri 撰寫，其執行相依於系統的 **WebView2** 執行階段。

部分精簡版系統或較低版本系統（Windows 7 以下）可能預設不含 WebView2，因此無法開啟設定介面。解決方法：

1. 手動安裝微軟 Edge WebView2：<https://developer.microsoft.com/zh-tw/microsoft-edge/webview2>
2. 或下載帶有 **`win7`** 標示的軟體包（部分版本提供）。

::: info
設定介面無法開啟**不影響核心的隱藏功能**——核心是純原生程式，不相依於 WebView2。您仍可用已設定好的快速鍵正常隱藏視窗。
:::

## 復原時會把已「關閉」到通知區域的程式彈出來嗎？

不會。隱藏時只記錄**當時可見**的視窗，復原只逆轉 ZoneDeck 自己的隱藏動作；程式自行藏到通知區域的視窗（如 Steam 的關閉按鈕只是隱藏視窗）不受影響，復原時不會被彈出。

## 防毒軟體誤判／攔截怎麼辦？

ZoneDeck 會監聽全域快速鍵、隱藏視窗，這類行為有時會被防毒軟體誤判。v3 已改用 Rust 原生單一檔案實作，顯著降低了誤判機率。若仍被攔截：

- 將 ZoneDeck 的**程式資料夾**加入防毒軟體信任區／白名單；
- 從 [官方 Release 頁面](https://github.com/IvanHanloth/ZoneDeck/releases) 下載，避免第三方來源。

::: tip 驗證產物來源
官方發布的產物帶有建置來源證明（Sigstore attestation）。進階使用者可用 `gh attestation verify <檔案> -R IvanHanloth/ZoneDeck` 驗證產物確實由官方儲存庫建置。
:::

## 按了隱藏快速鍵沒反應？

請依序檢查：

1. **核心是否在執行**？查看通知區域圖示或設定介面底部狀態列。
2. **是否已綁定視窗**？未綁定任何視窗且未開啟「同時隱藏目前使用中的視窗」時，可能沒有可隱藏的目標。見 [綁定視窗與程序](/zh-tw/guide/binding)。
3. **快速鍵是否被佔用**？換一個組合鍵試試，或為該快速鍵開啟 [不傳遞](/zh-tw/guide/hotkeys#快速鍵不傳遞)——改用鍵盤掛鉤後不受組合鍵佔用衝突的影響。見 [快速鍵設定](/zh-tw/guide/hotkeys)。
4. **是否正處於「快速鍵與滑鼠」設定頁**？該頁會暫時暫停監聽，離開後復原。

## 視窗被隱藏後顯示不回來了？

使用 [視窗復原工具](/zh-tw/guide/recovery)（通用設定 → 工具）勾選並復原。若開啟了「一併隱藏 ZoneDeck 通知區域圖示」，請用您的**復原快速鍵**復原。

## 能隱藏其他程式的通知區域圖示嗎？

ZoneDeck 只能隱藏[自身的通知區域圖示](/zh-tw/guide/hiding)，無法操作其他程式的通知區域圖示。可使用 Windows 內建的功能手動設定：詳細步驟參見微軟官方教學 [在 Windows 中自訂工作列 · 系統匣](https://support.microsoft.com/zh-tw/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray)，或直接開啟 [工作列設定](ms-settings:taskbar)（該連結僅在 Windows 上有效），選擇哪些圖示顯示在工作列角落。

## 增強凍結的開關是灰的，按不了？

增強凍結需要同時滿足三個條件，缺一即被停用：

1. 已開啟「隱藏視窗時凍結程序」；
2. 核心**以系統管理員身分執行**；
3. 程式資料夾下有 `pssuspend64.exe`。

設定介面會提示目前缺少哪一項。詳見 [程序凍結](/zh-tw/guide/freeze)。

## 提示「儲存設定失敗」（拒絕存取／os error 5）怎麼辦？

先升級到 v3.1.0 或更高版本。舊版本把設定固定存在程式所在資料夾，而選了「為所有使用者安裝」時程式裝在 `C:\Program Files`，一般權限寫不進去，於是每次改設定都儲存失敗。新版本的安裝版一律把設定存到 `%APPDATA%\ZoneDeck`，並把已有的 `config.json` 移轉過去，不需手動處理。可攜版仍存在程式資料夾裡，若那裡不可寫入也會自動改用 `%APPDATA%\ZoneDeck` 並彈出說明。

升級後仍然報錯，多半是另外兩種情況：

- **被防毒軟體攔截**：把 ZoneDeck 的程式資料夾與 `%APPDATA%\ZoneDeck` 加入防毒軟體信任區。Windows 安全性中心的「受控資料夾存取權」也會以同樣的方式攔截寫入。
- **設定檔被設為唯讀**：在檔案總管中右鍵 `config.json` → 內容，取消「唯讀」。

錯誤訊息裡帶有實際路徑，據此可判斷問題出在哪個資料夾。

## 更新後設定會遺失嗎？

不會。`config.json` 結構與舊版**完全相容**，更新後您的綁定、快速鍵、選項都會保留。v2 版的扁平綁定也會自動移轉到新的規則格式。

## ZoneDeck 支援哪些系統？

Windows 10 以上開箱即用；Windows 7 需自行確保 WebView2 可用才能開啟設定介面。部分版本提供 Windows 7 軟體包可直接在 Win7 版本中使用。目前不支援 macOS／Linux。

## 還有其他問題？

- 查閱本使用說明的對應章節；
- 前往 GitHub 提交 [Issue](https://github.com/IvanHanloth/ZoneDeck/issues)；
- 在設定介面的 [關於與意見回饋](/zh-tw/guide/update) 頁提出意見回饋。
