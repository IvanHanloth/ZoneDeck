---
title: Configuration fields
---

# Configuration field reference

ZoneDeck stores its configuration in `config.json` — in the program folder for a portable copy, in `%APPDATA%\ZoneDeck` for an installed one; see [Data folder](/en/dev/architecture#data-folder). When the location changes, the old configuration is migrated across automatically. The structure is **fully compatible with older versions**, so existing configurations carry over. If the file is missing on first run, defaults are used. The field definitions live in `crates/common/src/config.rs`.

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
| `whitelist` | object[] | Whitelist: per-process opt-out of hiding / freezing / muting |
| `hide_binding` | object[] | *v2* flat bindings, used only for migration; cleared afterwards and never written back |

## `hotkey`

| Field | Default | Description |
| --- | --- | --- |
| `hide_hotkey` | `"Ctrl+Q"` | Hide / show windows |
| `close_hotkey` | `"Win+Esc"` | Close the core |
| `hide_only_hotkey` | `""` | [Hide windows only](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `show_only_hotkey` | `""` | [Show windows only](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `hide_foreground_hotkey` | `""` | [Hide foreground window](/en/guide/hotkeys#one-way-hotkeys-and-hiding-the-foreground-window); empty means disabled |
| `hide_hook` | `false` | [Trigger the hide hotkey through the low-level keyboard hook](/en/guide/hotkeys#the-keyboard-hook-and-keeping-keys-from-other-apps) |
| `close_hook` | `false` | Trigger the close hotkey through the low-level keyboard hook |
| `hide_only_hook` | `false` | Trigger the hide-only hotkey through the low-level keyboard hook |
| `show_only_hook` | `false` | Trigger the show-only hotkey through the low-level keyboard hook |
| `hide_foreground_hook` | `false` | Trigger the hide-foreground hotkey through the low-level keyboard hook |
| `hide_intercept` | `false` | [Don't pass the hide hotkey through](/en/guide/hotkeys#the-keyboard-hook-and-keeping-keys-from-other-apps); when true, `hide_hook` is forced true as well |
| `close_intercept` | `false` | Don't pass the close hotkey through |
| `hide_only_intercept` | `false` | Don't pass the hide-only hotkey through |
| `show_only_intercept` | `false` | Don't pass the show-only hotkey through |
| `hide_foreground_intercept` | `false` | Don't pass the hide-foreground hotkey through |

Hotkey strings support [richer combinations](/en/guide/hotkeys#richer-combinations): several main keys joined with `+` (up to four, e.g. `"Q+W"`), or modifiers alone for a modifier-only hotkey (e.g. `"Ctrl+Shift"`). Only the keyboard hook can carry those two kinds, so the core routes them through it even when the matching `*_hook` is off. Punctuation keys are stored by key position (`OEM_1`, `OEM_PLUS` and so on) and do not shift with the keyboard layout.

## `setting`

| Field | Type | Default | Feature |
| --- | --- | --- | --- |
| `mute_after_hide` | bool | `true` | [Mute after hiding](/en/guide/hiding) |
| `send_before_hide` | bool | `false` | [Send the pause key before hiding](/en/guide/hiding) |
| `minimize_before_hide` | bool | `false` | [Minimise windows before hiding](/en/guide/hiding) |
| `hide_current` | bool | `true` | [Also hide the active window](/en/guide/hiding) |
| `hide_icon_after_hide` | bool | `false` | [Also hide ZoneDeck's tray icon](/en/guide/hiding) |
| `tray_enabled` | bool | `true` | [Show the tray icon](/en/guide/notifications#show-the-tray-icon); when false the icon never appears and balloons / badges go with it |
| `tray_clicks` | object | See below | [Tray icon click actions](/en/guide/notifications#tray-icon-click-actions) |
| `tray_badges` | object | See below | [Tray icon status](/en/guide/notifications#tray-icon-status) |
| `tray_show_tooltip` | bool | `true` | [Tray icon tooltip](/en/guide/notifications#tray-icon-tooltip) |
| `freeze_after_hide` | bool | `false` | [Freezing master switch](/en/guide/freeze) |
| `enhanced_freeze` | bool | `false` | [Enhanced freezing](/en/guide/freeze) |
| `power_scope` | string | `"self"` | [Freezing & memory scope](/en/guide/freeze): `self` (target process only) ｜ `tree` (and all its children) ｜ `image` (all instances of the same image name); governs freezing and memory trimming, unknown values normalise to `self` |
| `efficiency_after_hide` | bool | `false` | [Efficiency mode](/en/guide/freeze): drop hidden processes to EcoQoS + low priority; independent of freezing |
| `efficiency_scope` | string | `"self"` | [Efficiency mode scope](/en/guide/freeze), same values as `power_scope`, independent of the freezing scope |
| `trim_memory_after_freeze` | bool | `false` | [Reduce memory usage](/en/guide/freeze) (frozen processes only) |
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

::: details Legacy "freeze the whole process tree" switch (deprecated)
`freeze_whole_tree` exists only for deserialisation and migration: if a config has no `power_scope`, `true` migrates to `power_scope: "tree"` and `false` to `"self"`, after which the old key is cleared and never written back. Configs that already set `power_scope` explicitly are unaffected.
:::

::: details Legacy "toggle hiding by clicking the tray icon" switch (deprecated)
`click_to_hide` exists only for deserialisation and migration: if a config has no `tray_clicks`, `true` migrates to `tray_clicks.left: "toggle"` and `false` to `"none"`, after which the old key is cleared and never written back. Configs that lack the key entirely are treated as `true` (the old default was on). Configs that already set `tray_clicks` explicitly are unaffected.
:::

### `setting.tray_clicks`

[Tray icon click actions](/en/guide/notifications#tray-icon-click-actions): each of the three clicks gets one action, out of `none` (do nothing) ｜ `toggle` (hide / show windows) ｜ `menu` (open the tray menu) ｜ `settings` (open the settings window). Unknown values normalise to `none`.

| Field | Default | Description |
| --- | --- | --- |
| `left` | `"toggle"` | Single click |
| `double` | `"none"` | Double click. When it is not `none`, a single click waits out the system double-click time before running |
| `right` | `"menu"` | Right click |

### `setting.mouse`

Each button is a `MouseButton`: `{ enabled: bool, clicks: 1..=3, modifiers: string }`.

| Field | Default | Description |
| --- | --- | --- |
| `left` / `middle` / `right` / `side1` / `side2` | See below | The trigger condition for each of the five buttons |
| `multi_click_ms` | `350` | Multi-click interval (milliseconds, 150–1000) |
| `allow_click_restore` | `true` | Allow restoring by pressing again |

::: info Defaults for a fresh installation
A fresh installation enables **a middle-button double click** (`middle.enabled = true`, `clicks = 2`) and leaves the other four off. An old configuration without a `mouse` section reads as **all off**.
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
| `on_recovery_mismatch` | `true` | Notify when, after an abnormal exit, some hidden-window records no longer match what was recorded |

## `verhub`

| Field | Default | Description |
| --- | --- | --- |
| `include_preview` | `false` | Whether update checks include preview releases |
| `seen_announcement_id` | `""` | The id of the newest announcement already read |
| `analytics` | `null` | Consent for anonymous usage statistics: `null` means the user has not been asked yet and the first run prompts for it; `true` granted, `false` declined |
| `analytics_consent_sent` | `false` | Whether the "took part" event has already been sent. One per device; toggling the switch again does not resend it |

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

## `whitelist`

Declares, per process, which modes to skip; see [Whitelist](/en/guide/whitelist). Matching mirrors `process_rules`, except it defaults to **file-name** matching, and both file names and paths compare case-insensitively.

| Field | Default | Description |
| --- | --- | --- |
| `process` | | Process name |
| `path` | | Executable path |
| `regex` | | Regex (applied to the path or the file name) |
| `by_name` | `true` | Match on the file name only, ignoring the path |
| `ignore_hide` | `false` | Never hide this program's windows |
| `ignore_freeze` | `false` | Never freeze this program's processes after hiding |
| `ignore_mute` | `false` | Never mute this program's processes after hiding |

::: tip Missing key vs empty array
When the `whitelist` **key is absent** (old or brand-new configs), a default `explorer.exe` entry is seeded. Writing `[]` means the user emptied the list and nothing is seeded again. After normalization the field is always an array, never `null`.
:::

ZoneDeck's own core and settings app (`ZoneDeck.exe` / `core.exe` / `config.exe` / `zonedeck-config.exe`) are **always excluded from freezing**. That guard lives in `BUILTIN_FREEZE_GUARDS` in `crates/common/src/matching.rs`, never appears in the config file, and cannot be bypassed by editing it.

## Compatibility and migration

- **Unknown fields are ignored**: fields added later do not break parsing in an older core.
- **Missing fields use defaults**: any missing field falls back to its default.
- **Corrupt files are backed up first**: if the whole file fails to parse, it is renamed to `config.json.bad` in the same directory before defaults take over, so rules can be recovered manually; the backup location is recorded in the log.
- **Old bindings migrate automatically**: `hide_binding` → `window_rules`; the old mouse switches → `mouse` click triggers. Migration is **idempotent**.
- **Uppercase `PID`**: serialisation emits an uppercase `PID`, compatible with the old Python versions.
