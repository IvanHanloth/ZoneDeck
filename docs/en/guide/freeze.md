---
title: Process freezing
---

# Process freezing

**Process freezing** enhances window hiding: the matching process is suspended along with the window, so it stops consuming CPU (pausing background video decoding, game rendering, and so on). It is resumed automatically when the windows are restored.

The related settings live in the **Power & Memory** tab of the settings window.

::: warning Freezing is an advanced feature
Freezing stops the target process entirely, so its background work (downloads, incoming messages) is paused too. Enable it only once you understand the impact.
:::

::: info Freezing does not lower memory usage
Suspending a process merely stops its threads from being scheduled; it **never touches the memory manager**. Not one page of the physical memory it already holds is handed back, and the "Memory" column in Task Manager does not budge. To bring memory down as well, see [Reduce memory usage](#reduce-memory-usage) below.
:::

## Freeze processes when hiding

This is the **master switch** for freezing. When on, ZoneDeck suspends the matching process each time it hides its windows, and resumes it when they are restored. It uses normal freezing by default, which works on the current user's processes **without administrator rights**.

Suspending and resuming take time, so turning this on may add some delay when hiding and restoring.

## Use enhanced freezing

Normal freezing may not be thorough enough for complex programs (multi-process architectures, renderer subprocesses). **Enhanced freezing** suspends processes with Microsoft's official `pssuspend64.exe` tool instead, which is more effective.

Enhanced freezing requires **all** of the following, otherwise the option is greyed out:

1. The "Freeze processes when hiding" master switch is on;
2. The **core is running as administrator** (see below and [Other options](/en/guide/options));
3. The file **`pssuspend64.exe`** exists in the program folder.

### Getting pssuspend64.exe

1. Open Microsoft Sysinternals' [PSTools download page](https://download.sysinternals.com/files/PSTools.zip).
2. Extract the downloaded `PSTools.zip`.
3. Find **`pssuspend64.exe`** inside and copy it into ZoneDeck's **installation root folder**.
4. Return to the settings window and click **Check again** in the "Process freezing" section so ZoneDeck picks the file up.

Enhanced freezing invokes an external program, so it likewise adds some delay when hiding and restoring.

## Scope

**Scope** decides how many processes freezing and [reducing memory usage](#reduce-memory-usage) reach — both share one scope, so there is nothing to set separately. By default it covers only the process the matched window belongs to.

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

To keep a program running, add it to the [whitelist](/en/guide/whitelist) and tick "Skip freezing" — no need to turn off the master switch.

## Reduce memory usage

Freezing on its own saves no memory. With **Reduce memory usage** on, ZoneDeck **empties the process's working set** right after freezing it, pushing the data it holds in physical memory out to the page file. The "Memory" column in Task Manager drops immediately.

::: warning
This option is still in beta. It **may lengthen the time restoring takes in exchange for saving some memory** — the data has to be read back from the page file on restore, and the bigger the process (or the slower the disk), the more noticeable the stall. Enable it only once you understand the impact.
:::

- **Off** by default, and requires "Freeze processes when hiding" to be on — otherwise the option is greyed out.
- Applies only to processes that were **actually frozen**, over the same scope freezing uses (see [Scope](#scope)). A running process pulls its pages straight back in, so emptying its working set achieves nothing.
- Part of the saving is on paper only: the evicted pages go to the system's standby list first, and Windows would have reclaimed them anyway once memory got tight. This option just pays that cost up front.

### Why freezing alone does not lower memory

Suspending goes through `NtSuspendProcess`, which does exactly one thing: stop every thread in the process from being scheduled. The process's address space, committed memory, and working set are all left exactly as they were — suspending is a **scheduling** operation and has nothing to do with memory management.

Windows only reclaims physical memory in two situations: the memory manager trims working sets when the system is under pressure, or something explicitly asks for a process's working set to be emptied. Freezing neither creates memory pressure nor triggers a trim, so on a machine with plenty of RAM a frozen process happily sits on hundreds of megabytes. "Reduce memory usage" is that explicit request.

## Prerequisites at a glance

| Feature | Needs administrator | Needs pssuspend64.exe |
| --- | :---: | :---: |
| Freeze processes when hiding | ❌ | ❌ |
| Use enhanced freezing | ✅ | ✅ |
| Scope | Depends on the freezing method used | Depends on the freezing method used |
| Reduce memory usage | ❌ | ❌ |

## Crash safety

ZoneDeck records which processes are currently frozen in `recovery.json`. Even if the core exits abnormally, the previously suspended processes are **resumed automatically** on the next start.

See [Window recovery & crash self-healing](/en/guide/recovery) for how to resume frozen processes manually.

## Credits

The approach to process freezing is inspired by and credited to **HsFreezer**.

## See also

[PSTools – Microsoft Sysinternals](https://learn.microsoft.com/sysinternals/downloads/pstools)
