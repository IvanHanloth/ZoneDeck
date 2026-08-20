---
title: Whitelist
---

# Whitelist

You can give ZoneDeck a process whitelist so it skips certain steps for the windows of specific processes.

When hiding windows, ZoneDeck may do three things to a target — hide its windows, freeze its process, mute its audio. The whitelist lets you turn off any of those three, per program, while the rest keep working.

Open the settings window and switch to the **Whitelist** tab. The layout mirrors [Binding windows & processes](/en/guide/binding): existing windows on the left, whitelist entries on the right.

## The three skip modes

Every entry carries three independent switches:

| Switch | Effect |
| --- | --- |
| **Skip hiding** | The program's windows are never hidden; they stay on the desktop |
| **Skip freezing** | The process is never [frozen](/en/guide/freeze), so background work continues |
| **Skip muting** | The program is never muted |

The three switches are **independent**. Whether freezing and muting kick in depends on whether the program still has a window left on the desktop — while a window is open neither is applied, and once it tucks itself away to the tray both are.

## Adding an entry

1. **Tick** the target windows in the "Existing windows" list on the left.
2. Click **Add process** on the right.
3. Tick the skip modes you want on the new row. New entries start with all three off, which means they do nothing yet.

## Match on

Like [process rules](/en/guide/binding), each entry matches on either the **file name** or the **path**:

- **File name** (default): only `xxx.exe` is compared, so the entry keeps working no matter where the program is installed. Whitelists default to file-name matching — protection that breaks when a program moves is no protection at all.
- **Path**: only the copy at that exact location is skipped; other copies with the same name are unaffected.

The **Regex** button adds a regex entry. Whether the pattern applies to the file name or the full path is decided by the same "Match on" selector.

::: tip
File names and paths are compared **case-insensitively**: `Explorer.EXE` and `explorer.exe` are the same target.
:::

## The default entry: File Explorer

A fresh installation ships with one entry for `explorer.exe`, with **Skip hiding** and **Skip freezing** ticked.

File Explorer *is* the desktop and the taskbar: hiding it takes your desktop icons with it, and freezing it locks up the entire Windows shell. This is an ordinary entry — you can edit or delete it.

## Built-in entries: ZoneDeck itself

Two locked entries sit at the top of the list. They **cannot be edited or removed**:

| Entry | Image names covered |
| --- | --- |
| ZoneDeck core | `ZoneDeck.exe`, `core.exe` |
| ZoneDeck settings | `config.exe`, `zonedeck-config.exe` |

Both are permanently set to "Skip freezing". Here is why: the core is usually launched by File Explorer, which makes it a child process of `explorer.exe`, and the settings app is in turn a child of the core. Freeze File Explorer with [Freeze the whole process tree](/en/guide/freeze) enabled and the entire tree — ZoneDeck included — is suspended. Hotkeys stop responding, the settings window will not open, and windows you already hid become **unrecoverable**.

Manual freezing from the [window recovery tool](/en/guide/recovery) respects it too.

## Precedence over hiding rules

The whitelist wins over every rule in [Binding windows & processes](/en/guide/binding). When a program is matched by a process rule *and* has "Skip hiding" ticked, the whitelist decides. You do not have to delete the rule — tick the whitelist entry to take the program out of hiding for a while, untick it to put it back.

## See Also

- [Binding windows & processes](/en/guide/binding)
- [Process freezing](/en/guide/freeze)
- [Window recovery & crash self-healing](/en/guide/recovery)
