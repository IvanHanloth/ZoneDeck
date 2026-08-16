---
title: 安装与版本选择
---

# 安装与版本选择

ZoneDeck 的所有版本均可从 GitHub [Release 页面](https://github.com/IvanHanloth/ZoneDeck/releases) 下载。

## 选择合适的软件包

自 **v3.0.0** 起，每个版本都会提供两种类型的程序包：

| 类型 | 说明 | 适用场景 |
| --- | --- | --- |
| **installer（安装版）** | 完整封装的安装程序，提供一键安装、更新、卸载 | **推荐**。更高效、安全地管理 ZoneDeck |
| **portable（便携版）** | 包含核心程序与配置程序的压缩包，解压即用 | 便携、绿色使用、放入 U 盘随身携带 |

::: tip Windows 7 用户
Windows 7 系统及部分精简版系统可能默认不包含 WebView2，导致配置界面无法打开。请前往 <https://developer.microsoft.com/zh-cn/microsoft-edge/webview2> 手动安装 WebView2 运行时。

部分版本会额外提供带有 `win7` 标识的软件包。若你使用 Windows 7 系统，请下载带该标识的版本。
:::

## 使用安装版

1. 下载 `ZoneDeck-<版本>-Setup.exe`。
2. 双击运行，按照向导完成安装。安装程序在安装前会**自动结束正在运行的核心进程**，避免文件占用。
3. 安装完成后会自动启动，并弹出配置界面。

安装程序会先问你装给谁：

- **仅为我安装**（默认，无需管理员权限）：装到 `%LocalAppData%\Programs\ZoneDeck`。
- **为所有用户安装**（需要管理员权限）：装到 `C:\Program Files\ZoneDeck`。

两种模式下设置都存在 `%APPDATA%\ZoneDeck`，不在安装目录里，见下方[数据存放位置](#数据存放位置)。安装包会在安装目录里放一个 `installed.marker` 文件，程序据它认出自己是安装版，请勿删除。

## 使用便携版

1. 下载 `ZoneDeck-<版本>-portable.zip`。
2. 解压到任意位置。压缩包内已包含一层 `ZoneDeck` 目录，解压后把它整个挪到你想放的位置即可。
3. 运行该目录中的 **`ZoneDeck.exe`**。首次运行会自动拉起配置界面。

解压后目录结构如下：

```
ZoneDeck/
├── ZoneDeck.exe      常驻核心（后台运行，负责隐藏窗口 / 热键监听）
├── config.exe        配置界面（按需打开，关闭即退出）
├── cleanup.ps1       残留数据清理脚本（见下方「卸载」）
├── LICENSE.txt       许可证文件
├── README.md         使用说明（简体中文）
├── README.en.md      使用说明（English）
└── README.zh-TW.md   使用说明（繁體中文）
```

::: warning 两个程序需放在同一目录
`ZoneDeck.exe` 与 `config.exe` 通过共用的 `config.json` 与命名管道协作，请勿将它们分开放置。
:::

## 数据存放位置

设置、日志、恢复文件与缓存放在同一个目录里，位置取决于你用的是哪个版本：

- **便携版**：就在**程序目录**里。拷走整个文件夹就带走了全部设置，这正是便携版该有的样子。
- **安装版**：在 **`%APPDATA%\ZoneDeck`**。安装目录可能是 `C:\Program Files`，普通权限写不进去，设置存在那里每次保存都会失败。

程序凭安装包放在程序目录里的 `installed.marker` 分辨自己是哪一种，请勿删除该文件。

::: warning 便携版放在了不可写的位置
若便携版所在目录写不进去（放在了 `C:\Program Files` 之类的地方，或只读介质上），程序会改用 `%APPDATA%\ZoneDeck` 并在配置界面弹出提示，说明是权限问题以及怎么处理。功能不受影响，只是设置不再跟着程序文件夹走。
:::

配置界面用到的浏览器组件另有一份数据在 `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`，两个版本都一样。

位置发生变化时（例如把便携版装成了安装版），原先的 `config.json` 会在首次启动时自动搬过去，你的绑定与热键都会保留。配置界面状态栏的**打开日志目录**按钮总是打开当前实际使用的那个目录。

## 关于两个可执行文件

ZoneDeck 采用 **核心 + 配置分离** 的双进程设计：

- **`ZoneDeck.exe`（核心）**：后台常驻，负责监听热键、隐藏 / 显示窗口。
- **`config.exe`（配置界面）**：仅在你需要修改设置时才打开，改完关闭即退出，不常驻内存。

日常使用中，你通常只会与托盘图标和配置界面打交道。核心会在后台默默运行。

## 更新

- **安装版**：重新运行新版本的安装程序即可覆盖更新；可在配置界面的 [检查更新](/guide/update) 中获取最新版下载链接。
- **便携版**：下载新版压缩包，覆盖旧文件即可（覆盖前请先退出核心程序）。

::: tip 从 v2.x升级
从v2版本升级时推荐先复制保留`config.json`和`pssuspend64.exe`后，现使用原卸载软件再运行新版安装程序，安装完成后再将`config.json`和`pssuspend64.exe`覆盖到新安装目录中。
:::

## 卸载

- **安装版**：通过系统"应用和功能"或安装目录中的卸载程序卸载。卸载时会一并删除日志、缓存、配置界面的浏览器数据等运行时产生的文件，并**询问是否保留配置文件**：选择保留则留下 `config.json`（重装后可继续使用），选择不保留则连同 `%APPDATA%\ZoneDeck` 一起删除。静默卸载不弹窗，默认保留配置。
- **便携版**：先退出核心程序，再运行目录中的 `cleanup.ps1` 清理用户目录下的残留，最后删除整个程序目录（设置就在里面，随目录一起删掉）。

便携版的清理命令（在程序目录中打开 PowerShell 执行）：

```powershell
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

脚本会先列出将要删除的内容并等你确认，随后清理 `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`、可能存在的 `%APPDATA%\ZoneDeck`（程序目录不可写时才有），以及开机自启留下的计划任务 `ZoneDeckAutostart` 和注册表项 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\ZoneDeck Application`。程序目录本身不会被删，跑完后自行删除即可。
