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

There are two customisable global hotkeys:

| Function | Default |
| --- | --- |
| **Hide / show windows** | `Ctrl + Q` |
| **Close programs** | `Win + Esc` |

Click **Record**, then press the combination you want; it is recognised and filled in automatically.

Click **Clear** to unset the hotkey. Once unset it reads "Disabled", the core no longer registers it, and that function cannot be triggered from the keyboard. The two hotkeys are independent and can be unset separately.

::: info About global hotkeys
Boss Key registers global hotkeys through the system's `RegisterHotKey`. If a combination is already taken by another program, registration may fail — pick a different one.
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
