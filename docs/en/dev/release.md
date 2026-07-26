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

**Trigger**: manual (`workflow_dispatch`), taking the version to release as input. **Run it from `dev`.**

**What it does**:
1. Writes the version into the four files with `version.ps1 apply`;
2. Commits onto `dev` and tags it `v<version>`, pushing both;
3. Makes sure a `dev` → `main` pull request exists (reusing an open one, creating it otherwise).

**This step does not build.** A tag pushed with GITHUB_TOKEN triggers no workflow, which is exactly what this flow wants: the production build starts only once the PR is merged and the tag enters `main`'s history.

::: warning Merge the release PR with a merge commit
The tag points at the version commit on `dev`. **Squash and rebase merges create a different commit**, so the tag never enters `main`'s history, `release.yml` cannot detect it, and **no build is triggered at all**. Use a **merge commit**.

If you squash-merged by mistake, trigger `release.yml` manually with the tag to recover.
:::

::: tip No checks on the PR?
A PR created with GITHUB_TOKEN does not raise a `pull_request` event, so the PR page shows no CI runs (the push event from your own Merge click still triggers `build-test.yml` normally). If branch protection on `main` requires status checks, add a repo-scoped PAT as the `RELEASE_PAT` secret and the workflow will prefer it when opening the PR.
:::

### `release.yml` — build and publish the release

**Triggers**: a push to `main` (it continues only if a new `v*` tag entered `main`'s history with that push), or a manual run with an explicit tag.

**What it does**: detect the tag the push brought into `main` → check that tag out → verify the tag matches the code version → frontend / Rust tests → `package.ps1 -Installer` to assemble `dist/Boss-Key` and `dist/installer` → zip `dist/Boss-Key` as the portable archive → generate **build provenance** (a Sigstore attestation) → compose the release notes (the auto-generated changelog with a security notice appended) → create a **draft** Release and upload the zip and the installer.

::: info Why it does not listen for `push: tags`
The tag was pushed to `dev` by `tag.yml` using GITHUB_TOKEN, and that push triggers nothing. **Merging a PR does not produce a tag push event either** — a tag is an independent ref, and merging merely makes the commit it points at reachable from `main`. Detecting it from `main`'s push event is the only option left.

Detection compares the set of `v*` tags reachable from `main` before and after the push and takes what is new. It cannot use "the tag on HEAD": HEAD is the merge commit, and the tag points at one of its parents.
:::

### `deploy-docs.yml` — documentation site deployment

**Triggers**: pushes to `main` / `dev` that touch `docs/`, a Release being published / edited / deleted, and manual runs.

**What it does**: [i18n consistency check](/en/dev/contributing#i18n-consistency-check) → regenerate `docs/public/releases.json` (the update-check feed for older clients, **excluding drafts**) → build with VitePress → deploy to GitHub Pages.

::: warning The docs only refresh once you hit "Publish"
`release.yml` creates a **draft** release, and drafts do not raise the `published` event. You have to click **Publish release** on the web for the docs site to redeploy and for the version to land in `releases.json`.
:::

### Releasing a new version

```
dev ──① Bump version and tag──▶ dev (version commit + v3.0.1 tag)
                                 │
                                 ②  PR, merged with a merge commit
                                 ▼
                               main ──③ release.yml sees the new tag──▶ build + draft Release
                                                                             │
                                                                             ④ Publish
                                                                             ▼
                                                                docs site + releases.json refresh
```

1. Once the features are done and you are ready to release, switch to `dev` and run **"Bump version and tag"** in GitHub Actions, entering the version (for example `3.0.1`). The workflow lands the version commit and the tag on `dev` and makes sure a `dev` → `main` PR exists.
2. Review that PR and merge it into `main` with a **merge commit**.
3. The merge triggers `release.yml`: it detects that `v3.0.1` came along into `main`, checks that tag out, runs the production build, and leaves a **draft** Release behind.
4. Check the artefacts and the release notes, then click **Publish release** — this also refreshes the docs site and `releases.json`.

To rebuild or re-publish, trigger `release.yml` manually with the tag.
