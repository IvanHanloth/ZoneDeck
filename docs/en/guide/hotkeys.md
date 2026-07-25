---
title: Hotkeys & mouse gestures
---

# Hotkeys & mouse gestures

Boss Key offers several ways to trigger hiding and restoring: global keyboard hotkeys, mouse button clicks, fast movement into a screen corner, and auto-hide on idle. All are configured on the **Hotkeys & Mouse** tab of the settings window.

![Boss Key hotkeys and mouse settings page](/static/screenshot-2.png)

::: tip Listening pauses automatically while editing
When the pointer enters the "Hotkeys & Mouse" settings area, the core **temporarily pauses** hotkey and mouse monitoring so you do not trigger hiding while recording or adjusting settings. It resumes when you leave the area or the window loses focus. The monitoring state is shown at the bottom right of the window.
:::

## Keyboard hotkeys

There are five customisable global hotkeys:

| Function | Default | What it does |
| --- | --- | --- |
| **Hide / show windows** | `Ctrl + Q` | Restores when hidden, hides otherwise |
| **Hide windows only** | Disabled | Hides, never restores |
| **Show windows only** | Disabled | Restores, never hides |
| **Hide foreground window** | Disabled | Hides just the currently active window |
| **Close the core** | `Win + Esc` | Quits the core (restoring every window first) |

Click **Record**, then press the combination you want; it is recognised and filled in automatically.

Click **Clear** to unset the hotkey. Once unset it reads "Disabled", the core no longer registers it, and that function cannot be triggered from the keyboard. The five hotkeys are independent and can be unset separately.

### One-way hotkeys and hiding the foreground window

The last three are unset by default; record them when you need them:

- **Hide windows only**: hides according to your rules, and never restores. Useful for splitting hide and restore across two keys so a stray press can't reveal what you just hid.
- **Show windows only**: restores hidden windows, and never hides.
- **Hide foreground window**: ignores your [window / process rules](/en/guide/binding) and hides just the currently active window. Press it repeatedly to hide one more window each time.

Hiding is **cumulative**: no matter how a window was hidden, it goes into the same list and comes back with everything else. So windows hidden by these three hotkeys can only be brought back by **Show windows only**, **Hide / show windows** (which restores in that state), or another restore method such as the tray menu or a mouse gesture.

::: tip
**Hide foreground window** targets the ordinary top-level window that currently has focus. If focus is elsewhere — on the desktop or on certain tool windows — the press does nothing and says so in the log.
:::

### Keeping hotkeys from other apps

Each hotkey has its own **don't pass through** switch (off by default), at the right end of that hotkey's row. When enabled, both the key press and release of the triggering combination never reach the foreground app, so games or chat input boxes won't receive the keys.

::: info About global hotkeys
By default Boss Key registers global hotkeys through the system's `RegisterHotKey`. If a combination is already taken by another program, registration may fail — pick a different one.

A hotkey with "don't pass through" enabled is handled by a low-level keyboard hook (`WH_KEYBOARD_LL`) instead of `RegisterHotKey`, so it is not affected by hotkey-occupancy conflicts. If installing the hook fails, the hotkey falls back to normal registration: it still works, but the keys can no longer be withheld from other apps. A few programs that read Raw Input directly bypass keyboard hooks and may still observe the keys.
:::

## Hiding with mouse buttons

Besides the keyboard, mouse buttons can trigger hiding. Five buttons are supported: **left, middle, right, side 1 (forward), side 2 (back)**.

**Hide on middle button click** is enabled by default, together with "restore with the same button".

Each enabled button is configured independently:

- **Click count**: trigger on a single, double or triple click.
- **Modifier keys**: optionally require `Ctrl` / `Shift` / `Alt` / `Win` to be held — for example, "double-click the middle button while holding Ctrl". Leave empty to require no modifier.

Two options apply to all five buttons:

- **Restore with the same button**: press the same button again to restore the hidden windows.
- **Multi-click interval**: clicks count as a multi-click only when the gap between them is within this interval (milliseconds). Default **350 ms**.

::: warning
When assigning the **left or right button**, always combine it with a multi-click or a modifier key; otherwise it interferes with normal clicking.
:::

## Hiding by moving to a screen corner

![Boss Key mouse button and corner settings](/static/screenshot-3.png)

Once enabled, moving the pointer **into a corner of the screen** triggers hiding / restoring. Each of the four corners (top-left, top-right, bottom-left, bottom-right) can be enabled separately.

Related options:

- **Only trigger on fast movement** (on by default): only a **fast** flick into the corner triggers it, so normal work does not set it off. When off, simply reaching the corner triggers it.
- **Restore from a corner**: allows moving the pointer to a corner to **restore** hidden windows (by default corners only hide).

::: info
Corner hiding relies on a low-level mouse hook (`WH_MOUSE_LL`), installed only when you enable mouse buttons or corners. If you use no mouse triggers at all, the core does not install the hook.
:::

## Auto-hide when idle

With **Enable auto-hide** on, Boss Key hides the bound windows automatically once the keyboard and mouse have been **idle** for the configured time.

- **Idle time**: the period without input (in minutes) that triggers it. Default **5 minutes**.

## Summary

| Trigger | Best for |
| --- | --- |
| Keyboard hotkey | The most general and fastest option |
| Mouse clicks / side buttons | Handier when your hand is already on the mouse |
| Screen corners | Nothing to memorise — just flick |
| Auto-hide on idle | A fallback when you step away and forget to hide |

Once the triggers are set up, you may want to adjust what else happens when hiding (muting, pausing, freezing). See [Other options](/en/guide/options) and [Process freezing](/en/guide/freeze).
