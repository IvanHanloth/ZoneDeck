# Boss Key v3（Rust 重写版）开发与使用说明

> 本文件对应 **v3 重构版**（Rust + Tauri）。v2.x 及更早的 Python + wxPython 版本说明见 [README.md](README.md)。
> 旧 Python 源码仍保留在 [`main/`](main/) 目录，作为功能迁移的对照参考。

## 1. 这是什么

Boss Key 是 Windows 平台的“老板键”摸鱼工具：一键隐藏/显示指定窗口，另配静音、进程冻结、鼠标手势、空闲自动隐藏等。

v3 用 Rust 重写，目标是：**更低内存、更稳、单文件原生二进制（不易被杀软误报）、更现代的配置界面**。核心常驻二进制仅约 **350 KB**，配置界面约 **3 MB**（前端已内嵌）。

## 2. 系统架构

采用 **核心 + 配置分离** 的双进程架构，二者通过 **命名管道** 通信：

```
┌────────────────────────────────────────────────────────────────┐
│ 用户交互会话（Session 1+）                                       │
│                                                                  │
│  ┌──────────────────────────┐        ┌────────────────────────┐ │
│  │ bosskey-core.exe（常驻）  │        │ bosskey-config.exe     │ │
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

关键设计：

- **核心必须运行在用户交互会话**（不能做成 Session 0 的 Windows 服务，否则无法枚举/隐藏用户窗口、无法装钩子）。
- **IPC**：命名管道 `\\.\pipe\bosskey`，一行一条 JSON（`Command` / `Response`）。配置保存后发送 `reload_config`，核心热重载并重注册热键/钩子/定时器，**无需重启核心**。
- **监听方式**：全部用户态——`RegisterHotKey`（全局热键，最不像键盘记录器）、`WH_MOUSE_LL`（仅在启用鼠标/四角时安装）、`GetLastInputInfo`（空闲检测，无需常驻键盘监听）。**不使用内核驱动**。
- **管理员**：核心默认 `asInvoker`，不强制 UAC。仅“增强冻结”和“计划任务最高权限自启”需要管理员——通过配置界面的“以管理员身份重启核心”按需提权。

### 工程结构（Cargo workspace）

```
Boss-Key/
├── Cargo.toml                      workspace（含 release profile 调优）
├── crates/
│   ├── common/                     共享库（无平台依赖，可跨平台编译）
│   │   └── src/{model,config,matching,ipc}.rs
│   │       model     WindowInfo（serde 兼容旧 config.json，PID 大写）
│   │       config    Config/Setting/Hotkey（兼容读取旧配置）
│   │       matching  is_same_window 窗口匹配（移植自 tools.py）
│   │       ipc       Command/Response 协议 + PipeClient 客户端
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
│           idle.rs       GetLastInputInfo 空闲 + 自动隐藏判定
│           tray.rs       Shell_NotifyIcon 托盘 + 气泡
│           ipc_server.rs 命名管道服务端
│           autostart.rs  开机自启（计划任务 XML 含失败自动重启 + 注册表回退）
│           elevation.rs  管理员检测 + UAC 提权重启
│           logging.rs    崩溃日志（bosskey.log 大小轮转 + panic 钩子）
│           recovery.rs   崩溃恢复（隐藏状态落盘，异常退出后找回窗口）
│           icon.rs       进程图标提取（HICON → 手写 PNG/base64 编码）
│           single_instance.rs  命名互斥单实例
└── apps/config/                    配置界面（Tauri 2）
    ├── src-tauri/  Rust 后端命令 + tauri.conf.json + capabilities
    └── dist/       前端（index.html / styles.css / main.js，无打包器）
