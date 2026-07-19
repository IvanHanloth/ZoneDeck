---
title: Installation & editions
---

# Installation & editions

Every release of Boss Key can be downloaded from the GitHub [Releases page](https://github.com/IvanHanloth/Boss-Key/releases).

## Choosing a package

Since **v3.0.0**, each release ships two kinds of package:

| Type | Description | Best for |
| --- | --- | --- |
| **installer** | A full installer providing one-click install, update and uninstall | **Recommended**. Manages Boss Key more efficiently and safely |
| **portable** | An archive containing the core and settings programs; extract and run | Portable use, no installation, carrying on a USB drive |

::: tip Windows 7 users
Windows 7 and some stripped-down Windows builds may not include WebView2, which prevents the settings window from opening. Install the WebView2 runtime manually from <https://developer.microsoft.com/microsoft-edge/webview2>.

Some releases additionally provide a package marked `win7`. On Windows 7, download that one.
:::

## Using the installer

1. Download `Boss-Key-<version>-Setup.exe`.
2. Run it and follow the wizard. The installer **terminates the running core process automatically** before installing, to avoid file locks.
3. Boss Key starts automatically once installation finishes and opens the settings window.

## Using the portable edition

1. Download `Boss-Key-<version>-portable.zip`.
2. Extract it anywhere (preferably a fixed, writable location). The archive already contains a `Boss-Key` folder — move that whole folder wherever you want it.
3. Run **`Boss Key.exe`** from that folder. On first run it opens the settings window automatically.

The extracted folder looks like this:

```
Boss-Key/
├── Boss Key.exe    Resident core (runs in the background; hides windows / listens for hotkeys)
├── config.exe      Settings window (opened on demand; exits when closed)
├── LICENSE.txt     License file
└── README.md       Readme
```

::: warning Both programs must stay in the same folder
`Boss Key.exe` and `config.exe` cooperate through the `config.json` file in the same folder and a named pipe. Do not separate them.
:::

## About the two executables

Boss Key uses a two-process design that separates **core and settings**:

- **`Boss Key.exe` (core)**: resident in the background; listens for hotkeys and hides / shows windows.
- **`config.exe` (settings window)**: opened only when you want to change settings; it exits when closed and does not stay in memory.

Day to day you interact only with the tray icon and the settings window. The core runs quietly in the background.

## Updating

- **Installer**: run the new version's installer to upgrade in place. The [update check](/en/guide/update) in the settings window provides a download link for the latest version.
- **Portable**: download the new archive and overwrite the old files (exit the core first).

::: tip Upgrading from v2.x
When upgrading from v2, first copy and keep `config.json` and `pssuspend64.exe`. Uninstall using the old uninstaller, then run the new installer, and finally copy `config.json` and `pssuspend64.exe` into the new installation folder.
:::

## Uninstalling

- **Installer**: uninstall through Windows "Apps & features" or the uninstaller in the installation folder. Uninstalling also removes runtime files such as logs, and **asks whether to keep the configuration file**: keep it to leave `config.json` behind (reusable after reinstalling), or decline to delete the entire installation folder. A silent uninstall shows no prompt and keeps the configuration by default.
- **Portable**: exit the core, then delete the whole folder. If you enabled startup with Windows, turn it off from the tray menu first.
