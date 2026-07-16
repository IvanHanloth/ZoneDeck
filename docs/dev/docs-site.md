---
title: 文档站维护
---

# 文档站维护

本文档站基于 **VitePress** 构建，源码位于仓库的 `docs/` 目录，并通过 GitHub Actions 自动部署到 GitHub Pages。

## 目录结构

```
docs/
├── .vitepress/
│   ├── config.mts        站点配置（导航、侧边栏、主题、base）
│   └── theme/            自定义主题
├── public/               静态资源（图片 / 图标，按根路径引用）
├── index.md              首页（hero + features）
├── guide/                使用文档（面向用户）
│   ├── index.md          简介
│   ├── installation.md
│   ├── getting-started.md
│   └── …
└── dev/                  开发文档（面向开发者）
    ├── index.md          导览
    ├── getting-started.md
    └── …
```

两类文档分别放在 `guide/` 与 `dev/` 两个主目录下，并在 `config.mts` 中配置了各自的**顶部导航入口**与**独立侧边栏**。

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
- 站点的 `base` 为 `/Boss-Key/`（GitHub Pages 项目站点），VitePress 会自动为 `public` 资源补上前缀，Markdown 里无需手写 `base`。

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

- 使用**专业、客观**的中文描述。
- 面向用户处统一称 **Boss Key**（不使用小写 `bosskey`）。
- 每新增一个功能，请同时在 [使用文档](/guide/) 补充对应说明，并在需要时更新 [配置文件字段](/dev/config-reference)。
