---
title: Hiding options
---

# Hiding options

The **Hiding** tab of the settings window gathers the extra behaviours applied when windows are hidden: muting, the pause key, the tray icon and minimising. For how hiding is triggered see [Hotkeys & mouse gestures](/en/guide/hotkeys); for process freezing and memory trimming see [Process freezing](/en/guide/freeze) (those settings live in the **Power & Memory** tab); for whether the tray icon is shown at all and what clicking it does, see [Alerts](/en/guide/notifications#show-the-tray-icon).

## Mute after hiding

**Mutes the audio of the target process** while hidden, and unmutes it automatically when restored. Useful when hiding a window that is playing video or music.

- **On** by default.
- Based on Windows Core Audio session muting: only the target process is muted; other system sounds are unaffected.
- Only mutes set by ZoneDeck are lifted: a program you had already muted yourself in the volume mixer stays muted after restoring.

## Also hide the active window

When on, pressing the hide hotkey also hides **the foreground window you are currently using**, in addition to the bound windows.

- **On** by default.
- Useful when the window you happen to be looking at was not bound in advance.

## Also hide ZoneDeck's tray icon

When on, hiding the windows **hides ZoneDeck's own tray icon as well** for extra discretion; triggering restore brings back both the windows and the icon.

- **Off** by default.
- This option only affects ZoneDeck's own tray icon — it **does not hide other programs' tray icons** (including the icons of the programs whose windows are hidden).
- To keep the icon hidden **all the time**, use the [Show the tray icon](/en/guide/notifications#show-the-tray-icon) switch instead; this one only applies while windows are hidden.

::: warning
Once the tray icon is hidden you cannot click it to restore or open the settings. Be sure to remember your **hide / show hotkey** or mouse gesture, and use that to restore.
:::

::: tip Hiding other programs' tray icons
Windows itself controls which tray icons are visible: you can choose which icons appear in the taskbar corner by hand. For detailed steps see Microsoft's guide [Customize the taskbar in Windows · System tray](https://support.microsoft.com/en-us/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray), or open the [taskbar settings](ms-settings:taskbar) directly (the `ms-settings:taskbar` link works only on Windows; the browser asks for confirmation first).
:::

## Pause playback as well

Tries to pause any playing media as the windows are hidden.

- **Off** by default.
- Only the hidden program is affected: ZoneDeck goes through the system media controls to pause that program's own media session.
- Programs that never register a media session with the system fall back to sending the media pause key.
- It differs from "mute after hiding": muting only silences the audio, whereas the pause key actually stops playback.
- ZoneDeck's **process freezing** has the same pausing effect while also cutting resource usage — see [Process freezing](/en/guide/freeze).

### Resume playback on restore

Carries on playing the media this round paused when the windows come back. Requires "Pause playback as well".

- **Off** by default: playback stays paused after restoring.
- Only media this program paused, and that is still paused, is resumed — if you pressed play yourself or moved on to something else while hidden, nothing interrupts it.
- Only programs paused through the system media controls are resumed precisely; those that fell back to the pause key are not.

When on, each window is **minimised** before it is hidden, and restored to the size it had beforehand when you bring it back.

- **Off** by default.
- A window that was **maximised** before hiding comes back maximised — it is not shrunk to a normal size.
- A window that was **already minimised** stays minimised: ZoneDeck did not minimise it for you, so it does not restore it for you either.
- Some programs release resources when they receive the minimise message (pausing rendering, collapsing to the tray). Pair this with [process freezing](/en/guide/freeze) for a more pronounced effect.

::: tip Hiding and restoring are one paired switch
ZoneDeck's "restore" only reverses the changes **it made itself**: a window it never minimised is not resized, and a window it never hid (one the program hid on its own) is not popped back up. Side effects such as muting and freezing are still applied and undone as usual.
:::

## See also

[Customize the taskbar in Windows – Microsoft Support](https://support.microsoft.com/en-us/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray)
