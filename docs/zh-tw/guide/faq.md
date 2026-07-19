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

## 防毒軟體誤判／攔截怎麼辦？

Boss Key 會監聽全域快速鍵、隱藏視窗，這類行為有時會被防毒軟體誤判。v3 已改用 Rust 原生單一檔案實作，顯著降低了誤判機率。若仍被攔截：

- 將 Boss Key 的**程式資料夾**加入防毒軟體信任區／白名單；
- 從 [官方 Release 頁面](https://github.com/IvanHanloth/Boss-Key/releases) 下載，避免第三方來源。

::: tip 驗證產物來源
官方發布的產物帶有建置來源證明（Sigstore attestation）。進階使用者可用 `gh attestation verify <檔案> -R IvanHanloth/Boss-Key` 驗證產物確實由官方儲存庫建置。
:::

## 按了隱藏快速鍵沒反應？

請依序檢查：

1. **核心是否在執行**？查看通知區域圖示或設定介面底部狀態列。
2. **是否已綁定視窗**？未綁定任何視窗且未開啟「同時隱藏目前使用中的視窗」時，可能沒有可隱藏的目標。見 [綁定視窗與程序](/zh-tw/guide/binding)。
3. **快速鍵是否被佔用**？換一個組合鍵試試。見 [快速鍵設定](/zh-tw/guide/hotkeys)。
4. **是否正處於「快速鍵與滑鼠」設定頁**？該頁會暫時暫停監聽，離開後復原。

## 視窗被隱藏後顯示不回來了？

使用 [視窗復原工具](/zh-tw/guide/recovery)（其他選項 → 工具）勾選並復原。若開啟了「隱藏通知區域圖示」，請用您的**復原快速鍵**復原。

## 增強凍結的開關是灰的，按不了？

增強凍結需要同時滿足三個條件，缺一即被停用：

1. 已開啟「隱藏視窗時凍結程序」；
2. 核心**以系統管理員身分執行**；
3. 程式資料夾下有 `pssuspend64.exe`。

設定介面會提示目前缺少哪一項。詳見 [程序凍結](/zh-tw/guide/freeze)。

## 更新後設定會遺失嗎？

不會。`config.json` 結構與舊版**完全相容**，更新後您的綁定、快速鍵、選項都會保留。v2 版的扁平綁定也會自動移轉到新的規則格式。

## Boss Key 支援哪些系統？

Windows 10 以上開箱即用；Windows 7 需自行確保 WebView2 可用才能開啟設定介面。部分版本提供 Windows 7 軟體包可直接在 Win7 版本中使用。目前不支援 macOS／Linux。

## 還有其他問題？

- 查閱本使用說明的對應章節；
- 前往 GitHub 提交 [Issue](https://github.com/IvanHanloth/Boss-Key/issues)；
- 在設定介面的 [關於與意見回饋](/zh-tw/guide/update) 頁提出意見回饋。
