---
title: Updates & feedback
---

# Updates & feedback

The **About & Feedback** tab of the settings window gathers version information, update checks, announcements and feedback reporting.

## Checking for updates

Two ways to open it:

1. **Tray menu** → "About".
2. **Settings window** → About & Feedback tab.

The update check fetches the latest version information and download URL from the server:

- If a new version exists, its version number and release notes are shown; click through to download.
- The settings window also runs one update check **automatically** on startup.

### Including preview releases

Only **stable** releases are checked by default. To try features before their official release, enable "Receive preview releases" so the update check also considers pre-releases (such as `-rc`).

::: warning
Preview releases may contain features that are not fully tested and are less stable than official releases. Use them with care.
:::

## Announcements

ZoneDeck can display **announcements** from the server (important update notices, known issues, and so on). New announcements pop up automatically; ones you have already read are not shown again.

## Language of announcements and release notes

Announcements and release notes are fetched in the **current interface language**, and are fetched again as soon as you switch languages. Anything without a translation for that language falls back to its original text.

## Anonymous usage statistics

On first launch ZoneDeck asks whether you want to share anonymous usage statistics. **No statistics are collected before you answer.**

What is sent:

- whether each feature switch is on or off;
- how many rules and whitelist entries there are, how many use a regex, and which hotkeys are commonly set;
- the app version, the language, and how it is running;
- a randomly generated anonymous id that carries no device characteristics.

What is never sent:

- which windows were hidden, which processes are bound, or when any hide happened;
- window titles, process names, file paths, the regexes themselves and feedback text;
- your name, email or anything else that points back to you.

You can opt out any time under **General → Privacy**. Once it is off nothing further is ever sent, and the anonymous id on this device is deleted as well.

## Feedback and reporting

You can send **feedback or a problem report** directly from the "About & Feedback" tab, or open an [issue](https://github.com/IvanHanloth/ZoneDeck/issues) on the **GitHub repository**.

::: tip Include details in your report
To help diagnose the problem, please include:
- The ZoneDeck version;
- Your operating system version;
- Steps to reproduce;
- The relevant [log file](/en/guide/options).
:::

::: warning Please leave a contact
Without a contact there is no way for us to reply to your feedback directly. So for feature requests and bugs we still recommend converting them into an issue.
:::

### Also converting feedback into a GitHub issue

Below the feedback box there is an **Also convert into a GitHub issue** option (the server decides whether it is available; it is hidden when it is not). Tick it before submitting and the Verhub bot turns your feedback into a GitHub issue automatically:

- **No special network setup** and **no GitHub sign-in** required;
- The contact is **required, and should be your GitHub account** (in the form `@IvanHanloth`) — it is how the issue gets followed up with you;
- If the issue cannot be created, the feedback is not recorded at all; file it manually in that case.

You can also open an [issue](https://github.com/IvanHanloth/ZoneDeck/issues) on GitHub directly. The repository provides **bug report** and **feature request** templates; filling one in helps maintainers understand your problem faster.
