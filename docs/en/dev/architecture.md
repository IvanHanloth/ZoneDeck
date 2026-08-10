---
title: Architecture
---

# Architecture

ZoneDeck v3 uses a **two-process architecture** that separates the **core from the settings program**, connected by a **named pipe**. This chapter covers the overall design and the project layout.

## Two-process overview

```
┌────────────────────────────────────────────────────────────────┐
│ Interactive user session (Session 1+)                            │
│                                                                  │
│  ┌──────────────────────────┐        ┌────────────────────────┐ │
│  │ ZoneDeck.exe (resident)   │        │ config.exe             │ │
│  │ Pure native Rust, ~350KB  │◀─IPC──▶│ Tauri (Rust + WebView) │ │
│  │ Hidden message window     │ named  │ Opened on demand,      │ │
│  │ + wndproc                 │ pipe   │ exits when closed      │ │
│  │ • RegisterHotKey hotkeys  │        │ • Binding / recording  │ │
│  │ • WH_MOUSE_LL mouse       │        │ • Options / about      │ │
│  │ • GetLastInputInfo idle   │        │ • Elevation / recovery │ │
│  │ • Enumerate/hide/show     │        └────────────┬───────────┘ │
│  │ • Core Audio muting       │                     │ read/write   │
│  │ • NtSuspend freezing      │        ┌────────────▼───────────┐ │
│  │ • Tray icon / balloons    │        │ config.json (data folder) │
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

- Uses the named pipe `\\.\pipe\zonedeck`, with **one JSON object per line** (`Command` / `Response`).
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
│   │   └── src/{model,config,matching,ipc,i18n,paths}.rs
│   │       model     WindowInfo / WindowRule / ProcessRule (serde-compatible with the old config.json; PID uppercase)
│   │       config    Config/Setting/Hotkey (reads old configurations + migration; saves via tmp + rename)
│   │       matching  Window matching logic
│   │       paths     Data folder resolution (%APPDATA% when installed, in place when portable; see below)
│   │       ipc       Command/Response protocol + PipeClient
│   │       i18n      Language tags (Lang) and preference resolution, shared by core and settings
│   └── core/                       Resident core (lib + bin)
│       └── src/
│           platform/win32.rs  Window enumeration/hiding/showing (WindowManager trait)
│           agent.rs      Message loop; aggregates hotkeys/tray/IPC/mouse/timers
│           hotkey.rs     Hotkey string → RegisterHotKey parsing
│           hide.rs       Hiding selection logic + HideController (plan/commit two-phase orchestration
│                         with stale-handle pruning and window/process identity checks before restore)
│           effects.rs    Effects trait (muting/freezing/pause key; mockable)
│           effects_worker.rs  Dedicated side-effect thread (FIFO queue; the message loop only does SW_HIDE)
│           audio.rs      Core Audio session muting
│           freeze.rs     NtSuspend/Resume + pssuspend64 enhanced freezing
│           input_hooks.rs Dedicated input-hook thread (owns both low-level hooks, above-normal priority)
│           mouse_hook.rs WH_MOUSE_LL (middle/side buttons, corners)
│           keyboard_hook.rs WH_KEYBOARD_LL ("don't pass through" hotkey interception)
│           idle.rs       GetLastInputInfo idle detection + auto-hide decision
│           win_event.rs  SetWinEventHook window-event tracking (destroy/show/title change → live record upkeep)
│           tray.rs       Shell_NotifyIcon tray + balloons
│           ipc_server.rs Named-pipe server (retries pipe creation with backoff instead of exiting)
│           autostart.rs  Startup (scheduled-task XML with restart-on-failure + registry fallback)
│           elevation.rs  Administrator detection + UAC elevation restart
│           i18n.rs       Catalog of user-visible core strings (tray menu / balloons / IPC errors; logs excluded)
│           logging.rs    Levelled file logging (daily rotation + level filter + redaction + panic hook)
│           recovery.rs   Crash recovery (intent persisted before acting + atomic writes; snapshots carry
│                         boot time and process creation times, snapshots from a previous boot are discarded)
│           icon.rs       Process icon extraction (HICON → hand-written PNG/base64 encoding)
│           single_instance.rs  Named-mutex single instance
└── apps/config/                    Settings window (Tauri 2 + Svelte 5)
    ├── src-tauri/  Rust backend commands + tauri.conf.json + capabilities
    │   └── src/verhub.rs  Verhub client (versions/announcements/feedback/logs/project links, built on verhub-sdk;
    │                      feedback may optionally be converted into a GitHub issue by the Verhub bot,
    │                      which makes the GitHub account mandatory;
    │                      project links are cached: in memory + verhub_cache.json in the data folder, valid for one day)
    ├── ui/         Frontend source (Vite + Svelte 5)
    │   └── src/    lib/ (pure logic + vitest tests) + components/ (Svelte components)
    │                + locales/ (three-language catalogs; zh-CN.js is the source of truth)
    └── dist/       Frontend build output (gitignored; produced from ui/ by vite build)
```

