---
title: 系统架构
---

# 系统架构

Boss Key v3 采用 **核心 + 配置分离** 的**双进程架构**，两者通过**命名管道**通信。本章介绍整体设计与工程结构。

## 双进程总览

```
┌────────────────────────────────────────────────────────────────┐
│ 用户交互会话（Session 1+）                                       │
│                                                                  │
│  ┌──────────────────────────┐        ┌────────────────────────┐ │
│  │ Boss Key.exe（常驻）      │        │ config.exe             │ │
│  │ 纯 Rust 原生，~350KB      │◀─IPC──▶│ Tauri（Rust + WebView）│ │
│  │ 隐藏的消息窗口 + wndproc  │ 命名   │ 按需打开，关闭即退出   │ │
│  │                          │ 管道   │                        │ │
│  │ • RegisterHotKey 全局热键 │        │ • 窗口绑定 / 热键录制   │ │
│  │ • WH_MOUSE_LL 鼠标/四角   │        │ • 选项 / 关于 / 检查更新│ │
│  │ • GetLastInputInfo 空闲   │        │ • 管理员重启 / 窗口恢复 │ │
│  │ • 枚举/隐藏/显示窗口       │        └────────────┬───────────┘ │
│  │ • Core Audio 静音         │                     │ 读写         │
│  │ • NtSuspend 进程冻结       │        ┌────────────▼───────────┐ │
│  │ • 托盘图标 / 气泡通知      │        │ config.json（与 exe 同目录）│
│  │ • 开机自启（计划任务/注册表）        │ 核心收到 reload 后热重载 │ │
│  └──────────────────────────┘        └────────────────────────┘ │
│         ▲ 随登录自启                                             │
└─────────┼────────────────────────────────────────────────────────┘
          │ 计划任务(登录触发,最高权限) / 注册表 Run 回退
```

## 关键设计决策

### 核心必须运行在用户交互会话

核心**不能**做成 Session 0 的 Windows 服务，否则无法枚举 / 隐藏用户窗口、无法安装钩子。因此核心以随登录自启的普通程序形式运行在用户会话中。

### 进程间通信（IPC）

- 使用命名管道 `\\.\pipe\bosskey`，**一行一条 JSON**（`Command` / `Response`）。
- 配置保存后，配置界面发送 `reload_config`，核心**热重载**并重新注册热键 / 钩子 / 定时器，**无需重启核心**。
- 协议细节见 [IPC 协议](/dev/ipc-protocol)。

### 全部用户态监听，不使用内核驱动

| 能力 | 使用的 API | 说明 |
| --- | --- | --- |
| 全局热键 | `RegisterHotKey` | 最规范、完善的触发方式 |
| 热键不传递 | `WH_KEYBOARD_LL` | 仅在有热键开启「不传递」时才安装 |
| 鼠标 / 四角 | `WH_MOUSE_LL` | 仅在启用鼠标 / 四角时才安装 |
| 空闲检测 | `GetLastInputInfo` | 无需常驻监听键盘 |

不依赖任何内核驱动，降低误报与安全风险。

### 权限模型

核心默认 `asInvoker`，**不强制 UAC**。仅以下两项需要管理员，通过配置界面的"以管理员身份重启核心"**按需提权**：

- 增强冻结（`pssuspend64.exe`）；
- 计划任务最高权限自启。

## 工程结构（Cargo workspace）

