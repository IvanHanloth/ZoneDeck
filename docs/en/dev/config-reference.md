---
title: Configuration fields
---

# Configuration field reference

Boss Key stores its configuration in `config.json`, in the **same folder** as the executable. The structure is **fully compatible with older versions**, so existing configurations carry over. If the file is missing on first run, defaults are used. The field definitions live in `crates/common/src/config.rs`.

::: tip You normally do not edit this by hand
The settings window reads and writes the configuration automatically. This page is for developers who need to understand the fields.
:::

## Top level

| Field | Type | Description |
| --- | --- | --- |
| `version` | string | Configuration version |
| `history` | number[] | History (timestamps) |
| `frozen_pids` | number[] | PIDs of currently frozen processes (used for recovery) |
| `hotkey` | object | Keyboard hotkeys; see below |
| `setting` | object | Main settings; see below |
| `notifications` | object | Notification switches; see below |
| `verhub` | object | Updates / announcements; see below |
| `window_rules` | object[] | Window rules (fine-grained) |
| `process_rules` | object[] | Process rules (coarse-grained) |
| `hide_binding` | object[] | *v2* flat bindings, used only for migration; cleared afterwards and never written back |

## `hotkey`

| Field | Default | Description |
| --- | --- | --- |
| `hide_hotkey` | `"Ctrl+Q"` | Hide / show windows |
| `close_hotkey` | `"Win+Esc"` | Close programs |

## `setting`

| Field | Type | Default | Feature |
| --- | --- | --- | --- |
| `mute_after_hide` | bool | `true` | [Mute after hiding](/en/guide/options) |
| `send_before_hide` | bool | `false` | [Send the pause key before hiding](/en/guide/options) |
| `hide_current` | bool | `true` | [Also hide the active window](/en/guide/options) |
| `click_to_hide` | bool | `true` | [Toggle hiding by clicking the tray icon](/en/guide/options) |
| `hide_icon_after_hide` | bool | `false` | [Also hide the tray icon](/en/guide/options) |
| `freeze_after_hide` | bool | `false` | [Freezing master switch](/en/guide/freeze) |
| `enhanced_freeze` | bool | `false` | [Enhanced freezing](/en/guide/freeze) |
| `freeze_whole_tree` | bool | `false` | [Freeze the whole process tree](/en/guide/freeze) |
| `show_float_window` | bool | `false` | Floating window (in development) |
| `mouse` | object | See below | [Hiding with mouse buttons](/en/guide/hotkeys) |
| `auto_hide_enabled` | bool | `false` | [Auto-hide when idle](/en/guide/hotkeys) |
| `auto_hide_time` | number | `5` | Idle time (minutes, 1–120) |
| `top_left_hide` and the other corners | bool | `false` | [Corner hiding](/en/guide/hotkeys) |
| `corner_fast_only` | bool | `true` | Only trigger on fast movement |
| `allow_move_restore` | bool | `false` | Restore from a corner |
| `log_retention_days` | number | `7` | [Log retention](/en/guide/options) (0 = off) |
| `autostart_admin` | bool | `false` | [Start as administrator](/en/guide/autostart) (scheduled-task method only) |
| `language` | string | `"auto"` | [Display language](/en/guide/options): `auto` \| `zh-CN` \| `en` \| `zh-TW` |

::: info Values and normalisation of `language`
`auto` follows the system display language. Values are normalised on read: valid BCP-47 tags collapse to `zh-CN` / `en` / `zh-TW` (for example `zh_TW` and `zh-Hant` → `zh-TW`; `en-US` → `en`), and values with no matching translation (such as `ja-JP`) fall back to `auto`. The core and the settings program share this field.
:::

::: details Legacy flat mouse switches (deprecated)
`middle_button_hide` / `side_button1_hide` / `side_button2_hide` exist only for deserialisation and migration; they are cleared afterwards and never written back. Use the `mouse` structure instead.
:::

### `setting.mouse`

Each button is a `MouseButton`: `{ enabled: bool, clicks: 1..=3, modifiers: string }`.

| Field | Default | Description |
| --- | --- | --- |
| `left` / `middle` / `right` / `side1` / `side2` | See below | The trigger condition for each of the five buttons |
| `multi_click_ms` | `350` | Multi-click interval (milliseconds, 150–1000) |
| `allow_click_restore` | `true` | Allow restoring by pressing again |

::: info Defaults for a fresh installation
A fresh installation enables **a middle-button single click** (`middle.enabled = true`, `clicks = 1`) and leaves the other four off. An old configuration without a `mouse` section reads as **all off**.
:::

## `notifications`

| Field | Default | Description |
| --- | --- | --- |
| `on_start` | `true` | Core started |
| `on_quit` | `true` | Core exited |
| `on_autostart` | `true` | Startup setting changed |
| `on_hide` | `false` | Every hide |
| `on_show` | `false` | Every show |

## `verhub`

| Field | Default | Description |
| --- | --- | --- |
| `include_preview` | `false` | Whether update checks include preview releases |
| `seen_announcement_id` | `""` | The id of the newest announcement already read |

## `window_rules`

Fine-grained rules targeting a single window by handle plus title. When `regex` is `Some`, matching is by title regex.

| Field | Description |
| --- | --- |
| `title` | Window title |
| `hwnd` | Window handle |
| `process` | Process name |
| `PID` | Process ID (**uppercase** key, for compatibility with old configurations) |
| `path` | Executable path |
| `regex` | Title regex (advanced mode; omitted means an exact rule) |
| `include_untitled` | Whether the regex includes untitled windows |
| `include_background` | Whether the regex includes background windows |

## `process_rules`

Coarse-grained rules that hide every window of a program, matched by executable.

| Field | Default | Description |
| --- | --- | --- |
| `process` | | Process name |
| `path` | | Executable path |
| `regex` | | Regex (applied to the path or the file name) |
| `by_name` | `false` | Match on the file name only, ignoring the path |
| `include_untitled` | `true` | Whether to include untitled windows (process rules include them by default) |
| `include_background` | `false` | Whether to include background windows |

## Compatibility and migration

- **Unknown fields are ignored**: fields added later do not break parsing in an older core.
- **Missing fields use defaults**: any missing field falls back to its default.
- **Old bindings migrate automatically**: `hide_binding` → `window_rules`; the old mouse switches → `mouse` click triggers. Migration is **idempotent**.
- **Uppercase `PID`**: serialisation emits an uppercase `PID`, compatible with the old Python versions.
