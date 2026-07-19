---
title: Packaging & releasing
---

# Packaging & releasing

This chapter covers building artefacts locally and the automated release process based on GitHub Actions.

## One-shot local packaging

```powershell
# Portable folder: build the frontend → cargo release → assemble dist/
powershell -File scripts/package.ps1

# Also produce the Inno Setup installer (requires Inno Setup)
powershell -File scripts/package.ps1 -Installer

# Reuse the existing frontend output (faster when the frontend is unchanged)
powershell -File scripts/package.ps1 -SkipFrontend
```

What `package.ps1` does: build the frontend (Vite + Svelte) → release-build the Rust workspace (the Tauri build script embeds the frontend `dist` into `bosskey-config.exe`) → assemble the portable folder → optionally produce the installer.

### Artefact layout

The portable edition and the installer each occupy their own subfolder of `dist/`, without interfering:

```
dist/
├── Boss-Key/                    Portable edition (copy and run; zipped whole for release)
│   ├── Boss Key.exe               Resident core (embedded DPI/long-path manifest + version info + icon)
│   ├── config.exe                 Settings window (frontend embedded; self-contained)
│   ├── LICENSE.txt
│   └── README.md
└── installer/                   Installer (produced with -Installer)
    └── Boss-Key-<version>-Setup.exe  Inno Setup (terminates a running core before installing)
```

The portable edition **needs no installation and has no external dependencies** (beyond the system's WebView2). The two programs cooperate through `config.json` in the same folder and a named pipe.

## Version management

::: info The single source of truth
The single source of truth for the version is `[workspace.package] version` in `Cargo.toml`. Three other places must match it: `apps/config/src-tauri/tauri.conf.json`, `apps/config/ui/package.json` and `Cargo.lock`.
:::

`scripts/version.ps1` writes and verifies it:

```powershell
# Write the version into all four files (and sync Cargo.lock)
powershell -File scripts/version.ps1 apply 3.0.1

# Verify all four match this tag; fail if not
powershell -File scripts/version.ps1 check 3.0.1

# Without a tag, verify the other files against Cargo.toml
powershell -File scripts/version.ps1 check

# Print the current version
powershell -File scripts/version.ps1 show
```

Semver is supported (`3.0.1`, `3.1.0-rc.1`, optionally with a leading `v`). A version containing `-` is treated as a **pre-release**. The installer requires a purely numeric four-part version and converts automatically (`3.1.0-rc.1` → `3.1.0.0`).

## CI/CD workflows

The workflows live in `.github/workflows/`.

### `build-test.yml` — build and test

**Triggers**: PRs / pushes on any branch (except tags), and manual runs.

**What it does**: version consistency check → frontend install / test / build → `cargo fmt --check` → `cargo clippy` → `cargo test` → `cargo build --release` → upload the binaries for inspection.

A new push on the same branch cancels the previous run automatically (`concurrency` + `cancel-in-progress`).

### `tag.yml` — write the version and tag

**Trigger**: manual (`workflow_dispatch`), taking the version to release as input.

**What it does**:
1. Writes the version into the four files with `version.ps1 apply`;
2. Commits and pushes back to the current branch, then creates and pushes the `v<version>` tag;
3. Calls `release.yml` to build and publish.

### `release.yml` — build and publish the release

**Triggers**: a pushed `v*` tag, or a `workflow_call` from `tag.yml`.

**What it does**: verify the tag matches the code version → frontend / Rust tests → `package.ps1 -Installer` to assemble `dist/Boss-Key` and `dist/installer` → zip `dist/Boss-Key` as the portable archive → generate **build provenance** (a Sigstore attestation) → create the GitHub Release and upload the zip and the installer.

### Releasing a new version

Triggering `tag.yml` manually is recommended:

1. Run **"Bump version and tag"** in GitHub Actions and enter the version (for example `3.0.1`).
2. The workflow writes the version, tags, builds and publishes the release automatically.

You can also tag and push locally to trigger `release.yml` directly, but you must ensure the version is already consistent yourself.
