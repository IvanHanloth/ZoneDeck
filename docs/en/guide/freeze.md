---
title: Process freezing
---

# Process freezing

**Process freezing** enhances window hiding: the matching process is suspended along with the window, so it stops consuming CPU (pausing background video decoding, game rendering, and so on). It is resumed automatically when the windows are restored.

The related settings live under **Options → Process freezing** in the settings window.

::: warning Freezing is an advanced feature
Freezing stops the target process entirely, so its background work (downloads, incoming messages) is paused too. Enable it only once you understand the impact.
:::

## Freeze processes when hiding

This is the **master switch** for freezing. When on, Boss Key suspends the matching process each time it hides its windows, and resumes it when they are restored. It uses normal freezing by default, which works on the current user's processes **without administrator rights**.

## Use enhanced freezing

Normal freezing may not be thorough enough for complex programs (multi-process architectures, renderer subprocesses). **Enhanced freezing** suspends processes with Microsoft's official `pssuspend64.exe` tool instead, which is more effective.

Enhanced freezing requires **all** of the following, otherwise the option is greyed out:

1. The "Freeze processes when hiding" master switch is on;
2. The **core is running as administrator** (see below and [Other options](/en/guide/options));
3. The file **`pssuspend64.exe`** exists in the program folder.

### Getting pssuspend64.exe

1. Open Microsoft Sysinternals' [PSTools download page](https://download.sysinternals.com/files/PSTools.zip).
2. Extract the downloaded `PSTools.zip`.
3. Find **`pssuspend64.exe`** inside and copy it into Boss Key's **installation root folder**.
4. Return to the settings window and click **Check again** in the "Process freezing" section so Boss Key picks the file up.

## Freeze the whole process tree

By default freezing affects only the matched process itself. With **Freeze the whole process tree** on, Boss Key **recursively freezes that process's entire child-process tree** (including differently named child `exe` files, renderer processes, and so on) for a more thorough freeze.

::: warning
This option is still in testing and may cause problems with some programs. Enable it only once you understand the impact.
:::

## Prerequisites at a glance

| Feature | Needs administrator | Needs pssuspend64.exe |
| --- | :---: | :---: |
| Freeze processes when hiding | ❌ | ❌ |
| Use enhanced freezing | ✅ | ✅ |
| Freeze the whole process tree | Depends on the freezing method used | Depends on the freezing method used |

## Crash safety

Boss Key records which processes are currently frozen in `recovery.json`. Even if the core exits abnormally, the previously suspended processes are **resumed automatically** on the next start.

See [Window recovery & crash self-healing](/en/guide/recovery) for how to resume frozen processes manually.

## Credits

The approach to process freezing is inspired by and credited to **HsFreezer**.

## See also

[PSTools – Microsoft Sysinternals](https://learn.microsoft.com/sysinternals/downloads/pstools)
