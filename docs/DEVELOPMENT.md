# SoloSoul 开发者文档

> 本文档面向 SoloSoul 的开发者与贡献者，涵盖项目结构、技术栈、开发命令与构建发布流程。
> 产品介绍与功能说明见 [README](../README.md)。

---

## 项目架构

```
SoloSoul/
├── tauri/                     # 主项目：Tauri + React 跨平台客户端
│   ├── src/                   # React 前端 (TypeScript, Zustand, CSS Modules)
│   │   ├── components/        # UI 组件
│   │   ├── pages/             # 页面
│   │   │   ├── ai/            # AI 对话
│   │   │   ├── auth/          # 启动/登录
│   │   │   ├── editor/        # 对象/模板编辑器
│   │   │   ├── scan/          # OCR/本地扫描
│   │   │   ├── settings/      # 设置页
│   │   │   ├── system/        # 关于/调试
│   │   │   ├── workspace/     # 对象工作区
│   │   │   └── ...
│   │   ├── stores/            # Zustand 状态管理
│   │   ├── lib/               # 工具库 (i18n, IPC, theme)
│   │   ├── hooks/             # React Hooks
│   │   ├── locales/           # 国际化 (en-US / zh-CN)
│   │   ├── types/             # TypeScript 类型
│   │   └── styles/            # 全局 CSS
│   └── src-tauri/             # Rust 后端 (Tauri)
│       ├── src/
│       │   ├── commands/      # IPC 命令 (30+)
│       │   ├── core/          # 核心逻辑 (SensitivityManager, etc.)
│       │   ├── db/            # SQLite 数据库 + 迁移
│       │   ├── ipc/           # IPC 通信
│       │   ├── services/      # 业务服务 (vault, llm_context, sync)
│       │   └── state/         # 应用状态
│       └── crates/            # Workspace crates
├── solosoul_cli/              # 独立终端 CLI (TUI)
├── sdk/                       # SDK（js / python）
├── SoloSoul_plugin_market/    # 插件市场 (Git Submodule)
├── docs/                      # 文档（设计地图、构建脚本、用户指南、法律文件）
└── CHANGELOG.md               # 变更日志
```

---

## 技术栈

| 组件 | 技术 |
|------|------|
| 前端框架 | React 19, TypeScript, Vite |
| 状态管理 | Zustand |
| 样式 | CSS Modules + 全局 CSS 变量 |
| 国际化 | i18next + react-i18next |
| 后端框架 | Tauri v2 (Rust) |
| 数据库 | SQLite (rusqlite) |
| 密码学 | Argon2id, AES-256-GCM |
| OCR | PP-OCRv6 (ONNX Runtime) |

---

## 开发命令

### Tauri 客户端（主项目）

```bash
cd tauri

# 安装依赖
npm install

# 开发模式
npm run dev

# 代码检查 (TypeScript + Rust fmt + Clippy + Lint + Test)
npm run check-all

# Release 构建
npm run tauri build
```

### SoloSoul CLI

```bash
cd solosoul_cli

# 构建 Debug
cargo build

# 运行 TUI
cargo run

# 测试 / 格式化 / 静态检查
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

### 测试

```bash
cd tauri

# 前端单元测试（Vitest）
npm run test

# Rust 单元测试
cargo test --verbose

# 端到端测试（Playwright）
npm run test:e2e
```

---

## 构建与发布

完整发布流程（版本号同步、三平台构建、签名、GitHub Release）见 [`release_process.md`](release_process.md)。

```bash
# macOS / Windows 一键构建
./scripts/build_macos_release.sh
./scripts/build_windows_release.sh

# Android Release APK（需设置签名环境变量）
cd tauri
cargo tauri android build --apk
```

> 构建前需确保本地资源文件（ONNX 模型、PDFium 动态库）存在，详见 release_process.md「本地资源文件」章节。

---

## 文档索引

| 文档 | 说明 |
|------|------|
| [设计地图](design_map/) | 架构设计文档（技术选型、对象规范、IPC 规范等） |
| [插件市场](plugin_market/) | 插件系统技术文档（SDK、宿主、市场结构） |
| [产品哲学](manifesto/) | 产品使命与设计哲学 |
| [CLI 用户指南](solosoul_cli/USER_GUIDE.md) | 终端客户端使用说明 |
| [移动端平台](platform-mobile/) | 移动端现状总览与 Android 开发环境搭建 |
| [同步路线图](sync-roadmap.md) | 设备同步功能路线图 |
| [发布流程](release_process.md) | 版本发布构建流程 |
| [代码审查流程](review_code_process.md) | 代码审查流程 |
| [WASM 插件开发指南](wasm-plugin-development-guide.md) | WASM 插件开发 |
