# ZoneDeck Privacy Statement

**Effective date: 20 September 2026** | **Last updated: 20 September 2026**

Your privacy matters to us. This privacy statement explains what data ZoneDeck (the "app") processes, how it processes
that data, and what it is used for. The app is developed and maintained by an individual developer, **Ivan Hanloth** ("
we"). Please read this statement in full, paying particular attention to the passages in **bold**. If you do not agree
with its terms, please stop using the app. If you are under 14, please read this statement with your guardian and obtain
their consent before using any feature that connects to the internet.

### The data we collect and process

The app collects or processes a limited amount of data through your interactions with us and your use of the app. Which
data we process depends on the features you use and the privacy settings you choose.

**1. Data processed locally on your device only** To provide core features such as window management and process
freezing, the app creates and reads the following data locally on your device (in `%APPDATA%\ZoneDeck`, or the program
folder for the portable edition). **This data is never uploaded to any server:**

* **Settings file (`config.json`) and recovery data (`recovery.json`)**: the window rules you configure (window handle,
  process name, regular expressions and so on), process and whitelist rules, hotkey and mouse bindings, interface
  preferences, and the state records used to bring your windows back if the app exits unexpectedly.

* **Run logs (`logs\`) and power statistics (`stats.json`)**: daily run logs (sanitised before being written to disk, so
  they contain no window titles, user directories or other sensitive information), plus the freeze duration, memory
  released and similar figures shown as your power-saving results.

* **System interaction and permission data (held in memory only)**: the app enumerates the windows and processes
  currently running. When you enable the corresponding hotkeys, it installs Windows low-level keyboard and mouse hooks (
  `WH_KEYBOARD_LL` / `WH_MOUSE_LL`). We undertake that **these hooks serve only to match your hotkeys or evaluate
  gestures in real time. They have no keylogging capability and never store or transmit your keystrokes or mouse
  movements.**

**2. Data that leaves your device**
The data we collect and send comprises:

* **Necessary device and operating data**: to provide update checks and announcements, the app automatically sends a
  limited set of data to the server when you open the settings window (project identifier, platform, OS version, app
  version and interface language). **This contains nothing that identifies you.**

* **Diagnostic and usage data**: off by default. Only after you actively choose "Agree" do we collect anonymous usage
  statistics, comprising which features are switched on, counts of your rules, and a randomly generated anonymous
  identifier.

* **Error reports and feedback**:
    * **Error logs**: the fault log shown when the app fails, used to trace the problem, **sent only after you click "
      Report to developer"**.

    * **User feedback**: the text, rating and optional contact details you enter yourself on the "About & Feedback"
      page.

### Data we never collect

Under no circumstances does the app read, record or transmit the following:

* Keyboard input, clipboard contents, screen contents or screenshots.

* Window titles, process names, file paths or regular expressions.

* Your name, email address, phone number or other identity information.

* Unique physical device identifiers (MAC address, serial number, advertising identifier and the like), location data or
  sensor data.

* The list of other software installed on your machine, browsing history, or credentials such as passwords.

The app serves no advertising, builds no commercial user profiles, performs no cross-app tracking, and never sells,
rents or trades any information.

### How we use data

Specifically, we use the limited data collected above for the following purposes:

* **Providing and maintaining our product**: we use your settings data locally to carry out the window hiding, process
  suspension and other core operations you ask for. We use basic device data to deliver software updates and the latest
  announcements to you.

* **Product improvement and development**: if you choose to take part in anonymous usage statistics, we use that data to
  analyse how popular each feature is, so we can make informed decisions and keep improving the product.

* **Customer support and troubleshooting**: when you send us an error report or feedback yourself, we use it to diagnose
  faults, fix defects and keep the service secure, and to reply to you if you have left contact details.

### Reasons we share data

As a rule, we do not share your data with third parties. To provide updates, statistics and feedback, however, we rely
on trusted third parties, and in specific circumstances we may disclose data:

* **Verhub**: update checking, statistics, and the receipt of error reports and feedback are all provided and processed
  by the Verhub service (`https://verhub.hanloth.cn`). These network requests inevitably generate IP address and access
  log records. The detailed rules governing that data are set out in
  the [Verhub Privacy Policy](https://verhub.hanloth.cn/terms/privacy-policy).

* **GitHub (only when you choose it)**: if you tick "Also convert to a GitHub Issue" when submitting feedback, your
  feedback text and contact details are published automatically, via Verhub, to the ZoneDeck GitHub repository, where
  **the content will be publicly visible to everyone**. Please consider this carefully before ticking the box, and
  avoid including anything private.

* **Legality and safety**: we may process or retain limited data in order to comply with applicable law, or to protect
  the rights, property and safety of the developer and of users.

### How to access and control your data

You have complete control over your own data and can manage it at any time:

* **Accessing, changing and deleting local data**: you can inspect, change or delete the settings and log files on your
  machine directly through File Explorer at any time. Choosing "do not keep settings" when uninstalling the app erases
  all local data completely.

* **Controlling diagnostic and statistics data**: you can withdraw your consent at any time under "General settings →
  Privacy → Anonymous usage statistics". Once the option is off, the app stops sending any statistics and deletes the
  anonymous identifier from your device.

* **Exercising other data rights**: for feedback or error log data already sent to the server, you may request access, a
  copy, rectification or erasure, to the extent applicable law provides, by writing to our contact address
  (**ivan@hanloth.com**). Because the app creates no account, handling such a request normally requires you to supply
  the submission time, a summary of the content or similar details so we can locate the record.

### Other important privacy information

**Children's privacy**
The app is not directed at children and does not knowingly collect their data. It has no account system and does not ask
for your age. If a minor has submitted information without their guardian's consent, the guardian may write to our
contact address and ask us to delete it.

**Storage and retention**
Data sent over the network is stored on Verhub servers deployed within the People's Republic of China. Your local data
is kept until you delete it yourself. Feedback and error log data on the server is normally retained for 12 months from
submission (the Verhub Privacy Policy governs the specifics).

**Third-party products and services**

* If you obtained the app through the **Microsoft Store**, the handling of installation, update and store-account data
  by Microsoft is governed by the [Microsoft Privacy Statement](https://www.microsoft.com/privacy/privacystatement). The
  app itself does not read your Microsoft account information.

* The settings window is rendered with the **Microsoft Edge WebView2** component built into Windows, and the statements
  published by Microsoft likewise govern its data handling. The app uses the component to render a local interface only
  and loads no external web pages.

### Updates to this statement, and how to contact us

This privacy statement may be revised from time to time. Any update is published on this page with the "last updated"
date marked. For material changes — a change to the purposes of processing or to the categories of information, for
example — we will also notify you more prominently, such as through an in-app announcement.

If you have any question, comment or complaint about this privacy statement, please contact us:

* **Email**: [ivan@hanloth.com](mailto:ivan@hanloth.com)

* **Issue tracker**: [GitHub Issues](https://github.com/IvanHanloth/ZoneDeck/issues)

* **Project homepage**: [https://zonedeck.ivan-hanloth.cn/](https://zonedeck.ivan-hanloth.cn/)

We will respond within 15 working days of verifying your identity.
