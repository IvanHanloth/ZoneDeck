---
title: Development docs
---

# Development

Welcome to ZoneDeck development. This section is for **developers and contributors**: how to run the project locally, what is expected of contributions, how the project is managed, and how the system is designed.

::: tip
If you only want to use ZoneDeck, read the [user guide](/en/guide/) instead.
:::

## Technology overview

v3 is a complete rewrite. The main choices:

| Part | Technology |
| --- | --- |
| Resident core | **Rust** (fully native, calling the Windows API directly) |
| Settings backend | **Tauri 2** (Rust) |
| Settings frontend | **Svelte 5 + Vite** |
| Inter-process communication | **Named pipe** (one JSON object per line) |
| Project layout | **Cargo workspace** + an npm frontend subproject |
| Packaging | PowerShell scripts + Inno Setup |
| CI/CD | GitHub Actions |

## Design goals

The goals behind the v3 rewrite:

- **Lower memory**: a resident core binary of about 350 KB using roughly 1 MB in the background.
- **More stable**: crash logs, crash recovery and a watchdog form three layers of defence.
- **A single native binary**: no Python runtime, which reduces antivirus false positives.
- **A more modern settings window**: frameless, themeable, saving automatically.

## Suggested reading order

1. [Running locally](/en/dev/getting-started) — environment setup and common commands; get the project running first.
2. [Architecture](/en/dev/architecture) — the overall two-process plus named-pipe design.
3. [Frontend & settings UI](/en/dev/frontend) — the Svelte / Tauri choices and structure.
4. [Contribution guide](/en/dev/contributing) and [Project management](/en/dev/project-management) — required reading before collaborating.
5. [Testing strategy](/en/dev/testing) and [Packaging & releasing](/en/dev/release) — quality and delivery.

Reference material: [Configuration fields](/en/dev/config-reference), [IPC protocol](/en/dev/ipc-protocol).

## Repository

<https://github.com/IvanHanloth/ZoneDeck>
