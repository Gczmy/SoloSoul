# AGENTS.md — SoloSoul Project Guide

> 本文件供 AI 编程助手阅读。读者应被假设对项目一无所知。
> 项目主要文档语言为**中文**（夹杂英文技术术语）。代码注释同样以中文为主。

---

## 项目概述

**SoloSoul（独灵）** 是一个本地优先、隐私优先的个人数字孪生与通用身份引擎。

- **核心理念**：「Centralized Schema definition, decentralized data storage」（集中式 Schema 定义，去中心化数据存储）
- **核心哲学**：「独奏生命数据，重塑数字原点」
- **架构定位**：所有敏感数据仅本地存储，绝不上传云端；采用零知识架构（Zero-Knowledge），服务端/开发者无法解密用户数据。

### 项目状态

| 组件 | 状态 | 说明 |
|------|------|------|
| **Tauri 客户端** | ✅ 主项目，活跃开发 | React + Tauri 跨平台客户端，macOS/Windows 已适配 |
| **Rust 原生核心** | ✅ 完整 | Argon2id + AES-256-GCM，通过 Tauri Commands 供前端调用 |
| **JS SDK** | ❌ 未开始 | `sdk/js/` 为空占位目录 |
| **Python SDK** | ❌ 未开始 | `sdk/python/` 为空占位目录 |

---

## 技术栈

| 组件 | 技术 | 备注 |
|------|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand, `@tauri-apps/api` | 状态管理以 Zustand 为主；UI 采用自定义 CSS Modules |
| Rust 核心（Tauri） | Rust 2021, `argon2`, `aes-gcm`, `rusqlite`, `tokio`, `ort` | 位于 `tauri/src-tauri/` 及 `tauri/crates/`，通过 Tauri Commands 暴露给前端 |
| 构建脚本 | Bash | 无 Makefile |

### 关键外部依赖

**Tauri 前端关键依赖：**
- `react` / `react-dom` — UI 框架
- `zustand` — 状态管理
- `@tauri-apps/api` / `@tauri-apps/plugin-*` — Tauri IPC 与系统插件
- `react-router-dom` — 路由
- `i18next` / `react-i18next` — 国际化
- `framer-motion` — 动画
- `lucide-react` — 图标

**Tauri Rust 关键依赖：**
- `tauri` v2 — 桌面应用框架
- `rusqlite` — SQLite 数据库
- `ort` — ONNX Runtime 本地 Embedding
- `reqwest` — HTTP 客户端（LLM 代理）
- `mdns-sd` — 本地同步 mDNS 发现

---

## 项目结构

```
SoloSoul/
├── tauri/                      # 主项目：Tauri + React 跨平台客户端
│   ├── src/                    # React 前端源码
│   │   ├── components/         # UI 组件
│   │   ├── hooks/              # React Hooks
│   │   ├── lib/                # 工具库（ipc, i18n, theme, api 等）
│   │   ├── stores/             # Zustand 状态管理
│   │   ├── types/              # TypeScript 类型
│   │   ├── App.tsx             # 应用根组件
│   │   └── main.tsx            # 前端入口
│   ├── src-tauri/              # Rust 后端（Tauri）
│   │   ├── src/
│   │   │   ├── commands/       # IPC 命令（25+ 个模块）
│   │   │   ├── core/           # 核心逻辑（SensitivityManager）
│   │   │   ├── db/             # SQLite 连接与迁移
│   │   │   ├── ipc/            # IPC 通信
│   │   │   ├── services/       # 业务服务（vault, llm_context）
│   │   │   ├── state/          # 应用状态
│   │   │   ├── local_embed.rs  # 本地 Embedding（ONNX）
│   │   │   ├── main.rs         # 入口
│   │   │   └── lib.rs          # 库入口
│   │   ├── crates/             # Workspace Crates
│   │   │   ├── solosoul-crypto/# 密码学（Argon2id + AES-256-GCM）
│   │   │   ├── solosoul-vault/ # Vault 存储接口与实现
│   │   │   └── solosoul-sync/  # 同步引擎（mDNS + Noise）
│   │   ├── resources/          # 打包资源（docs, ONNX models）
│   │   └── tauri.conf.json     # Tauri 应用配置
│   ├── scripts/                # 构建脚本（搜索索引等）
│   └── package.json            # npm 依赖
│
├── SoloSoul_plugin_market/     # 插件市场子模块（独立 Git 仓库）
│   ├── plugins/
│   ├── SDK/
│   └── ...
│
├── sdk/                        # SDK 占位目录（未实现）
│   ├── js/
│   └── python/
│
├── docs/                       # 项目文档（中文为主）
│   ├── TODO.md                 # 开发任务清单（P0–P7 优先级系统）
│   ├── USER_GUIDE.md           # 终端用户指南
│   ├── CLIENT_ROADMAP.md       # 客户端技术架构路线图
│   ├── CLIENT_USER_GUIDE.md    # 客户端专用指南
│   ├── CHANGELOG.md            # 版本历史（SemVer）
│   ├── WORKLOG.md              # 事件日志
│   ├── PRIVACY_POLICY.md       # 隐私政策
│   └── TERMS_OF_SERVICE.md     # 服务条款
│
└── .github/workflows/          # GitHub Actions CI/CD
    ├── ci_cd.yml               # 完整流水线：前端检查 / Rust 测试 / Tauri 构建 / Release
    └── pr_check.yml            # PR 快速反馈：Format + Clippy + TypeScript 检查 + 测试
```

