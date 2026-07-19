---
title: 打包與發布
---

# 打包與發布

本章介紹如何在本機打包產物，以及基於 GitHub Actions 的自動化發布流程。

## 本機一鍵打包

```powershell
# 可攜資料夾：前端建置 → cargo release → 組裝 dist/
powershell -File scripts/package.ps1

# 追加產生 InnoSetup 安裝包（需 Inno Setup）
powershell -File scripts/package.ps1 -Installer

# 重複使用已有前端產物（前端沒改時提速）
powershell -File scripts/package.ps1 -SkipFrontend
```

`package.ps1` 的流程：編譯前端（Vite + Svelte）→ 生產編譯 Rust workspace（Tauri 建置指令碼會把前端 `dist` 內嵌進 `bosskey-config.exe`）→ 組裝可攜資料夾 → 選擇性產生安裝包。

### 產物結構

可攜版與安裝包各佔 `dist/` 下的一個子資料夾，互不干擾：

```
dist/
├── Boss-Key/                    可攜版（複製走即可用，發布時整個資料夾壓成 zip）
│   ├── Boss Key.exe               常駐核心（內嵌 DPI／長路徑 manifest + 版本資訊 + 圖示）
│   ├── config.exe                 設定介面（前端已內嵌，自包含）
│   ├── LICENSE.txt
│   └── README.md
└── installer/                   安裝包（-Installer 時產生）
    └── Boss-Key-<版本>-Setup.exe  InnoSetup（安裝前自動結束執行中的核心）
```

可攜版**不需安裝、無外部相依**（除系統內建的 WebView2）。兩個程式透過同資料夾的 `config.json` 與具名管道協作。

## 版本號管理

::: info 版本號唯一真實來源
版本號的唯一真實來源是 `Cargo.toml` 的 `[workspace.package] version`。另外三處必須與之一致：`apps/config/src-tauri/tauri.conf.json`、`apps/config/ui/package.json`、`Cargo.lock`。
:::

`scripts/version.ps1` 負責寫入與驗證：

```powershell
# 把版本號寫入四處檔案（並同步 Cargo.lock）
powershell -File scripts/version.ps1 apply 3.0.1

# 驗證四處與該 tag 一致，不一致則失敗
powershell -File scripts/version.ps1 check 3.0.1

# 不給 tag 時以 Cargo.toml 為基準驗證其餘檔案
powershell -File scripts/version.ps1 check

# 印出目前版本號
powershell -File scripts/version.ps1 show
```

支援 semver（如 `3.0.1`、`3.1.0-rc.1`，可帶前導 `v`）。帶 `-` 的版本視為**預先發行**。安裝包要求純數字四段號，會自動換算（`3.1.0-rc.1` → `3.1.0.0`）。

## CI/CD 工作流程

工作流程位於 `.github/workflows/`。

### `build-test.yml` — 建置與測試

**觸發**：任意分支的 PR／push（tag 除外），以及手動觸發。

**做什麼**：版本號一致性檢查 → 前端安裝／測試／建置 → `cargo fmt --check` → `cargo clippy` → `cargo test` → `cargo build --release` → 上傳二進位檔供檢查。

同一分支有新推送時會自動取消舊工作（`concurrency` + `cancel-in-progress`）。

### `tag.yml` — 版本號寫入並打 tag

**觸發**：手動（`workflow_dispatch`），輸入要發布的版本號。

**做什麼**：
1. 用 `version.ps1 apply` 把版本號寫入四處檔案；
2. 提交並推回目前分支，打上 `v<版本>` tag 並推送；
3. 呼叫 `release.yml` 建置並發布。

### `release.yml` — 建置並發布 Release

**觸發**：推送 `v*` tag，或被 `tag.yml` 透過 `workflow_call` 呼叫。

**做什麼**：驗證 tag 與程式碼版本一致 → 前端／Rust 測試 → `package.ps1 -Installer` 組裝 `dist/Boss-Key` 與 `dist/installer` → 把 `dist/Boss-Key` 壓成可攜 zip → 產生**建置來源證明**（Sigstore attestation）→ 建立 GitHub Release 並上傳 zip 與安裝包。

### 發布一個新版本

建議透過 `tag.yml` 手動觸發：

1. 在 GitHub Actions 中執行 **「Bump version and tag」**，填入版本號（如 `3.0.1`）。
2. 工作流程自動寫版本號、打 tag、建置並發布 Release。

也可以在本機手動打 tag 推送來直接觸發 `release.yml`，但需自行保證版本號已一致。
