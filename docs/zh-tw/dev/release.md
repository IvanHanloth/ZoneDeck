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
│   ├── cleanup.ps1                殘留資料清理指令碼（可攜版沒有解除安裝程式）
│   ├── LICENSE.txt
│   ├── README.md                  簡體中文
│   ├── README.en.md               English
│   └── README.zh-TW.md            繁體中文
└── installer/                   安裝包（-Installer 時產生）
    └── Boss-Key-<版本>-Setup.exe  InnoSetup（安裝前自動結束執行中的核心）
```

可攜版**不需安裝、無外部相依**（除系統內建的 WebView2）。兩個程式透過[資料目錄](/zh-tw/dev/architecture#資料目錄)下的 `config.json` 與具名管道協作。

三語 README 都要帶上：可攜版沒有安裝精靈，README 是唯一的隨附說明，其中「資料存放位置與清理」一節交代了程式在使用者資料夾下留了什麼、怎麼用 `cleanup.ps1` 清掉。

::: danger 可攜資料夾裡不能出現 installed.marker
程式憑它認出自己是安裝版並改用 `%APPDATA%\BossKey`（見[資料目錄](/zh-tw/dev/architecture#資料目錄)）。該檔案由 `.iss` 從指令碼資料夾直取，不經過 `dist\Boss-Key`——若混進可攜包，可攜版就不可攜了。
:::

安裝包預設走**一般權限**安裝（`%LocalAppData%\Programs\Boss Key`），使用者可在精靈首頁改選「為所有使用者安裝」裝進 `Program Files`。兩種模式下資料都在 `%APPDATA%\BossKey`，不在安裝資料夾裡。

## 版本號管理

::: info 版本號唯一真實來源
版本號只寫在 `Cargo.toml` 的 `[workspace.package] version` 一處，`Cargo.lock` 跟著它走。其餘地方**不再各存一份**，一律在建置時取真實版本號：

| 位置 | 版本號從哪來 |
| --- | --- |
| 兩個 exe 的檔案版本資訊 | `CARGO_PKG_VERSION`（tauri-winres／tauri-build；`tauri.conf.json` 不寫 `version` 即回落到 Cargo.toml） |
| 核心資訊清單的 `assemblyIdentity` | `crates/core/build.rs` 按 `CARGO_PKG_VERSION` 填入（換算成純數字四段號） |
| 安裝包的 `MyAppVersion` | `scripts/package.ps1` 從 `Cargo.toml` 讀出後傳給 Inno；未傳則編譯報錯，不留過期的預設值 |
| 程式內與回報給 Verhub 的版本 | `env!("CARGO_PKG_VERSION")` |
| 設定檔的 `app_version` | 核心啟動時寫入 `bosskey_common::APP_VERSION` |
:::

`scripts/version.ps1` 負責寫入與驗證：

```powershell
# 把版本號寫入 Cargo.toml（並同步 Cargo.lock）
powershell -File scripts/version.ps1 apply 3.0.1

# 驗證 Cargo.toml 與該 tag 一致，不一致則失敗
powershell -File scripts/version.ps1 check 3.0.1

# 不給 tag 時只回顯目前版本號
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

### `release.yml` — 一鍵發版

**觸發**：手動（`workflow_dispatch`），輸入要發布的版本號。**請從 `main` 觸發**（待發布內容合併進 `main` 之後）。

**做什麼**：
1. 用 `version.ps1 apply` 把版本號寫入 `Cargo.toml` 並同步 `Cargo.lock`；
2. 以 OIDC 身分向 [octo-sts](https://octo-sts.dev) 換取本儲存庫 `contents:write` 的短期 token；
3. 經 GraphQL `createCommitOnBranch` 把版本號變更提交到觸發分支，並打上 `v<版本>` 附註 tag——API 建立的提交由 GitHub 伺服器端簽章，帶 **Verified** 徽章；
4. 檢出該 tag → 驗證 tag 與程式碼版本一致 → 前端／Rust 測試；
5. `package.ps1 -Installer` 組裝 `dist/Boss-Key` 與 `dist/installer` → 把 `dist/Boss-Key` 壓成可攜 zip；
6. 產生**建置來源證明**（Sigstore attestation）→ 產生發布說明（自動產生的更新日誌，結尾附安全提示）→ 建立**草稿** Release 並上傳 zip 與安裝包。

tag 已存在時跳過第 2、3 步，直接檢出該 tag 重新建置——重跑／補發就是再次執行並填入同一版本號。

::: info 憑證從哪來
儲存庫不保存任何長期憑證。工作流程用 GitHub Actions 的 OIDC 身分向 octo-sts 換取短期 token，放行條件由 `.github/chainguard/tag-release.sts.yaml` 宣告（只允許 `main`／`dev` 上的執行），token 在 job 結束時自動撤銷。Octo STS App 在分支保護的 bypass 名單中，因此版本號提交無需發版 PR。

提交能帶 Verified 徽章，是因為它經 GitHub API 建立、由 GitHub 伺服器端簽章；tag 沒有伺服器端簽章機制，是普通附註 tag。
:::

### `deploy-docs.yml` — 文件站部署

**觸發**：`main` / `dev` 上涉及 `docs/` 的推送、Release 的發布／編輯／刪除，以及手動觸發。

**做什麼**：[i18n 一致性檢查](/zh-tw/dev/contributing#i18n-一致性檢查) → 重新產生 `docs/public/releases.json`（舊版用戶端的更新檢查資料來源，只收錄**非草稿** Release）→ VitePress 建置 → 部署到 GitHub Pages。

::: warning Release 要點了「Publish」才會刷新文件
`release.yml` 建的是**草稿** Release，草稿不發 `published` 事件。必須在網頁上點 **Publish release**，才會觸發文件站重新部署、把這個版本寫進 `releases.json`。
:::

### 發布一個新版本

```
main ──① Release──▶ 版本號提交（Verified）+ v3.0.1 tag ──▶ 建置 + 草稿 Release
                                                              │
                                                              ② Publish
                                                              ▼
                                                    文件站 + releases.json 刷新
```

1. 功能開發完畢，照常把 `dev` 經 PR 合併進 `main`。
2. 在 `main` 上執行 **「Release」**，填入版本號（如 `3.0.1`）。工作流程把版本號提交與 tag 落在 `main`，隨後完成生產建置，留下一個**草稿** Release。
3. 檢查產物與發布說明，點 **Publish release** 正式發布 —— 這一步同時會刷新文件站與 `releases.json`。
4. 把 `main` 合回 `dev`（或在下次從 `dev` 開 PR 前先合併 `main`），讓版本號提交回到 `dev`。

需要重跑或補發時，再次執行 **「Release」** 並填入同一版本號即可。
