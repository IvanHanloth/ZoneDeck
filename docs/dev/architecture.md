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
│  │ • 托盘图标 / 气泡通知      │        │ config.json（数据目录）     │
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
│   │   └── src/{model,config,matching,ipc,i18n,paths}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule（serde 兼容旧 config.json，PID 大写）
│   │       config    Config/Setting/Hotkey（兼容读取旧配置 + 迁移；保存走 tmp + rename 原子替换）
│   │       matching  窗口匹配逻辑
│   │       paths     数据目录定位（安装版走 %APPDATA%，便携版就地，见下）
│   │       ipc       Command/Response 协议 + PipeClient 客户端
│   │       i18n      界面语言标签（Lang）与语言偏好解析，核心与配置程序共用
│   └── core/                       常驻核心（lib + bin）
│       └── src/
│           platform/win32.rs  窗口枚举/隐藏/显示（WindowManager trait）
│           agent.rs      消息循环，聚合热键/托盘/IPC/鼠标/定时器
│           hotkey.rs     热键字符串 → RegisterHotKey 解析
│           hide.rs       隐藏选择逻辑 + HideController（plan/commit 两段式编排，
│                         含死句柄剪枝与恢复前的窗口/进程身份校验）
│           effects.rs    Effects trait（静音/冻结/暂停键，可注入 mock）
│           effects_worker.rs  副作用专职线程（FIFO 队列；消息循环只做 SW_HIDE）
│           audio.rs      Core Audio 会话静音
│           freeze.rs     NtSuspend/Resume + pssuspend64 增强冻结
│           input_hooks.rs 输入钩子专职线程（承载两个低级钩子，优先级 above normal）
│           mouse_hook.rs WH_MOUSE_LL（中键/侧键/四角）
│           keyboard_hook.rs WH_KEYBOARD_LL（「不传递」热键拦截）
│           idle.rs       GetLastInputInfo 空闲 + 自动隐藏判定
│           win_event.rs  SetWinEventHook 窗口事件追踪（销毁/显示/改标题 → 实时维护记录）
│           tray.rs       Shell_NotifyIcon 托盘 + 气泡
│           ipc_server.rs 命名管道服务端（创建失败退避重试，不退出）
│           autostart.rs  开机自启（计划任务 XML 含失败自动重启 + 注册表回退）
│           elevation.rs  管理员检测 + UAC 提权重启
│           i18n.rs       核心用户可见文案 catalog（托盘菜单 / 气泡 / IPC 错误；日志不走它）
│           logging.rs    分级文件日志（logs/BossKey-YYYY-MM-DD.log 按天切割 + panic 钩子）
│           recovery.rs   崩溃恢复（意图先行落盘 + 原子写；快照带开机时刻与
│                         进程创建时刻，跨重启的快照会被丢弃）
│           icon.rs       进程图标提取（HICON → 手写 PNG/base64 编码）
│           single_instance.rs  命名互斥单实例
└── apps/config/                    配置界面（Tauri 2 + Svelte 5）
    ├── src-tauri/  Rust 后端命令 + tauri.conf.json + capabilities
    │   └── src/verhub.rs  Verhub 客户端（版本/公告/反馈/日志/项目链接，基于 verhub-sdk；
    │                      项目链接带缓存：内存 + 数据目录下的 verhub_cache.json，有效期一天）
    ├── ui/         前端源码（Vite + Svelte 5）
    │   └── src/    lib/（纯逻辑 + vitest 测试）+ components/（Svelte 组件）
    │                + locales/（三语文案 catalog，以 zh-CN.js 为基准）
    └── dist/       前端构建产物（gitignore；由 ui/ 经 vite build 生成）
