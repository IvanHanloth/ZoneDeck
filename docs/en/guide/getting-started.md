---
title: Quick start
---

# Quick start

This chapter covers the basics: opening the settings window, controlling ZoneDeck from the tray menu, and the default hotkeys.

## First run

The **first time** you open ZoneDeck after installing or updating, the **settings window** appears automatically so you can change hotkeys, bind windows and processes, and so on right away.

## The settings window

| Tab | Contents | Documentation |
| --- | --- | --- |
| **Windows** | Choose the windows / processes to hide | [Binding windows & processes](/en/guide/binding) |
| **Hotkeys & Mouse** | Keyboard hotkeys, mouse clicks, corner gestures, auto-hide on idle | [Hotkeys & mouse gestures](/en/guide/hotkeys) |
| **Alerts** | Notifications and tray icon status badges | [Alerts](/en/guide/notifications) |
| **Options** | Muting, pause key, freezing, privileges, logs, recovery tool, and more | [Other options](/en/guide/options) |
| **About & Feedback** | Version information, update checks, announcements, feedback | [Updates & feedback](/en/guide/update) |

::: tip Settings save themselves
Every change is **written to disk automatically and takes effect immediately** once you stop interacting. The core **hot-reloads** on configuration changes; no manual restart is needed.
:::

## The status bar

The status bar at the bottom of the settings window shows the current core status and offers a few shortcuts:

![Status bar](/static/status-bar.png)

### Left: core status and actions

The left side shows whether the core is running and offers matching shortcuts.

**① Core stopped** — the core is not running and cannot hide windows. Start buttons are offered:

<StatusBar variant="offline" />

- <Play class="lucide-inline" style="color:#0e7490" /> **Start core**: start the core as a standard user.
- <Shield class="lucide-inline" style="color:#3b82f6" /> **Start as administrator**: start the core with administrator rights (required by features such as enhanced freezing).

**② Core running** — the core is running; the status text and indicator are cyan:

<StatusBar variant="user" />

- <Shield class="lucide-inline" style="color:#3b82f6" /> **Restart as administrator**: restart the core with elevated privileges.
- <RotateCw class="lucide-inline" style="color:#d97706" /> **Restart core**: restart the core process.
- <Power class="lucide-inline" style="color:#e5484d" /> **Stop core**: end the core process.

::: tip Telling the privilege level apart

- <span class="status-dot" style="color:#0e7490"></span> **Dot indicator**: the core is running as a standard user.
- <Shield class="lucide-inline" style="color:#0e7490" /> **Shield indicator**: the core is running as administrator.
:::


### Right: tools and save status

- <ScrollText class="lucide-inline" /> **Open log folder**: open the `logs` folder containing the runtime logs, for troubleshooting.
- <span style="opacity:.7">◐</span> **Switch theme**: cycle the interface theme between **follow system → light → dark**.
- <span class="status-dot" style="color:#0e7490"></span> **Hotkey status**: shows whether the core is currently listening for hotkeys and mouse input. <span class="status-dot" style="color:#0e7490"></span> **Hotkeys active** means normal listening; while recording on the "Hotkeys & Mouse" page it temporarily shows <span class="status-dot" style="color:#e5484d"></span> **Hotkeys paused** to avoid accidental triggers.
- <Check class="lucide-inline" style="color:#0e7490" /> **Save status**: shows **Saved** / **Saving…**. Changes save automatically; there is no save button.

## The tray menu

Day to day, **right-click the tray icon** to open the menu. It offers:

![ZoneDeck core tray menu](/static/screenshot-5.png)

- **Settings**: open the settings window.
- **Hide Windows / Show Windows**: quickly hide or show the bound windows.
- **Window Recovery Tool**: open the settings window on the window recovery tool — see [Window recovery & crash self-healing](/en/guide/recovery).
- **Auto Hide**: pause / resume [auto-hide on idle](/en/guide/hotkeys#auto-hide-when-idle); a check mark means it is currently enabled.
- **Start with Windows**: toggle starting automatically when you sign in.
- **Exit**: end the core process.

::: info Clicking the tray icon
With [“Toggle hiding by clicking the tray icon”](/en/guide/options) enabled, a left click on the tray icon hides / shows the windows.
:::

## Default hotkeys

ZoneDeck ships with two keyboard hotkeys, both customisable under [Hotkeys & mouse gestures](/en/guide/hotkeys):

| Function | Default hotkey | Description |
| --- | --- | --- |
| **Hide / show windows** | `Ctrl + Q` | Press once to hide the bound windows, again to restore them |
| **Close the core** | `Win + Esc` | Immediately ends the ZoneDeck core process |

A fresh installation also enables **hide on middle mouse button click** by default; you can adjust or disable it in the settings.

## Basic workflow

1. Open the settings window → **Windows**, select the windows to hide on the left, and click "Add window" to bind them on the right.
2. (Optional) Adjust the triggers under **Hotkeys & Mouse**.
3. Close the settings window; the core stands by in the background.
4. When you need to hide, press the hide hotkey or perform the mouse gesture — the bound windows vanish and are muted instantly.
5. Trigger again to restore them.

Next, read [Binding windows & processes](/en/guide/binding) to learn how to pick hiding targets precisely.