::: tip Why common has no platform dependency
`crates/common` deliberately avoids the Windows API so it can be compiled cross-platform, and its pure logic (configuration parsing, matching, protocol) is easier to unit test. Platform-specific code lives in `crates/core`.
:::

## Data folder

The configuration (`config.json`), logs (`logs/`), the recovery snapshot (`recovery.json`) and the cache (`verhub_cache.json`) all live in a single **data folder**, resolved by `crates/common/src/paths.rs`. Installed and portable copies are treated differently:

| Case | Data folder | `DataDirKind` |
| --- | --- | --- |
| Installed copy | `%APPDATA%\ZoneDeck` | `Installed` |
| Portable copy, program folder writable | The program folder | `Portable` |
| Portable copy, program folder not writable | `%APPDATA%\ZoneDeck` | `PortableFallback` |

A portable copy keeps its data in the program folder, so copying that folder takes the whole setup along. An installed copy cannot do the same: the installer may land in `Program Files`, which normal privileges cannot write to, so every save from the settings program would fail with `os error 5`.

### Telling the two apart

By looking for traces of an installation in the program folder (`paths::is_installed`):

1. `installed.marker`, dropped by the installer (shipped via `[Files]`, removed on uninstall);
2. the uninstaller `unins*.exe` — a fallback, so that a deleted marker does not send the data back into `Program Files`. The number increases with repeated installs, hence the prefix match.

::: warning The test must be a file, never a privilege check
The core may run as administrator while the settings program does not: the core can write inside `Program Files`, the settings program cannot. If each side picked a folder based on what it could write to, the two would read different configs and the user's changes would appear to have no effect. Looking at files makes both sides agree by construction — which is also why an installed copy never probes for writability at all; the answer is the user folder either way.
:::

### Fallback and migration

When a portable copy finds the program folder unwritable it falls back to the user folder and records `PortableFallback`. The core writes that to the log; the settings program reads it through the `data_location` command and shows a notice explaining that this is a permissions problem and how to change it (see `DataNoticeModal.svelte`). Nothing else is affected.

Whenever the user folder is used, a `config.json` in the program folder is moved across: copied first, then the original is deleted on a best-effort basis. An existing config at the destination is left untouched — that is the one currently in use — and the old file is left alone as well. If the original cannot be deleted (no write permission, or the file is in use) it simply stays; it is never read again.

Data left behind by the brand rename (Boss Key → ZoneDeck) migrates automatically as well: whenever the data folder is located, an existing `%APPDATA%\BossKey` is renamed wholesale to `%APPDATA%\ZoneDeck` if the latter does not exist yet; if the rename is blocked because the old folder is in use, only `config.json` and `recovery.json` are copied, and the leftovers are handled by the uninstaller or `cleanup.ps1`. The old log prefix `BossKey-` is still recognised by retention cleanup and session lookback (on the same date the new prefix sorts first). Autostart entries registered under the old names (the `BossKeyAutostart` scheduled task and the `Boss Key Application` registry value) are migrated at core startup: the new name is registered first, with the original privilege preference, and only then are the old entries cleaned up. During an installer upgrade the installer has to delete the old watchdog task before it can replace files, so it writes a migration marker (`HKCU\Software\ZoneDeck\MigrateAutostart`) beforehand for the core to pick up on first launch. Before starting, the core also probes the old mutex `BossKey_SingleInstance_Mutex`: if the old core is still running it shows a warning and exits, avoiding two cores running at once and avoiding moving the data folder out from under the running old core.