```

::: tip common 为什么无平台依赖
`crates/common` 刻意不依赖 Windows API，因此可以跨平台编译，其纯逻辑（配置解析、匹配、协议）也更易做单元测试。平台相关代码集中在 `crates/core`。
:::

## 数据目录

配置 `config.json`、日志 `logs/`、恢复文件 `recovery.json`、缓存 `verhub_cache.json` 共处一个**数据目录**，由 `crates/common/src/paths.rs` 定位。安装版与便携版分开对待：

| 情形 | 数据目录 | `DataDirKind` |
| --- | --- | --- |
| 安装版 | `%APPDATA%\BossKey` | `Installed` |
| 便携版，程序目录可写 | 程序目录 | `Portable` |
| 便携版，程序目录写不进去 | `%APPDATA%\BossKey` | `PortableFallback` |

便携版把数据留在程序目录，拷走整个文件夹就带走了全部设置；安装版则不能这么做——安装包可以装进 `Program Files`，那里普通权限进程不可写，配置程序每次保存都会得到 `os error 5`。

### 怎么分辨是哪一种

看程序目录里有没有安装痕迹（`paths::is_installed`）：

1. 安装包放的标记文件 `installed.marker`（`[Files]` 里装，卸载时随之移除）；
2. 卸载程序 `unins*.exe` —— 兜底，标记文件被误删时仍认得出是安装版，不至于把数据写回 `Program Files`。序号随重复安装递增，故按前缀匹配。

::: warning 判据必须是文件，不能是进程权限
核心可能以管理员身份运行、配置程序不会：核心在 `Program Files` 下写得进去，配置程序写不进去。若两边各按自己能否写入来选目录，就会各读一份配置，用户改了设置却不生效。看文件则两边必然一致。也因此，安装版根本不做可写性探测——结果一样是用户目录。
:::

### 回退与迁移

便携版探测到程序目录不可写时退回用户目录，`kind` 记为 `PortableFallback`。核心把它写进日志，配置程序通过 `data_location` 命令读到后弹出提示，说明这是权限问题以及怎么改（见 `DataNoticeModal.svelte`）。程序功能不受影响。

用到用户目录时，程序目录里的 `config.json` 会搬过来：先复制，再尽力删掉原文件。目标已有配置就不动它——那是当前在用的一份，旧文件不得覆盖，也不去删。删不掉（没有写权限、文件被占用）就留在原处，反正不会再被读到。

::: tip 配置界面的浏览器数据另有一处
Tauri 按 `tauri.conf.json` 里的 identifier 把 WebView2 用户数据放在 `%LOCALAPPDATA%\cn.hanloth.bosskey.config`，不在数据目录里，也不由 `paths.rs` 管。安装包的卸载程序与便携版随包的 `scripts/cleanup.ps1` 都会清理它。
:::

每次启动的实际数据目录与判定结果会写进日志首屏，排查读写失败先看它。

## 核心内部：Agent 消息循环

`agent.rs` 是核心的中枢：它创建一个**隐藏的消息窗口**并运行 Windows 消息循环，聚合以下事件源：

- 全局热键（`WM_HOTKEY`）；
- 鼠标钩子（中键 / 侧键 / 四角）；
- 命名管道服务端（来自配置界面的命令）；
- 定时器（空闲检测、状态维护等）；
- 窗口事件（`SetWinEventHook`：顶层窗口销毁 / 显示 / 改标题）；
- 托盘图标交互。

消息循环状态由 `RefCell` 承载：托盘 / 悬浮窗菜单的模态循环（`TrackPopupMenu`）会重入 `wndproc`，重入期间的事件借用失败即被安全丢弃，避免出现两个可变引用的别名。IPC 线程创建命名管道失败时按退避（1s → 5s → 30s）重试，不会退出。

### 低级输入钩子不与消息循环同线程

`WH_MOUSE_LL` / `WH_KEYBOARD_LL` 的回调由**安装线程的消息泵**派发，且系统的输入线程要等钩子链返回才继续投递事件。若与 agent 同线程，枚举窗口、写恢复文件、处理全系统窗口事件这类操作会直接拖慢全局鼠标与键盘输入，单次超过 `LowLevelHooksTimeout`（默认 300ms）时系统还会丢弃该事件。

故 `input_hooks.rs` 单起一条只跑消息泵的线程承载这两个钩子，线程优先级提到 above normal，回调里只做纯内存判定与 `PostMessageW`（鼠标移动这条最热的路径上不加锁，采样存在原子里）。agent 线程通过一个仅消息窗口向它同步下发装卸请求，并据返回值决定是否回退（键盘钩子装不上时「不传递」热键退化为 `RegisterHotKey`）。

agent 线程本身**不**提优先级：它干的是枚举 / 冻结 / 落盘这类重活，抬高只会从前台程序手里抢 CPU。

窗口事件驱动隐藏记录的实时维护：被隐藏的窗口自行销毁或被外部恢复显示时，记录即刻移除并落盘；标题变化同步进隐藏记录与精确窗口规则（仅内存，随下次正常落盘写出），使「标题 + 进程路径」的追溯与找回始终基于最新信息。恢复时若句柄已失效，还会按「进程路径 + 标题」在当前不可见窗口中尝试重新找回。

当触发隐藏 / 显示时，交由 `HideController` 编排，流程为「意图先行」两段式：`plan_hide` 算出执行计划（剪掉失效记录、补齐 PID）→ 把计划后的快照写入 `recovery.json`（先落盘再动手，隐藏中途崩溃不丢记录）→ `commit_hide` 同步隐藏窗口（`SW_HIDE`），并把静音 / 冻结 / 暂停键交给副作用专职线程（`effects_worker.rs`）按 FIFO 异步执行——消息循环不被慢操作（音频枚举、pssuspend 等待）阻塞，热键与界面保持响应。

队列内的先后有讲究：暂停键→静音→静置→冻结。冻结让进程彻底停止响应消息，隐藏若还没在屏幕上画完就冻结，被冻结的窗口会留下残影；发出去的暂停键同样要有时间被目标程序处理掉。故冻结前统一静置一次（`FREEZE_SETTLE_DELAY`，整批只等一次，没有要冻结的进程就不等）。静音不排在这道等待之后——它走音频会话，与目标进程是否在跑无关。

恢复（显示）时逐条校验记录的有效性：句柄须仍存在且仍属于当初的进程（`IsWindow` + PID 比对），冻结 / 静音记录须匹配进程创建时刻——句柄与 PID 都会被系统回收复用，校验不过的记录跳过并如实计入日志。

::: info 可测试性设计
`Effects` 被抽象为 trait，测试时可注入 mock，从而在不真正静音 / 冻结系统的情况下验证隐藏编排逻辑。同理 `WindowManager` 也是 trait。
:::

## 稳定性设计（崩溃自愈三层防线）

1. **崩溃日志**：关键事件与 panic 写入[数据目录](#数据目录)下的 `logs/BossKey-YYYY-MM-DD.log`（按天切割，按 `log_retention_days` 保留，0 表示关闭日志；release 构建丢弃 DEBUG 级）。
2. **崩溃恢复**：隐藏动作执行前先把"将要隐藏 / 冻结 / 静音什么"写入 `recovery.json`（tmp + rename 原子替换），异常退出后重启自动找回；快照带开机时刻与进程创建时刻，跨重启的过期快照直接丢弃，不会对无关窗口 / 进程做恢复动作。
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
