---
title: Frontend & settings UI
---

# Frontend & settings UI

The settings window (`config.exe`) is built from **Tauri 2 (Rust backend) + Svelte 5 (frontend)**. This chapter covers the technology choice, the structure and the key designs.

## Why Svelte

The settings window is a **small, form-heavy application**. After comparing the options we chose **Svelte 5 + Vite**:

- **Two-way binding is the core need**: Svelte's `bind:` supports it natively (React has no two-way binding and needs hand-written controlled components; Vue's `v-model` works but has a larger runtime).
- **Compile-time reactivity, no virtual DOM**: the build output is only about 80 KB (roughly 28 KB gzipped), which fits this project's "very small" goal.
- **First-class support in Tauri**.

## Frontend layout

```
apps/config/ui/
├── src/
│   ├── App.svelte           Top level: tabs + auto-save + startup orchestration
│   ├── main.js              Entry point (applies the theme before mounting, to avoid a flash)
│   ├── components/          UI components (TitleBar, the settings panels, the modals…)
│   └── lib/                 Pure logic (unit-testable)
│       ├── state.svelte.js  Global state ($state) + actions
│       ├── ipc.js           Calling Tauri commands / listening for events
│       ├── hotkey.js        Hotkey parsing/formatting (with hotkey.test.js)
│       ├── pointer.js       Mouse click / interval constants (with pointer.test.js)
│       ├── grouping.js      Adding, removing and filtering window/process rules (with grouping.test.js)
│       ├── theme.js         Theme switching (with theme.test.js)
│       ├── i18n.svelte.js   Display language: catalog lookup + language resolution (with i18n.test.js)
│       ├── markdown.js      Markdown rendering for announcements/release notes (with markdown.test.js)
│       └── verhub.js        Update checks / announcements / feedback (incl. issue conversion) / project links / opening external links
├── locales/                 Three-language catalogs (zh-CN.js / en.js / zh-TW.js)
├── vite.config.js
└── svelte.config.js
```

The settings window is organised as tabs, one panel component each:

| Tab | Component | Contents |
| --- | --- | --- |
| Windows | `BindingPanel` | Window list + window/process rules |
| Hotkeys & Mouse | `HotkeysPanel` | Keyboard hotkeys, mouse clicks, corners, auto-hide on idle |
| Alerts | `NotificationsPanel` | Per-event notification switches and tray icon badges |
| Options | `OptionsPanel` | Muting/pausing/freezing/privileges/logs/tools |
| About & Feedback | `AboutPanel` | Version, updates, announcements, feedback (convertible into a GitHub issue) |

## A frameless, self-drawn window

Tauri sets `decorations: false`, and the frontend draws the title bar:

- `data-tauri-drag-region` provides dragging and double-click maximise;
- Self-drawn minimise / maximise / close buttons;
- Eight-direction `startResizeDragging` resize zones at the edges (disabled automatically when maximised);
- Light / dark / follow-system themes (persisted in `localStorage`, with a `prefers-color-scheme` listener).

## Automatic saving

`App.svelte` uses `$effect` to deep-track any change to the configuration object (including `window_rules` / `process_rules`) and writes it to disk through `scheduleSave` (internally debounced) once you pause. Saving is not triggered during loading; auto-save is only "armed" after `loadAll` completes. The scheduling logic lives in `lib/autosave.js`: consecutive changes coalesce into a single write, writes never run concurrently, and a change made while a write is in flight is written right after it completes.

A close request (title-bar button, Alt+F4) first flushes any unsaved changes before the window closes; if the write fails, the window stays open and an error dialog appears, and a second close attempt is not blocked.

After saving, the frontend tells the core to `reload_config` over IPC, and the core hot-reloads the configuration.

## Display language

Strings live in `src/locales/`, one flat key–value table per language; `lib/i18n.svelte.js` handles lookup and language resolution.

```js
import { t } from "../lib/i18n.svelte.js";

t("common.close");                    // → Close
t("restore.frozen", { n: 3 });        // → Froze 3 processes
```

- `t()` reads the current language from `$state`, so calling it in a template re-renders automatically when the language changes — no manual subscription needed.
- **`zh-CN.js` is the source of truth**: add a string there first, then mirror it in `en.js` and `zh-TW.js`. `i18n.test.js` asserts that all three catalogs have identical key sets and placeholders, so a missing translation fails the tests.
- A missing key falls back to Simplified Chinese, and then to the key itself, which makes gaps obvious during development.
- The current language comes from `setting.language`; with `auto` it is inferred from `navigator.language`. `App.svelte` tracks that field with `$effect`, so a change swaps the strings immediately and also writes `<html lang>` (which affects font fallback and line breaking).
- The pure-logic modules in `lib/` (`pointer.js`, `grouping.js`, `theme.js`) also fetch strings through `t()`; since the default language is Simplified Chinese, their existing unit tests needed no changes.

::: warning NO_TITLE is not a translatable string
`grouping.js`'s `NO_TITLE` (`"无标题窗口"`) matches the core's `bosskey_common::NO_TITLE`. It is a cross-process sentinel written into `config.json` and **must not be translated**; only the display swaps it for the current language via `t("common.noTitleWindow")`.
:::

## Markdown in announcements and release notes

The body text of announcements (both the list and the startup dialog) and of release notes is rendered as Markdown by `lib/markdown.js` (a pure function, with `markdown.test.js`) and `components/Markdown.svelte` (styling + link interception). No third-party dependency is involved.

The supported subset covers the common GitHub-flavoured constructs: headings, bold / italic / strikethrough, inline code and fenced code blocks, ordered / unordered / task lists (nesting by indentation), blockquotes, horizontal rules, links and bare URLs. Soft line breaks inside a paragraph become `<br>`, as they do in GitHub comments. Tables, footnotes, inline HTML and syntax highlighting are **not** supported.

::: warning The body comes from a remote server and must be escaped first
Content returned by Verhub is untrusted. `renderMarkdown()` escapes `& < > "` across the whole input before assembling any tags, so every tag in the output is produced by the renderer itself and HTML in the source is only ever displayed literally; link targets are restricted to `http(s)` and `mailto`, and anything else (`javascript:`, `data:`, …) is kept as plain text. Do not work around either rule when changing this module.
:::

Two further trade-offs follow from the runtime environment:

- **Images degrade to links.** Tauri's CSP allows only `self` and `data:` for `img-src`, so remote images cannot load and `![]()` is always rendered as a link.
- **Links do not navigate the webview.** `Markdown.svelte` intercepts the click and hands the URL to `open_external` for the system browser — once the webview really navigates away, there is no way back to the settings window.

## Status polling without blocking the UI

- Status retrieval is merged into a **single `GetStatus`** pipe command and uses **fail-fast connections** (no retries).
- Tauri commands are made asynchronous with `spawn_blocking` to avoid blocking.
- The frontend polls the status every 2 seconds (paused while the page is hidden).

## The frontend needs no dev server

In the final artefact the frontend is **embedded at build time** into `bosskey-config.exe` and runs statically, with **no local server**.

- Working on the frontend UI: `npm run dev` previews it in a browser (mock data, hot reload).
- Verifying the Tauri integration: `npm run build`, then `cargo run -p bosskey-config`.

## Examples of core interaction

- The core's tray "Window Recovery Tool" / "About" entries launch the settings window with an argument: on a cold start it is read from the launch arguments; when already running, the single-instance plugin delivers an event.
- On first start, if the core is not running, the settings window calls `startCore` automatically.
