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

**觸發**：手動（`workflow_dispatch`），輸入要發布的版本號。**請從 `dev` 分支觸發**。

**做什麼**：
1. 用 `version.ps1 apply` 把版本號寫入四處檔案；
2. 提交到 `dev` 並打上 `v<版本>` tag，兩者一起推送；
3. 確保有一個 `dev` → `main` 的 PR（已存在就沿用，沒有才新開）。

**這一步不建置**。用 GITHUB_TOKEN 推的 tag 不會觸發任何工作流程，正合本流程的意圖：等 PR 合併、tag 隨之進入 `main` 的歷史，`release.yml` 才開始生產建置。

::: warning 用 merge commit 合併發版 PR
tag 指向 `dev` 上的那個版本號提交。**squash／rebase 合併會另造提交**，tag 就進不了 `main` 的歷史，`release.yml` 偵測不到，**建置根本不會觸發**。請用 **merge commit**。

真的誤用了 squash，可以手動觸發 `release.yml` 並指定 tag 來補救。
:::

::: tip PR 上沒有檢查記錄？
GITHUB_TOKEN 建立的 PR 不會觸發 `pull_request` 事件，PR 頁面上不會有 CI 記錄（您點 Merge 時的 push 事件仍會正常觸發 `build-test.yml`）。若 `main` 的分支保護要求狀態檢查通過，請在儲存庫 secrets 中設定 `RELEASE_PAT`（repo 權限的 PAT），工作流程會優先使用它來開 PR。
:::

### `release.yml` — 建置並發布 Release

**觸發**：推送到 `main`（偵測到有新的 `v*` tag 隨之進入 `main` 的歷史才繼續），或手動觸發並指定 tag。

**做什麼**：偵測本次推送新帶進 `main` 的 tag → 檢出該 tag → 驗證 tag 與程式碼版本一致 → 前端／Rust 測試 → `package.ps1 -Installer` 組裝 `dist/Boss-Key` 與 `dist/installer` → 把 `dist/Boss-Key` 壓成可攜 zip → 產生**建置來源證明**（Sigstore attestation）→ 建立**草稿** Release 並上傳 zip 與安裝包。

::: info 為什麼不監聽 `push: tags`
tag 是 `tag.yml` 用 GITHUB_TOKEN 推到 `dev` 的，那次推送不會觸發任何工作流程。而**合併 PR 並不產生 tag 推送事件**——tag 是獨立的 ref，合併只是讓它指向的提交變得可從 `main` 追溯。所以只能從 `main` 的 push 事件裡偵測。

偵測方式是比較推送前後「可從 `main` 追溯的 `v*` tag」集合，取新增的那個。不能用「HEAD 上掛著的 tag」：merge commit 才是 HEAD，tag 指向的是它的父提交。
:::

### `deploy-docs.yml` — 文件站部署

**觸發**：`main` / `dev` 上涉及 `docs/` 的推送、Release 的發布／編輯／刪除，以及手動觸發。

**做什麼**：[i18n 一致性檢查](/zh-tw/dev/contributing#i18n-一致性檢查) → 重新產生 `docs/public/releases.json`（舊版用戶端的更新檢查資料來源，只收錄**非草稿** Release）→ VitePress 建置 → 部署到 GitHub Pages。

::: warning Release 要點了「Publish」才會刷新文件
`release.yml` 建的是**草稿** Release，草稿不發 `published` 事件。必須在網頁上點 **Publish release**，才會觸發文件站重新部署、把這個版本寫進 `releases.json`。
:::

### 發布一個新版本

```
dev ──① Bump version and tag──▶ dev（版本號提交 + v3.0.1 tag）
                                 │
                                 ②  PR，用 merge commit 合併
                                 ▼
                               main ──③ release.yml 偵測到新 tag──▶ 建置 + 草稿 Release
                                                                        │
                                                                        ④ Publish
                                                                        ▼
                                                              文件站 + releases.json 刷新
```

1. 功能開發完畢、準備發版時，切到 `dev`，在 GitHub Actions 中執行 **「Bump version and tag」**，填入版本號（如 `3.0.1`）。工作流程把版本號提交與 tag 落在 `dev`，並確保有一個 `dev` → `main` 的 PR。
2. 審查該 PR，用 **merge commit** 合併進 `main`。
3. 合併觸發 `release.yml`：它偵測到 `v3.0.1` 隨之進入 `main`，檢出該 tag 開始生產建置，完成後留下一個**草稿** Release。
4. 檢查產物與發布說明，點 **Publish release** 正式發布 —— 這一步同時會刷新文件站與 `releases.json`。

需要重跑或補發時，手動觸發 `release.yml` 並指定 tag 即可。
