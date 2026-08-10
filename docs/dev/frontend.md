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
│       ├── i18n.svelte.js   界面语言：catalog 查表 + 语言解析（配 i18n.test.js）
│       ├── markdown.js      公告/更新日志的 Markdown 渲染（配 markdown.test.js）
│       └── verhub.js        检查更新/公告/反馈（含转 Issue）/项目链接/打开外链
├── locales/                 三语文案 catalog（zh-CN.js / en.js / zh-TW.js）
├── vite.config.js
└── svelte.config.js
```

设置界面以标签页组织，对应五个面板组件：

| 标签 | 组件 | 内容 |
| --- | --- | --- |
| 窗口绑定 | `BindingPanel` | 窗口列表 + 窗口/进程规则 |
| 热键与鼠标 | `HotkeysPanel` | 键盘热键、鼠标连击、四角、空闲自动隐藏 |
| 提示设置 | `NotificationsPanel` | 逐事件通知开关与图标状态角标 |
| 其他选项 | `OptionsPanel` | 静音/暂停/冻结/权限/日志/工具 |
| 关于与反馈 | `AboutPanel` | 版本、更新、公告、反馈（可转为 GitHub Issue） |

## 无边框自绘窗口

Tauri 配置 `decorations: false`，标题栏由前端自绘：

- `data-tauri-drag-region` 实现拖动、双击最大化；
- 自绘最小化 / 最大化 / 关闭按钮；
- 八向边缘 `startResizeDragging` 缩放热区（最大化时自动禁用）；
- 亮 / 暗 / 跟随系统三态主题（`localStorage` 持久化 + `prefers-color-scheme` 监听）。

## 配置自动保存

`App.svelte` 中用 `$effect` 深度追踪配置对象（含 `window_rules` / `process_rules`）的任何变化，停顿后经 `scheduleSave`（内部 debounce）自动写盘。加载阶段不触发保存，`loadAll` 完成后才"武装"自动保存。调度逻辑在 `lib/autosave.js`：连续改动合并为一次写盘，写盘互不并发，写盘途中的新改动在上一笔完成后补写。

关窗请求（标题栏按钮、Alt+F4）会先把未落盘的改动写完再关闭；写盘失败时窗口保持打开并弹出错误框，再次关闭不再阻拦。

保存后前端通过 IPC 通知核心 `reload_config`，核心热重载配置。

## 界面语言

文案集中在 `src/locales/`，每个语言一个扁平键值表；`lib/i18n.svelte.js` 负责查表与语言解析。

```js
import { t } from "../lib/i18n.svelte.js";

t("common.close");                    // → 关闭
t("restore.frozen", { n: 3 });        // → 已冻结 3 个进程
```

- `t()` 读取 `$state` 里的当前语言，因此模板中调用它即可在切换语言时自动重渲染，无需手动订阅。
- **`zh-CN.js` 是文案基准**：新增文案先写它，再同步 `en.js` 与 `zh-TW.js`。`i18n.test.js` 会校验三份 catalog 的键集与占位符完全一致，漏译即测试失败。
- 缺失的键回落到简体中文，仍缺失则返回键本身，便于开发期发现漏译。
- 当前语言取自 `setting.language`；`auto` 时按 `navigator.language` 推断。`App.svelte` 用 `$effect` 追踪该字段，改动即时换文案，并同步写 `<html lang>`（影响字体回退与断行）。
- `lib/` 里的纯逻辑模块（`pointer.js`、`grouping.js`、`theme.js`）也经由 `t()` 取文案；因默认语言为简体中文，它们的既有单测无需改动。

::: warning NO_TITLE 不是文案
`grouping.js` 的 `NO_TITLE`（`"无标题窗口"`）与核心 `zonedeck_common::NO_TITLE` 一致，是写进 `config.json` 的跨进程哨兵值，**不可翻译**；仅在展示时用 `t("common.noTitleWindow")` 换成当前语言。
:::

## 公告与更新日志的 Markdown 渲染

公告（列表与启动弹窗）与更新日志的正文按 Markdown 渲染，由 `lib/markdown.js`（纯函数，配 `markdown.test.js`）与 `components/Markdown.svelte`（样式 + 链接拦截）承担，不引入第三方依赖。

支持 GitHub 风格的常用子集：标题、粗体 / 斜体 / 删除线、行内代码与围栏代码块、有序 / 无序 / 任务列表（含缩进嵌套）、引用、分割线、链接与裸链接。段落内的软换行渲染成 `<br>`，与 GitHub 评论一致。**不支持**表格、脚注、行内 HTML 与语法高亮。

::: warning 正文来自远端，渲染前必须转义
Verhub 返回的内容不可信。`renderMarkdown()` 先整体转义 `& < > "` 再拼标签，输出里出现的标签全部由渲染器自己生成，源文本中的 HTML 只会作为字面量显示；链接目标仅放行 `http(s)` 与 `mailto`，其余（`javascript:`、`data:` 等）按纯文本保留。改动该模块时勿绕过这两条。
:::

另有两处与运行环境相关的取舍：

- **图片退化成链接**：Tauri 的 CSP 中 `img-src` 只允许 `self` 与 `data:`，远端图片加载不出来，因此 `![]()` 一律渲染为链接。
- **链接不走 webview 导航**：`Markdown.svelte` 拦截点击并转交 `open_external`，由系统浏览器打开——webview 真导航走了就回不到配置界面。

## 状态轮询不卡界面

- 状态获取合并为**单条 `GetStatus`** 管道命令，并使用**连接快速失败**（不重试）。
- Tauri 命令用 `spawn_blocking` 异步化，避免阻塞。
- 前端每 2 秒轮询一次状态（页面不可见时暂停）。

## 前端不依赖 dev server

最终产物中，前端在**编译期被内嵌**进 `zonedeck-config.exe`，静态运行，**不需要任何本地服务器**。

- 开发前端 UI：`npm run dev` 在浏览器里预览（mock 数据、热重载）。
- 验证 Tauri 集成：`npm run build` 后 `cargo run -p zonedeck-config`。

## 与核心的联动示例

- 核心托盘的"窗口恢复工具" / "关于"会带参数拉起配置界面：冷启动从启动参数读取，已运行时由单实例插件发来事件。
- 首次启动若核心未运行，配置界面会自动 `startCore` 拉起核心。
