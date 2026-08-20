---
title: Alerts
---

# Alerts

ZoneDeck reflects the core's state through two kinds of hints: **tray notifications** and **tray icon badges**. The **Alerts** tab of the settings window lets you control which notifications appear and which states the icon badge shows, so you stay informed without being interrupted.

## Controllable events

| Event | Description | Default |
| --- | --- | :---: |
| **Core started** | Shown when the core begins running | On |
| **Core exited** | Shown when the core exits | On |
| **Startup setting changed** | Shows the new state when startup is toggled | On |
| **Windows hidden** | Shown each time windows are hidden | Off |
| **Windows shown** | Shown each time windows are restored | Off |

## Tray icon status

The tray icon can overlay **a colored dot badge in its bottom-right corner** that reflects the core's current state. Each of the four colors is bound to a state source; the defaults are:

| Color | Default binding |
| --- | --- |
| Red | Windows are hidden |
| Green | Auto hide is enabled |
| Yellow | Also-hide-active-window is enabled |
| Blue | Process freezing is enabled |

- Each color can be rebound to another state (besides the four above, **Running as administrator** and **Hotkey monitoring is paused** are also available), or set to "Do not show" to disable it.
- When several bound states are active at once, only one dot is shown, in **red > green > yellow > blue** priority order.

::: warning Discretion note
Badges — especially "Windows are hidden" — also reveal the core's state to onlookers. Disable the corresponding color if discretion matters more to you.
:::

## Tray icon tooltip

Hovering over the tray icon shows "ZoneDeck" by default. Turn this option off to show no text at all for extra discretion.

- **On** by default.

## Recommendations

- **Hide / show notifications are off by default**: hiding is there to protect your privacy, and a notification on every hide would only draw attention, so they are not shown. Turn them on if you want explicit feedback.
- **Start / exit / startup notifications are on by default**: these are infrequent events, and keeping them helps you confirm the core's state.

::: info Not the same as "Also hide ZoneDeck's tray icon"
[Also hide ZoneDeck's tray icon](/en/guide/hiding) controls whether **the icon itself is visible**; this page controls **notifications and icon badges**. They are independent.
:::