```
Boss-Key/
├── Cargo.toml                      workspace（含 release profile 调优）
├── crates/
│   ├── common/                     共享库（无平台依赖，可跨平台编译）
│   │   └── src/{model,config,matching,ipc,verhub,i18n}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule（serde 兼容旧 config.json，PID 大写）
│   │       config    Config/Setting/Hotkey（兼容读取旧配置 + 迁移）
│   │       matching  窗口匹配逻辑
│   │       ipc       Command/Response 协议 + PipeClient 客户端
│   │       verhub    版本 / 公告 / 更新检查相关模型
│   │       i18n      界面语言标签（Lang）与语言偏好解析，核心与配置程序共用
│   └── core/                       常驻核心（lib + bin）
│       └── src/
│           platform/win32.rs  窗口枚举/隐藏/显示（WindowManager trait）
│           agent.rs      消息循环，聚合热键/托盘/IPC/鼠标/定时器
│           hotkey.rs     热键字符串 → RegisterHotKey 解析
│           hide.rs       隐藏选择逻辑 + HideController（隐藏/显示编排）
│           effects.rs    Effects trait（静音/冻结/暂停键，可注入 mock）
│           audio.rs      Core Audio 会话静音
│           freeze.rs     NtSuspend/Resume + pssuspend64 增强冻结
│           mouse_hook.rs WH_MOUSE_LL（中键/侧键/四角）
│           keyboard_hook.rs WH_KEYBOARD_LL（「不传递」热键拦截）
│           idle.rs       GetLastInputInfo 空闲 + 自动隐藏判定
│           tray.rs       Shell_NotifyIcon 托盘 + 气泡
│           ipc_server.rs 命名管道服务端
│           autostart.rs  开机自启（计划任务 XML 含失败自动重启 + 注册表回退）
│           elevation.rs  管理员检测 + UAC 提权重启
│           i18n.rs       核心用户可见文案 catalog（托盘菜单 / 气泡 / IPC 错误；日志不走它）
│           logging.rs    分级文件日志（logs/BossKey-YYYY-MM-DD.log 按天切割 + panic 钩子）
│           recovery.rs   崩溃恢复（隐藏状态落盘，异常退出后找回窗口）
│           icon.rs       进程图标提取（HICON → 手写 PNG/base64 编码）
│           single_instance.rs  命名互斥单实例
└── apps/config/                    配置界面（Tauri 2 + Svelte 5）
    ├── src-tauri/  Rust 后端命令 + tauri.conf.json + capabilities
    ├── ui/         前端源码（Vite + Svelte 5）
    │   └── src/    lib/（纯逻辑 + vitest 测试）+ components/（Svelte 组件）
    │                + locales/（三语文案 catalog，以 zh-CN.js 为基准）
    └── dist/       前端构建产物（gitignore；由 ui/ 经 vite build 生成）
```

::: tip common 为什么无平台依赖
`crates/common` 刻意不依赖 Windows API，因此可以跨平台编译，其纯逻辑（配置解析、匹配、协议）也更易做单元测试。平台相关代码集中在 `crates/core`。
:::

## 核心内部：Agent 消息循环

`agent.rs` 是核心的中枢：它创建一个**隐藏的消息窗口**并运行 Windows 消息循环，聚合以下事件源：

- 全局热键（`WM_HOTKEY`）；
- 鼠标钩子（中键 / 侧键 / 四角）；
- 命名管道服务端（来自配置界面的命令）；
- 定时器（空闲检测、状态维护等）；
- 托盘图标交互。

当触发隐藏 / 显示时，交由 `HideController` 编排：选择命中的窗口 → 应用 `Effects`（静音 / 冻结 / 暂停键）→ 隐藏 / 显示窗口，并把状态写入 `recovery.json`。

::: info 可测试性设计
`Effects` 被抽象为 trait，测试时可注入 mock，从而在不真正静音 / 冻结系统的情况下验证隐藏编排逻辑。同理 `WindowManager` 也是 trait。
:::

## 稳定性设计（崩溃自愈三层防线）

1. **崩溃日志**：关键事件与 panic 写入 exe 同目录的 `logs/BossKey-YYYY-MM-DD.log`（按天切割，按 `log_retention_days` 保留，0 表示关闭日志；release 构建丢弃 DEBUG 级）。
2. **崩溃恢复**：隐藏时把"隐藏 / 冻结 / 静音了什么"写入 `recovery.json`，异常退出后重启自动找回。
3. **看门狗**：计划任务 `RestartOnFailure`（崩溃后 1 分钟内重启，最多 3 次）。release 构建 `panic = "abort"`，panic 钩子写完日志后以非零码退出，正好触发计划任务重启。

用户视角的说明见 [窗口恢复与崩溃自愈](/guide/recovery)。

## 界面语言

核心与配置程序共用 `crates/common` 的 `Lang`（`zh-CN` / `en` / `zh-TW`）与语言偏好解析，文案则各自维护：

| 位置 | 文案载体 | 说明 |
| --- | --- | --- |
| `crates/core/src/i18n.rs` | `Msg` 枚举 + 三语 `match` | 托盘菜单、气泡通知、IPC 错误；`tf()` 负责 `{名字}` 占位符替换 |
| `apps/config/ui/src/locales/*.js` | 扁平键值表 | 配置界面全部文案；`t(key, params)` 查表并替换占位符 |

- 生效语言由 `setting.language` 决定：`auto` 时按系统显示语言推断（核心用 `GetUserDefaultLocaleName`，前端用 `navigator.language`），推断不出回落到简体中文。
- 配置界面保存配置后会发 `reload_config`，核心据此同步语言，因此切换语言无需重启任一进程。
- **日志不参与 i18n**，一律使用简体中文，以便跨语言排查问题。
- `NO_TITLE`（`"无标题窗口"`）是跨进程、写进 `config.json` 的哨兵值，**不随语言变化**，仅在展示时翻译。

## 前端架构

配置界面前端使用 Svelte 5 + Vite，采用无边框自绘窗口、亮暗主题、配置自动保存等设计。详见 [前端与配置界面](/dev/frontend)。
