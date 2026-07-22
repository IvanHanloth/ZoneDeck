---
title: Architecture
---

# Architecture

Boss Key v3 uses a **two-process architecture** that separates the **core from the settings program**, connected by a **named pipe**. This chapter covers the overall design and the project layout.

## Two-process overview

```
┌────────────────────────────────────────────────────────────────┐
│ Interactive user session (Session 1+)                            │
│                                                                  │
│  ┌──────────────────────────┐        ┌────────────────────────┐ │
│  │ Boss Key.exe (resident)   │        │ config.exe             │ │
│  │ Pure native Rust, ~350KB  │◀─IPC──▶│ Tauri (Rust + WebView) │ │
│  │ Hidden message window     │ named  │ Opened on demand,      │ │
│  │ + wndproc                 │ pipe   │ exits when closed      │ │
│  │ • RegisterHotKey hotkeys  │        │ • Binding / recording  │ │
│  │ • WH_MOUSE_LL mouse       │        │ • Options / about      │ │
│  │ • GetLastInputInfo idle   │        │ • Elevation / recovery │ │
│  │ • Enumerate/hide/show     │        └────────────┬───────────┘ │
│  │ • Core Audio muting       │                     │ read/write   │
│  │ • NtSuspend freezing      │        ┌────────────▼───────────┐ │
│  │ • Tray icon / balloons    │        │ config.json (next to exe) │
│  │ • Startup (task/registry) │        │ hot-reloaded on reload │ │
│  └──────────────────────────┘        └────────────────────────┘ │
│         ▲ starts at logon                                        │
└─────────┼────────────────────────────────────────────────────────┘
          │ Scheduled task (logon trigger, highest privileges) / registry Run fallback
```

## Key design decisions

### The core must run in the interactive user session

The core **cannot** be a Session 0 Windows service, or it could not enumerate and hide user windows or install hooks. It therefore runs as an ordinary program in the user session, started at logon.

### Inter-process communication (IPC)

- Uses the named pipe `\\.\pipe\bosskey`, with **one JSON object per line** (`Command` / `Response`).
- After the settings are saved, the settings window sends `reload_config`; the core **hot-reloads** and re-registers hotkeys, hooks and timers, **without restarting**.
- For protocol details see [IPC protocol](/en/dev/ipc-protocol).

### All monitoring is user-mode; no kernel driver

| Capability | API used | Notes |
| --- | --- | --- |
| Global hotkeys | `RegisterHotKey` | The most standard and complete trigger mechanism |
| Hotkey interception | `WH_KEYBOARD_LL` | Installed only when a hotkey has "don't pass through" enabled |
| Mouse / corners | `WH_MOUSE_LL` | Installed only when mouse or corner triggers are enabled |
| Idle detection | `GetLastInputInfo` | No need to monitor the keyboard continuously |

No kernel driver is involved, which reduces false positives and security risk.

### Privilege model

The core defaults to `asInvoker` and **does not force UAC**. Only these two need administrator, obtained on demand through "Restart core as administrator" in the settings window:

- Enhanced freezing (`pssuspend64.exe`);
- Highest-privilege scheduled-task startup.

## Project layout (Cargo workspace)

