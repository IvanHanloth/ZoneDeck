---
title: Alerts
---

# Alerts

ZoneDeck reflects the core's state through two kinds of hints: **system notifications** and **tray icon badges**. The **General** tab of the settings window lets you decide whether the tray icon is shown at all, what clicking it does, and how notifications and badges behave — so you stay informed without being interrupted.

## Show the tray icon

The tray icon appears in the notification area by default. Turn this option off and it disappears entirely, for extra discretion.

- **On** by default.
- With the icon gone, its click actions, tooltip and status badges stop working and the corresponding settings are greyed out. **Notifications are unaffected** and keep coming through.
- **Also hide ZoneDeck's tray icon** ([Hiding options](/en/guide/hiding#also-hide-zonedeck-s-tray-icon)) is tied to hiding windows and is independent of this switch.
- The switch takes effect immediately — no need to restart the core.

::: warning Work out how to get back in before you turn it off
With no icon there is no menu to click. You can still **run ZoneDeck again** (double-click the program or a shortcut) to open the settings window — when the core is already running, launching it again opens the settings instead. But restoring your windows is down to your **hide / show hotkey** or mouse gesture, so make sure you remember it first.
:::

## Tray icon click actions

**Single click**, **double click** and **right click** on the tray icon each get their own action:

| Action | Description |
| --- | --- |
| **Do nothing** | No action at all |
| **Hide / show windows** | Toggles hiding, same as pressing the hide / show hotkey once |
| **Open the tray menu** | Pops up the [tray menu](/en/guide/getting-started#the-tray-menu) |
| **Open the settings window** | Opens this settings window |

Defaults:

| Click | Default action |
| --- | --- |
| Single click | Hide / show windows |
| Double click | Do nothing |
| Right click | Open the tray menu |

::: tip Binding the double click makes single clicks a beat slower
Windows cannot tell on the first click whether a second one is coming. So as soon as the double click has an action bound, a single click has to wait out the system double-click time (adjustable in the Windows mouse settings) before it runs. With the double click left on "Do nothing" there is no conflict and single clicks respond immediately.
:::

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
[Also hide ZoneDeck's tray icon](/en/guide/hiding#also-hide-zonedeck-s-tray-icon) is tied to **hiding windows** and comes back when you restore; "Show the tray icon" on this page keeps the tray icon **hidden for good**.
:::
