---
title: 项目管理策略
---

# 项目管理策略

为了高效、安全地推进开发，Boss Key 对分支、合并与项目规划有一套约定。

## 分支模型

仓库有两个**主要分支**：

| 分支 | 作用 | 提交约束 |
| --- | --- | --- |
| **`main`** | 存放**正式发布版本**的源代码 | **不可直接提交**。原则上只能在某版本功能开发完成、决定发布并通过测试后，从 `dev` 合并 |
| **`dev`** | 存放**所有正在开发**的功能源代码 | **不可直接提交**。原则上只能包含已完成开发并通过测试的功能 |

其他分支（`feat/*`、`fix/*`、`doc/*` 等）用于日常功能开发与提交，见[分支命名](/dev/contributing#分支命名)。

### 合并流向

```
feat/* · fix/* · doc/* ──PR──▶ dev ──PR（发版时）──▶ main
      （代码审查后合并）        （通过测试后合并）
```

- 新功能、Bug 修复等应通过 **Pull Request 提交至 `dev`**，经代码审查后合并。
- 积累若干功能、经过测试后，统一通过 PR 将 `dev` 合并至 `main`，并触发发布。

::: tip 为什么不直接提交主分支
`main` / `dev` 受保护可以避免未经审查的代码进入稳定线，保证任何时刻这两个分支都是可构建、可发布的状态。
:::

## Pull Request 规范

- 一个 PR 聚焦一件事，便于审查。
- PR 应通过 CI 的全部检查（格式、Clippy、前端测试与构建、Rust 测试、release 编译）。
- 关联相关 Issue（如 `Closes #123`）与 Project。
- 至少经过一次**代码审查**再合并。

## Project 规范

项目通常使用 **GitHub 的 Project 功能**进行统一规划管理。

::: info 三者关联
应尽量将 **Issue – Pull Request – Project** 三者相互关联，以实现统一管理：
- Issue 描述"要做什么 / 出了什么问题"；
- PR 实现并引用对应 Issue；
- Project 看板追踪整体进度。
:::

## 版本与发布管理

- 版本号的**唯一真源**是 `Cargo.toml` 的 `[workspace.package] version`，其余三处文件必须与之一致。
- 发布通过 GitHub Actions 手动触发的工作流完成：写入版本号 → 打 tag → 构建并发布 Release。
- 详见 [打包与发布](/dev/release)。

## Issue 模板

仓库在 `.github/ISSUE_TEMPLATE/` 下提供了结构化模板：

- `bug-report.yml` —— Bug 报告；
- `feature-request.yml` —— 功能建议；
- `config.yml` —— Issue 入口配置。

使用模板能保证反馈包含足够的排查信息。
