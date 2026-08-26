---
title: IPC protocol
---

# IPC protocol

The settings window (`config.exe`) and the resident core (`ZoneDeck.exe`) communicate over a **named pipe**. The protocol is defined in `crates/common/src/ipc.rs`.

## Transport

- Pipe name: `\\.\pipe\zonedeck`.
- Encoding: **one JSON object per line**, separated by `\n`. The client sends one `Command`; the server replies with one `Response`.
- The client is wrapped as `PipeClient`: it retries by default (25 attempts, 40 ms apart). `.fast()` selects **fail-fast** mode (a single attempt), for cases such as status polling where blocking is undesirable.

## Command (settings window → core)

Serialised with `#[serde(tag = "cmd", rename_all = "snake_case")]`.

| Command | JSON | Purpose |
| --- | --- | --- |
| `ReloadConfig` | `{"cmd":"reload_config"}` | Re-read the configuration and hot-reload (re-registering hotkeys / hooks / timers) |
| `GetState` | `{"cmd":"get_state"}` | Query the hidden state |
| `GetElevation` | `{"cmd":"get_elevation"}` | Query whether running as administrator |
| `GetStatus` | `{"cmd":"get_status"}` | **Fetch all state in one round trip** (hidden + privileges + monitoring), replacing the two above |
| `Hide` | `{"cmd":"hide"}` | Hide |
| `Show` | `{"cmd":"show"}` | Show |
| `Toggle` | `{"cmd":"toggle"}` | Toggle hide / show |
| `SetAutostart` | `{"cmd":"set_autostart","enabled":true}` | Configure startup |
| `SetHotkeys` | `{"cmd":"set_hotkeys","enabled":false}` | Temporarily suspend / resume hotkey and mouse monitoring |
| `ReleaseWindows` | `{"cmd":"release_windows","hwnds":[..]}` | Recovery tool: show the given handles. Windows tracked by the core are released per whole process (including unfreeze / unmute); untracked handles are simply shown |
| `AdoptWindows` | `{"cmd":"adopt_windows","hwnds":[..]}` | Recovery tool: hide the given handles and track them in the core (covered by crash recovery), without muting / freezing |
| `ResetPowerStats` | `{"cmd":"reset_power_stats"}` | Reset the power stats to zero. While the core is running this has to go through it, or the values it holds in memory would be written back over the file |
| `Quit` | `{"cmd":"quit"}` | Exit the core |

## Response (core → settings window)

Serialised with `#[serde(tag = "type", rename_all = "snake_case")]`.

| Response | JSON | Description |
| --- | --- | --- |
| `Ok` | `{"type":"ok"}` | The command succeeded with no extra data |
| `State` | `{"type":"state","hidden":true}` | The current hidden state |
| `Elevated` | `{"type":"elevated","elevated":true}` | Whether running as administrator |
| `Status` | `{"type":"status","hidden":..,"elevated":..,"monitoring":..}` | Aggregated state |
| `Error` | `{"type":"error","message":".."}` | An error message |

`Status.monitoring`: whether the core is currently listening for hotkeys and mouse input (`false` while suspended by `SetHotkeys`).

::: info Error messages follow the display language
The text of `Error.message` comes from the core's string catalog and follows `setting.language`. Treat it as text to show the user, not as a stable identifier to branch on.
:::

## Suspending monitoring, and the heartbeat

`SetHotkeys { enabled: false }` lets the settings window suspend core monitoring temporarily while **recording or adjusting hotkeys**, to avoid accidental triggers. The suspension is **stateful** and must be renewed with a heartbeat:

| Constant | Value | Meaning |
| --- | --- | --- |
| `SUSPEND_TIMEOUT_MS` | `15000` | Watchdog period: with no heartbeat for this long, the core resumes monitoring automatically |
| `SUSPEND_HEARTBEAT_MS` | `4000` | The suggested interval at which the settings window resends the heartbeat (must be well below the timeout) |

::: info Why a watchdog
If the settings window crashes or is killed while monitoring is suspended, it cannot send the resume command — so the core **resumes monitoring automatically** after the timeout, rather than leaving the user's hotkeys permanently dead.
:::

## Typical interaction sequence

```
Settings saved ──▶ reload_config ──▶ core hot-reloads ──▶ ok
Status polling (every 2s) ──▶ get_status(fast) ──▶ status{hidden,elevated,monitoring}
Entering the hotkey area ──▶ set_hotkeys{false} + heartbeat ──▶ core suspends monitoring
Leaving / losing focus ──▶ set_hotkeys{true} ──▶ core resumes monitoring
```
