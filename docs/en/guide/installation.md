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

The installer first asks who it is installing for:

- **Install for me only** (default, no administrator rights needed): installs into `%LocalAppData%\Programs\Boss Key`.
- **Install for all users** (needs administrator rights): installs into `C:\Program Files\Boss Key`.

Either way the settings are stored in `%APPDATA%\BossKey`, not in the installation folder — see [Where the data lives](#where-the-data-lives) below. The installer drops an `installed.marker` file in the installation folder so the program knows it is an installed copy; do not delete it.

## Using the portable edition

1. Download `Boss-Key-<version>-portable.zip`.
2. Extract it anywhere. The archive already contains a `Boss-Key` folder — move that whole folder wherever you want it.
3. Run **`Boss Key.exe`** from that folder. On first run it opens the settings window automatically.

The extracted folder looks like this:

```
Boss-Key/
├── Boss Key.exe      Resident core (runs in the background; hides windows / listens for hotkeys)
├── config.exe        Settings window (opened on demand; exits when closed)
├── cleanup.ps1       Leftover-data cleanup script (see "Uninstalling" below)
├── LICENSE.txt       License file
├── README.md         Readme (Simplified Chinese)
├── README.en.md      Readme (English)
└── README.zh-TW.md   Readme (Traditional Chinese)
```

::: warning Both programs must stay in the same folder
`Boss Key.exe` and `config.exe` cooperate through a shared `config.json` and a named pipe. Do not separate them.
:::

## Where the data lives

The settings, logs, recovery file and cache share one folder, and which folder that is depends on the edition:

- **Portable**: right inside the **program folder**. Copy that folder and your whole setup comes along — exactly what a portable copy should do.
- **Installer**: **`%APPDATA%\BossKey`**. The installation folder may be `C:\Program Files`, which normal privileges cannot write to, so settings kept there would fail to save every time.

The program tells the two apart by the `installed.marker` file the installer places in the installation folder; do not delete it.

::: warning A portable copy in an unwritable location
If the portable copy's folder is not writable (it sits somewhere like `C:\Program Files`, or on read-only media), the program switches to `%APPDATA%\BossKey` and shows a notice in the settings window explaining that this is a permissions problem and what to do about it. Nothing stops working; the settings just no longer travel with the program folder.
:::

The browser component used by the settings window keeps its own data in `%LOCALAPPDATA%\cn.hanloth.bosskey.config` in both editions.

When the location changes (for example after installing over a portable copy), the existing `config.json` is moved across on the first start, so your bindings and hotkeys are preserved. The **Open log folder** button in the settings window's status bar always opens the folder actually in use.

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

- **Installer**: uninstall through Windows "Apps & features" or the uninstaller in the installation folder. Uninstalling also removes runtime files — logs, caches and the settings window's browser data — and **asks whether to keep the configuration file**: keep it to leave `config.json` behind (reusable after reinstalling), or decline to delete it along with `%APPDATA%\BossKey`. A silent uninstall shows no prompt and keeps the configuration by default.
- **Portable**: exit the core, run `cleanup.ps1` from the folder to clear what is left in the user folder, then delete the whole program folder (the settings live inside it and go with it).

The portable cleanup command (open PowerShell in the program folder):

```powershell
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

The script lists what it is about to delete and waits for your confirmation, then removes `%LOCALAPPDATA%\cn.hanloth.bosskey.config`, any `%APPDATA%\BossKey` (present only if the program folder was not writable), and what autostart leaves behind: the scheduled task `BossKeyAutostart` and the registry entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Boss Key Application`. The program folder itself is left alone — delete it yourself once the script is done.
