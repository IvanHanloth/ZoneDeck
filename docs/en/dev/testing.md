---
title: Testing strategy
---

# Testing strategy

Boss Key is covered by several layers of tests, from pure-logic unit tests to system-level integration tests. CI runs the whole suite on every PR and push.

## Running the tests

```bash
# All Rust tests
cargo test --workspace

# Frontend unit tests
npm --prefix apps/config/ui test
```

::: warning Parallel core tests can crash on COM
Some `bosskey-core` tests initialise COM (the muting path, for example) and can crash when run in parallel. When that happens, run them single-threaded:

```bash
cargo test -p bosskey-core -- --test-threads=1
```
:::

## Rust coverage

### `bosskey-common`

**Pure-logic unit tests** for the models, configuration, matching and protocol, plus integration tests for reading and writing the configuration file. For example:

- Configuration parsing and defaults, and migration of old configurations (flat bindings → window rules, old mouse switches → click triggers);
- Compatibility of the uppercase `PID` field, and round-trip serialisation of regex rules;
- Clamping of click counts and the multi-click interval;
- Round-tripping of `Command` / `Response` and their snake_case tags;
- Language tag parsing and preference normalisation (`zh-Hant` → `zh-TW`, untranslated languages falling back to `auto`, and so on).

### `bosskey-core`

Integration tests of system behaviour (mostly creating real resources and verifying a round trip):

| Area | What is tested |
| --- | --- |
| Window enumeration / hiding / showing | Round trip against a real window |
| Hotkey parsing | String → RegisterHotKey parameters |
| Single-instance mutex | Named mutex |
| Named pipe | Server send/receive |
| Process freezing | Suspending / resuming a real child process |
| Muting | The Core Audio COM path |
| HideController | Mock injection verifying the muting / freezing / pause-key orchestration |
| Startup | Plain registry-key logic + a real `schtasks` accepting the task XML |
| Crash logs | Writing / rotation / the panic hook |
| Crash recovery | Snapshot persistence round trip + mock recovery orchestration |
| Icon encoding | CRC32 / Adler-32 / base64 known vectors + PNG structure parsing + extraction from a real explorer.exe |
| End-to-end IPC | IPC and crash-recovery tests driving a real agent through `PipeClient` |
| String catalog | Every message non-empty in all three languages, English distinct from Chinese, placeholders consistent across languages |

::: info About restricted environments
The `schtasks` startup integration tests may fail in restricted or unprivileged CI and sandbox environments. That is an environment limitation, not a functional regression; they should pass locally with the necessary privileges.
:::

## Frontend tests (vitest)

The frontend keeps its pure logic in `ui/src/lib/`, with matching `vitest` unit tests:

- `hotkey.test.js` — hotkey parsing / formatting;
- `pointer.test.js` — mouse clicks / the multi-click interval;
- `grouping.test.js` — adding, removing and filtering window / process rules;
- `theme.test.js` — theme preference logic;
- `i18n.test.js` — language resolution, and key-set / placeholder alignment across the three catalogs.

The UI components (`components/`) stay thin; the complexity lives in the testable `lib/`.

## Conventions for writing tests

- **Prefer unit tests for pure logic**: anything that can be extracted into a pure function belongs in `common` or the frontend `lib/`, with unit tests.
- **Integration tests for system behaviour**: for windows / COM / pipes, create real resources and verify a round trip, using mocks (the `Effects` / `WindowManager` traits) to isolate side effects where needed.
- Make sure the tests pass locally before committing; see the [contribution guide](/en/dev/contributing).
