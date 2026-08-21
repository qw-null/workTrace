<div align="center">

# 工作日迹 WorkTrace

**记录、回顾、汇报，一条工作台全搞定。**

本地优先、AI 辅助的个人工作日志与周报工具（macOS / Windows 桌面应用）。

</div>

<div align="center">

[![最新版本](https://img.shields.io/github/v/release/qw-null/workTrace?label=最新版本&color=378ADD)](https://github.com/qw-null/workTrace/releases)
[![平台 badge](https://img.shields.io/badge/平台-macOS%20%7C%20Windows-378ADD)](https://github.com/qw-null/workTrace/releases/latest)
[![自动更新](https://img.shields.io/badge/更新-在线自动更新-success)](https://github.com/qw-null/workTrace/releases/latest)
[![开源](https://img.shields.io/badge/开源-是-success)]()
<br/>
**☕ 喜欢这个工具？** [![给个 Star](https://img.shields.io/github/stars/qw-null/workTrace?style=social&label=Star)](https://github.com/qw-null/workTrace)

</div>

<br/>

## 📥 下载安装

> 前往 GitHub Releases 下载 **最新版安装包**，支持 macOS 与 Windows，在线自动更新。

<div align="center">

| 平台 | 安装包 | 架构 |
|---|---|---|
| 🍎 macOS | [⬇️ 下载 .dmg](https://github.com/qw-null/workTrace/releases/latest) | Apple Silicon (aarch64) |
| 🪟 Windows | [⬇️ 下载 .exe](https://github.com/qw-null/workTrace/releases/latest) | x64 |

**软件支持在线自动更新**：发现新版本后一键下载安装并重启，无需手动折腾。

</div>

<br/>

---

## ✨ 一句话流程

**统一输入框记录 → AI 转为结构化记录（时间/工作内容/进度结果/相关人员/备注下一步）→ 热力图 / 日历可视化回顾 → 一键生成周报 → WebDAV 云盘同步备份**，记录不丢失、多设备可同步。

---

<br/>

## 功能特性

- **每日记录**：WorkBuddy 式统一输入框，文字直接键入，图片 / 文件靠拖拽、粘贴（⌘V）或框内图标添加；模型选择器一键切换已配置的大模型，AI 自动转为结构化记录（摘要 / 任务 / 标签 / 成果 / 流程图 / 待办）。
- **附件解析**：Word（.docx）、PDF 本地提取文本；图片走多模态视觉模型识别内容与其中的流程图（Mermaid）。
- **周报生成**：汇总本周记录，AI 生成四板块周报；Markdown 默认，可导出 Word（.doc）/ PDF（.html 打印）。
- **可视化回顾**：GitHub 风格年度热力图 + 月历，双向联动；点击日期弹出当日详情（含 Mermaid 流程图渲染）。
- **大模型配置**：OpenAI 兼容协议 + 本地 Ollama；支持多模型，可指定用途（记录转化 / 周报生成 / 图片识别），带连通性测试。
- **备份同步**：WebDAV（坚果云 / InfiniCLOUD / 其他自建）；**支持多个备份账号**，可分别测试连通性、随时切换当前使用账号；自动适配 Basic 与 Digest 认证；仅备份结构化记录与周报，附件（一次性原料）不备份。

## 技术栈

Tauri 2（Rust + React 18 + TypeScript + Vite）· 文件化 JSON 存储 · macOS Keychain（密钥托管）· reqwest（LLM / WebDAV）· Mermaid（流程图）· 配色清新蓝白（主色 `#378ADD`）。

## 快速开始

```bash
# 前置：Rust（rustup，国内可配 rsproxy 镜像）+ Node 20+ + Xcode Command Line Tools
npm install
npm run tauri dev      # 开发运行（启动 Vite + 编译 Rust + 弹出应用窗口）
npm run tauri build    # 打包 .app / .dmg
```

## 数据存储

默认根目录 `~/Library/Application Support/WorkTrace/`：

| 目录 / 文件 | 说明 |
|---|---|
| `records/YYYY/MM/YYYY-MM-DD.json` | 每日记录（结构化 JSON） |
| `reports/` | 周报 Markdown 与导出文件 |
| `settings/app.json` | 应用配置（大模型、备份账号、同步设置；**密钥不入此文件**） |

- 密钥（大模型 API Key、WebDAV 密码）存 macOS Keychain，失败时降级为本地明文（保证功能可用）。
- 附件为一次性原料：仅交 AI 转化，本地临时保留、按策略自动清理，**不参与备份**。

## 目录结构

```
workTrace/
├── docs/                      # 文档：PRD、开发文档、原型
├── src-tauri/                 # Rust 后端
│   └── src/                   # lib.rs（命令注册/生命周期）+ storage/ai/parser/sync/report/settings
├── src/                       # Web 前端
│   ├── views/                 # Dashboard（工作台）/ Report（周报）/ Settings（设置）
│   ├── components/            # InputBox / Heatmap / Calendar / DetailModal / FlowDiagram
│   ├── api/                   # Tauri invoke 封装
│   └── types.ts               # 与后端对齐的 TS 类型
└── package.json
```

## 文档

| 文档 | 说明 |
|---|---|
| [docs/需求规格说明书.md](docs/需求规格说明书.md) | 需求基线（PRD），功能迭代先改这里 |
| [docs/开发文档.md](docs/开发文档.md) | 开发总纲：架构、模块、数据模型、Prompt 规范、里程碑 |
| [docs/原型设计.html](docs/原型设计.html) | 可交互界面原型（浏览器打开） |
