---
title: FAQ
---

# Frequently asked questions

## Why won't the settings program run on my PC?

The v3 settings program (`config.exe`) is built with Tauri and depends on the system's **WebView2** runtime.

Some stripped-down or older Windows builds (Windows 7 and earlier) may not include WebView2, so the settings window cannot open. To fix it:

1. Install Microsoft Edge WebView2 manually: <https://developer.microsoft.com/microsoft-edge/webview2>
2. Or download the package marked **`win7`** (provided with some releases).

::: info
A settings window that will not open **does not affect the core's hiding features** — the core is a fully native program and does not depend on WebView2. You can still hide windows with your configured hotkeys.
:::

## Will restoring pop up programs I "closed" to the tray?

No. Hiding only records windows that were **visible at the time**, and restoring only reverses Boss Key's own hiding; windows an app hid by itself (Steam's close button, for example, merely hides its window) are untouched and will not be shown on restore.

## Antivirus flags or blocks Boss Key — what now?

Boss Key listens for global hotkeys and hides windows, behaviour that antivirus software sometimes misreads. v3 uses a native single-file Rust implementation, which considerably reduces false positives. If it is still blocked:

- Add Boss Key's **program folder** to your antivirus allowlist;
- Download from the [official Releases page](https://github.com/IvanHanloth/Boss-Key/releases) rather than third-party sources.

::: tip Verifying build provenance
Official artefacts carry build provenance (a Sigstore attestation). Advanced users can run `gh attestation verify <file> -R IvanHanloth/Boss-Key` to confirm an artefact really was built by the official repository.
:::

## I pressed the hide hotkey and nothing happened

Check in order:

1. **Is the core running?** Look at the tray icon or the status bar at the bottom of the settings window.
2. **Have you bound any windows?** With nothing bound and "Also hide the active window" off, there may be no target to hide. See [Binding windows & processes](/en/guide/binding).
3. **Is the hotkey taken?** Try a different combination, or turn on [don't pass through](/en/guide/hotkeys#keeping-hotkeys-from-other-apps) for it — the keyboard hook it switches to is not affected by hotkey-occupancy conflicts. See [Hotkey settings](/en/guide/hotkeys).
4. **Are you on the "Hotkeys & Mouse" page?** That page pauses monitoring temporarily; it resumes when you leave.

## A window was hidden and won't come back

Use the [window recovery tool](/en/guide/recovery) (Options → Tools) to tick and restore it. If you enabled "Also hide the tray icon", use your **restore hotkey**.

## The enhanced freezing switch is greyed out

Enhanced freezing needs all three conditions; missing any one greys it out:

1. "Freeze processes when hiding" is on;
2. The core is **running as administrator**;
3. `pssuspend64.exe` is present in the program folder.

The settings window states which one is missing. See [Process freezing](/en/guide/freeze).

## Will updating lose my configuration?

No. The `config.json` structure is **fully compatible** with older versions, so your bindings, hotkeys and options are preserved. The flat bindings from v2 are migrated to the new rule format automatically.

## Which systems does Boss Key support?

Windows 10 and later work out of the box. On Windows 7 you must ensure WebView2 is available to open the settings window; some releases provide a Windows 7 package that works directly. macOS and Linux are not supported.

## Still stuck?

- Check the relevant chapter of this documentation;
- Open an [issue](https://github.com/IvanHanloth/Boss-Key/issues) on GitHub;
- Send feedback from the [About & Feedback](/en/guide/update) page in the settings window.
