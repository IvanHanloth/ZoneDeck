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

### `release.yml` — one-shot release

**Trigger**: manual (`workflow_dispatch`), taking the version to release as input. **Run it from `main`** (after the release content has been merged into `main`).

**What it does**:
1. Writes the version into the four files with `version.ps1 apply`;
2. Federates the workflow's OIDC identity through [octo-sts](https://octo-sts.dev) into a short-lived `contents:write` token for this repository;
3. Commits the version change onto the triggering branch via GraphQL `createCommitOnBranch` and creates the `v<version>` annotated tag — commits created through the API are signed by GitHub server-side and carry the **Verified** badge;
4. Checks that tag out → verifies the tag matches the code version → frontend / Rust tests;
5. `package.ps1 -Installer` to assemble `dist/Boss-Key` and `dist/installer` → zip `dist/Boss-Key` as the portable archive;
6. Generates **build provenance** (a Sigstore attestation) → composes the release notes (the auto-generated changelog with a security notice appended) → creates a **draft** Release and uploads the zip and the installer.

If the tag already exists, steps 2 and 3 are skipped and that tag is checked out and rebuilt — rerunning / re-publishing is just running again with the same version.

::: info Where the credentials come from
The repository stores no long-lived credentials. The workflow trades its GitHub Actions OIDC identity to octo-sts for a short-lived token; the conditions are declared in `.github/chainguard/tag-release.sts.yaml` (only runs on `main` / `dev` are allowed), and the token is revoked automatically when the job ends. The Octo STS App is on the branch protection bypass list, so the version commit needs no release PR.

The commit carries the Verified badge because it is created through the GitHub API and signed by GitHub server-side; tags have no server-side signing mechanism and remain plain annotated tags.
:::

### `deploy-docs.yml` — documentation site deployment

**Triggers**: pushes to `main` / `dev` that touch `docs/`, a Release being published / edited / deleted, and manual runs.

**What it does**: [i18n consistency check](/en/dev/contributing#i18n-consistency-check) → regenerate `docs/public/releases.json` (the update-check feed for older clients, **excluding drafts**) → build with VitePress → deploy to GitHub Pages.

::: warning The docs only refresh once you hit "Publish"
`release.yml` creates a **draft** release, and drafts do not raise the `published` event. You have to click **Publish release** on the web for the docs site to redeploy and for the version to land in `releases.json`.
:::

### Releasing a new version

```
main ──① Release──▶ version commit (Verified) + v3.0.1 tag ──▶ build + draft Release
                                                                    │
                                                                    ② Publish
                                                                    ▼
                                                       docs site + releases.json refresh
```

1. Once the features are done, merge `dev` into `main` through a PR as usual.
2. Run **"Release"** from `main`, entering the version (for example `3.0.1`). The workflow lands the version commit and the tag on `main`, then runs the production build and leaves a **draft** Release behind.
3. Check the artefacts and the release notes, then click **Publish release** — this also refreshes the docs site and `releases.json`.
4. Merge `main` back into `dev` (or merge `main` before the next PR from `dev`) so the version commit returns to `dev`.

To rebuild or re-publish, run **"Release"** again with the same version.

