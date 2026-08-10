---
title: 本機執行
---

# 本機執行

本章介紹如何在本機建置開發環境、執行核心與設定介面，以及日常開發用到的命令。

## 環境準備

| 相依項目 | 版本／說明 | 取得方式 |
| --- | --- | --- |
| **Rust** | stable，建議 1.85+（專案使用 edition 2024） | <https://rustup.rs> |
| **Node.js** | 18+，建議 24（用於前端建置） | <https://nodejs.org> |
| **WebView2** | 設定介面執行階段（Win10/11 通常已內建） | 系統內建／[微軟官網](https://developer.microsoft.com/zh-tw/microsoft-edge/webview2) |
| **Inno Setup 6** | *選用*，本機產生安裝包時需要 | `winget install JRSoftware.InnoSetup` |
| **pssuspend64.exe** | *選用*，測試增強凍結時需要 | [Microsoft PSTools](https://download.sysinternals.com/files/PSTools.zip) |

::: tip 設定介面不相依於 dev server
前端在編譯期被**內嵌**進 `zonedeck-config.exe`，最終產物靜態執行。開發前端時用 `npm run dev` 在瀏覽器裡預覽（mock 資料、熱重新載入），改完 `npm run build` 後用 `cargo run -p zonedeck-config` 驗證 Tauri 整合。
:::

## 複製專案

```bash
git clone https://github.com/IvanHanloth/Boss-Key.git
cd Boss-Key
```

## 常用命令

在**儲存庫根目錄**執行：

### 核心（zonedeck-core）

```bash
# 執行核心（開發）
cargo run -p zonedeck-core

# 核心冒煙自測：N 毫秒後自動結束
cargo run -p zonedeck-core -- smoke 3000
```

### 前端（apps/config/ui）

```bash
# 首次：安裝前端相依套件
npm --prefix apps/config/ui install

# 前端建置（產物輸出到 apps/config/dist，供 Tauri 內嵌）
npm --prefix apps/config/ui run build

# 前端單元測試（vitest）
npm --prefix apps/config/ui test

# 瀏覽器預覽（mock 資料、熱重新載入）
npm --prefix apps/config/ui run dev
```

### 設定介面（zonedeck-config，Tauri）

```bash
# 執行設定介面（需先建置前端）
npm --prefix apps/config/ui run build && cargo run -p zonedeck-config
```

### 品質檢查與測試

```bash
# 執行全部 Rust 測試
cargo test --workspace

# 靜態檢查（把 warning 當 error）
cargo clippy --workspace --all-targets -- -D warnings

# 程式碼格式化／僅檢查
cargo fmt --all
cargo fmt --all -- --check
```

### 生產編譯與打包

```bash
# 生產編譯（體積最小化）
cargo build --release

# 一鍵生產打包（前端 + Rust + 可攜資料夾 dist/ZoneDeck）
powershell -File scripts/package.ps1

# 一鍵打包 + 安裝包（dist/installer，首次會自動安裝 Inno Setup）
powershell -File scripts/package.ps1 -Installer
```

更多打包細節見 [打包與發布](/zh-tw/dev/release)。

## 命令速查表

| 目的 | 命令 |
| --- | --- |
| 執行核心 | `cargo run -p zonedeck-core` |
| 核心冒煙自測 | `cargo run -p zonedeck-core -- smoke 3000` |
| 前端安裝相依套件 | `npm --prefix apps/config/ui install` |
| 前端建置 | `npm --prefix apps/config/ui run build` |
| 前端測試 | `npm --prefix apps/config/ui test` |
| 前端瀏覽器預覽 | `npm --prefix apps/config/ui run dev` |
| 執行設定介面 | `npm --prefix apps/config/ui run build && cargo run -p zonedeck-config` |
| 生產編譯 | `cargo build --release` |
| 全部 Rust 測試 | `cargo test --workspace` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| 格式檢查 | `cargo fmt --all -- --check` |
| 一鍵打包 | `powershell -File scripts/package.ps1` |
| 打包 + 安裝包 | `powershell -File scripts/package.ps1 -Installer` |

## 常見開發問題

::: warning 防毒軟體攔截新編譯的執行檔
若表現為 `os error 5 拒絕存取`，通常是防毒軟體鎖住了剛編譯出的 exe。請將專案的 `target` 資料夾加入防毒軟體信任區。
:::

::: tip 核心測試需單執行緒執行
部分核心測試涉及 COM 初始化，多執行緒並行可能導致當機。若遇到此類問題，使用 `cargo test -p zonedeck-core -- --test-threads=1` 單執行緒執行。詳見 [測試策略](/zh-tw/dev/testing)。
:::