```

## 3. 环境准备

- **Rust**：stable（建议 1.85+，本项目用 edition 2024）。安装：<https://rustup.rs>
- **配置界面运行时**：Microsoft Edge **WebView2**（Windows 10/11 通常已内置）。
- **可选** `tauri-cli`（仅用于热重载开发与打生成安装包）：`cargo install tauri-cli`
- **可选** `pssuspend64.exe`（增强冻结，来自 [Microsoft PSTools](https://download.sysinternals.com/files/PSTools.zip)）。

## 4. 常用命令

在仓库根目录执行：

| 目的 | 命令 |
|---|---|
| **运行核心**（开发） | `cargo run -p bosskey-core` |
| 核心冒烟自测（N 毫秒后自动退出） | `cargo run -p bosskey-core -- smoke 3000` |
| **运行配置界面**（开发，用内嵌前端） | `cargo run -p bosskey-config` |
| 配置界面热重载开发（需 tauri-cli） | `cd apps/config/src-tauri && cargo tauri dev` |
| **开发编译**（不优化，快） | `cargo build` |
| **生产编译**（优化，体积最小） | `cargo build --release` |
| **运行全部测试** | `cargo test --workspace` |
| 静态检查 | `cargo clippy --workspace --all-targets -- -D warnings` |
| 代码格式化 / 检查 | `cargo fmt --all` / `cargo fmt --all -- --check` |
| **一键生产打包**（独立文件夹） | `powershell -File scripts/package.ps1` |

> 提示：若杀软拦截了新编译出的可执行文件（表现为 `os error 5 拒绝访问`），请将本项目 `target` 目录加入杀软信任区。

## 5. 生产打包（独立、可直接使用的文件夹）

运行：

```powershell
powershell -File scripts/package.ps1
```

脚本会执行 `cargo build --release`，并把可直接分发的独立文件夹组装到 **`package/Boss-Key/`**：

```
package/Boss-Key/
├── bosskey-core.exe     常驻核心（双击启动，后台 + 托盘）
├── bosskey-config.exe   配置界面（前端已内嵌，自包含）
├── icon.ico             托盘图标
└── 使用说明.txt
```

该文件夹**无需安装、无外部依赖**（除系统自带的 WebView2）：拷到任意目录，双击 `bosskey-core.exe` 即可使用；配置界面从托盘“设置”打开，或直接运行 `bosskey-config.exe`。两个程序通过同目录的 `config.json` 与命名管道协作，`bosskey-config.exe` 会在同目录定位 `bosskey-core.exe`（如“以管理员身份重启核心”）。

## 6. 配置文件

`config.json` 与可执行文件同目录，**结构与旧版完全兼容**（旧用户配置可直接沿用）。首次运行若不存在则使用默认值。字段见 `crates/common/src/config.rs`。

## 7. 稳定性设计（崩溃自愈三层防线）

1. **崩溃日志**：核心把关键事件与 panic 信息写入 exe 同目录的 `bosskey.log`（超过 512KB 自动轮转为 `bosskey.log.old`）。排查问题先看这个文件。
2. **崩溃恢复**：每次隐藏窗口时把「隐藏了哪些窗口、冻结/静音了哪些进程」写入 `recovery.json`；正常显示/退出时删除。核心重启时若发现该文件，说明上次是异常退出——自动找回窗口、解冻、取消静音，**窗口不会因为核心崩溃而永远消失**。
3. **看门狗**：开机自启的计划任务以 XML 注册，带 `RestartOnFailure`（崩溃后 1 分钟内自动重启，最多 3 次）且取消了默认 3 天运行时长限制。release 构建 `panic = "abort"`，panic 钩子写完日志后进程以非零码退出，正好触发计划任务重启，重启后走第 2 条防线恢复窗口。

## 8. 测试说明

- `bosskey-common`：模型/配置/匹配/协议的纯逻辑单元测试 + 配置文件读写集成测试。
- `bosskey-core`：窗口枚举/隐藏/显示（创建真实窗口往返）、热键解析、单实例互斥、命名管道服务端、进程冻结（真实子进程）、静音 COM 链路、HideController（mock 注入验证静音/冻结/暂停键编排）、开机自启（打普通注册表键验证逻辑 + 真实 schtasks 接受任务 XML）、崩溃日志（写入/轮转/panic 钩子）、崩溃恢复（快照落盘往返 + mock 恢复编排）、图标编码（CRC32/Adler-32/base64 已知向量 + PNG 结构解析往返 + 真实 explorer.exe 提取）、以及经 `PipeClient` 驱动真实 agent 的端到端 IPC / 崩溃恢复测试。
- 运行：`cargo test --workspace`。
