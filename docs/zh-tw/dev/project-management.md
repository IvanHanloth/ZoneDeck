---
title: 專案管理策略
---

# 專案管理策略

為了高效、安全地推進開發，ZoneDeck 對分支、合併與專案規劃有一套約定。

## 分支模型

儲存庫有兩個**主要分支**：

| 分支 | 作用 | 提交約束 |
| --- | --- | --- |
| **`main`** | 存放**正式發布版本**的原始碼 | **不可直接提交**。原則上只能在某版本功能開發完成、決定發布並通過測試後，從 `dev` 合併 |
| **`dev`** | 存放**所有正在開發**的功能原始碼 | **不可直接提交**。原則上只能包含已完成開發並通過測試的功能 |

其他分支（`feat/*`、`fix/*`、`doc/*` 等）用於日常功能開發與提交，見[分支命名](/zh-tw/dev/contributing)。

### 合併流向

```
feat/* · fix/* · doc/* ──PR──▶ dev ──PR（發版時）──▶ main
      （程式碼審查後合併）        （通過測試後合併）
```

- 新功能、Bug 修復等應透過 **Pull Request 提交至 `dev`**，經程式碼審查後合併。
- 累積若干功能、經過測試後，統一透過 PR 將 `dev` 合併至 `main`，並觸發發布。

::: tip 為什麼不直接提交主分支
`main`／`dev` 受保護可以避免未經審查的程式碼進入穩定線，保證任何時刻這兩個分支都是可建置、可發布的狀態。
:::

## Pull Request 規範

- 一個 PR 聚焦一件事，便於審查。
- PR 應通過 CI 的全部檢查（格式、Clippy、前端測試與建置、Rust 測試、release 編譯）。
- 關聯相關 Issue（如 `Closes #123`）與 Project。
- 至少經過一次**程式碼審查**再合併。

## Project 規範

專案通常使用 **GitHub 的 Project 功能**進行統一規劃管理。

::: info 三者關聯
應盡量將 **Issue – Pull Request – Project** 三者相互關聯，以實現統一管理：
- Issue 描述「要做什麼／出了什麼問題」；
- PR 實作並引用對應 Issue；
- Project 看板追蹤整體進度。
:::

## 版本與發布管理

- 版本號的**唯一真實來源**是 `Cargo.toml` 的 `[workspace.package] version`，其餘地方在建置時取自它。
- 發布透過 GitHub Actions 手動觸發的工作流程完成：寫入版本號 → 打 tag → 建置並發布 Release。
- 詳見 [打包與發布](/zh-tw/dev/release)。

## Issue 範本

儲存庫在 `.github/ISSUE_TEMPLATE/` 下提供了結構化範本：

- `bug-report.yml` —— Bug 回報；
- `feature-request.yml` —— 功能建議；
- `config.yml` —— Issue 入口設定。

使用範本能保證回報包含足夠的排查資訊。
