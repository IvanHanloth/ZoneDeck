---
title: 测试策略
---

# 测试策略

Boss Key 覆盖了从纯逻辑单元测试到系统级集成测试的多层测试。CI 会在每次 PR / 推送时运行全部测试。

## 运行测试

```bash
# 全部 Rust 测试
cargo test --workspace

# 前端单元测试
npm --prefix apps/config/ui test
```

::: warning 核心并行测试可能因 COM 崩溃
`bosskey-core` 的部分测试涉及 COM（如静音链路）初始化，多线程并行运行可能崩溃。遇到此类问题时改用单线程：

```bash
cargo test -p bosskey-core -- --test-threads=1
```
:::

## Rust 测试覆盖

### `bosskey-common`

模型 / 配置 / 匹配 / 协议的**纯逻辑单元测试**，以及配置文件读写的集成测试。例如：

- 配置解析与默认值、旧配置迁移（扁平绑定 → 窗口规则、旧鼠标开关 → 连击触发）；
- `PID` 大写字段兼容、正则规则往返序列化；
- 连击次数 / 连击窗口的范围钳制；
- 协议 `Command` / `Response` 的 round-trip 与 snake_case 标签；
- 语言标签解析与偏好归一化（`zh-Hant` → `zh-TW`、无翻译的语言回落 `auto` 等）。

### `bosskey-core`

系统行为的集成测试（多为创建真实资源往返验证）：

| 领域 | 测试内容 |
| --- | --- |
| 窗口枚举 / 隐藏 / 显示 | 创建真实窗口往返 |
| 热键解析 | 字符串 → RegisterHotKey 参数 |
| 热键拦截 | 键盘钩子判定的纯逻辑：修饰键完全吻合、长按只触发一次、按下被吞则抬起也吞 |
| 单实例互斥 | 命名互斥体 |
| 命名管道 | 服务端收发 |
| 进程冻结 | 真实子进程挂起 / 恢复 |
| 静音 | Core Audio COM 链路 |
| HideController | mock 注入验证静音 / 冻结 / 暂停键编排 |
| 开机自启 | 普通注册表键逻辑 + 真实 `schtasks` 接受任务 XML |
| 崩溃日志 | 写入 / 轮转 / panic 钩子 |
| 崩溃恢复 | 快照落盘往返 + mock 恢复编排 |
| 图标编码 | CRC32 / Adler-32 / base64 已知向量 + PNG 结构解析 + 真实 explorer.exe 提取 |
| 端到端 IPC | 经 `PipeClient` 驱动真实 agent 的 IPC / 崩溃恢复测试 |
| 文案 catalog | 三种语言逐条非空、英文与中文不重复、占位符跨语言一致 |

::: info 受限环境说明
`schtasks` 相关的开机自启集成测试在受限 / 无权限的 CI 或沙盒环境中可能失败，这属于环境限制而非功能回归。本地具备权限时应能通过。
:::

## 前端测试（vitest）

前端把纯逻辑抽到 `ui/src/lib/`，并配套 `vitest` 单元测试：

- `hotkey.test.js` —— 热键解析 / 格式化；
- `pointer.test.js` —— 鼠标连击 / 连击窗口；
- `grouping.test.js` —— 窗口 / 进程规则增删与过滤；
- `theme.test.js` —— 主题偏好逻辑；
- `i18n.test.js` —— 语言解析，以及三份 catalog 的键集 / 占位符对齐。

UI 组件（`components/`）保持薄逻辑，主要复杂度都下沉到可测试的 `lib/`。

## 编写测试的约定

- **纯逻辑优先单测**：任何可抽成纯函数的逻辑都应放入 `common` 或前端 `lib/` 并单测。
- **系统行为集成测试**：涉及窗口 / COM / 管道等，创建真实资源往返验证，必要时用 mock（`Effects` / `WindowManager` trait）隔离副作用。
- 提交前请确保本地测试通过，具体见 [贡献指南](/dev/contributing#提交前检查清单)。
