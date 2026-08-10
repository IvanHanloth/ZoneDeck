---
title: Configuration fields
---

# Configuration field reference

Boss Key stores its configuration in `config.json` — in the program folder for a portable copy, in `%APPDATA%\BossKey` for an installed one; see [Data folder](/en/dev/architecture#data-folder). When the location changes, the old configuration is migrated across automatically. The structure is **fully compatible with older versions**, so existing configurations carry over. If the file is missing on first run, defaults are used. The field definitions live in `crates/common/src/config.rs`.

::: tip You normally do not edit this by hand
The settings window reads and writes the configuration automatically. This page is for developers who need to understand the fields.
:::

## Top level

| Field | Type | Description |
| --- | --- | --- |
| `version` | string | Configuration schema version; only changes when the structure does |
| `app_version` | string | The program version last seen; when it differs from the current one this is the first run after an update, and the core opens the settings window automatically. Empty by default |
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
| `close_hotkey` | `"Win+Esc"` | Close the core |
| `hide_only_hotkey` | `""` | [Hide windows only](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `show_only_hotkey` | `""` | [Show windows only](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `hide_foreground_hotkey` | `""` | [Hide foreground window](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `hide_intercept` | `false` | [Don't pass the hide hotkey through](/en/guide/hotkeys#keeping-hotkeys-from-other-apps) (keyboard-hook interception) |
| `close_intercept` | `false` | [Don't pass the close hotkey through](/en/guide/hotkeys#keeping-hotkeys-from-other-apps) (keyboard-hook interception) |
| `hide_only_intercept` | `false` | Don't pass the hide-only hotkey through |
| `show_only_intercept` | `false` | Don't pass the show-only hotkey through |
| `hide_foreground_intercept` | `false` | Don't pass the hide-foreground hotkey through |

## `setting`

| Field | Type | Default | Feature |
| --- | --- | --- | --- |
| `mute_after_hide` | bool | `true` | [Mute after hiding](/en/guide/options) |
| `send_before_hide` | bool | `false` | [Send the pause key before hiding](/en/guide/options) |
| `hide_current` | bool | `true` | [Also hide the active window](/en/guide/options) |
| `click_to_hide` | bool | `true` | [Toggle hiding by clicking the tray icon](/en/guide/options) |
| `hide_icon_after_hide` | bool | `false` | [Also hide Boss Key's tray icon](/en/guide/options) |
| `tray_badges` | object | See below | [Tray icon status](/en/guide/notifications#tray-icon-status) |
| `tray_show_tooltip` | bool | `true` | [Tray icon tooltip](/en/guide/notifications#tray-icon-tooltip) |
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
| `log_level` | string | `"warn"` | [Log level](/en/guide/options): `debug` \| `info` \| `warn` \| `error` |
| `autostart_admin` | bool | `false` | [Start as administrator](/en/guide/autostart) (scheduled-task method only) |
| `language` | string | `"auto"` | [Display language](/en/guide/options): `auto` \| `zh-CN` \| `en` \| `zh-TW` |

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

### `setting.tray_badges`

[Tray icon status](/en/guide/notifications#tray-icon-status): each of the four colored dot badges is bound to a state source; when several bound states are active at once, only one dot is shown, in **red > green > yellow > blue** priority order.

| Field | Default | Default meaning |
| --- | --- | --- |
| `red` | `"hidden"` | Windows are hidden |
| `green` | `"auto_hide"` | Auto hide is enabled |
| `yellow` | `"hide_current"` | Also-hide-active-window is enabled |
| `blue` | `"freeze"` | Process freezing is enabled |

Each field accepts `hidden` (windows are hidden) \| `auto_hide` (auto hide is enabled) \| `hide_current` (also-hide-active-window is enabled) \| `freeze` (process freezing is enabled) \| `elevated` (running as administrator) \| `monitor_paused` (hotkey monitoring is paused) \| `""` (empty = do not show that color); unknown values are normalised to empty on read.

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
- **Corrupt files are backed up first**: if the whole file fails to parse, it is renamed to `config.json.bad` in the same directory before defaults take over, so rules can be recovered manually; the backup location is recorded in the log.
- **Old bindings migrate automatically**: `hide_binding` → `window_rules`; the old mouse switches → `mouse` click triggers. Migration is **idempotent**.
- **Uppercase `PID`**: serialisation emits an uppercase `PID`, compatible with the old Python versions.
