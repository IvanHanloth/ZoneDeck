---
title: 配置文件字段
---

# 配置文件字段参考

ZoneDeck 的配置保存在 `config.json` 中，便携版存在程序目录，安装版存在 `%APPDATA%\ZoneDeck`，详见[数据目录](/dev/architecture#数据目录)。位置变动时旧配置会自动迁移过去。**结构与旧版完全兼容**，旧用户配置可直接沿用。首次运行若不存在则使用默认值。字段定义见 `crates/common/src/config.rs`。

::: tip 一般无需手改
配置由配置界面自动读写并保存，通常无需手动编辑。本页面向需要理解字段含义的开发者。
:::

## 顶层结构

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `version` | string | 配置 schema 版本，结构变动时才动 |
| `app_version` | string | 上次运行过的**程序**版本；与当前程序版本不符即「更新后首次启动」，核心据此自动弹出配置界面。缺省置空 |
| `history` | number[] | 历史记录（时间戳） |
| `frozen_pids` | number[] | 当前被冻结的进程 PID（用于恢复） |
| `hotkey` | object | 键盘热键，见下 |
| `setting` | object | 主要设置，见下 |
| `notifications` | object | 通知开关，见下 |
| `verhub` | object | 更新 / 公告相关，见下 |
| `window_rules` | object[] | 窗口规则（细粒度） |
| `process_rules` | object[] | 进程规则（粗粒度） |
| `whitelist` | object[] | 白名单：逐进程声明忽略隐藏 / 冻结 / 静音 |
| `hide_binding` | object[] | *V2版*扁平绑定，仅用于迁移，迁移后清空、不再写回 |

## `hotkey`

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `hide_hotkey` | `"Ctrl+Q"` | 隐藏 / 显示窗口 |
| `close_hotkey` | `"Win+Esc"` | 关闭核心 |
| `hide_only_hotkey` | `""` | [仅隐藏窗口](/guide/hotkeys#单向热键与隐藏前台窗口)，置空为关闭 |
| `show_only_hotkey` | `""` | [仅显示窗口](/guide/hotkeys#单向热键与隐藏前台窗口)，置空为关闭 |
| `hide_foreground_hotkey` | `""` | [隐藏前台窗口](/guide/hotkeys#单向热键与隐藏前台窗口)，置空为关闭 |
| `hide_intercept` | `false` | [隐藏热键不传递](/guide/hotkeys#热键不传递)（键盘钩子拦截） |
| `close_intercept` | `false` | [关闭热键不传递](/guide/hotkeys#热键不传递)（键盘钩子拦截） |
| `hide_only_intercept` | `false` | 仅隐藏热键不传递 |
| `show_only_intercept` | `false` | 仅显示热键不传递 |
| `hide_foreground_intercept` | `false` | 隐藏前台窗口热键不传递 |

## `setting`

| 字段 | 类型 | 默认 | 对应功能 |
| --- | --- | --- | --- |
| `mute_after_hide` | bool | `true` | [隐藏后静音](/guide/hiding#隐藏窗口后静音) |
| `send_before_hide` | bool | `false` | [隐藏前发送暂停键](/guide/hiding#隐藏前发送暂停键) |
| `minimize_before_hide` | bool | `false` | [隐藏前先最小化窗口](/guide/hiding#隐藏前先最小化窗口) |
| `hide_current` | bool | `true` | [同时隐藏当前活动窗口](/guide/hiding#同时隐藏当前活动窗口) |
| `click_to_hide` | bool | `true` | [单击托盘切换隐藏](/guide/hiding#单击托盘图标切换隐藏) |
| `hide_icon_after_hide` | bool | `false` | [同时隐藏 ZoneDeck 托盘图标](/guide/hiding#同时隐藏-zonedeck-托盘图标) |
| `tray_badges` | object | 见下 | [图标状态提示](/guide/notifications#图标状态提示) |
| `tray_show_tooltip` | bool | `true` | [显示图标悬浮名称](/guide/notifications#显示图标悬浮名称) |
| `freeze_after_hide` | bool | `false` | [进程冻结总开关](/guide/freeze#隐藏窗口时冻结进程) |
| `enhanced_freeze` | bool | `false` | [增强冻结](/guide/freeze#使用增强冻结) |
| `power_scope` | string | `"self"` | [冻结及内存控制范围](/guide/freeze#作用范围)：`self`（仅目标进程）｜`tree`（及所有子进程）｜`image`（同映像名的所有实例）；决定冻结与降低内存占用的覆盖面，未知取值归一为 `self` |
| `efficiency_after_hide` | bool | `false` | [效率模式](/guide/freeze#效率模式)：隐藏后把进程降为 EcoQoS + 低优先级，与冻结相互独立 |
| `efficiency_scope` | string | `"self"` | [效率模式范围](/guide/freeze#作用范围)，取值同 `power_scope`，与冻结的范围互不影响 |
| `trim_memory_after_freeze` | bool | `false` | [降低内存占用](/guide/freeze#降低内存占用)（仅对被冻结的进程生效） |
| `show_float_window` | bool | `false` | 悬浮窗（开发中） |
| `mouse` | object | 见下 | [鼠标按键隐藏](/guide/hotkeys#鼠标按键隐藏) |
| `auto_hide_enabled` | bool | `false` | [空闲自动隐藏](/guide/hotkeys#空闲自动隐藏) |
| `auto_hide_time` | number | `5` | 空闲时长（分钟，1–120） |
| `top_left_hide` 等四角 | bool | `false` | [四角隐藏](/guide/hotkeys#移动到屏幕四角隐藏) |
| `corner_fast_only` | bool | `true` | 仅快速移动触发 |
| `allow_move_restore` | bool | `false` | 角落恢复 |
| `log_retention_days` | number | `7` | [日志保留天数](/guide/options#日志保留天数)（0 = 关闭） |
| `log_level` | string | `"warn"` | [日志输出等级](/guide/options#日志输出等级)：`debug`｜`info`｜`warn`｜`error` |
| `autostart_admin` | bool | `false` | [以管理员身份自启](/guide/autostart)（仅计划任务方式生效） |
| `language` | string | `"auto"` | [界面语言](/guide/options#界面语言)：`auto`｜`zh-CN`｜`en`｜`zh-TW` |



::: details 旧版扁平鼠标开关（已废弃）
`middle_button_hide` / `side_button1_hide` / `side_button2_hide` 仅用于反序列化迁移，迁移后清零、不再写回文件。请使用 `mouse` 结构。
:::

::: details 旧版「冻结完整进程」开关（已废弃）
`freeze_whole_tree` 仅用于反序列化迁移：读取时若配置里没有 `power_scope`，`true` 迁移为 `power_scope: "tree"`、`false` 迁移为 `"self"`，随后清零、不再写回文件。已显式配过 `power_scope` 的文件不受它影响。
:::

### `setting.mouse`

每颗按键为一个 `MouseButton`：`{ enabled: bool, clicks: 1..=3, modifiers: string }`。

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `left` / `middle` / `right` / `side1` / `side2` | 见下 | 五颗按键各自的触发条件 |
| `multi_click_ms` | `350` | 连击判定窗口（毫秒，150–1000） |
| `allow_click_restore` | `true` | 允许再按一次恢复 |

::: info 全新安装默认
全新安装默认开启**中键双击**（`middle.enabled = true`，`clicks = 2`），其余四颗关闭。配置文件缺 `mouse` 一节的老配置读进来则**全关**。
:::

### `setting.tray_badges`

[图标状态提示](/guide/notifications#图标状态提示)：四种颜色的圆点角标各自绑定一个状态源，多个状态同时活跃时按**红 > 绿 > 黄 > 蓝**的优先级只显示一个圆点。

| 字段 | 默认 | 默认含义 |
| --- | --- | --- |
| `red` | `"hidden"` | 存在隐藏中的窗口 |
| `green` | `"auto_hide"` | 启用了自动隐藏 |
| `yellow` | `"hide_current"` | 启用了同时隐藏当前窗口 |
| `blue` | `"freeze"` | 启用了进程冻结 |

每项取值：`hidden`（存在隐藏中的窗口）｜`auto_hide`（启用了自动隐藏）｜`hide_current`（启用了同时隐藏当前窗口）｜`freeze`（启用了进程冻结）｜`elevated`（以管理员身份运行）｜`monitor_paused`（热键监控已暂停）｜`""`（置空 = 不显示该颜色）；未知取值读取时归一为置空。

## `notifications`

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `on_start` | `true` | 核心启动通知 |
| `on_quit` | `true` | 核心退出通知 |
| `on_autostart` | `true` | 开机自启状态变更通知 |
| `on_hide` | `false` | 每次隐藏通知 |
| `on_show` | `false` | 每次显示通知 |

## `verhub`

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `include_preview` | `false` | 更新检查是否纳入预览版 |
| `seen_announcement_id` | `""` | 已读的最新公告 id |

## `window_rules`（窗口规则）

细粒度规则，按句柄 + 标题锁定单个窗口；`regex` 为 `Some` 时按标题正则命中。

| 字段 | 说明 |
| --- | --- |
| `title` | 窗口标题 |
| `hwnd` | 窗口句柄 |
| `process` | 进程名 |
| `PID` | 进程 ID（**大写** key，兼容旧配置） |
| `path` | 可执行文件路径 |
| `regex` | 标题正则（高级模式；省略表示精确规则） |
| `include_untitled` | 正则是否纳入无标题窗口 |
| `include_background` | 正则是否纳入后台窗口 |

## `process_rules`（进程规则）

粗粒度规则，按可执行文件隐藏该程序的所有窗口。

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `process` | | 进程名 |
| `path` | | 可执行文件路径 |
| `regex` | | 正则（作用于路径或文件名） |
| `by_name` | `false` | 只按文件名匹配，忽略路径 |
| `include_untitled` | `true` | 是否纳入无标题窗口（进程规则默认纳入） |
| `include_background` | `false` | 是否纳入后台窗口 |

## `whitelist`（白名单）

按进程声明在哪些模式下跳过，见[白名单](/guide/whitelist)。匹配方式与 `process_rules` 同构，但**默认按文件名**匹配；文件名与路径的比较不区分大小写。

| 字段 | 默认 | 说明 |
| --- | --- | --- |
| `process` | | 进程名 |
| `path` | | 可执行文件路径 |
| `regex` | | 正则（作用于路径或文件名） |
| `by_name` | `true` | 只按文件名匹配，忽略路径 |
| `ignore_hide` | `false` | 隐藏时跳过该程序的窗口 |
| `ignore_freeze` | `false` | 隐藏后不冻结该程序的进程 |
| `ignore_mute` | `false` | 隐藏后不静音该程序的进程 |

::: tip 键缺失 vs 空数组
`whitelist` **键不存在**（老配置 / 全新配置）时会播种一条默认的 `explorer.exe`；写成 `[]` 表示用户清空了列表，不会再被播种。归一后该字段恒为数组，不会是 `null`。
:::

ZoneDeck 自身的核心与配置程序（`ZoneDeck.exe` / `core.exe` / `config.exe` / `zonedeck-config.exe`）**恒被排除在冻结之外**。该保护写死在 `crates/common/src/matching.rs` 的 `BUILTIN_FREEZE_GUARDS` 里，不出现在配置文件中，手改配置也绕不过去。

## 兼容与迁移

- **未知字段被忽略**：未来新增字段不会导致旧核心解析失败。
- **缺失字段用默认值**：任意字段缺失都回退到默认值。
- **解析失败先备份**：整份配置解析失败时回退默认值启动，原文件先改名为同目录 `config.json.bad` 备份，规则可从中手工找回；备份去向记录在日志里。
- **旧绑定自动迁移**：`hide_binding` → `window_rules`；旧鼠标开关 → `mouse` 连击触发。迁移是**幂等**的。
- **`PID` 大写**：序列化输出使用大写 `PID`，与旧 Python 版本兼容。
