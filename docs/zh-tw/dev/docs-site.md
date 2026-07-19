---
title: 文件站維護
---

# 文件站維護

本文件站基於 **VitePress** 建置，原始碼位於儲存庫的 `docs/` 資料夾，並透過 GitHub Actions 自動部署到 GitHub Pages。

## 資料夾結構

```
docs/
├── .vitepress/
│   ├── config.mts        站台設定（三語 locales：導覽、側邊欄、佈景主題）
│   └── theme/            自訂佈景主題（元件文案隨 useData().lang 切換）
├── public/               靜態資源（圖片／圖示，按根路徑引用；三語共用）
├── index.md              簡體中文首頁（hero + features）
├── guide/                簡體中文使用說明（面向使用者）
├── dev/                  簡體中文開發文件（面向開發者）
├── changelog/            更新日誌（由 GitHub Releases 動態產生）
├── en/                   英文，結構與根目錄一致
│   ├── index.md
│   ├── guide/
│   ├── dev/
│   └── changelog/
└── zh-tw/                繁體中文，結構與根目錄一致
    ├── index.md
    ├── guide/
    ├── dev/
    └── changelog/
```

兩類文件分別放在 `guide/` 與 `dev/` 兩個主資料夾下，並在 `config.mts` 中設定了各自的**頂部導覽入口**與**獨立側邊欄**。

## 多語言

站台用 VitePress 的 [locales](https://vitepress.dev/zh/guide/i18n) 提供三種語言：

| 語言 | 路徑前綴 | 資料夾 |
| --- | --- | --- |
| 简体中文 | `/`（root） | `docs/` |
| English | `/en/` | `docs/en/` |
| 繁體中文 | `/zh-tw/` | `docs/zh-tw/` |

- **簡體中文是內容基準**，且留在根路徑，因此既有外部連結與 SEO 不受影響。新增或修改文件時**先寫簡體中文**，再同步另外兩種語言。
- 每種語言在 `config.mts` 的 `locales` 下有自己的 `nav`、`sidebar`、`editLink`、`footer`；側邊欄連結必須帶各自的路徑前綴（如 `/en/guide/binding`）。
- 文件內的**站內連結要指向同語言頁面**，否則讀者會被跳出目前語言。
- `.vitepress/theme/` 下的自訂元件（`StatusBar.vue`、`ReleaseMeta.vue`）透過 `useData().lang` 取目前語言切換文案，三語共用同一份元件。

::: warning 更新日誌內容不翻譯
`changelog/` 的版本頁由 `[version].paths.ts` 從 GitHub Releases 動態產生，內文即 Release body（以簡體中文撰寫），三種語言下都按原文顯示；僅頁面外殼（標題、「下載」、發布時間等）隨語言切換。
:::

## 本機預覽

文件相關的指令碼在根 `package.json` 中：

```bash
# 安裝相依套件（使用 pnpm；也可用 npm）
pnpm install

# 本機開發預覽（熱重新載入）
pnpm docs:dev

# 生產建置
pnpm docs:build

# 預覽生產建置產物
pnpm docs:preview
```

::: tip 建置產物已忽略
`docs/.vitepress/dist` 與 `docs/.vitepress/cache` 已在 `.gitignore` 中忽略，不要提交。
:::

## 圖片與螢幕擷取畫面

- 將圖片放入 `docs/public/`，在 Markdown 中以**根路徑**引用，例如 `![說明](/screenshot-1.png)`。
- 站台部署在自訂網域下，`base` 為 `/`。若改為 GitHub Pages 專案站台（`/Boss-Key/`），VitePress 會自動為 `public` 資源補上前綴，Markdown 裡始終不需手寫 `base`。

## VitePress 特性

文件中廣泛使用了 VitePress 的[容器提示框](https://vitepress.dev/zh/guide/markdown#custom-containers)，撰寫時請保持一致：

```md
::: tip 提示
用於補充建議、最佳做法。
:::

::: warning 注意
用於提醒潛在風險／易錯點。
:::

::: danger 危險
用於強調破壞性操作。
:::

::: info 資訊
中性的補充說明。
:::

::: details 展開檢視
預設摺疊的次要內容。
:::
```

## 部署工作流程

文件透過 `.github/workflows/deploy-docs.yml` 自動部署到 GitHub Pages：

- **自動觸發**：改動涉及 `docs/**` 或工作流程本身。
- **手動觸發**：在 GitHub Actions 頁面透過 `workflow_dispatch` 手動執行。

工作流程執行 `pnpm install` → `pnpm docs:build` → 上傳並部署到 Pages。

## 撰寫約定

- 使用**專業、客觀**的描述。
- 面向使用者處統一稱 **Boss Key**（不使用小寫 `bosskey`）。
- 每新增一個功能，請同時在 [使用說明](/zh-tw/guide/) 補充對應說明，並在需要時更新 [設定檔欄位](/zh-tw/dev/config-reference)。
- 改動文件時**三種語言一併更新**；繁體中文使用台灣用語（如「視窗」「程式」「檔案」「滑鼠」「快速鍵」），不要只做簡繁字形轉換。
