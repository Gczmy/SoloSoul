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
| **Rust 原生核心** | ✅ 完整 | Argon2id + AES-256-GCM，通过 Tauri Commands / FFI 供前端/Go 调用 |
| **Go 后端** | ✅ 功能完整，维护模式 | HTTP API 服务器 + CLI 工具；最小依赖设计 |
| **Web UI** | ⚠️ 遗留项目，维护模式 | Next.js 15 实现，无测试覆盖，无 CI 集成 |
| **JS SDK** | ❌ 未开始 | `sdk/js/` 为空占位目录 |
| **Python SDK** | ❌ 未开始 | `sdk/python/` 为空占位目录 |

---

## 技术栈

| 组件 | 技术 | 备注 |
|------|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand, `@tauri-apps/api` | 状态管理以 Zustand 为主；UI 采用自定义 CSS Modules |
| Rust 核心（Tauri） | Rust 2021, `argon2`, `aes-gcm`, `rusqlite`, `tokio`, `ort` | 位于 `tauri/src-tauri/` 及 `tauri/crates/`，通过 Tauri Commands 暴露给前端 |
| Rust 核心（Go） | Rust 2021, `argon2`, `aes-gcm`, `rand` | 位于 `crypto-argon2/`，构建为 `staticlib/cdylib` |
| Go 后端 | Go 1.26.1, 标准库 `net/http`（Go 1.22 pattern mux） | **注意：不使用 Gin 框架** |
| Web UI | Next.js 15.1.0, React 19, TypeScript 5.7, Zustand 5 | 不使用 Tailwind，使用纯 CSS Modules + 全局 CSS |
| 构建脚本 | Bash (`build_rust.sh`, `validate_ffi.sh`) | 无 Makefile |

### 关键外部依赖

**Go（极度精简，仅 2 个直接依赖）：**
- `golang.org/x/term v0.41.0` — 安全密码输入
- `google.golang.org/protobuf v1.36.11` — 插件管理相关转换辅助

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
├── web/                        # 遗留项目：Next.js Web UI（维护模式）
│   ├── app/                    # App Router
│   │   ├── (auth)/             # 受保护路由组：dashboard, home, ocr, plugins, profile, settings, vault
│   │   ├── (public)/           # 公开路由组：login, setup
│   │   └── api/                # Next.js API Routes（代理层，部分实现）
│   ├── components/             # 共享组件
│   ├── lib/
│   │   ├── api.ts              # 前端直接调用 Go 后端的 fetch 封装
│   │   └── store.ts            # Zustand 全局状态（localStorage 持久化）
│   └── styles/                 # 全局 CSS（Design Tokens + OKLCH 色彩）
│
├── cmd/                        # Go 后端入口
│   ├── solosoul/               # CLI 客户端（操作本地 Vault）
│   └── solosould/              # HTTP API 守护进程（默认 :8080）
│
├── core/                       # Go 业务核心库
│   ├── api/                    # HTTP REST + gRPC 风格 API、账户管理、插件管理
│   ├── crypto/                 # 密码学：AES-256-GCM、Argon2 KDF、安全内存擦除
│   ├── vault/                  # 加密文件存储接口与实现
│   ├── schema/                 # Profile 数据模型与验证器
│   └── ocr/                    # OCR 引擎（PaddleOCR 封装）、MRZ 解析、图像预处理
│
├── crypto-argon2/              # Rust 密码学库（供 Go 后端 FFI 调用）
│   └── src/lib.rs              # 导出 Argon2id + AES-256-GCM 的 C FFI 接口
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
    ├── ci_cd.yml               # 完整流水线：Rust 测试 / 前端检查 / Tauri 构建 / Release
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

### Go 后端

```bash
# ⚠️ 重要：Go 后端必须配合 Rust 静态库编译，没有纯 Go 回退实现！
# 先构建 crypto-argon2：
./build_rust.sh

# 然后构建 Go（必须带 rust + cgo tags）
go build -tags "rust cgo" ./...

# 运行 HTTP 守护进程
./solosould
# 或指定地址
./solosould -addr :8080

# 运行 CLI 客户端
./solosoul
```

**环境变量：**
- `SOLOSOUL_SECURE=1` — 启用生产级 Argon2 参数（64MiB / 3 iterations / 4 parallelism）。**开发环境（尤其 Apple Silicon）默认 8MiB / 2 iterations，避免挂起。**
- `SOLOSOUL_USE_RUST=1` — 显式启用 Rust KDF 实现（当前默认已通过 `rust` build tag 启用）。
- `SOLOSOUL_VAULT_PATH` — 覆盖 Vault 存储路径（默认 `~/.solosoul`）。

### Rust 密码学库（crypto-argon2，供 Go 专用）

```bash
cd crypto-argon2

# 当前平台 Release 构建
cargo build --release
# 输出: target/release/libsolosoul_crypto.a

# 全平台交叉编译（macOS Universal + Linux + Windows）
./build_rust.sh --all

# 测试
./build_rust.sh --test
# 或 cargo test

# 清理
./build_rust.sh --clean

# FFI 完整性验证（Rust 函数签名 ↔ Go extern 声明）
./validate_ffi.sh
```

### Web UI

```bash
cd web
npm install

# 开发服务器
npm run dev

# 构建
npm run build

# 代码检查（ESLint 配置未显式存在，但脚本保留）
npm run lint
```

**注意事项：**
- Web UI 为**维护模式**，大量目录为空（`components/ui/`, `hooks/` 等）。
- 无测试框架，无 CI 集成。
- 客户端直接调用 Go 后端 `http://localhost:8080`，部分 Next.js API Routes 为代理/存根。

---

## 测试指令

### Go 后端测试