---

## 构建命令

### Tauri 客户端（主项目）

```bash
cd tauri

# 安装依赖
npm install

# 开发模式
npm run dev

# 代码检查（TypeScript + Rust fmt + Clippy + Lint + Test）
npm run check-all

# Release 构建
npm run tauri build
```

**注意事项：**
- macOS 构建需要 Xcode Command Line Tools。
- Release 产物位于 `tauri/src-tauri/target/release/bundle/`。

### Rust Workspace（Tauri）

```bash
cd tauri

# 构建 Release
cargo build --release

# 测试
cargo test --verbose

# 格式化与静态检查
cargo fmt --check
cargo clippy -- -D warnings
```

---

## 测试指令

### Tauri 测试

```bash
cd tauri

# 前端单元测试（Vitest）
npm run test

# Rust 单元测试
cargo test --verbose

# 端到端测试（Playwright）
npm run test:e2e
```

**Tauri 测试结构：**
- `src/**/*.test.ts(x)` — Vitest 单元测试（前端逻辑、组件）。
- `src-tauri/src/` 中的 `#[cfg(test)]` — Rust 单元测试。
- `e2e/` — Playwright 端到端测试（应用启动、导航、核心流程）。

---

## 安全架构与约定

### 零知识安全模型

1. **主密码从不存储** — 仅用于内存中的密钥派生（Argon2id），使用后立即安全擦除。
2. **Salt 与验证令牌** — 存储在 `~/.solosoul/{account_id}/config.json`，验证令牌是 `"SOLOSOUL_VAULT_V1"` 的加密密文，用于验证密码正确性而不存储密码哈希。
3. **内存安全** — `crypto.SecureWipe` + `runtime.SetFinalizer` 自动清零；`Lock()` 擦除派生密钥。
4. **文件权限** — 目录 `0700`，文件 `0600`。
5. **常数时间比较** — `subtle.ConstantTimeCompare` 防止时序攻击。
6. **会话管理** — Token 24 小时过期；基于 `time.Now().UnixNano()` 生成 32 字节随机值。
7. **插件授权** — 字段级显式授权（Consent），支持撤销与会话过期。

### 加密参数

| 模式 | Memory | Iterations | Parallelism | 适用场景 |
|------|--------|------------|-------------|---------|
| 开发模式（默认） | 8 MiB | 2 | 4 | 本地开发、Apple Silicon 避免挂起 |
| 生产模式（`SOLOSOUL_SECURE=1`） | 64 MiB | 3 | 4 | OWASP 推荐 |

### 敏感数据分级

Tauri 前端所有字段均有 `SensitivityLevel`：
- `public` — 公开
- `internal` / `private` — 内部（组件自动掩码）
- `sensitive` / `restricted` / `critical` — 敏感/受限/关键（需密码重新验证，1 分钟缓存）

**约定：** 必须使用共享组件 `SensitiveValueWidget` / `SensitivityBlurredWidget` / `SensitivityTag`，禁止在各页面自行实现掩码逻辑。

---

## 代码风格与开发约定

### 语言与注释

- **文档和注释以中文为主**，技术术语保留英文（如 Argon2id、AES-256-GCM、FFI、Vault）。
- 部分代码含中英混合注释（如 `唯一真理来源`、`双重加密`）。

### Tauri / React 前端

- **状态管理**：Zustand。避免直接混用 `useState` 进行跨页面状态共享。
- **密码验证对话框**：统一使用 `src/components/PasswordVerificationDialog.tsx`，禁止多处复制对话框代码。
- **防抖保存**：Profile 修改采用 500ms debounce，关键操作可强制立即保存。
- **操作日志**：每次 CRUD 生成 `OperationEntry`，含 before→after 差异描述，支持 30 天软删除后永久清理。
- **自动锁定**：监听窗口焦点变化与系统休眠事件，超时锁定 Vault 并擦除敏感状态。
- **IPC 调用**：前端通过 `invoke` 调用 Rust Commands，禁止直接操作文件系统或网络。

---

## 插件市场子模块（Git Submodule）提交规则

