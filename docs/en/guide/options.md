---
title: General settings
---

# General settings

The **General** tab of the settings window gathers startup privileges, the display language, logs and tools. For the extra behaviours applied when hiding see [Hiding options](/en/guide/hiding); for process freezing and memory trimming see [Process freezing](/en/guide/freeze) (those settings live in the **Power & Memory** tab); for the window recovery tool see [Window recovery & crash self-healing](/en/guide/recovery).

![ZoneDeck general settings page](/static/screenshot-4.png)

## Startup & permissions

### Start with Windows

Toggles whether the core starts automatically when you sign in. For the mechanism, see [Start with Windows](/en/guide/autostart).

### Core privileges

Shows the core's current state (stopped / standard user / administrator) and offers buttons to **start or restart the core as administrator**.

The following features require the core to run as administrator:

- [Enhanced freezing](/en/guide/freeze);
- [Highest-privilege scheduled-task startup](/en/guide/autostart).

## Language

### Display language

Sets the language used by the settings window and by the core's tray menu and notifications.

Options: **Follow system / 简体中文 / English / 繁體中文**. Default **Follow system**.

- With "Follow system", the Windows display language decides: Chinese (Taiwan / Hong Kong / Macau) shows Traditional Chinese, other Chinese variants show Simplified Chinese, English shows English, and any other language falls back to Simplified Chinese.
- Changes take effect immediately in both the settings window and the core; no restart is needed.
- Runtime logs are unaffected by this setting and are always written in Simplified Chinese.

## Logs

### Log retention

Core runtime logs are stored per day in the **`logs` folder inside the program directory**, and older logs are cleaned up automatically when the core starts.

Options: **Off / 3 days / 7 days / 14 days / 30 days**. Default **7 days**. Choosing "Off" disables logging entirely.

### Log level

Only entries at the selected level **or above** are recorded. Options: **Debug / Info / Warning / Error**. Default **Warning**.

| Level | What gets recorded |
| --- | --- |
| Debug | Everything, including every hide/restore and hotkey registration result |
| Info | Milestones beyond that routine activity, such as the first launch after an update |
| Warning (default) | Warnings and errors only: hotkey registration failures, rules that matched no window, an unclean shutdown detected on the previous run |
| Error | Errors only: the core failing to start, an unreadable config, crash reports |

At the default "Warning", everyday hide and restore activity is **not written to the log**, leaving only entries that deserve attention. Before reporting an issue you can lower it to "Debug", reproduce the problem once, then set it back. The option is unavailable while log retention is set to "Off".

Whatever the level, each run writes one session marker on startup and one on a clean exit, carrying the version and the data folder.

::: tip Check the logs first when troubleshooting
When something goes wrong, the logs in the `logs` folder are the primary source for diagnosing it. Attaching the relevant log to a report greatly speeds up investigation. See also [Window recovery & crash self-healing](/en/guide/recovery).
:::

## Tools

### Window recovery tool

For **recovering windows that were hidden by mistake and cannot be restored with a hotkey**. Click "Open" to list every window, then tick the ones to restore. See [Window recovery & crash self-healing](/en/guide/recovery).
