---
title: 文档站维护
---

# 文档站维护

本文档站基于 **VitePress** 构建，源码位于仓库的 `docs/` 目录，并通过 GitHub Actions 自动部署到 GitHub Pages。

## 目录结构

```
docs/
├── .vitepress/
│   ├── config.mts        站点配置（三语 locales：导航、侧边栏、主题）
│   └── theme/            自定义主题（组件文案随 useData().lang 切换）
├── public/               静态资源（图片 / 图标，按根路径引用；三语共用）
├── index.md              简体中文首页（hero + features）
├── guide/                简体中文使用文档（面向用户）
├── dev/                  简体中文开发文档（面向开发者）
├── changelog/            更新日志（由 GitHub Releases 动态生成）
├── en/                   英文，结构与根目录一致
│   ├── index.md
│   ├── guide/
│   ├── dev/
│   └── changelog/
└── zh-tw/                繁体中文，结构与根目录一致
    ├── index.md
    ├── guide/
    ├── dev/
    └── changelog/
```

两类文档分别放在 `guide/` 与 `dev/` 两个主目录下，并在 `config.mts` 中配置了各自的**顶部导航入口**与**独立侧边栏**。

## 多语言

站点用 VitePress 的 [locales](https://vitepress.dev/zh/guide/i18n) 提供三种语言：

| 语言 | 路径前缀 | 目录 |
| --- | --- | --- |
| 简体中文 | `/`（root） | `docs/` |
| English | `/en/` | `docs/en/` |
| 繁體中文 | `/zh-tw/` | `docs/zh-tw/` |

- **简体中文是内容基准**，且留在根路径，因此既有外链与 SEO 不受影响。新增或修改文档时**先写简体中文**，再同步另外两种语言。
- 每种语言在 `config.mts` 的 `locales` 下有自己的 `nav`、`sidebar`、`editLink`、`footer`；侧边栏链接必须带各自的路径前缀（如 `/en/guide/binding`）。
- 文档内的**站内链接要指向同语言页面**，否则读者会被跳出当前语言。
- `.vitepress/theme/` 下的自定义组件（`StatusBar.vue`、`ReleaseMeta.vue`）通过 `useData().lang` 取当前语言切换文案，三语共用同一份组件。

::: warning 更新日志内容不翻译
`changelog/` 的版本页由 `[version].paths.ts` 从 GitHub Releases 动态生成，正文即 Release body（简体中文撰写），三种语言下都按原文显示；仅页面外壳（标题、"下载"、发布时间等）随语言切换。
:::

## 本地预览

文档相关的脚本在根 `package.json` 中：

```bash
# 安装依赖（使用 pnpm；也可用 npm）
pnpm install

# 本地开发预览（热重载）
pnpm docs:dev

# 生产构建
pnpm docs:build

# 预览生产构建产物
pnpm docs:preview
```

::: tip 构建产物已忽略
`docs/.vitepress/dist` 与 `docs/.vitepress/cache` 已在 `.gitignore` 中忽略，不要提交。
:::

## 图片与截图

- 将图片放入 `docs/public/`，在 Markdown 中以**根路径**引用，例如 `![说明](/screenshot-1.png)`。
- 站点部署在自定义域名下，`base` 为 `/`。若改为 GitHub Pages 项目站点（`/ZoneDeck/`），VitePress 会自动为 `public` 资源补上前缀，Markdown 里始终无需手写 `base`。

## VitePress 特性

文档中广泛使用了 VitePress 的[容器提示框](https://vitepress.dev/zh/guide/markdown#custom-containers)，编写时请保持一致：

```md
::: tip 提示
用于补充建议、最佳实践。
:::

::: warning 注意
用于提醒潜在风险 / 易错点。
:::

::: danger 危险
用于强调破坏性操作。
:::

::: info 信息
中性的补充说明。
:::

::: details 展开查看
默认折叠的次要内容。
:::
```

## 部署工作流

文档通过 `.github/workflows/deploy-docs.yml` 自动部署到 GitHub Pages：

- **自动触发**：改动涉及 `docs/**` 或工作流本身。
- **手动触发**：在 GitHub Actions 页面通过 `workflow_dispatch` 手动运行。

工作流执行 `pnpm install` → `pnpm docs:build` → 上传并部署到 Pages。

## 写作约定

- 使用**专业、客观**的描述。
- 面向用户处统一称 **ZoneDeck**（不使用小写 `zonedeck`）。
- 每新增一个功能，请同时在 [使用文档](/guide/) 补充对应说明，并在需要时更新 [配置文件字段](/dev/config-reference)。
- 改动文档时**三种语言一并更新**；繁体中文使用台湾用语（如「视窗」「程式」「档案」「滑鼠」「快速键」），不要只做简繁字形转换。