```
Boss-Key/
├── Cargo.toml                      workspace (including release profile tuning)
├── crates/
│   ├── common/                     Shared library (no platform dependency; builds cross-platform)
│   │   └── src/{model,config,matching,ipc,verhub,i18n}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule (serde-compatible with the old config.json; PID uppercase)
│   │       config    Config/Setting/Hotkey (reads old configurations + migration)
│   │       matching  Window matching logic
│   │       ipc       Command/Response protocol + PipeClient
│   │       verhub    Models for versions / announcements / update checks
│   │       i18n      Language tags (Lang) and preference resolution, shared by core and settings
│   └── core/                       Resident core (lib + bin)
│       └── src/
│           platform/win32.rs  Window enumeration/hiding/showing (WindowManager trait)
│           agent.rs      Message loop; aggregates hotkeys/tray/IPC/mouse/timers
│           hotkey.rs     Hotkey string → RegisterHotKey parsing
│           hide.rs       Hiding selection logic + HideController (hide/show orchestration)
│           effects.rs    Effects trait (muting/freezing/pause key; mockable)
│           audio.rs      Core Audio session muting
│           freeze.rs     NtSuspend/Resume + pssuspend64 enhanced freezing
│           mouse_hook.rs WH_MOUSE_LL (middle/side buttons, corners)
│           keyboard_hook.rs WH_KEYBOARD_LL ("don't pass through" hotkey interception)
│           idle.rs       GetLastInputInfo idle detection + auto-hide decision
│           tray.rs       Shell_NotifyIcon tray + balloons
│           ipc_server.rs Named-pipe server
│           autostart.rs  Startup (scheduled-task XML with restart-on-failure + registry fallback)
│           elevation.rs  Administrator detection + UAC elevation restart
│           i18n.rs       Catalog of user-visible core strings (tray menu / balloons / IPC errors; logs excluded)
│           logging.rs    Levelled file logging (logs/BossKey-YYYY-MM-DD.log, rotated daily + panic hook)
│           recovery.rs   Crash recovery (hidden state persisted; windows recovered after an abnormal exit)
│           icon.rs       Process icon extraction (HICON → hand-written PNG/base64 encoding)
│           single_instance.rs  Named-mutex single instance
└── apps/config/                    Settings window (Tauri 2 + Svelte 5)
    ├── src-tauri/  Rust backend commands + tauri.conf.json + capabilities
    ├── ui/         Frontend source (Vite + Svelte 5)
    │   └── src/    lib/ (pure logic + vitest tests) + components/ (Svelte components)
    │                + locales/ (three-language catalogs; zh-CN.js is the source of truth)
    └── dist/       Frontend build output (gitignored; produced from ui/ by vite build)
```

::: tip Why common has no platform dependency
`crates/common` deliberately avoids the Windows API so it can be compiled cross-platform, and its pure logic (configuration parsing, matching, protocol) is easier to unit test. Platform-specific code lives in `crates/core`.
:::

## Inside the core: the agent message loop

`agent.rs` is the hub: it creates a **hidden message window**, runs the Windows message loop, and aggregates these event sources:

- Global hotkeys (`WM_HOTKEY`);
- The mouse hook (middle / side buttons, corners);
- The named-pipe server (commands from the settings window);
- Timers (idle detection, state maintenance, and so on);
- Tray icon interaction.

When hiding or showing is triggered, `HideController` orchestrates it: select the matching windows → apply `Effects` (muting / freezing / pause key) → hide or show the windows, and write the state to `recovery.json`.

::: info Designed for testability
`Effects` is a trait, so tests can inject a mock and verify the hiding orchestration without actually muting or freezing the system. `WindowManager` is a trait for the same reason.
:::

## Stability (three layers of crash self-healing)

1. **Crash logs**: key events and panics are written to `logs/BossKey-YYYY-MM-DD.log` next to the exe (rotated daily, retained per `log_retention_days`; 0 disables logging; release builds drop the DEBUG level).
2. **Crash recovery**: hiding writes what was hidden / frozen / muted into `recovery.json`, recovered automatically on the next start after an abnormal exit.
3. **Watchdog**: the scheduled task's `RestartOnFailure` (restart within a minute of a crash, up to 3 times). Release builds use `panic = "abort"`, and the panic hook exits with a non-zero code once the log is written — exactly what triggers the scheduled-task restart.

For the user-facing explanation see [Window recovery & crash self-healing](/en/guide/recovery).

## Display language

The core and the settings program share `Lang` (`zh-CN` / `en` / `zh-TW`) and preference resolution from `crates/common`, but maintain their strings separately:

| Location | String storage | Notes |
| --- | --- | --- |
| `crates/core/src/i18n.rs` | A `Msg` enum + a `match` per language | Tray menu, balloons, IPC errors; `tf()` substitutes `{name}` placeholders |
| `apps/config/ui/src/locales/*.js` | Flat key–value tables | Every string in the settings window; `t(key, params)` looks up and substitutes |

- The effective language comes from `setting.language`: with `auto` it is inferred from the system display language (the core uses `GetUserDefaultLocaleName`, the frontend uses `navigator.language`), falling back to Simplified Chinese when it cannot be inferred.
- The settings window sends `reload_config` after saving, so the core picks up the language too — switching language restarts neither process.
- **Logs are excluded from i18n** and are always Simplified Chinese, so problems can be diagnosed across languages.
- `NO_TITLE` (`"无标题窗口"`) is a cross-process sentinel written into `config.json`; it **does not change with the language** and is only translated for display.

## Frontend architecture

The settings frontend uses Svelte 5 + Vite, with a frameless self-drawn window, light/dark themes and automatic saving. See [Frontend & settings UI](/en/dev/frontend).
