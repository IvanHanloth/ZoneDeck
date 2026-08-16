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

`package.ps1` 的流程：编译前端（Vite + Svelte）→ 生产编译 Rust workspace（Tauri 构建脚本会把前端 `dist` 内嵌进 `zonedeck-config.exe`）→ 组装便携文件夹 → 可选生成安装包。

### 产物结构

便携版与安装包各占 `dist/` 下的一个子目录，互不干扰：

```
dist/
├── ZoneDeck/                    便携版（拷走即用，发布时整个文件夹压成 zip）
│   ├── ZoneDeck.exe               常驻核心（内嵌 DPI/长路径 manifest + 版本信息 + 图标）
│   ├── config.exe                 配置界面（前端已内嵌，自包含）
│   ├── cleanup.ps1                残留数据清理脚本（便携版没有卸载程序）
│   ├── LICENSE.txt
│   ├── README.md                  简体中文
│   ├── README.en.md               English
│   └── README.zh-TW.md            繁體中文
└── installer/                   安装包（-Installer 时生成）
    └── ZoneDeck-<版本>-Setup.exe  InnoSetup（安装前自动结束运行中的核心）
```

便携版**无需安装、无外部依赖**（除系统自带的 WebView2）。两个程序通过[数据目录](/dev/architecture#数据目录)下的 `config.json` 与命名管道协作。

三语 README 都要带上：便携版没有安装向导，README 是唯一的随包说明，其中「数据存放位置与清理」一节交代了程序在用户目录下留了什么、怎么用 `cleanup.ps1` 清掉。

::: danger 便携文件夹里不能出现 installed.marker
程序凭它认出自己是安装版并改用 `%APPDATA%\ZoneDeck`（见[数据目录](/dev/architecture#数据目录)）。该文件由 `.iss` 从脚本目录直取，不经过 `dist\ZoneDeck`——若混进便携包，便携版就不便携了。
:::

安装包默认走**普通权限**安装（`%LocalAppData%\Programs\ZoneDeck`），用户可在向导首屏改选「为所有用户安装」装进 `Program Files`。两种模式下数据都在 `%APPDATA%\ZoneDeck`，不在安装目录里。

## 版本号管理

::: info 版本号唯一真源
版本号只写在 `Cargo.toml` 的 `[workspace.package] version` 一处，`Cargo.lock` 跟着它走。其余地方**不再各存一份**，一律在构建时取真实版本号：

| 位置 | 版本号从哪来 |
| --- | --- |
| 两个 exe 的文件版本信息 | `CARGO_PKG_VERSION`（tauri-winres / tauri-build；`tauri.conf.json` 不写 `version` 即回落到 Cargo.toml） |
| 核心清单的 `assemblyIdentity` | `crates/core/build.rs` 按 `CARGO_PKG_VERSION` 填入（换算成纯数字四段号） |
| 安装包的 `MyAppVersion` | `scripts/package.ps1` 从 `Cargo.toml` 读出后传给 Inno；未传则编译报错，不留过期的默认值 |
| 程序内与上报给 Verhub 的版本 | `env!("CARGO_PKG_VERSION")` |
| 配置文件的 `app_version` | 核心启动时写入 `zonedeck_common::APP_VERSION` |
:::

`scripts/version.ps1` 负责写入与校验：

```powershell
# 把版本号写入 Cargo.toml（并同步 Cargo.lock）
powershell -File scripts/version.ps1 apply 3.0.1

# 校验 Cargo.toml 与该 tag 一致，不一致则失败
powershell -File scripts/version.ps1 check 3.0.1

# 不给 tag 时只回显当前版本号
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

### `release.yml` — 一键发版

**触发**：手动（`workflow_dispatch`），输入要发布的版本号。**请从 `main` 触发**（待发布内容合并进 `main` 之后）。

**做什么**：
1. 用 `version.ps1 apply` 把版本号写入 `Cargo.toml` 并同步 `Cargo.lock`；
2. 以 OIDC 身份向 [octo-sts](https://octo-sts.dev) 换取本仓库 `contents:write` 的短期 token；
3. 经 GraphQL `createCommitOnBranch` 把版本号变更提交到触发分支，并打上 `v<版本>` 附注 tag——API 创建的提交由 GitHub 服务端签名，带 **Verified** 徽章；
4. 检出该 tag → 校验 tag 与代码版本一致 → 前端 / Rust 测试；
5. `package.ps1 -Installer` 组装 `dist/ZoneDeck` 与 `dist/installer` → 把 `dist/ZoneDeck` 压成便携 zip；
6. 生成**构建来源证明**（Sigstore attestation）→ 生成发布说明（自动生成的更新日志，末尾附安全提示）→ 创建**草稿** Release 并上传 zip 与安装包。

tag 已存在时跳过第 2、3 步，直接检出该 tag 重新构建——重跑 / 补发就是再次运行并填入同一版本号。

::: info 凭据从哪来
仓库不保存任何长期凭据。工作流用 GitHub Actions 的 OIDC 身份向 octo-sts 换取短期 token，放行条件由 `.github/chainguard/tag-release.sts.yaml` 声明（只允许 `main` / `dev` 上的运行），token 在 job 结束时自动吊销。Octo STS App 在分支保护的 bypass 名单中，因此版本号提交无需发版 PR。

提交能带 Verified 徽章，是因为它经 GitHub API 创建、由 GitHub 服务端签名；tag 没有服务端签名机制，是普通附注 tag。
:::

### `deploy-docs.yml` — 文档站部署

**触发**：`main` / `dev` 上涉及 `docs/` 的推送、Release 的发布 / 编辑 / 删除，以及手动触发。

**做什么**：[i18n 一致性检查](/dev/contributing#i18n-一致性检查) → 重新生成 `docs/public/releases.json`（旧版客户端的更新检查数据源，只收录**非草稿** Release）→ VitePress 构建 → 部署到 GitHub Pages。

::: warning Release 要点了「Publish」才会刷新文档
`release.yml` 建的是**草稿** Release，草稿不发 `published` 事件。必须在网页上点 **Publish release**，才会触发文档站重新部署、把这个版本写进 `releases.json`。
:::

### 发布一个新版本

```
main ──① Release──▶ 版本号提交（Verified）+ v3.0.1 tag ──▶ 构建 + 草稿 Release
                                                              │
                                                              ② Publish
                                                              ▼
                                                    文档站 + releases.json 刷新
```

1. 功能开发完毕，照常把 `dev` 经 PR 合并进 `main`。
2. 在 `main` 上运行 **"Release"**，填入版本号（如 `3.0.1`）。工作流把版本号提交与 tag 落在 `main`，随后完成生产构建，留下一个**草稿** Release。
3. 检查产物与发布说明，点 **Publish release** 正式发布 —— 这一步同时会刷新文档站与 `releases.json`。
4. 把 `main` 合回 `dev`（或在下次从 `dev` 开 PR 前先合并 `main`），让版本号提交回到 `dev`。

需要重跑或补发时，再次运行 **"Release"** 并填入同一版本号即可。