::: tip The settings window's browser data lives elsewhere
Following the identifier in `tauri.conf.json`, Tauri puts the WebView2 user data in `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`. It is not part of the data folder and is not managed by `paths.rs`. Both the installer's uninstaller and the `scripts/cleanup.ps1` shipped with the portable edition remove it.
:::

The data folder actually in use, and how it was chosen, are written to the log's `[START]` marker on every start; check that first when diagnosing read/write failures (the user folder in the path is redacted to `%USERPROFILE%`).

## Inside the core: the agent message loop

`agent.rs` is the hub: it creates a **hidden message window**, runs the Windows message loop, and aggregates these event sources:

- Global hotkeys (`WM_HOTKEY`);
- The mouse hook (middle / side buttons, corners);
- The named-pipe server (commands from the settings window);
- Timers (idle detection, state maintenance, and so on);
- Window events (`SetWinEventHook`: top-level window destroy / show / title change);
- Tray icon interaction.

Message-loop state lives in a `RefCell`: the modal loops of the tray / floating-window menus (`TrackPopupMenu`) re-enter `wndproc`, and events arriving during re-entry fail the borrow and are safely dropped, so no aliased mutable references can exist. The IPC thread retries pipe creation with backoff (1s → 5s → 30s) instead of exiting.

### Low-level input hooks do not share the message loop's thread

`WH_MOUSE_LL` / `WH_KEYBOARD_LL` callbacks are dispatched by the **installing thread's message pump**, and the system's input thread waits for the hook chain to return before delivering the event onward. Sharing a thread with the agent means enumerating windows, writing the recovery file, or handling system-wide window events would directly slow down global mouse and keyboard input; a single callback exceeding `LowLevelHooksTimeout` (300ms by default) also makes the system drop that event.

`input_hooks.rs` therefore runs a dedicated thread that does nothing but pump messages for these two hooks, at above-normal priority. The callbacks only perform in-memory checks and `PostMessageW` (the hottest path, mouse movement, takes no lock — samples live in atomics). The agent thread issues install/uninstall requests synchronously through a message-only window and uses the return value to decide whether to fall back (when the keyboard hook cannot be installed, "do not pass through" hotkeys degrade to `RegisterHotKey`).

The agent thread's own priority is **not** raised: it does the heavy work — enumeration, freezing, persistence — and raising it would only steal CPU from foreground programs.

Window events keep the hidden records maintained in real time: when a hidden window is destroyed or shown externally, its record is removed and persisted immediately; title changes are synced into hidden records and exact window rules (in memory only, written out with the next regular persist), so reacquisition and refinding by "title + process path" always work on fresh data. On restore, records with dead handles are additionally refound among currently invisible windows by "process path + title".

When hiding or showing is triggered, `HideController` orchestrates it with a two-phase, intent-first flow: `plan_hide` computes the execution plan (pruning stale records and backfilling PIDs) → the planned snapshot is written to `recovery.json` (persist first, act second — a crash mid-hide loses no records) → `commit_hide` hides the windows synchronously (`SW_HIDE`) and hands muting / freezing / the pause key to the dedicated side-effect thread (`effects_worker.rs`), executed asynchronously in FIFO order — the message loop is never blocked by slow operations (audio enumeration, waiting on pssuspend), so hotkeys and the UI stay responsive.

The order within the queue matters: pause key → mute → settle → freeze. Freezing stops a process from responding to messages at all, so freezing before the hide has finished painting leaves a ghost of the window on screen; the pause key likewise needs time to be handled by the target program. Hence a single settle before the batch of freezes (`FREEZE_SETTLE_DELAY`, once per batch, skipped when there is nothing to freeze). Muting is deliberately not placed behind that wait — it goes through the audio session and does not care whether the target process is running.

When restoring (showing), every record is validated first: the handle must still exist and still belong to the original process (`IsWindow` + PID comparison), and frozen / muted records must match the process creation time — both handles and PIDs are recycled by the system, and records that fail validation are skipped and reported truthfully in the log.

::: info Designed for testability
`Effects` is a trait, so tests can inject a mock and verify the hiding orchestration without actually muting or freezing the system. `WindowManager` is a trait for the same reason.
:::

