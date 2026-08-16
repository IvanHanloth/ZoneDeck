---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "ZoneDeck"
  text: "Hide instantly, with one key"
  tagline: Boss coming? Hide, mute and freeze your windows with a single key. (formerly the Boss-Key open-source project)
  image:
    src: /static/logo.svg
    alt: ZoneDeck
  actions:
    - theme: brand
      text: Quick start
      link: /en/guide/getting-started
    - theme: alt
      text: Download
      link: https://github.com/IvanHanloth/ZoneDeck/releases

features:
  - title: Instant hiding
    icon: ⚡️
    details: Hide multiple windows and processes at once, triggered by keyboard hotkeys, mouse clicks, screen-corner gestures, or an idle timer.
  - title: Minimal footprint
    icon: 🪄
    details: v3 is rewritten in Rust; the resident core uses about 1 MB of memory, and its native implementation rarely trips antivirus false positives.
  - title: Highly configurable
    icon: 💅
    details: Mute after hiding, send a pause key, freeze processes, hide its own tray icon, match windows and processes by regex — tailor it to how you work.
  - title: Built to stay up
    icon: 🛡️
    details: Crash logs, crash recovery and scheduled-task startup form three layers of defence for long-running background operation.
  - title: Modern interface
    icon: 🎨
    details: A frameless settings window with light / dark / system themes, automatic saving, and built-in announcements and feedback reporting.
  - title: Free and open source
    icon: 🧩
    details: Open source under the MIT license — contributions and issue reports are welcome. Works out of the box on Windows 10+.
---

