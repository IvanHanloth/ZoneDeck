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
        <a href="https://boss-key.ivan-hanloth.cn/zh-tw/">專案官網</a>
        <span> • </span>
        <a href="https://boss-key.ivan-hanloth.cn/zh-tw/guide/">使用說明</a>
        <span> • </span>
        <a href="https://boss-key.ivan-hanloth.cn/zh-tw/dev/">開發文件</a>
        <span> • </span>
        <a href="https://github.com/IvanHanloth/Boss-Key/releases">下載位址</a>
    </h3>
</div>

<p align="center">
    <a href="/README.md">简体中文</a>
    <span> • </span>
    <a href="/README.en.md">English</a>
    <span> • </span>
    <strong>繁體中文</strong>
</p>

<div align="center">
    <strong>老闆來了？快用 ZoneDeck 老闆鍵一鍵隱藏視窗！上班摸魚必備神器。</strong><br>

支援多視窗隱藏、多程序隱藏、自訂快速鍵、隱藏使用中視窗、靜音視窗、暫停影片播放、程序凍結等超多功能。

超高自訂程度，滿足您的不同隱藏需求。極簡記憶體，背景常駐僅 1M 記憶體佔用。

</div><br>

## 應用程式螢幕擷取畫面

![ZoneDeck 設定視窗綁定頁](/docs/public/static/screenshot-1.png)

![ZoneDeck 設定滑鼠快速鍵設定頁-1](/docs/public/static/screenshot-2.png)

![ZoneDeck 設定滑鼠快速鍵設定頁-2](/docs/public/static/screenshot-3.png)

![ZoneDeck 設定視窗其他選項頁](/docs/public/static/screenshot-4.png)

## 使用說明

從 v3.0.0 版本開始，每個版本都會提供兩種類型的程式，可以從 [Release 頁面](https://github.com/IvanHanloth/Boss-Key/releases) 下載：

- installer - 安裝程式（建議），完整封裝的 ZoneDeck 程式安裝程式，提供一鍵安裝、更新、解除安裝，可以更有效率且安全地管理 ZoneDeck 程式
- portable - 可攜版，包含 ZoneDeck 的核心程式和設定程式的壓縮檔，解壓縮後可以執行

部分版本會提供 Windows 7 系統的軟體包，帶有 win7 標示的可以在 Windows 7 系統上執行。

完整的圖文使用說明，請參閱 ZoneDeck [使用說明](https://boss-key.ivan-hanloth.cn/zh-tw/guide/)。

### 基礎使用

安裝或更新後首次開啟 ZoneDeck，會自動開啟設定頁面，可以在其中進行快速鍵修改、程序及視窗綁定等操作。

而一般使用時，可以透過以滑鼠右鍵按一下通知區域圖示開啟選單。按一下選單中的「設定」即可開啟設定頁面。

以滑鼠右鍵按一下通知區域圖示還有結束程式、檢查更新、設定開機自動啟動等功能。

按下隱藏／顯示視窗快速鍵可以一鍵隱藏所綁定的視窗。按下關閉核心快速鍵可以一鍵關閉 ZoneDeck 程式。

### 綁定視窗

透過綁定視窗，可以同時隱藏多個視窗，摸魚更安全～

設定視窗中上方部分，左邊清單是目前存在的視窗，右邊清單是已經綁定的視窗。

在左邊清單中選取希望隱藏的視窗，按一下「新增」可以將視窗資訊加入右邊。同理，在右邊清單中選擇不需要綁定的規則，按一下「移除」即可刪除。

如果發現新開啟的視窗沒有在清單中顯示，可以按一下「重新整理」按鈕，重新整理左邊的清單。

### 修改快速鍵

按一下「錄製」按鈕，開啟快速鍵錄製視窗進行錄製；按下的組合鍵將被記錄並自動填入。按一下「清除」可清空該快速鍵。

### 滑鼠隱藏

ZoneDeck 支援以滑鼠中鍵、側鍵 1、側鍵 2 切換隱藏狀態，並可搭配連按次數與輔助按鍵。

也可以啟用快速移動滑鼠至四角隱藏視窗（啟用角落復原功能以允許透過快速移動滑鼠至四角復原視窗）。

### 介面語言

設定介面與核心的通知區域選單、通知支援**簡體中文、English、繁體中文**。預設跟隨系統顯示語言，也可在「其他選項 → 語言」中手動指定。

### 更多功能

完整功能介紹及使用指南，請參閱 ZoneDeck [使用說明](https://boss-key.ivan-hanloth.cn/zh-tw/guide/)。

## 資料存放位置與清理

**可攜版**把設定、記錄檔、復原檔與快取放在**程式資料夾裡**，複製走整個資料夾就帶走了全部設定。若該資料夾不可寫入（放在了 `C:\Program Files` 之類的地方，或唯讀媒體上），程式會改存到 `%APPDATA%\ZoneDeck` 並在介面上說明原因。

**安裝版**一律存到 `%APPDATA%\ZoneDeck`：安裝資料夾可能在 `C:\Program Files`，一般權限寫不進去。程式憑安裝程式放的 `installed.marker` 分辨自己是哪一種。

無論哪種，設定介面用到的瀏覽器元件另有一份資料在 `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`，刪程式資料夾清不掉它。隨附有 `cleanup.ps1`，在程式資料夾裡開啟 PowerShell 執行即可：

```powershell
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

它會列出將要刪除的內容並等你確認，隨後清理 `%LOCALAPPDATA%\cn.hanloth.zonedeck.config`、可能存在的 `%APPDATA%\ZoneDeck`，以及開機自動啟動留下的排程工作 `ZoneDeckAutostart` 與登錄項目 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\ZoneDeck Application`。程式資料夾本身不會被刪，跑完後自行刪除即可。

安裝版不需要這一步：解除安裝程式已經做了同樣的事，並會詢問是否保留設定檔。

## 開發及貢獻指南

有關開發和貢獻的詳細資訊，請參閱 ZoneDeck [開發文件](https://boss-key.ivan-hanloth.cn/zh-tw/dev/)。

## 鳴謝

感謝雪藏 HsFreezer 提供的程序凍結實作思路。

## 更新日誌

完整的更新日誌請參閱 ZoneDeck [更新日誌](https://boss-key.ivan-hanloth.cn/zh-tw/changelog/)。
