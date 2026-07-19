---
title: 開發文件導覽
---

# 開發文件

歡迎參與 Boss Key 的開發！本部分文件面向**開發者與貢獻者**，介紹如何在本機執行專案程式碼、參與貢獻的要求、專案管理策略，以及系統架構。

::: tip
如果您只是想使用 Boss Key，請閱讀 [使用說明](/zh-tw/guide/)。
:::

## 技術堆疊概覽

v3 版本是一次徹底的重構，核心技術選型如下：

| 部分 | 技術 |
| --- | --- |
| 常駐核心 | **Rust**（純原生，直接呼叫 Windows API） |
| 設定介面後端 | **Tauri 2**（Rust） |
| 設定介面前端 | **Svelte 5 + Vite** |
| 程序間通訊 | **具名管道**（一行一條 JSON） |
| 工程組織 | **Cargo workspace** + npm 前端子專案 |
| 打包 | PowerShell 指令碼 + Inno Setup |
| CI/CD | GitHub Actions |

## 設計目標

v3 重寫的核心目標：

- **更低記憶體**：核心常駐二進位檔約 350 KB，背景記憶體約 1 MB。
- **更穩定**：當機記錄 + 當機復原 + 監控程式三層防線。
- **單一檔案原生二進位檔**：不相依於 Python 執行階段，降低防毒軟體誤判。
- **更現代的設定介面**：無邊框、佈景主題切換、設定自動儲存。

## 閱讀順序建議

1. [本機執行](/zh-tw/dev/getting-started) —— 環境準備與常用命令，先把專案跑起來。
2. [系統架構](/zh-tw/dev/architecture) —— 理解雙程序 + 具名管道的整體設計。
3. [前端與設定介面](/zh-tw/dev/frontend) —— Svelte／Tauri 部分的選型與結構。
4. [貢獻指南](/zh-tw/dev/contributing) 與 [專案管理策略](/zh-tw/dev/project-management) —— 參與協作前必讀。
5. [測試策略](/zh-tw/dev/testing) 與 [打包與發布](/zh-tw/dev/release) —— 保證品質與交付。

參考資料：[設定檔欄位](/zh-tw/dev/config-reference)、[IPC 協定](/zh-tw/dev/ipc-protocol)。

## 儲存庫位址

<https://github.com/IvanHanloth/Boss-Key>
