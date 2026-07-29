---
title: Project management
---

# Project management

To move development forward efficiently and safely, Boss Key follows a set of conventions for branching, merging and planning.

## Branch model

The repository has two **main branches**:

| Branch | Purpose | Commit rules |
| --- | --- | --- |
| **`main`** | Holds the source of **officially released versions** | **No direct commits.** In principle it only receives merges from `dev` once a version's features are complete, a release is decided, and testing has passed |
| **`dev`** | Holds the source of **everything under development** | **No direct commits.** In principle it should only contain features that are complete and tested |

Other branches (`feat/*`, `fix/*`, `doc/*`, …) are for day-to-day work; see [branch naming](/en/dev/contributing).

### Merge direction

```
feat/* · fix/* · doc/* ──PR──▶ dev ──PR (at release time)──▶ main
      (merged after code review)      (merged after testing passes)
```

- New features, bug fixes and so on should be **submitted to `dev` by pull request** and merged after code review.
- Once several features have accumulated and been tested, `dev` is merged into `main` by PR, triggering a release.

::: tip Why not commit to the main branches directly
Protecting `main` and `dev` keeps unreviewed code out of the stable line, so both branches are buildable and releasable at any moment.
:::

## Pull request conventions

- One PR, one concern — it keeps review manageable.
- A PR must pass all CI checks (formatting, Clippy, frontend tests and build, Rust tests, the release build).
- Link the relevant issue (for example `Closes #123`) and project.
- At least one **code review** before merging.

## Project conventions

Planning is generally tracked with **GitHub Projects**.

::: info Linking all three
Try to link **issue – pull request – project** together for unified tracking:
- The issue describes what to do or what went wrong;
- The PR implements it and references the issue;
- The project board tracks overall progress.
:::

## Versioning and releases

- The **single source of truth** for the version is `[workspace.package] version` in `Cargo.toml`; everywhere else takes it at build time.
- Releases are made through a manually triggered GitHub Actions workflow: write the version → tag → build and publish the release.
- See [Packaging & releasing](/en/dev/release).

## Issue templates

The repository provides structured templates under `.github/ISSUE_TEMPLATE/`:

- `bug-report.yml` — bug reports;
- `feature-request.yml` — feature requests;
- `config.yml` — the issue entry configuration.

Using a template ensures a report contains enough information to investigate.
