<p align="center">

![Boss-Key logo bannar](/docs/public/static/bannar.jpg)

</p>

<h1 align="center">Boss-Key</h1>

<p align="center">

<img src="https://img.shields.io/github/v/release/IvanHanloth/Boss-Key?style=flat-square" alt="Github Release Version">
<img src="https://img.shields.io/github/license/IvanHanloth/Boss-Key?style=flat-square" alt="Github Repo License">
<img src="https://img.shields.io/github/actions/workflow/status/IvanHanloth/Boss-Key/tag-release.yml?style=flat-square" alt="GitHub Actions Workflow Status">
<img src="https://img.shields.io/badge/Platform-Windows_10\+-cornflowerblue?style=flat-square" alt="Supported Platform">

</p>

<div align="center">
    <h3>
        <a href="https://boss-key.ivan-hanloth.cn/">项目官网</a>
        <span> • </span>
        <a href="https://boss-key.ivan-hanloth.cn/guide/">使用文档</a>
        <span> • </span>
        <a href="https://boss-key.ivan-hanloth.cn/dev/">开发文档</a>
        <span> • </span>
        <a href="https://github.com/IvanHanloth/Boss-Key/releases">下载地址</a>
    </h3>
</div>

<p align="center">
    <strong>简体中文</strong>
    <span> • </span>
    <a href="/README.en.md">English</a>
    <span> • </span>
    <a href="/README.zh-TW.md">繁體中文</a>
</p>

<div align="center">
    <strong>老板来了？快用Boss-Key老板键一键隐藏窗口！上班摸鱼必备神器。</strong><br>

支持多窗口隐藏、多进程隐藏、自定义热键、隐藏活动窗口、静音窗口、暂停视频播放、进程冻结等超多功能。

超高自定义度，满足你的不同隐藏需求。极简内存，后台常驻仅1M内存占用，

</div><br>

## 应用截图

![Boss-Key设置窗口绑定页](/docs/public/static/screenshot-1.png)

![Boss-Key设置鼠标热键设置页-1](/docs/public/static/screenshot-2.png)

![Boss-Key设置鼠标热键设置页-2](/docs/public/static/screenshot-3.png)

![Boss-Key设置窗口其他选项页](/docs/public/static/screenshot-4.png)

## 使用说明

从v3.0.0版本开始，每个版本都会提供两种类型的程序，可以从[Release页面](https://github.com/IvanHanloth/Boss-Key/releases)下载

- installer - 安装程序（推荐），完整封装的Boss-Key程序安装程序，提供一键安装、更新、卸载，可以更高效安全的管理Boss-Key程序
- portable - 便携版，包含Boss-Key的核心程序和配置程序的压缩包，解压后可以运行

部分版本会提供win7系统的软件包，带有win7标识的可以在Windows 7系统上运行

完整的图文使用说明，请参阅 Boss-Key [使用文档](https://boss-key.ivan-hanloth.cn/guide)

### 基础使用

安装或更新后首次打开Boss-Key，会自动弹出设置页面，可以在其中进行热键修改、进程及窗口绑定的等操作。

而一般使用时，可以通过右键点击托盘图标打开菜单。点击菜单中的“设置”即可打开设置页面。

右键点击托盘图标还有退出程序、检查更新、设置开机自启等功能。

按下隐藏/显示窗口热键可以一键隐藏所绑定的窗口。按下一键关闭程序热键可以一键关闭Boss-Key程序

### 绑定窗口

通过绑定窗口，可以同时隐藏多个窗口，摸鱼更安全~

设置窗口中上方部分，左边列表是当前存在的窗口，右边列表是已经绑定的窗口

在左边列表中选中希望隐藏的窗口，点击“添加绑定”可以将窗口信息添加到右边。同理，在右边窗口中选择不需要绑定的窗口，点击“删除绑定”可以将绑定信息移动到左边。

如果发现新打开的窗口没有在列表中显示，可以点击“刷新进程”按钮，刷新左边的列表。

### 修改热键

修改热键有两种方式，可以通过直接编辑文本框中的内容来修改绑定的热键，或者点击“录制热键”按钮，打开热键录制窗口进行录制。

打开热键录制窗口后，按下的组合键将被记录，并显示在窗口中，如果确认无误，点击确认，将自动填写至热键文本框中。

### 鼠标隐藏

v2.1.0版本加入了鼠标相关操作隐藏绑定，可以选择鼠标中键、侧键1、侧键2切换串口隐藏状态。

可以勾选快速移动鼠标至四角隐藏窗口（启用允许移动恢复功能以允许通过快速移动鼠标至四角恢复窗口）

### 界面语言

配置界面与核心的托盘菜单、通知支持**简体中文、English、繁體中文**。默认跟随系统显示语言，也可在「其他选项 → 语言」中手动指定。

### 更多功能

完整功能介绍及使用指南，请参阅 Boss-Key [使用文档](https://boss-key.ivan-hanloth.cn/guide)

## 开发及贡献指南

有关开发和贡献的详细信息，请参阅 Boss-Key [开发文档](https://boss-key.ivan-hanloth.cn/dev)

## 鸣谢

感谢雪藏HsFreezer提供的进程冻结实现思路

## 更新日志

完整的更新日志请参阅 Boss-Key [更新日志](https://boss-key.ivan-hanloth.cn/changelog)
