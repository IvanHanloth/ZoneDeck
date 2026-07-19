---
title: 貢獻指南
---

# 貢獻指南

感謝您願意為 Boss Key 貢獻程式碼！為了高效、安全地協作，請在提交前閱讀本頁的要求。

## 貢獻流程

1. **Fork 儲存庫**並複製到本機，或（有權限時）在儲存庫內建立分支。
2. 從 `dev` 分支切出**功能分支**進行開發（分支命名見下文）。
3. 完成開發並**通過本機測試與檢查**（見下文「提交前檢查清單」）。
4. 向 **`dev` 分支**發起 Pull Request，關聯相關 Issue／Project。
5. 經過**程式碼審查**後合併。正式發布時再由 `dev` 統一合併至 `main`。

分支與合併的完整策略見 [專案管理策略](/zh-tw/dev/project-management)。

## 分支命名

分支命名沒有強制規範，但為便於維護，**建議使用 `類型/功能` 的形式**：

| 類型前綴 | 用途 | 範例 |
| --- | --- | --- |
| `feat/` | 新功能 | `feat/checkUpdate` |
| `fix/` | Bug 修復 | `fix/hideWindow` |
| `refactor/` | 重構 | `refactor/agent` |
| `doc/` | 文件 | `doc/init` |
| `chore/` | 雜項／建置 | `chore/ci` |

::: tip 文件相關分支
所有**文件相關**的改動請使用 `doc` 開頭的分支（如 `doc/guide-freeze`）。文件站的部署工作流程會在推送到此類分支時自動建置預覽／部署，詳見 [文件站維護](/zh-tw/dev/docs-site)。
:::

## 提交前檢查清單

提交 PR 前，請確保本機通過以下檢查（與 CI 一致）：

```bash
# 1. 格式
cargo fmt --all -- --check

# 2. 靜態檢查（warning 視為 error）
cargo clippy --workspace --all-targets -- -D warnings

# 3. 前端測試與建置
npm --prefix apps/config/ui test
npm --prefix apps/config/ui run build

# 4. Rust 測試
cargo test --workspace

# 5. 生產編譯能通過
cargo build --release
```

::: warning 版本號一致性
若您改動了版本號，務必保證 `Cargo.toml`、`tauri.conf.json`、`ui/package.json`、`Cargo.lock` **四處一致**。CI 會用 `scripts/version.ps1 check` 驗證。日常功能開發一般**不要**手動改版本號——版本號由發布流程統一管理，詳見 [打包與發布](/zh-tw/dev/release)。
:::

## 程式碼風格

- **Rust**：遵循 `rustfmt` 預設風格；`clippy` 零警告。
- **前端**：Svelte 5 + 現代 JS；純邏輯放入 `ui/src/lib/` 並配套 `vitest` 測試，UI 放入 `ui/src/components/`。
- **提交訊息**：建議使用類似 `feat: …`、`fix: …`、`doc: …`、`refactor: …` 的前綴，與分支類型呼應。

## 測試要求

- 新增／修改**純邏輯**（設定、比對、協定、快速鍵解析、前端 lib 等）應補充單元測試。
- 涉及視窗／凍結／IPC 等系統行為的改動，盡量補充對應的整合測試或說明手動驗證步驟。
- 詳見 [測試策略](/zh-tw/dev/testing)。

## 使用者可見文案

新增任何使用者可見文案都必須補齊三種語言，否則測試不通過：

- **設定介面**：先在 `apps/config/ui/src/locales/zh-CN.js`（文案基準）加鍵，再同步 `en.js` 與 `zh-TW.js`。
- **核心**（通知區域選單／通知／IPC 錯誤）：在 `crates/core/src/i18n.rs` 的 `Msg` 列舉加變體，補齊三種語言，並登記到測試模組的 `ALL_MSGS`。
- 繁體中文使用台灣用語（如「視窗」「程式」「檔案」「滑鼠」「快速鍵」），不要只做簡繁字形轉換。
- **記錄檔不翻譯**，一律使用簡體中文。

## 提交 Issue

儲存庫提供了 Issue 範本：

- **Bug 回報**：請附版本號、系統版本、重現步驟與記錄檔。
- **功能建議**：請描述使用情境與期望行為。

盡量將 **Issue–Pull Request–Project** 三者關聯，方便統一管理（見 [專案管理策略](/zh-tw/dev/project-management)）。

## 授權

本專案基於 **MIT** 授權條款發布。提交貢獻即表示您同意您的程式碼以相同授權條款發布。
