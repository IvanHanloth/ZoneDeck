---
title: 打包与发布
---

# 打包与发布

本章介绍如何在本机打包产物，以及基于 GitHub Actions 的自动化发布流程。

## 本机一键打包

```powershell
# 便携文件夹：前端构建 → cargo release → 组装 dist/
powershell -File scripts/package.ps1

# 追加生成 InnoSetup 安装包（需 Inno Setup）
powershell -File scripts/package.ps1 -Installer

# 复用已有前端产物（前端没改时提速）
powershell -File scripts/package.ps1 -SkipFrontend
```

`package.ps1` 的流程：编译前端（Vite + Svelte）→ 生产编译 Rust workspace（Tauri 构建脚本会把前端 `dist` 内嵌进 `bosskey-config.exe`）→ 组装便携文件夹 → 可选生成安装包。

### 产物结构

便携版与安装包各占 `dist/` 下的一个子目录，互不干扰：

```
dist/
├── Boss-Key/                    便携版（拷走即用，发布时整个文件夹压成 zip）
│   ├── Boss Key.exe               常驻核心（内嵌 DPI/长路径 manifest + 版本信息 + 图标）
│   ├── config.exe                 配置界面（前端已内嵌，自包含）
│   ├── LICENSE.txt
│   └── README.md
└── installer/                   安装包（-Installer 时生成）
    └── Boss-Key-<版本>-Setup.exe  InnoSetup（安装前自动结束运行中的核心）
```

便携版**无需安装、无外部依赖**（除系统自带的 WebView2）。两个程序通过同目录的 `config.json` 与命名管道协作。

## 版本号管理

::: info 版本号唯一真源
版本号的唯一真源是 `Cargo.toml` 的 `[workspace.package] version`。另外三处必须与之一致：`apps/config/src-tauri/tauri.conf.json`、`apps/config/ui/package.json`、`Cargo.lock`。
:::

`scripts/version.ps1` 负责写入与校验：

```powershell
# 把版本号写入四处文件（并同步 Cargo.lock）
powershell -File scripts/version.ps1 apply 3.0.1

# 校验四处与该 tag 一致，不一致则失败
powershell -File scripts/version.ps1 check 3.0.1

# 不给 tag 时以 Cargo.toml 为基准校验其余文件
powershell -File scripts/version.ps1 check

# 打印当前版本号
powershell -File scripts/version.ps1 show
```

支持 semver（如 `3.0.1`、`3.1.0-rc.1`，可带前导 `v`）。带 `-` 的版本视为**预发布**。安装包要求纯数字四段号，会自动换算（`3.1.0-rc.1` → `3.1.0.0`）。

## CI/CD 工作流

工作流位于 `.github/workflows/`。

### `build-test.yml` — 构建与测试

**触发**：任意分支的 PR / push（tag 除外），以及手动触发。

**做什么**：版本号一致性检查 → 前端安装 / 测试 / 构建 → `cargo fmt --check` → `cargo clippy` → `cargo test` → `cargo build --release` → 上传二进制供检查。

同一分支有新推送时会自动取消旧任务（`concurrency` + `cancel-in-progress`）。

### `tag.yml` — 版本号写入并打 tag

**触发**：手动（`workflow_dispatch`），输入要发布的版本号。**请从 `dev` 分支触发**。

**做什么**：
1. 用 `version.ps1 apply` 把版本号写入四处文件；
2. 提交到 `dev` 并打上 `v<版本>` tag，两者一起推送；
3. 确保有一个 `dev` → `main` 的 PR（已存在就复用，没有才新开）。

**这一步不构建**。用 GITHUB_TOKEN 推的 tag 不会触发任何工作流，正合本流程的意图：等 PR 合并、tag 随之进入 `main` 的历史，`release.yml` 才开始生产构建。

::: warning 用 merge commit 合并发版 PR
tag 指向 `dev` 上的那个版本号提交。**squash / rebase 合并会另造提交**，tag 就进不了 `main` 的历史，`release.yml` 检测不到，**构建根本不会触发**。请用 **merge commit**。

真的误用了 squash，可以手动触发 `release.yml` 并指定 tag 来补救。
:::

::: tip PR 上没有检查记录？
GITHUB_TOKEN 创建的 PR 不会触发 `pull_request` 事件，PR 页面上不会有 CI 记录（你点 Merge 时的 push 事件仍会正常触发 `build-test.yml`）。若 `main` 的分支保护要求状态检查通过，请在仓库 secrets 中配置 `RELEASE_PAT`（repo 权限的 PAT），工作流会优先使用它来开 PR。
:::

### `release.yml` — 构建并发布 Release

**触发**：推送到 `main`（检测到有新的 `v*` tag 随之进入 `main` 的历史才继续），或手动触发并指定 tag。

**做什么**：检测本次推送新带进 `main` 的 tag → 检出该 tag → 校验 tag 与代码版本一致 → 前端 / Rust 测试 → `package.ps1 -Installer` 组装 `dist/Boss-Key` 与 `dist/installer` → 把 `dist/Boss-Key` 压成便携 zip → 生成**构建来源证明**（Sigstore attestation）→ 创建**草稿** Release 并上传 zip 与安装包。

::: info 为什么不监听 `push: tags`
tag 是 `tag.yml` 用 GITHUB_TOKEN 推到 `dev` 的，那次推送不会触发任何工作流。而**合并 PR 并不产生 tag 推送事件**——tag 是独立的 ref，合并只是让它指向的提交变得可从 `main` 追溯。所以只能从 `main` 的 push 事件里检测。

检测方式是比较推送前后「可从 `main` 追溯的 `v*` tag」集合，取新增的那个。不能用「HEAD 上挂着的 tag」：merge commit 才是 HEAD，tag 指向的是它的父提交。
:::

### `deploy-docs.yml` — 文档站部署

**触发**：`main` / `dev` 上涉及 `docs/` 的推送、Release 的发布 / 编辑 / 删除，以及手动触发。

**做什么**：[i18n 一致性检查](/dev/contributing#i18n-一致性检查) → 重新生成 `docs/public/releases.json`（旧版客户端的更新检查数据源，只收录**非草稿** Release）→ VitePress 构建 → 部署到 GitHub Pages。

::: warning Release 要点了「Publish」才会刷新文档
`release.yml` 建的是**草稿** Release，草稿不发 `published` 事件。必须在网页上点 **Publish release**，才会触发文档站重新部署、把这个版本写进 `releases.json`。
:::

### 发布一个新版本

```
dev ──① Bump version and tag──▶ dev（版本号提交 + v3.0.1 tag）
                                 │
                                 ②  PR，merge commit 合并
                                 ▼
                               main ──③ release.yml 检测到新 tag──▶ 构建 + 草稿 Release
                                                                        │
                                                                        ④ Publish
                                                                        ▼
                                                              文档站 + releases.json 刷新
```

1. 功能开发完毕、准备发版时，切到 `dev`，在 GitHub Actions 中运行 **"Bump version and tag"**，填入版本号（如 `3.0.1`）。工作流把版本号提交与 tag 落在 `dev`，并确保有一个 `dev` → `main` 的 PR。
2. 审查该 PR，用 **merge commit** 合并进 `main`。
3. 合并触发 `release.yml`：它检测到 `v3.0.1` 随之进入 `main`，检出该 tag 开始生产构建，完成后留下一个**草稿** Release。
4. 检查产物与发布说明，点 **Publish release** 正式发布 —— 这一步同时会刷新文档站与 `releases.json`。

需要重跑或补发时，手动触发 `release.yml` 并指定 tag 即可。
