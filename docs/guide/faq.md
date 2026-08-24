---
title: 常见问题
---

# 常见问题（FAQ）

## 为什么我的电脑运行不了配置程序？

v3 的配置程序（`config.exe`）使用 Tauri 编写，其运行依赖系统的 **WebView2** 运行时。

部分精简版系统或较低版本系统（Windows 7 及以下）可能默认不包含 WebView2，因此无法打开配置界面。解决方法：

1. 手动安装微软 Edge WebView2：<https://developer.microsoft.com/zh-cn/microsoft-edge/webview2>
2. 或下载带有 **`win7`** 标识的软件包（部分版本提供）。

::: info
配置界面无法打开**不影响核心的隐藏功能**——核心是纯原生程序，不依赖 WebView2。你仍可用已配置好的热键正常隐藏窗口。
:::

## 恢复时会把已"关闭"到托盘的程序弹出来吗？

不会。隐藏时只记录**当时可见**的窗口，恢复只逆转 ZoneDeck 自己的隐藏动作；程序自行藏到托盘的窗口（如 Steam 的关闭按钮只是隐藏窗口）不受影响，恢复时不会被弹出。

## 杀毒软件报毒 / 拦截怎么办？

ZoneDeck 会监听全局热键、隐藏窗口，这类行为有时会被杀软误判。v3 已改用 Rust 原生单文件实现，显著降低了误报概率。若仍被拦截：

- 将 ZoneDeck 的**程序目录**加入杀软信任区 / 白名单；
- 从 [官方 Release 页面](https://github.com/IvanHanloth/ZoneDeck/releases) 下载，避免第三方来源。

::: tip 校验产物来源
官方发布的产物带有构建来源证明（Sigstore attestation）。进阶用户可用 `gh attestation verify <文件> -R IvanHanloth/ZoneDeck` 核验产物确实由官方仓库构建。
:::

## 按了隐藏热键没反应？

请依次检查：

1. **核心是否在运行**？查看托盘图标或配置界面底部状态栏。
2. **是否已绑定窗口**？未绑定任何窗口且未开启"同时隐藏当前活动窗口"时，可能没有可隐藏的目标。见 [绑定窗口与进程](/guide/binding)。
3. **热键是否被占用**？换一个组合键试试，或为该热键开启 [低级键盘钩子](/guide/hotkeys#低级键盘钩子与不传递)——改用键盘钩子后不受组合键占用冲突的影响。见 [热键设置](/guide/hotkeys#键盘热键)。
4. **是否正处于"热键与鼠标"设置页**？该页会临时暂停监听，离开后恢复。

## 窗口被隐藏后显示不回来了？

使用 [窗口恢复工具](/guide/recovery#窗口恢复工具)（通用设置 → 工具）勾选并恢复。若开启了"同时隐藏 ZoneDeck 托盘图标"，请用你的**恢复热键**恢复。

## 能隐藏其他程序的托盘图标吗？

ZoneDeck 只能隐藏[自身的托盘图标](/guide/hiding#同时隐藏-zonedeck-托盘图标)，无法操作其他程序的托盘图标。可使用 Windows 自带的功能手动设置：具体步骤参见微软官方教程 [在 Windows 中自定义任务栏 · 系统托盘](https://support.microsoft.com/zh-cn/windows/experience/personalization/customize-the-taskbar-in-windows#system-tray)，或直接打开 [任务栏设置](ms-settings:taskbar)（该链接仅在 Windows 上有效），选择哪些图标显示在任务栏角落。

## 增强冻结的开关是灰的，点不了？

增强冻结需要同时满足三个条件，缺一即被置灰：

1. 已开启"隐藏窗口时冻结进程"；
2. 核心**以管理员身份运行**；
3. 程序目录下有 `pssuspend64.exe`。

配置界面会提示当前缺少哪一项。详见 [进程冻结](/guide/freeze#使用增强冻结)。

## 提示"保存配置失败"（拒绝访问 / os error 5）怎么办？

先升级到 v3.1.0 或更高版本。旧版本把设置固定存在程序所在目录，而选了「为所有用户安装」时程序装在 `C:\Program Files`，普通权限写不进去，于是每次改设置都保存失败。新版本的安装版一律把设置存到 `%APPDATA%\ZoneDeck`，并把已有的 `config.json` 迁过去，无需手动处理。便携版仍存在程序目录里，若那里不可写也会自动改用 `%APPDATA%\ZoneDeck` 并弹出说明。

升级后仍然报错，多半是另外两种情况：

- **被杀软拦截**：把 ZoneDeck 的程序目录与 `%APPDATA%\ZoneDeck` 加入杀软信任区。Windows 安全中心的"受控文件夹访问"也会以同样的方式拦截写入。
- **配置文件被设为只读**：在资源管理器中右键 `config.json` → 属性，取消"只读"。

报错信息里带有实际路径，据此可判断问题出在哪个目录。

## 更新后配置会丢失吗？

不会。`config.json` 结构与旧版**完全兼容**，更新后你的绑定、热键、选项都会保留。V2版的扁平绑定也会自动迁移到新的规则格式。

## ZoneDeck 支持哪些系统？

Windows 10 及以上开箱即用；Windows 7 需自行确保 WebView2 可用才能打开配置界面。部分版本提供 Windows 7 软件包可直接在win7版本中使用。目前不支持 macOS / Linux。

## 还有其他问题？

- 查阅本使用文档的对应章节；
- 前往 GitHub 提交 [Issue](https://github.com/IvanHanloth/ZoneDeck/issues)。
- 在配置界面的 [关于与反馈](/guide/update#问题反馈与上报) 页提交反馈；
