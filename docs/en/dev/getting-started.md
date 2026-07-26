---
title: Running locally
---

# Running locally

This chapter covers setting up a development environment, running the core and the settings window, and the commands used day to day.

## Prerequisites

| Dependency | Version / notes | Where to get it |
| --- | --- | --- |
| **Rust** | stable, 1.85+ recommended (the project uses edition 2024) | <https://rustup.rs> |
| **Node.js** | 18+, 24 recommended (for the frontend build) | <https://nodejs.org> |
| **WebView2** | Runtime for the settings window (normally bundled with Win10/11) | Bundled with Windows / [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2) |
| **Inno Setup 6** | *Optional*, needed to build the installer locally | `winget install JRSoftware.InnoSetup` |
| **pssuspend64.exe** | *Optional*, needed to test enhanced freezing | [Microsoft PSTools](https://download.sysinternals.com/files/PSTools.zip) |

::: tip The settings window does not need a dev server
The frontend is **embedded** into `bosskey-config.exe` at build time, so the final artefact runs statically. While working on the frontend, preview it in a browser with `npm run dev` (mock data, hot reload); after `npm run build`, verify the Tauri integration with `cargo run -p bosskey-config`.
:::

## Cloning the project

```bash
git clone https://github.com/IvanHanloth/Boss-Key.git
cd Boss-Key
```

## Common commands

Run these from the **repository root**:

### Core (bosskey-core)

```bash
# Run the core (development)
cargo run -p bosskey-core

# Core smoke test: exits automatically after N milliseconds
cargo run -p bosskey-core -- smoke 3000
```

### Frontend (apps/config/ui)

```bash
# First time: install frontend dependencies
npm --prefix apps/config/ui install

# Build the frontend (output goes to apps/config/dist for Tauri to embed)
npm --prefix apps/config/ui run build

# Frontend unit tests (vitest)
npm --prefix apps/config/ui test

# Browser preview (mock data, hot reload)
npm --prefix apps/config/ui run dev
```

### Settings window (bosskey-config, Tauri)

```bash
# Run the settings window (build the frontend first)
npm --prefix apps/config/ui run build && cargo run -p bosskey-config
```

### Checks and tests

```bash
# Run all Rust tests
cargo test --workspace

# Static analysis (warnings as errors)
cargo clippy --workspace --all-targets -- -D warnings

# Format / check formatting only
cargo fmt --all
cargo fmt --all -- --check
```

### Release builds and packaging

```bash
# Release build (minimised size)
cargo build --release

# One-shot packaging (frontend + Rust + the portable folder dist/Boss-Key)
powershell -File scripts/package.ps1

# Packaging plus installer (dist/installer; installs Inno Setup automatically on first run)
powershell -File scripts/package.ps1 -Installer
```

For packaging details see [Packaging & releasing](/en/dev/release).

## Command cheat sheet

| Purpose | Command |
| --- | --- |
| Run the core | `cargo run -p bosskey-core` |
| Core smoke test | `cargo run -p bosskey-core -- smoke 3000` |
| Install frontend deps | `npm --prefix apps/config/ui install` |
| Build the frontend | `npm --prefix apps/config/ui run build` |
| Frontend tests | `npm --prefix apps/config/ui test` |
| Frontend browser preview | `npm --prefix apps/config/ui run dev` |
| Run the settings window | `npm --prefix apps/config/ui run build && cargo run -p bosskey-config` |
| Release build | `cargo build --release` |
| All Rust tests | `cargo test --workspace` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format check | `cargo fmt --all -- --check` |
| One-shot packaging | `powershell -File scripts/package.ps1` |
| Packaging + installer | `powershell -File scripts/package.ps1 -Installer` |

## Common development problems

::: warning Antivirus blocks a freshly compiled executable
If you see `os error 5 access denied`, antivirus software has usually locked the newly built exe. Add the project's `target` folder to your antivirus allowlist.
:::

::: tip Core tests must run single-threaded
Some core tests initialise COM, and running them in parallel can crash. If you hit this, run them single-threaded with `cargo test -p bosskey-core -- --test-threads=1`. See [Testing strategy](/en/dev/testing).
:::
