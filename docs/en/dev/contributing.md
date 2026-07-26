---
title: Contribution guide
---

# Contribution guide

Thank you for contributing to Boss Key. To keep collaboration efficient and safe, please read this page before submitting.

## Contribution workflow

1. **Fork the repository** and clone it locally, or (if you have access) create a branch in the repository.
2. Branch off `dev` into a **feature branch** (naming below).
3. Finish the work and make sure it **passes the local tests and checks** (see the pre-submission checklist).
4. Open a pull request against the **`dev` branch**, linking the relevant issue / project.
5. It is merged after **code review**. `dev` is merged into `main` only when a release is made.

For the full branching and merging strategy, see [Project management](/en/dev/project-management).

## Branch naming

Branch names are not enforced, but for maintainability we **recommend the form `type/feature`**:

| Prefix | Purpose | Example |
| --- | --- | --- |
| `feat/` | New feature | `feat/checkUpdate` |
| `fix/` | Bug fix | `fix/hideWindow` |
| `refactor/` | Refactoring | `refactor/agent` |
| `doc/` | Documentation | `doc/init` |
| `chore/` | Chores / build | `chore/ci` |

::: tip Documentation branches
Use a branch starting with `doc` for all **documentation** changes (for example `doc/guide-freeze`). The docs deployment workflow builds a preview / deploys automatically on pushes to such branches; see [Docs site](/en/dev/docs-site).
:::

## Pre-submission checklist

Before opening a PR, make sure these pass locally (they match CI):

```bash
# 1. Formatting
cargo fmt --all -- --check

# 2. Static analysis (warnings as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Frontend tests and build
npm --prefix apps/config/ui test
npm --prefix apps/config/ui run build

# 4. Rust tests
cargo test --workspace

# 5. The release build succeeds
cargo build --release
```

::: warning Version consistency
If you change the version number, keep `Cargo.toml`, `tauri.conf.json`, `ui/package.json` and `Cargo.lock` **consistent across all four**. CI verifies this with `scripts/version.ps1 check`. During ordinary feature work you should generally **not** change the version by hand — the release process manages it; see [Packaging & releasing](/en/dev/release).
:::

## Code style

- **Rust**: follow the default `rustfmt` style; zero `clippy` warnings.
- **Frontend**: Svelte 5 + modern JS; pure logic goes in `ui/src/lib/` with `vitest` tests, UI in `ui/src/components/`.
- **Commit messages**: prefixes such as `feat: …`, `fix: …`, `doc: …`, `refactor: …` are recommended, mirroring the branch type.

## Testing requirements

- New or changed **pure logic** (configuration, matching, protocol, hotkey parsing, frontend lib, and so on) should come with unit tests.
- For changes touching system behaviour such as windows / freezing / IPC, add integration tests where possible, or describe the manual verification steps.
- See [Testing strategy](/en/dev/testing).

## User-visible strings

Any new user-visible string must be added to all three languages, otherwise the tests fail:

- **Settings window**: add the key to `apps/config/ui/src/locales/zh-CN.js` first (the source of truth), then mirror it into `en.js` and `zh-TW.js`.
- **Core** (tray menu / notifications / IPC errors): add a variant to the `Msg` enum in `crates/core/src/i18n.rs`, provide all three translations, and register it in `ALL_MSGS` in the test module.
- Traditional Chinese uses Taiwanese terminology (視窗, 程式, 檔案, 滑鼠, 快速鍵); do not simply convert Simplified characters to Traditional ones.
- **Logs are not translated** and stay in Simplified Chinese.

### i18n consistency check

Unit tests can only validate a catalog from the inside (key sets, empty values, placeholders). Cross-file slips are caught by `scripts/i18n-check.ps1`:

| Check | What slips through otherwise |
| --- | --- |
| The three doc trees hold the same pages | A language is missing a page and readers hit a 404 |
| Internal links stay within one language | An English page linking to `/guide/…` drops the reader into Chinese; VitePress's dead-link check cannot see this |
| Every key used in `t("key")` exists | The UI renders the raw key name |
| No orphaned catalog keys | Strings left behind by removed features pile up |
| Every `Msg` variant is listed in `ALL_MSGS` | That message skips the cross-language check, so a missing translation goes unnoticed |

```bash
# Run the full check manually
pwsh -File scripts/i18n-check.ps1
```

The repository ships a `pre-commit` hook that runs it (limited to the parts your staged changes touch). **Enable it once per clone:**

```bash
git config core.hooksPath .githooks
```

Use `git commit --no-verify` to skip it when you really need to.

## Opening an issue

The repository provides issue templates:

- **Bug report**: include the version, your OS version, reproduction steps and logs.
- **Feature request**: describe the use case and the expected behaviour.

Where possible, link **issue – pull request – project** together for unified tracking (see [Project management](/en/dev/project-management)).

## License

This project is released under the **MIT** license. By contributing you agree that your code is released under the same license.