```bash
# 必须带 rust + cgo tags，否则编译失败
go test -tags "rust cgo" ./...
```

**Go 测试约定：**
- 仅使用标准库 `testing`，**不引入 testify、mock 框架等外部依赖**。
- 广泛使用表驱动测试（table-driven）和子测试（`t.Run`）。
- 文件系统测试统一使用 `t.TempDir()`。
- 错误比较使用精确错误变量比对（`err != ErrVaultLocked`），不使用字符串包含判断。

| 测试文件 | 包 | 覆盖范围 |
|---------|-----|---------|
| `core/crypto/*_test.go` | `crypto` | AES-GCM 往返、篡改检测、KDF、安全内存、工具函数 |
| `core/vault/file_store_test.go` | `vault` | Vault 全生命周期（init/unlock/lock/changePassword/CRUD） |
| `core/schema/*_test.go` | `schema` | Profile 构造、字段类型、验证器正则 |
| `core/api/*_test.go` | `api` | API 类型构造、插件/会话/授权逻辑 |
| `core/ocr/*_test.go` | `ocr` | MRZ 解析、图像预处理、Job 生命周期 |

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

### Go 后端

- **最小依赖原则**：除 `x/term` 和 `protobuf` 外，全部使用标准库。
- HTTP 路由使用 Go 1.22 `net/http` 的 pattern mux（`GET /api/...`），**不使用 Gin**。
- 错误处理：包级别预定义错误变量（`var ErrVaultLocked = errors.New(...)`），跨包传递。
- 并发：Vault 操作使用 `sync.Mutex` 保护；OCR Job 使用 goroutine + context timeout（60s）。

### Tauri / React 前端

- **状态管理**：Zustand。避免直接混用 `useState` 进行跨页面状态共享。
- **密码验证对话框**：统一使用 `src/components/PasswordVerificationDialog.tsx`，禁止多处复制对话框代码。
- **防抖保存**：Profile 修改采用 500ms debounce，关键操作可强制立即保存。
- **操作日志**：每次 CRUD 生成 `OperationEntry`，含 before→after 差异描述，支持 30 天软删除后永久清理。
- **自动锁定**：监听窗口焦点变化与系统休眠事件，超时锁定 Vault 并擦除敏感状态。
- **IPC 调用**：前端通过 `invoke` 调用 Rust Commands，禁止直接操作文件系统或网络。

### Web UI

- 使用 CSS Modules + 全局 CSS，不使用 Tailwind。
- 自定义 SVG 图标系统（`components/Icons.tsx`），通过 CSS 类控制动画（`anim-icon`, `anim-icon-lift`）。
- 所有受保护页面均为 Client Component（`'use client'`），权限检查通过 `useEffect` + Zustand `sessionToken` 手动重定向。

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

GitHub Actions 覆盖 **Tauri 前端 + Rust Workspace + Release 构建**，不覆盖 Go 后端和 `crypto-argon2/`。

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

### 1. Go 后端编译陷阱
`go build ./...` **不带 tags 会编译失败**，因为 `core/crypto` 的 `deriveKeyImpl` / `generateSaltImpl` 仅定义在 `//go:build rust && cgo` 文件中，没有纯 Go 回退实现。必须始终使用：
```bash
go build -tags "rust cgo" ./...
```

### 2. Apple Silicon Argon2 性能问题
Go 标准库 `golang.org/x/crypto/argon2` 在 macOS ARM64 上缺少 NEON SIMD 优化，64MiB 参数下可能挂起数秒甚至更久。
- **开发环境**默认使用 8MiB / 2 iterations。
- 生产环境通过 `SOLOSOUL_SECURE=1` 切换至 64MiB / 3 iterations。
- 如需极致性能，通过 `SOLOSOUL_USE_RUST=1` + `crypto-argon2/` 的 Rust SIMD 实现加速。

### 3. Rust 密码学库
存在**两套独立的 Rust 密码学实现**：
- `tauri/src-tauri/crates/solosoul-crypto/` — 供 Tauri 客户端使用。
- `crypto-argon2/` — 供 Go 后端 FFI 使用。
两者功能类似（均含 Argon2id + AES-256-GCM），但接口不同（Tauri Commands vs C FFI）。修改密码学逻辑时，需确认目标平台，避免改错仓库。

### 4. Web UI 与 Go 后端的状态不一致
Web UI 的部分 Next.js API Routes 为存根（stub）或返回 mock 数据，客户端代码（`lib/api.ts`）通常**直接调用** Go 后端 `localhost:8080`。调试时需确认请求实际落点。

### 5. SDK 占位
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
| Go HTTP API 服务器 | `core/api/server.go` |
| Go Vault 文件存储 | `core/vault/file_store.go` |
| Go 密码学（KDF 参数） | `core/crypto/kdf_common.go` |
| Go CLI 入口 | `cmd/solosoul/main.go` |
| Go Daemon 入口 | `cmd/solosould/main.go` |
| Rust（Go 专用）FFI | `crypto-argon2/src/lib.rs` |
| Rust（Tauri 专用）crypto crate | `tauri/src-tauri/crates/solosoul-crypto/` |
| Rust（Tauri 专用）vault crate | `tauri/src-tauri/crates/solosoul-vault/` |
| Rust（Tauri 专用）sync crate | `tauri/src-tauri/crates/solosoul-sync/` |
| Web API 客户端 | `web/lib/api.ts` |
| Web 全局状态 | `web/lib/store.ts` |
| 任务清单 | `docs/TODO.md` |
| 用户指南 | `docs/USER_GUIDE.md` |
| 技术路线图 | `docs/CLIENT_ROADMAP.md` |
| 事件日志 | `docs/WORKLOG.md` |
