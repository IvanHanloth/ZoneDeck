---
title: IPC 协议
---

# IPC 协议

配置界面（`config.exe`）与常驻核心（`ZoneDeck.exe`）通过**命名管道**通信。协议定义在 `crates/common/src/ipc.rs`。

## 传输

- 管道名：`\\.\pipe\zonedeck`。
- 编码：**一行一条 JSON**，以 `\n` 分隔。客户端发送一条 `Command`，服务端回一条 `Response`。
- 客户端封装为 `PipeClient`：默认带重试（25 次、40ms 间隔）；`.fast()` 为**快速失败**模式（只尝试一次），用于状态轮询等不希望阻塞的场景。

## Command（配置界面 → 核心）

序列化使用 `#[serde(tag = "cmd", rename_all = "snake_case")]`。

| 命令 | JSON | 作用 |
| --- | --- | --- |
| `ReloadConfig` | `{"cmd":"reload_config"}` | 重新读取配置并热重载（重注册热键 / 钩子 / 定时器） |
| `GetState` | `{"cmd":"get_state"}` | 查询隐藏状态 |
| `GetElevation` | `{"cmd":"get_elevation"}` | 查询是否管理员 |
| `GetStatus` | `{"cmd":"get_status"}` | **一次往返取回全部状态**（隐藏 + 权限 + 监控），替代前两者连发 |
| `Hide` | `{"cmd":"hide"}` | 隐藏 |
| `Show` | `{"cmd":"show"}` | 显示 |
| `Toggle` | `{"cmd":"toggle"}` | 切换隐藏 / 显示 |
| `SetAutostart` | `{"cmd":"set_autostart","enabled":true}` | 设置开机自启 |
| `SetHotkeys` | `{"cmd":"set_hotkeys","enabled":false}` | 临时停用 / 恢复热键与鼠标监控 |
| `ReleaseWindows` | `{"cmd":"release_windows","hwnds":[..]}` | 窗口恢复工具：恢复显示指定句柄。在核心记录里的窗口按整进程释放（连同解冻 / 取消静音）；记录外的句柄直接显示 |
| `AdoptWindows` | `{"cmd":"adopt_windows","hwnds":[..]}` | 窗口恢复工具：隐藏指定句柄并纳入核心记录（享受崩溃恢复），不施加静音 / 冻结 |
| `Quit` | `{"cmd":"quit"}` | 退出核心 |

## Response（核心 → 配置界面）

序列化使用 `#[serde(tag = "type", rename_all = "snake_case")]`。

| 响应 | JSON | 说明 |
| --- | --- | --- |
| `Ok` | `{"type":"ok"}` | 命令成功、无额外数据 |
| `State` | `{"type":"state","hidden":true}` | 当前隐藏状态 |
| `Elevated` | `{"type":"elevated","elevated":true}` | 是否管理员 |
| `Status` | `{"type":"status","hidden":..,"elevated":..,"monitoring":..}` | 聚合状态 |
| `Error` | `{"type":"error","message":".."}` | 出错信息 |

`Status.monitoring`：核心是否正在监听热键与鼠标（被 `SetHotkeys` 停用时为 `false`）。

::: info 错误信息随界面语言变化
`Error.message` 的文案取自核心的文案 catalog，随 `setting.language` 变化。它是展示给用户的文本，**不要**当作稳定标识去做条件判断。
:::

## 监控停用与心跳

`SetHotkeys { enabled: false }` 用于配置界面在**录制 / 调试热键**时临时停用核心监控，避免误触发。该停用**有状态**，需持续心跳续期：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `SUSPEND_TIMEOUT_MS` | `15000` | 看门狗时长：超过这么久没收到心跳，核心自动恢复监控 |
| `SUSPEND_HEARTBEAT_MS` | `4000` | 配置界面重发心跳的建议间隔（须显著小于超时） |

::: info 为什么要看门狗
若配置界面在停用监控期间崩溃 / 被强杀，来不及发送恢复命令，核心也会在超时后**自动恢复监控**，不会让用户的热键永久失灵。
:::

## 典型交互时序

```
配置界面保存配置 ──▶ reload_config ──▶ 核心热重载 ──▶ ok
状态轮询（每 2s） ──▶ get_status(fast) ──▶ status{hidden,elevated,monitoring}
进入热键设置区 ──▶ set_hotkeys{false} + 定时心跳 ──▶ 核心暂停监控
离开 / 失焦 ──▶ set_hotkeys{true} ──▶ 核心恢复监控
```
