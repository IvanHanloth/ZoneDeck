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

**触发**：手动（`workflow_dispatch`），输入要发布的版本号。

**做什么**：
1. 用 `version.ps1 apply` 把版本号写入四处文件；
2. 提交并推回当前分支，打上 `v<版本>` tag 并推送；
3. 调用 `release.yml` 构建并发布。

### `release.yml` — 构建并发布 Release

**触发**：推送 `v*` tag，或被 `tag.yml` 通过 `workflow_call` 调用。

**做什么**：校验 tag 与代码版本一致 → 前端 / Rust 测试 → `package.ps1 -Installer` 组装 `dist/Boss-Key` 与 `dist/installer` → 把 `dist/Boss-Key` 压成便携 zip → 生成**构建来源证明**（Sigstore attestation）→ 创建 GitHub Release 并上传 zip 与安装包。

### 发布一个新版本

推荐通过 `tag.yml` 手动触发：

1. 在 GitHub Actions 中运行 **"Bump version and tag"**，填入版本号（如 `3.0.1`）。
2. 工作流自动写版本号、打 tag、构建并发布 Release。

也可以本地手动打 tag 推送来直接触发 `release.yml`，但需自行保证版本号已一致。
