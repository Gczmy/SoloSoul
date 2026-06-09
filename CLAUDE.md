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
├── web/              # 遗留项目：Next.js Web UI (维护模式)
├── cmd/              # Go 后端服务
│   ├── solosould/    # HTTP API 服务器
│   └── solosoul/     # CLI 工具
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

### Go 后端

```bash
# 构建 (需要 Rust)
go build -tags "rust,cgo" ./...

# 构建 (纯 Go)
go build ./...

# 运行
./solosould
```

### Web

```bash
cd web
npm install
npm run dev
```

## 技术栈

| 组件 | 技术 |
|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand |
| Rust 核心 | Rust, Argon2id, AES-256-GCM |
| Go 后端 | Go, 标准库 net/http |
| Web UI | Next.js 15, React, Zustand |

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
- 错误处理使用预定义错误变量，跨包传递
- Vault 操作使用 `sync.Mutex` 保护

## 注意事项

1. Go 后端编译必须带 tags: `go build -tags "rust cgo" ./...`
2. Rust 密码学库有两套实现，注意修改时确认目标平台
3. Web UI 部分 API Routes 为存根，客户端通常直接调用 Go 后端
