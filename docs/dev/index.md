---
title: 开发文档导览
---

# 开发文档

欢迎参与 ZoneDeck 的开发！本部分文档面向**开发者与贡献者**，介绍如何在本地运行项目代码、参与贡献的要求、项目管理策略，以及系统架构。

::: tip 
如果你只是想使用 ZoneDeck，请阅读 [使用文档](/guide/)。
:::

## 技术栈概览

v3 版本是一次彻底的重构，核心技术选型如下：

| 部分 | 技术 |
| --- | --- |
| 常驻核心 | **Rust**（纯原生，Windows API 直调） |
| 配置界面后端 | **Tauri 2**（Rust） |
| 配置界面前端 | **Svelte 5 + Vite** |
| 进程间通信 | **命名管道**（一行一条 JSON） |
| 工程组织 | **Cargo workspace** + npm 前端子工程 |
| 打包 | PowerShell 脚本 + Inno Setup |
| CI/CD | GitHub Actions |

## 设计目标

v3 重写的核心目标：

- **更低内存**：核心常驻二进制约 350 KB，后台内存约 1 MB。
- **更稳**：崩溃日志 + 崩溃恢复 + 看门狗三层防线。
- **单文件原生二进制**：不依赖 Python 运行时，降低杀软误报。
- **更现代的配置界面**：无边框、主题切换、配置自动保存。

## 阅读顺序建议

1. [本地运行](/dev/getting-started) —— 环境准备与常用命令，先把项目跑起来。
2. [系统架构](/dev/architecture) —— 理解双进程 + 命名管道的整体设计。
3. [前端与配置界面](/dev/frontend) —— Svelte / Tauri 部分的选型与结构。
4. [贡献指南](/dev/contributing) 与 [项目管理策略](/dev/project-management) —— 参与协作前必读。
5. [测试策略](/dev/testing) 与 [打包与发布](/dev/release) —— 保证质量与交付。

参考资料：[配置文件字段](/dev/config-reference)、[IPC 协议](/dev/ipc-protocol)。

## 仓库地址

<https://github.com/IvanHanloth/Boss-Key>
