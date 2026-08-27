---
title: Process freezing
---

# Process freezing

**Process freezing** enhances window hiding: the matching process is suspended along with the window, so it stops consuming CPU (pausing background video decoding, game rendering, and so on). It is resumed automatically when the windows are restored.

The related settings live in the **Power & Memory** tab of the settings window, which also holds [efficiency mode](#efficiency-mode) — a way to save power without stopping the process.

::: warning Freezing is an advanced feature
Freezing stops the target process entirely, so its background work (downloads, incoming messages) is paused too. Enable it only once you understand the impact.

Suspending merely stops the process's threads from being scheduled; it does not lower memory usage on its own. To bring memory down as well, see [Reduce memory usage](#reduce-memory-usage) below.
:::

## Freeze processes when hiding

This is the **master switch** for freezing. When on, ZoneDeck suspends the matching process each time it hides its windows, and resumes it when they are restored. It uses normal freezing by default, which works on the current user's processes **without administrator rights**.

Suspending and resuming take time, so turning this on may add some delay when hiding and restoring.

## Use enhanced freezing

Normal freezing may not be thorough enough for complex programs (multi-process architectures, renderer subprocesses). **Enhanced freezing** suspends processes with Microsoft's official `pssuspend64.exe` tool instead, which is more effective.

Enhanced freezing requires **all** of the following, otherwise the option is greyed out:

1. The "Freeze processes when hiding" master switch is on;
2. The **core is running as administrator** (see below and [General settings](/en/guide/options));
3. The file **`pssuspend64.exe`** exists in the program folder.

### Getting pssuspend64.exe

1. Open Microsoft Sysinternals' [PSTools download page](https://download.sysinternals.com/files/PSTools.zip).
2. Extract the downloaded `PSTools.zip`.
3. Find **`pssuspend64.exe`** inside and copy it into ZoneDeck's **installation root folder**.
4. Return to the settings window and click **Check again** in the "Process freezing" section so ZoneDeck picks the file up.

Enhanced freezing invokes an external program, so it likewise adds some delay when hiding and restoring.

## Efficiency mode

**Efficiency mode** is the other route besides freezing: instead of stopping the process, ZoneDeck drops it into Windows' efficiency mode — it keeps running, but on efficiency cores at a lower clock and a lower scheduling priority, cutting power draw and heat. This is exactly what the "Efficiency mode" marker in Task Manager does.

It is **independent of freezing**, with its own switch and its own scope. Enable either one, or both.

| | Freezing | Efficiency mode |
| --- | --- | --- |
| Process state | Fully suspended, no longer executing | Keeps running, just slower |
| Background work (downloads, messages) | Paused too | Carries on |
| Suits | Programs that can stop entirely | Background programs that must not be stopped |
| Needs administrator | ❌ | ❌ |
| Extra files | Enhanced freezing needs pssuspend64.exe | None |

- **Off** by default. Once on, it is applied on every hide and lifted automatically when the windows are restored.
- Works on the current user's processes without administrator rights, and depends on no external tool.
- Only processes at **normal priority** get their priority lowered. If a process set its own high or low priority, ZoneDeck applies the efficiency scheduling but leaves that priority alone.
- **Works best on Windows 11.** Windows 10 accepts it too, but only throttles execution speed — there is no full EcoQoS scheduling.

## Scope

**Scope** decides how many processes power control reaches; by default it covers only the process the matched window belongs to. Freezing and efficiency mode each have **their own independent scope setting**:

- **Freezing & memory scope** — shared by process freezing and [reducing memory usage](#reduce-memory-usage);
- **Efficiency mode scope** — used by efficiency mode alone, independent of freezing.

Both offer the same values:

| Value | Covers | Suits |
| --- | --- | --- |
| **Target process only** | The process the matched window belongs to | The default. Single-process programs, or when you only want the main process touched |
| **Target process and all its children** | The above ∪ all its descendants | Browsers, Electron apps — anything with renderer or helper child processes |
| **All instances of the same program** | Every process in the system with the **same image name** | A program you run several copies of |

::: tip "All instances of the same program" matches by file name
It goes by image name (`chrome.exe`, say), ignoring any parent-child relationship and not distinguishing install directories — two identically named programs in different locations are both included. It does **not** include differently named child processes; that is what "Target process and all its children" is for.
:::

::: warning
A wider scope is more thorough, more likely to disturb background work you did not mean to touch, and adds more delay when hiding and restoring. "Target process and all its children" is still in testing.
:::

::: danger Do not freeze File Explorer
`explorer.exe` *is* the desktop and the taskbar; freezing it locks up the entire Windows shell. Worse, ZoneDeck is usually launched by File Explorer and is therefore its child process — freezing File Explorer with "Target process and all its children" selected suspends ZoneDeck along with it.

That is why ZoneDeck's core and settings app are **built into** the [whitelist](/en/guide/whitelist) as permanently non-freezable entries that cannot be turned off, and why `explorer.exe` ships as a removable whitelist entry.
:::

## Excluding specific programs

To keep a program running, add it to the [whitelist](/en/guide/whitelist) and tick "Skip freezing" — no need to turn off the master switch. That same tick also makes it **skip efficiency mode**: both share this one whitelist.

## Reduce memory usage

Freezing on its own saves no memory. With **Reduce memory usage** on, ZoneDeck **empties the process's working set** right after freezing it, pushing the data it holds in physical memory out to the page file. The "Memory" column in Task Manager drops immediately.

::: warning
This option is still in beta. It **may lengthen the time restoring takes in exchange for saving some memory** — the data has to be read back from the page file on restore, and the bigger the process (or the slower the disk), the more noticeable the stall. Enable it only once you understand the impact.
:::

- **Off** by default, and requires "Freeze processes when hiding" to be on — otherwise the option is greyed out.
- Applies only to processes that were **actually frozen**, sharing the "Freezing & memory scope" with freezing (see [Scope](#scope)). A running process pulls its pages straight back in, so emptying its working set achieves nothing.
- Part of the memory saved is not reclaimed outright: the evicted pages go to the system's standby list first, and Windows would have reclaimed them anyway once memory got tight.

## Efficiency report

The bottom of the Power & Memory page carries an **efficiency report**, covering the statistics from ZoneDeck's power control: how many processes it has frozen, how often efficiency mode was applied, how much memory was paged out, and the electricity and CO₂ estimated from all of that.

| Figure | Source |
| --- | --- |
| Processes frozen / efficiency mode applied | **Measured**. Counted per action; freezing the same process again counts again |
| Time spent frozen | **Measured**. Every process's frozen time added up — three processes frozen for an hour each counts as three hours; settled on resume |
| Memory released | **Measured**. The working set is read before and after trimming, and the difference accumulated |
| Energy saved | **Estimated**. The wattages below, multiplied by how long each process stayed frozen or throttled |
| CO₂ avoided, equivalent trees | **Estimated**. Derived from the energy figure |

The coefficients behind the estimates:

| Coefficient | Value | Rationale |
| --- | --- | --- |
| Saved per frozen process | 0.9 W | Roughly 0.5 W on the CPU side, adjusted for power-supply conversion loss and then scaled up for the GPU, memory and I/O work that stops along with it |
| Saved per process in efficiency mode | 0.35 W | Four tenths of freezing: the process keeps running, just on efficiency cores at a lower clock |
| Grid emission factor | 0.55 kg CO₂/kWh | An approximation of the national average for electricity (the official 2023 figure is 0.5306) |
| Absorbed by one tree per year | 20 kg CO₂ | Commonly quoted as 20–25; this takes the low end |

**Reset** at the bottom right of the report clears the tally, and that cannot be undone.

## Prerequisites at a glance

| Feature | Needs administrator | Needs pssuspend64.exe |
| --- | :---: | :---: |
| Freeze processes when hiding | ❌ | ❌ |
| Enable efficiency mode | ❌ | ❌ |
| Use enhanced freezing | ✅ | ✅ |
| Scope | Depends on the freezing method used | Depends on the freezing method used |
| Reduce memory usage | ❌ | ❌ |

## Crash safety

ZoneDeck records which processes are currently frozen, and which ones have efficiency mode applied, in `recovery.json`. Even if the core exits abnormally, the next start takes those records over; if they cannot be taken over properly, the previously suspended processes are **resumed right away** and efficiency mode is lifted along with them.

See [Window recovery & crash self-healing](/en/guide/recovery) for how to resume frozen processes manually.

## Credits

The approach to process freezing is inspired by and credited to **HsFreezer**.

## See also

[PSTools – Microsoft Sysinternals](https://learn.microsoft.com/sysinternals/downloads/pstools)
