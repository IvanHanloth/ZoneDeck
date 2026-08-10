<p align="center">

![ZoneDeck logo banner](/docs/public/static/banner.svg)

</p>

<h1 align="center">ZoneDeck</h1>

<p align="center">

<img src="https://img.shields.io/github/v/release/IvanHanloth/Boss-Key?style=flat-square" alt="Github Release Version">
<img src="https://img.shields.io/github/license/IvanHanloth/Boss-Key?style=flat-square" alt="Github Repo License">
<img src="https://img.shields.io/github/actions/workflow/status/IvanHanloth/Boss-Key/release.yml?style=flat-square" alt="GitHub Actions Workflow Status">
<img src="https://img.shields.io/badge/Platform-Windows_10\+-cornflowerblue?style=flat-square" alt="Supported Platform">

</p>

<div align="center">
    <h3>
        <a href="https://zonedeck.ivan-hanloth.cn/en/">Website</a>
        <span> • </span>
        <a href="https://zonedeck.ivan-hanloth.cn/en/guide/">Guide</a>
        <span> • </span>
        <a href="https://zonedeck.ivan-hanloth.cn/en/dev/">Development</a>
        <span> • </span>
        <a href="https://github.com/IvanHanloth/Boss-Key/releases">Download</a>
    </h3>
</div>

<p align="center">
    <a href="/README.md">简体中文</a>
    <span> • </span>
    <strong>English</strong>
    <span> • </span>
    <a href="/README.zh-TW.md">繁體中文</a>
</p>

<div align="center">
    <strong>Boss coming? Hide your windows with a single key.</strong><br>

Hide multiple windows and processes, customise hotkeys, hide the active window, mute windows, pause video playback, freeze processes, and more.

Highly configurable to fit how you work, and very light: about 1 MB of memory while resident in the background.

</div><br>

## Screenshots

![ZoneDeck settings – window binding page](/docs/public/static/screenshot-1.png)

![ZoneDeck settings – hotkeys and mouse page 1](/docs/public/static/screenshot-2.png)

![ZoneDeck settings – hotkeys and mouse page 2](/docs/public/static/screenshot-3.png)

![ZoneDeck settings – other options page](/docs/public/static/screenshot-4.png)

## Getting started

Since v3.0.0 every release ships two kinds of package, both available from the [Releases page](https://github.com/IvanHanloth/Boss-Key/releases):

- **installer** (recommended) — a fully packaged installer providing one-click install, update and uninstall, for managing ZoneDeck more efficiently and safely.
- **portable** — an archive containing the ZoneDeck core and settings programs; extract it and run.

Some releases provide a package for Windows 7; those marked `win7` run on Windows 7.

For the complete illustrated documentation, see the ZoneDeck [guide](https://zonedeck.ivan-hanloth.cn/en/guide/).

### Basics

The first time you open ZoneDeck after installing or updating, the settings window appears automatically, where you can change hotkeys and bind processes and windows.

Day to day, right-click the tray icon to open the menu, and choose "Settings" to open the settings window.

The tray menu also offers exiting the program, checking for updates, and toggling startup with Windows.

Press the hide / show hotkey to hide the bound windows at once. Press the close hotkey to close ZoneDeck.

### Binding windows

Binding several windows lets you hide them all at once.

In the upper part of the settings window, the left list holds the windows currently open and the right list holds the ones already bound.

Select a window to hide on the left and add it to the right; likewise, select a rule you no longer need on the right and remove it.

If a newly opened window is missing from the list, refresh the left-hand list.

### Changing hotkeys

Click the record button and press the combination you want; it is recognised and filled in automatically. Clearing a hotkey disables it.

### Mouse triggers

ZoneDeck supports hiding via the middle mouse button and side buttons 1 and 2, optionally with a click count and modifier keys.

You can also enable hiding by moving the pointer quickly into a screen corner (turn on corner restore to allow restoring the same way).

### Display language

The settings window and the core's tray menu and notifications are available in **Simplified Chinese, English and Traditional Chinese**. The language follows the system display language by default, and can be set explicitly under Options → Language.

### More

For the full feature list and usage guide, see the ZoneDeck [guide](https://zonedeck.ivan-hanloth.cn/en/guide/).

## Where the data lives, and how to remove it

The **portable edition** keeps its settings, logs, recovery file and cache **inside the program folder**, so copying the folder takes your whole setup with it. If that folder is not writable (it sits somewhere like `C:\Program Files`, or on read-only media), the program stores them in `%APPDATA%\ZoneDeck` instead and says so in the settings window.

The **installer edition** always uses `%APPDATA%\ZoneDeck`: the installation folder may be `C:\Program Files`, which normal privileges cannot write to. The program tells the two apart by the `installed.marker` file the installer drops.

Either way, the browser component used by the settings window keeps its own data in `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`, which deleting the program folder does not remove. The package ships a `cleanup.ps1`; open PowerShell in the program folder and run:

```powershell
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

It lists what it is about to delete and waits for your confirmation, then removes `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`, any `%APPDATA%\ZoneDeck`, and what autostart leaves behind: the scheduled task `ZoneDeckAutostart` and the registry entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\ZoneDeck Application`. The program folder itself is left alone — delete it yourself once the script is done.

The installer edition does not need this: the uninstaller already does the same, and asks whether to keep your settings file.

## Development and contributing

For details on development and contributing, see the ZoneDeck [development docs](https://zonedeck.ivan-hanloth.cn/en/dev/).

## Credits

Thanks to HsFreezer for the approach to process freezing.

## Changelog

For the full changelog, see the ZoneDeck [changelog](https://zonedeck.ivan-hanloth.cn/en/changelog/).
