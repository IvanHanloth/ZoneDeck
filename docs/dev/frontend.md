---
title: 前端与配置界面
---

# 前端与配置界面

配置界面（`config.exe`）由 **Tauri 2（Rust 后端）+ Svelte 5（前端）** 构成。本章介绍其技术选型、结构与关键设计。

## 为什么选择 Svelte

配置界面是一个**表单密集型的小应用**，经过选型对比后采用 **Svelte 5 + Vite**：

- **双向绑定是核心诉求**：Svelte 的 `bind:` 原生支持双向绑定（React 无双向绑定，需手写受控组件；Vue 的 `v-model` 可用但运行时更大）。
- **编译期反应式、无虚拟 DOM**：构建产物仅约 80 KB（gzip 约 28 KB），契合本项目"体积极小"的目标。
- **Tauri 官方一等支持**。

## 前端工程结构

```
apps/config/ui/
├── src/
│   ├── App.svelte           顶层：标签页 + 自动保存 + 启动编排
│   ├── main.js              入口（挂载前先应用一次主题，避免闪烁）
│   ├── components/          UI 组件（TitleBar、各设置面板、各弹窗…）
│   └── lib/                 纯逻辑（可单测）
│       ├── state.svelte.js  全局状态（$state）+ 各类动作
│       ├── ipc.js           调用 Tauri 命令 / 监听事件
│       ├── hotkey.js        热键解析/格式化（配 hotkey.test.js）
│       ├── pointer.js       鼠标连击/连击窗口常量（配 pointer.test.js）
│       ├── grouping.js      窗口/进程规则的增删与过滤（配 grouping.test.js）
│       ├── theme.js         主题切换（配 theme.test.js）
│       └── verhub.js        检查更新/公告/打开外链
├── vite.config.js
└── svelte.config.js
```

设置界面以标签页组织，对应五个面板组件：

| 标签 | 组件 | 内容 |
| --- | --- | --- |
| 窗口绑定 | `BindingPanel` | 窗口列表 + 窗口/进程规则 |
| 热键与鼠标 | `HotkeysPanel` | 键盘热键、鼠标连击、四角、空闲自动隐藏 |
| 通知设置 | `NotificationsPanel` | 逐事件通知开关 |
| 其他选项 | `OptionsPanel` | 静音/暂停/冻结/权限/日志/工具 |
| 关于与反馈 | `AboutPanel` | 版本、更新、公告、反馈 |

## 无边框自绘窗口

Tauri 配置 `decorations: false`，标题栏由前端自绘：

- `data-tauri-drag-region` 实现拖动、双击最大化；
- 自绘最小化 / 最大化 / 关闭按钮；
- 八向边缘 `startResizeDragging` 缩放热区（最大化时自动禁用）；
- 亮 / 暗 / 跟随系统三态主题（`localStorage` 持久化 + `prefers-color-scheme` 监听）。

## 配置自动保存

`App.svelte` 中用 `$effect` 深度追踪配置对象（含 `window_rules` / `process_rules`）的任何变化，停顿后经 `scheduleSave`（内部 debounce）自动写盘。加载阶段不触发保存，`loadAll` 完成后才"武装"自动保存。

保存后前端通过 IPC 通知核心 `reload_config`，核心热重载配置。

## 状态轮询不卡界面

- 状态获取合并为**单条 `GetStatus`** 管道命令，并使用**连接快速失败**（不重试）。
- Tauri 命令用 `spawn_blocking` 异步化，避免阻塞。
- 前端每 2 秒轮询一次状态（页面不可见时暂停）。

## 前端不依赖 dev server

最终产物中，前端在**编译期被内嵌**进 `bosskey-config.exe`，静态运行，**不需要任何本地服务器**。

- 开发前端 UI：`npm run dev` 在浏览器里预览（mock 数据、热重载）。
- 验证 Tauri 集成：`npm run build` 后 `cargo run -p bosskey-config`。

## 与核心的联动示例

- 核心托盘的"窗口恢复工具" / "关于"会带参数拉起配置界面：冷启动从启动参数读取，已运行时由单实例插件发来事件。
- 首次启动若核心未运行，配置界面会自动 `startCore` 拉起核心。