`SoloSoul_plugin_market/` 是独立 Git 仓库，作为主项目的子模块引用。其工作流已优化为**本地预生成 + CI 验证**模式。

### 修改插件时的正确流程

```bash
cd SoloSoul_plugin_market

# 1. 首次克隆后安装 Git Hooks（只需一次）
bash scripts/install-hooks.sh

# 2. 修改插件源码 + manifest.json
# 3. 编译：cargo build --target wasm32-wasip1 --release
# 4. 复制产物：cp target/wasm32-wasip1/release/*.wasm plugin.wasm

# 5. 回到子模块根目录，提交（pre-commit hook 会自动生成 registry.json）
cd ../..
git add -A
git commit -m "feat(xxx): 描述"
git push origin main
```

### 关键规则

| 规则 | 说明 |
|------|------|
| **必须本地生成 `registry.json`** | 修改插件后运行 `python3 scripts/generate_registry.py`，将变更随代码提交 |
| **已安装 hooks 则自动完成** | `bash scripts/install-hooks.sh` 后，提交 `plugins/` 变更时自动生成 |
| **CI 会验证一致性** | `validate-registry.yml` 在 push/PR 时做 diff 检查，不一致则 ❌ 失败 |
| **禁止远程自动提交** | 旧方案中 CI 自动 commit/push `registry.json` 已被移除，不再产生"远程自动修改" |
| **手动触发仅作兜底** | `update-registry.yml` 保留 `workflow_dispatch`，仅维护者紧急重建时使用 |

### CI 失败修复

若 `validate-registry.yml` 报告 `registry.json` 不一致：

```bash
cd SoloSoul_plugin_market
python3 scripts/generate_registry.py
git add registry.json

# PR 分支推荐：修正当前 commit
git commit --amend --no-edit
git push --force-with-lease

# 或新增修复 commit
git commit -m "chore: update registry.json"
git push
```

---

## CI/CD

GitHub Actions 覆盖 **Tauri 前端 + Rust Workspace + Release 构建**。

### ci_cd.yml（Push 到 master/main 或 PR）

1. **frontend-check**（ubuntu-latest）— TypeScript 类型检查、ESLint、Vitest 单元测试。
2. **rust-test**（ubuntu-latest）— `tauri/` 的 `cargo test` + `cargo fmt --check` + `cargo clippy`。
3. **build-macos**（macos-latest，仅 master push）— Tauri Release 构建、DMG 打包。
4. **build-windows**（windows-latest，仅 master push）— Tauri Release 构建、MSI 打包。
5. **release**（ubuntu-latest，仅 master push）— Draft Pre-release 发布。

### pr_check.yml（PR 快速反馈）

1. **frontend-check** — `tsc --noEmit` + `npm run lint` + `npm run test`。
2. **rust-check** — `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`。

### CI 版本号

- Node.js: `22`
- Rust: `stable`

---

## 已知问题与陷阱

### 1. Apple Silicon Argon2 性能问题
Rust `argon2` crate 在 macOS ARM64 上开发环境默认使用 8MiB / 2 iterations，避免挂起。
- 生产环境通过 `SOLOSOUL_SECURE=1` 切换至 64MiB / 3 iterations。

### 2. SDK 占位
`sdk/js/` 和 `sdk/python/` 为空目录，任何 SDK 相关需求需从零开始实现。

---

## 常用文件速查

| 目的 | 路径 |
|------|------|
| Tauri 前端入口 | `tauri/src/main.tsx` |
| Tauri 应用根组件 | `tauri/src/App.tsx` |
| Tauri IPC 封装 | `tauri/src/lib/ipc.ts` |
| Tauri Vault 服务 | `tauri/src-tauri/src/services/vault_service.rs` |
| Tauri SensitivityManager | `tauri/src-tauri/src/core/sensitivity_manager.rs` |
| Tauri Auth 命令 | `tauri/src-tauri/src/commands/auth.rs` |
| Tauri Profile 命令 | `tauri/src-tauri/src/commands/profile.rs` |
| Tauri 对象命令 | `tauri/src-tauri/src/commands/unified_object.rs` |
| Tauri 数据库模块 | `tauri/src-tauri/src/db/` |
| Tauri 本地 Embedding | `tauri/src-tauri/src/local_embed.rs` |
| Rust crypto crate | `tauri/src-tauri/crates/solosoul-crypto/` |
| Rust vault crate | `tauri/src-tauri/crates/solosoul-vault/` |
| Rust sync crate | `tauri/src-tauri/crates/solosoul-sync/` |
| 任务清单 | `docs/TODO.md` |
| 用户指南 | `docs/USER_GUIDE.md` |
| 技术路线图 | `docs/CLIENT_ROADMAP.md` |
| 事件日志 | `docs/WORKLOG.md` |
