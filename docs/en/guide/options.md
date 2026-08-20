---
title: Other options
---

# Other options

The **Options** tab of the settings window gathers the enhancement switches for hiding, startup privileges, logs and tools. This page covers the general options, privileges and logs. For process freezing and memory trimming see [Process freezing](/en/guide/freeze) (the settings live in the **Power & Memory** tab); for the window recovery tool see [Window recovery & crash self-healing](/en/guide/recovery).

![ZoneDeck options page](/static/screenshot-4.png)

## General

### Mute after hiding

**Mutes the audio of the target process** while hidden, and unmutes it automatically when restored. Useful when hiding a window that is playing video or music.

- **On** by default.
- Based on Windows Core Audio session muting: only the target process is muted; other system sounds are unaffected.

### Also hide the active window

When on, pressing the hide hotkey also hides **the foreground window you are currently using**, in addition to the bound windows.

- **On** by default.
- Useful when the window you happen to be looking at was not bound in advance.

### Toggle hiding by clicking the tray icon

When on, a **left click on the tray icon** hides / shows the windows.

- **On** by default.

### Also hide ZoneDeck's tray icon

When on, hiding the windows **hides ZoneDeck's own tray icon as well** for extra discretion; triggering restore brings back both the windows and the icon.

- **Off** by default.
- This option only affects ZoneDeck's own tray icon — it **does not hide other programs' tray icons** (including the icons of the programs whose windows are hidden).

::: warning
Once the tray icon is hidden you cannot click it to restore or open the settings. Be sure to remember your **hide / show hotkey** or mouse gesture, and use that to restore.
:::

::: tip Hiding other programs' tray icons
Windows itself controls which tray icons are visible: you can choose which icons appear in the taskbar corner by hand. For detailed steps see Microsoft's guide [Customize the taskbar in Windows · System tray](https://support.microsoft.com/en-us/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray), or open the [taskbar settings](ms-settings:taskbar) directly (the `ms-settings:taskbar` link works only on Windows; the browser asks for confirmation first).
:::

### Send the pause key before hiding

When on, ZoneDeck sends the **media pause key** to the window **before** hiding it, to try to pause any video or music playing inside.

- **Off** by default.
- It differs from "mute after hiding": muting only silences the audio, whereas the pause key actually stops playback.
- ZoneDeck's **process freezing** has the same pausing effect while also cutting resource usage — see [Process freezing](/en/guide/freeze).

### Minimise windows before hiding

When on, each window is **minimised** before it is hidden, and restored to the size it had beforehand when you bring it back.

- **Off** by default.
- A window that was **maximised** before hiding comes back maximised — it is not shrunk to a normal size.
- A window that was **already minimised** stays minimised: ZoneDeck did not minimise it for you, so it does not restore it for you either.
- Some programs release resources when they receive the minimise message (pausing rendering, collapsing to the tray). Pair this with [process freezing](/en/guide/freeze) for a more pronounced effect.

::: tip Hiding and restoring are one paired switch
ZoneDeck's "restore" only reverses the changes **it made itself**: a window it never minimised is not resized, and a window it never hid (one the program hid on its own) is not popped back up. Side effects such as muting and freezing are still applied and undone as usual.
:::

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

## See also

[Customize the taskbar in Windows – Microsoft Support](https://support.microsoft.com/en-us/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray)
