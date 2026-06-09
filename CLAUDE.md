# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

SoloSoul (独灵) is a **Local Digital Twin & Universal Identity Engine** - a decentralized, local-encrypted personal super-profile and automation execution engine.

**Core Philosophy**: "Centralized Schema definition, decentralized data storage"

## 项目架构

```
SoloSoul/
├── tauri/            # 主项目：Tauri + React 跨平台客户端
│   ├── src/          # React 前端源码
│   ├── src-tauri/    # Rust 后端 (Tauri)
│   │   ├── src/
│   │   │   ├── commands/   # IPC 命令
│   │   │   ├── core/       # 核心逻辑
│   │   │   ├── db/         # SQLite 数据库
│   │   │   ├── services/   # 业务服务
│   │   │   └── ...
│   │   └── crates/         # Workspace crates (crypto, vault, sync)
│   └── package.json
├── SoloSoul_plugin_market/  # 插件市场（Git Submodule）
├── sdk/              # SDK 占位目录（未实现）
│   ├── js/
│   └── python/
└── docs/             # 文档
```

## 安全要求 (Zero-Knowledge)

- Master Password **从不存储** - 仅在内存中用于密钥派生
- Salt 存储在 `~/.solosoul/{account_id}/config.json` 用于密钥验证
- 敏感字段使用后**销毁** (secure zeroing)
- 外部插件访问需要用户**显式授权** (Consent Manager)
- Session tokens 24小时过期
- 数据仅本地存储于 `~/.solosoul/`，**绝不上传云端**
- 每个账户独立加密在 `~/.solosoul/acc_xxx123/`

## 开发命令

### Tauri 客户端

```bash
cd tauri

# 开发模式
npm run dev

# 代码检查
npm run check-all

# Release 构建
npm run tauri build
```

## 技术栈

| 组件 | 技术 |
|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand |
| Rust 核心 | Rust, Argon2id, AES-256-GCM, rusqlite |

## Tauri 项目结构

```
tauri/src/
├── App.tsx, main.tsx
├── lib/              # 工具库 (ipc, i18n, theme, api)
├── stores/           # Zustand 状态管理
├── hooks/            # React Hooks
├── components/       # UI 组件
└── types/            # TypeScript 类型

tauri/src-tauri/src/
├── main.rs, lib.rs   # 入口
├── commands/         # IPC 命令 (25+)
├── core/             # 核心逻辑 (SensitivityManager)
├── db/               # SQLite 连接与迁移
├── ipc/              # IPC 通信
├── services/         # 业务服务 (vault, llm_context)
└── state/            # 应用状态
```

## 代码规范

- 使用 TypeScript 严格模式
- 状态管理使用 Zustand
- UI 组件使用纯 CSS Modules + 全局 CSS（不使用 Tailwind）
- 所有受保护页面为 Client Component，权限检查通过 Zustand sessionToken 手动重定向
- Rust 代码遵循标准 fmt + clippy
- 错误处理使用预定义错误变量

## 注意事项

1. SDK 占位：`sdk/js/` 和 `sdk/python/` 为空目录，任何 SDK 相关需求需从零开始实现。
2. 插件市场子模块：`SoloSoul_plugin_market/` 是独立 Git 仓库，修改时需遵循其 Git Hooks 流程。
3. Apple Silicon Argon2 性能：开发环境默认 8MiB / 2 iterations，生产环境通过环境变量切换至 64MiB / 3 iterations。