## Stability (three layers of crash self-healing)

1. **Crash logs**: key events and panics are written to `logs/ZoneDeck-YYYY-MM-DD.log` in the [data folder](#data-folder) (rotated daily, retained per `log_retention_days`; 0 disables logging; filtered by `log_level`, which defaults to WARN and above — see [Log levels and redaction](#log-levels-and-redaction)).
2. **Crash recovery**: before any hide action executes, what is *about to be* hidden / frozen / muted is written to `recovery.json` (tmp + rename atomic replace); windows are recovered automatically on the next start after an abnormal exit. Snapshots carry the boot time and process creation times, so stale snapshots from a previous boot are discarded instead of acting on unrelated windows / processes.
3. **Watchdog**: the scheduled task's `RestartOnFailure` (restart within a minute of a crash, up to 3 times). Release builds use `panic = "abort"`, and the panic hook exits with a non-zero code once the log is written — exactly what triggers the scheduled-task restart.

For the user-facing explanation see [Window recovery & crash self-healing](/en/guide/recovery).

## Log levels and redaction

`crates/core/src/logging.rs` provides four levels — `debug` / `info` / `warn` / `error` — plus two session markers, and redacts every entry before it is written. The `log_level` the user picks in the settings program is the **recording threshold**: anything below it is dropped, and the default `warn` keeps only warnings and errors. The level is applied on `reload_config`, so it takes effect without restarting the core.

Classify new log entries as follows:

| Level | What belongs here | Examples |
| --- | --- | --- |
| `error` | A feature is unavailable or data may be lost, and the user needs to know | Agent window creation failed, config parsing fell back to defaults, the recovery file could not be written |
| `warn` | Degraded but still usable, or an anomaly the user will notice | Hotkey registration failed, a hook could not be installed, a rule matched no window, an unclean shutdown was detected |
| `info` | Milestones that occur at most once or twice per run | First launch after an update, which also opens the settings program |
| `debug` | Per-action activity and self-healing steps, needed only while troubleshooting | Details of each hide/restore, successful hotkey registration, tray icon re-attachment |

The two **session markers** bypass the level filter, one of each per run: `[START]` records the version, config schema, effective level and data folder; `[EXIT]` records a clean exit and what caused it (close hotkey, tray menu, a quit command from the settings program, the smoke-test timer). A log that ends without `[EXIT]` means the previous run crashed or was killed, and `[START]` is the only thing that tells you which build an uploaded log excerpt came from.

Hide and restore entries name their **trigger** (`Trigger`: hotkey, mouse button, screen corner, idle timer, tray, floating window, settings program). When a user reports that "windows disappeared out of nowhere", telling these apart is the first step.

### Uploads cover the current run only

`logging::latest_session` takes everything from the most recent `[START]` up to now, looking back one more log file when the run spans midnight; the settings program's `current_session_log` command returns exactly that. A fixed number of trailing lines is not used because it both mixes in earlier runs and drops the version and data folder recorded at the start of this one. When the excerpt exceeds the upload budget ([`verhub::LOG_EXCERPT_MAX`]), the first line and the tail are kept and the number of omitted lines is stated — the first line carries the version, the tail carries the failure, and neither can be lost.

::: warning Logs may be uploaded by users
The feedback feature in the settings program sends log lines to Verhub, so logs must **never contain window titles** (which may be file names, contacts, or page titles). Windows are referred to by process name, handle and PID; window rules by index and process name. The user folder is replaced with `%USERPROFILE%` centrally in `logging.rs`, so call sites do not need to handle it.
:::

### What an error entry carries

- **Subject**: the path, PID, hotkey or pipe name involved; paths go in via `display()` (redaction is the logging layer's job).
- **Cause**: `util::win_err` for Windows APIs (message plus hex code), `{e}` for IO and child-process errors.
- **Consequence**: which feature is now unavailable, or whether data may be lost — not merely that something "failed".
- **Location**: use the `log_error!` / `log_warn!` macros, which append `file:line` automatically.

For example: `重新加载配置失败，本次改动未生效，核心仍在用上一次加载的配置: <path> — <cause> (agent.rs:123)`.

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
