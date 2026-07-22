---
title: 贡献指南
---

# 贡献指南

感谢你愿意为 Boss Key 贡献代码！为了高效、安全地协作，请在提交前阅读本页的要求。

## 贡献流程

1. **Fork 仓库**并克隆到本地，或（有权限时）在仓库内创建分支。
2. 从 `dev` 分支切出**功能分支**进行开发（分支命名见下文）。
3. 完成开发并**通过本地测试与检查**（见下文"提交前检查清单"）。
4. 向 **`dev` 分支**发起 Pull Request，关联相关 Issue / Project。
5. 经过**代码审查**后合并。正式发布时再由 `dev` 统一合并至 `main`。

分支与合并的完整策略见 [项目管理策略](/dev/project-management)。

## 分支命名

分支命名没有强制规范，但为便于维护，**推荐使用 `类型/功能` 的形式**：

| 类型前缀 | 用途 | 示例 |
| --- | --- | --- |
| `feat/` | 新功能 | `feat/checkUpdate` |
| `fix/` | Bug 修复 | `fix/hideWindow` |
| `refactor/` | 重构 | `refactor/agent` |
| `doc/` | 文档 | `doc/init` |
| `chore/` | 杂项 / 构建 | `chore/ci` |

::: tip 文档相关分支
所有**文档相关**的改动请使用 `doc` 开头的分支（如 `doc/guide-freeze`）。文档站的部署工作流会在推送到此类分支时自动构建预览 / 部署，详见 [文档站维护](/dev/docs-site)。
:::

## 提交前检查清单

提交 PR 前，请确保本地通过以下检查（与 CI 一致）：

```bash
# 1. 格式
cargo fmt --all -- --check

# 2. 静态检查（warning 视为 error）
cargo clippy --workspace --all-targets -- -D warnings

# 3. 前端测试与构建
npm --prefix apps/config/ui test
npm --prefix apps/config/ui run build

# 4. Rust 测试
cargo test --workspace

# 5. 生产编译能通过
cargo build --release
```

::: warning 版本号一致性
若你改动了版本号，务必保证 `Cargo.toml`、`tauri.conf.json`、`ui/package.json`、`Cargo.lock` **四处一致**。CI 会用 `scripts/version.ps1 check` 校验。日常功能开发一般**不要**手动改版本号——版本号由发布流程统一管理，详见 [打包与发布](/dev/release)。
:::

## 代码风格

- **Rust**：遵循 `rustfmt` 默认风格；`clippy` 零警告。
- **前端**：Svelte 5 + 现代 JS；纯逻辑放入 `ui/src/lib/` 并配套 `vitest` 测试，UI 放入 `ui/src/components/`。
- **提交信息**：推荐使用类似 `feat: …`、`fix: …`、`doc: …`、`refactor: …` 的前缀，与分支类型呼应。

## 测试要求

- 新增/修改**纯逻辑**（配置、匹配、协议、热键解析、前端 lib 等）应补充单元测试。
- 涉及窗口 / 冻结 / IPC 等系统行为的改动，尽量补充对应的集成测试或说明手动验证步骤。
- 详见 [测试策略](/dev/testing)。

## 用户可见文案

新增任何用户可见文案都必须补齐三种语言，否则测试不通过：

- **配置界面**：先在 `apps/config/ui/src/locales/zh-CN.js`（文案基准）加键，再同步 `en.js` 与 `zh-TW.js`。
- **核心**（托盘菜单 / 通知 / IPC 错误）：在 `crates/core/src/i18n.rs` 的 `Msg` 枚举加变体，补齐三种语言，并登记到测试模块的 `ALL_MSGS`。
- 繁体中文使用台湾用语（如「视窗」「程式」「档案」「滑鼠」「快速键」），不要只做简繁字形转换。
- **日志不翻译**，一律使用简体中文。

### i18n 一致性检查

单元测试只能校验 catalog 内部（键集、空值、占位符），跨文件的疏漏由 `scripts/i18n-check.ps1` 兜底：

| 检查项 | 漏掉会怎样 |
| --- | --- |
| 文档三语页面集一致 | 某语言少一页，读者点进去 404 |
| 站内链接不跨语言 | 英文页链到 `/guide/…` 会把读者甩回中文页；VitePress 的死链检查发现不了 |
| `t("键")` 引用的键存在 | 界面直接显示原始键名 |
| catalog 无死键 | 删功能时残留的文案越积越多 |
| `Msg` 变体已登记进 `ALL_MSGS` | 该条文案跳过跨语言校验，漏译不会被发现 |

```bash
# 手动全量检查
pwsh -File scripts/i18n-check.ps1
```

仓库带了 `pre-commit` 钩子跑这套检查（只检查与本次暂存内容相关的部分）。**每个克隆需启用一次**：

```bash
git config core.hooksPath .githooks
```

确有需要时用 `git commit --no-verify` 跳过。

## 提交 Issue

仓库提供了 Issue 模板：

- **Bug 报告**：请附版本号、系统版本、复现步骤与日志。
- **功能建议**：请描述使用场景与期望行为。

尽量将 **Issue–Pull Request–Project** 三者关联，方便统一管理（见 [项目管理策略](/dev/project-management)）。

## 许可

本项目基于 **MIT** 许可发布。提交贡献即表示你同意你的代码以相同许可发布。
