---
title: 本地运行
---

# 本地运行

本章介绍如何在本地搭建开发环境、运行核心与配置界面，以及日常开发用到的命令。

## 环境准备

| 依赖 | 版本 / 说明 | 获取方式 |
| --- | --- | --- |
| **Rust** | stable，建议 1.85+（项目使用 edition 2024） | <https://rustup.rs> |
| **Node.js** | 18+，建议 24（用于前端构建） | <https://nodejs.org> |
| **WebView2** | 配置界面运行时（Win10/11 通常已内置） | 系统自带 / [微软官网](https://developer.microsoft.com/zh-cn/microsoft-edge/webview2) |
| **Inno Setup 6** | *可选*，本机生成安装包时需要 | `winget install JRSoftware.InnoSetup` |
| **pssuspend64.exe** | *可选*，测试增强冻结时需要 | [Microsoft PSTools](https://download.sysinternals.com/files/PSTools.zip) |

::: tip 配置界面不依赖 dev server
前端在编译期被**内嵌**进 `zonedeck-config.exe`，最终产物静态运行。开发前端时用 `pnpm run dev` 在浏览器里预览（mock 数据、热重载），改完 `pnpm run build` 后用 `cargo run -p zonedeck-config` 验证 Tauri 集成。
:::

## 克隆项目

```bash
git clone https://github.com/IvanHanloth/ZoneDeck.git
cd ZoneDeck
```

## 常用命令

在**仓库根目录**执行：

### 核心（zonedeck-core）

```bash
# 运行核心（开发）
cargo run -p zonedeck-core

# 核心冒烟自测：N 毫秒后自动退出
cargo run -p zonedeck-core -- smoke 3000
```

### 前端（apps/config/ui）

```bash
# 首次：安装前端依赖
pnpm --dir apps/config/ui install

# 前端构建（产物输出到 apps/config/dist，供 Tauri 内嵌）
pnpm --dir apps/config/ui run build

# 前端单元测试（vitest）
pnpm --dir apps/config/ui test

# 浏览器预览（mock 数据、热重载）
pnpm --dir apps/config/ui run dev
```

### 配置界面（zonedeck-config，Tauri）

```bash
# 运行配置界面（需先构建前端）
pnpm --dir apps/config/ui run build && cargo run -p zonedeck-config
```

### 质量检查与测试

```bash
# 运行全部 Rust 测试
cargo test --workspace

# 静态检查（把 warning 当 error）
cargo clippy --workspace --all-targets -- -D warnings

# 代码格式化 / 仅检查
cargo fmt --all
cargo fmt --all -- --check
```

### 生产编译与打包

```bash
# 生产编译（体积最小化）
cargo build --release

# 一键生产打包（前端 + Rust + 便携文件夹 dist/ZoneDeck）
powershell -File scripts/package.ps1

# 一键打包 + 安装包（dist/installer，首次会自动装 Inno Setup 7）
powershell -File scripts/package.ps1 -Installer
```

更多打包细节见 [打包与发布](/dev/release)。

## 命令速查表

| 目的 | 命令 |
| --- | --- |
| 运行核心 | `cargo run -p zonedeck-core` |
| 核心冒烟自测 | `cargo run -p zonedeck-core -- smoke 3000` |
| 前端装依赖 | `pnpm --dir apps/config/ui install` |
| 前端构建 | `pnpm --dir apps/config/ui run build` |
| 前端测试 | `pnpm --dir apps/config/ui test` |
| 前端浏览器预览 | `pnpm --dir apps/config/ui run dev` |
| 运行配置界面 | `pnpm --dir apps/config/ui run build && cargo run -p zonedeck-config` |
| 生产编译 | `cargo build --release` |
| 全部 Rust 测试 | `cargo test --workspace` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| 格式检查 | `cargo fmt --all -- --check` |
| 一键打包 | `powershell -File scripts/package.ps1` |
| 打包 + 安装包 | `powershell -File scripts/package.ps1 -Installer` |

## 常见开发问题

::: warning 杀软拦截新编译的可执行文件
若表现为 `os error 5 拒绝访问`，通常是杀软锁住了刚编译出的 exe。请将项目的 `target` 目录加入杀软信任区。
:::

::: tip 核心测试需单线程运行
部分核心测试涉及 COM 初始化，多线程并行可能导致崩溃。若遇到此类问题，使用 `cargo test -p zonedeck-core -- --test-threads=1` 单线程运行。详见 [测试策略](/dev/testing)。
:::
