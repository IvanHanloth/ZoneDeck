---
title: Start with Windows
---

# Start with Windows

With startup enabled, the ZoneDeck core **starts in the background automatically when you sign in**, so you do not have to run it manually.

## Turning it on / off

There are two entry points with identical effect:

1. **Tray menu**: right-click the tray icon → select **"Start with Windows"** to toggle.
2. **Settings window**: **General → Startup & permissions → Start with Windows**.

After toggling (unless you disabled the matching notification), a notification reports the new state. Small text beside the switch shows the registration method in use (**scheduled task** or **registry**).

## How startup works

ZoneDeck uses a **two-tier** approach that balances privileges and reliability:

| Mechanism | Description |
| --- | --- |
| **Scheduled task (preferred)** | Registered as a logon-triggered scheduled task; can request **highest privileges** and supports **automatic restart on failure** |
| **Registry startup** | Used as a fallback when the scheduled task is unavailable |

The small text beside the **General → Startup & permissions → Start with Windows** switch shows which method is actually in effect, so you can confirm whether the scheduled task registered successfully (it falls back to the registry if not).

## About highest-privilege startup

If you want the core to start **as administrator** at boot (for example to use [enhanced freezing](/en/guide/freeze)), turn on **Startup & permissions → Start as administrator**. It registers startup as a scheduled task with the highest privileges.

- The switch is **off by default**, meaning standard-privilege startup, which suits almost every case.
- It **only applies to the scheduled-task method**; if startup fell back to the registry, the core can only start with standard privileges.
- Toggling it while startup is already enabled **re-registers** immediately with the new privileges; there is no need to turn startup off and on again.

::: warning Disable startup before uninstalling or moving
If you are going to delete the portable folder or move the program, **turn off startup from the tray menu first**, so no stale startup entry pointing at the old path is left behind.
:::
