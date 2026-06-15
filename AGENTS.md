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
| **Tauri 客户端（GUI）** | ✅ 主项目，活跃开发 | React + Tauri 跨平台客户端，macOS/Windows 已适配 |
| **Rust 原生核心** | ✅ 完整 | Argon2id + AES-256-GCM，通过 Tauri Commands 供前端调用 |
| **SoloSoul CLI** | ✅ Phase 4 已交付 | 独立终端 TUI 客户端；首次启动自动进入创建账户向导。已支持 `/unlock`、`/lock`、`/list`、`/open`、`/size`、`/search`、`/history`、`/rollback`、`/newpage`、`/newobject`、`/edit`、`/delete`、`/trash`、`/restore`、`/purge`、`/operation_log`、`/export_log`、`/about`、`/help` |
| **JS SDK** | ❌ 未开始 | `sdk/js/` 为空占位目录 |
| **Python SDK** | ❌ 未开始 | `sdk/python/` 为空占位目录 |

---

## 技术栈

| 组件 | 技术 | 备注 |
|------|------|------|
| Tauri 客户端 | React 19, TypeScript, Vite, Zustand, `@tauri-apps/api` | 状态管理以 Zustand 为主；UI 采用自定义 CSS Modules |
| Rust 核心（Tauri） | Rust 2021, `argon2`, `aes-gcm`, `rusqlite`, `tokio`, `ort` | 位于 `tauri/src-tauri/` 及 `tauri/crates/`，通过 Tauri Commands 暴露给前端 |
| SoloSoul CLI | Rust 2021, `ratatui` 0.30.1, `crossterm` 0.28.1, `clap` 4, `color-eyre` | 独立 Cargo 项目 `solosoul_cli/`，binary 名 `solosoul` |
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

**CLI 关键依赖：**
- `ratatui` 0.30.1 — 终端用户界面（TUI）
- `crossterm` 0.28.1 — 跨平台终端事件与光标控制
- `clap` 4 — 命令行参数解析
- `color-eyre` — 增强错误报告
- `tracing-appender` — 文件日志输出
- `fs2` — 进程级文件锁
- `zeroize` — 敏感内存安全清零
- `sys-locale` — 首次启动检测系统语言以导入对应默认模板

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
│   │   │   ├── solosoul-sync/  # 同步引擎（mDNS + Noise）
│   │   │   └── solosoul-core/  # 共享核心逻辑（Vault/模板/生物识别）
│   │   ├── resources/          # 打包资源（docs, ONNX models）
│   │   └── tauri.conf.json     # Tauri 应用配置
│   ├── scripts/                # 构建脚本（搜索索引等）
│   └── package.json            # npm 依赖
│
├── solosoul_cli/               # 独立终端 CLI（TUI）
│   ├── src/
│   │   ├── app.rs              # 全局状态机与事件循环
│   │   ├── cli.rs              # 命令行参数定义
│   │   ├── commands/           # 命令实现（doctor 等）
│   │   ├── events.rs           # 终端事件轮询
│   │   ├── screens/            # 屏幕渲染（welcome/locked/unlock/onboarding/home/object_list/object_detail/size/doctor/account_list/new_object/edit_object/trash_list/search_results/history_list/operation_log/about/help）
│   │   ├── tui.rs              # TUI 启动/恢复与帧绘制
│   │   ├── widgets/            # 可复用 TUI 组件（command_input 等）
│   │   └── lib.rs              # CLI 库入口
│   ├── Cargo.toml
│   └── Cargo.lock
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

### SoloSoul CLI

```bash
cd solosoul_cli

# 构建 Debug
cargo build

# 运行 TUI
cargo run

# 运行一次性子命令（如 upgrade 占位）
cargo run -- upgrade

# 测试
cargo test

# 格式化与静态检查
cargo fmt --check
cargo clippy -- -D warnings
```

**CLI 注意事项：**
- CLI 是独立 Cargo 项目，不混入 `tauri/` workspace，因此单独维护 `Cargo.lock`。
- 默认数据目录为 `~/.solosoul/`，可通过 `--data-dir` 或 `SOLOSOUL_DATA_DIR` 覆盖。
- 日志写入 `{data_dir}/logs/cli.log`，避免污染全屏 TUI；所有日志路径已做脱敏审查，不输出主密码、session key 等敏感信息。
- CLI 启动时会获取进程级排他锁（`solosoul_core::process_lock::ProcessLock`），防止多个 CLI/GUI 实例并发修改同一数据目录。
- 无本地账户时首次启动自动进入创建账户向导（账户名 → 主密码 → 确认密码 → 提示词 → 确认），支持 Esc 回退/取消。
- 登录密码与创建账户密码均使用 `Zeroizing<String>` 管理，通过 `VaultService::unlock_secure`/`create_account` 传递，失败后立即 zeroize。
- 已登录态 5 分钟无键盘操作自动锁定 Vault，状态栏显示剩余锁定倒计时。
- `/search` 在解密后的对象属性中做流式匹配，命中 200 条后提前截断并提示；支持 `"quoted phrase"` 多词关键词。
- `/operation_log`、`/export_log` 必须先解锁 Vault，审计日志通过 `VaultStore::list_audit_log` 读取并解密。
- 命令补全根据当前阶段动态过滤，未解锁时不会提示 `/list`、`/open` 等需登录命令；向导内部仅提供 `/cancel`、`/save`、`/back` 等不会丢失未保存数据的命令。
- 模态提示（字段编辑、确认对话框）打开期间自动暂停自动锁定计时，关闭后恢复。
- 状态栏明确显示 `🔒 进程锁已持有 · GUI 不可用`，提示 CLI 持有进程锁时 GUI 无法访问同一数据目录。

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

### CLI 测试

```bash
cd solosoul_cli

# Rust 单元测试
cargo test
```

**CLI 测试结构：**
- `src/widgets/command_input.rs` — 命令输入框光标、历史、补全单元测试。
- `src/widgets/password_input.rs` — 密码输入框掩码、光标、zeroize 单元测试。
- `src/app.rs` — 登录/锁定/自动锁定/状态机/渲染集成测试。
- `src/commands/auth.rs` — `/unlock`、`/lock`、`/logout` 命令测试。
- `src/commands/vault_read.rs` — `/list`、`/open`、`/size` 只读命令测试。
- `src/commands/vault_write.rs` — `/newpage`、`/newobject`、`/edit`、`/delete`、`/trash`、`/restore`、`/purge` 写入命令测试。
- `tests/integration_wizard.rs` — 完整向导链路集成测试（解锁 → 创建页面 → 创建对象 → 编辑 → 保存）。
- `src/commands/doctor.rs` — `/doctor` 诊断报告单元测试。
- `src/commands/search.rs` — `/search` 关键词提取与搜索结果单元测试。
- `src/commands/history.rs` — `/history`、`/rollback` 快照与回滚单元测试。
- `src/commands/log.rs` — `/operation_log`、`/export_log` 审计日志单元测试。
- `src/commands/system.rs` — `/about`、`/help` 系统信息单元测试。
- `tauri/crates/solosoul-core/src/process_lock.rs` — 进程锁获取/释放单元测试。

**注意**：涉及 `VaultService`/`SOLOSOUL_DATA_DIR` 的测试使用全局锁 `crate::VAULT_TEST_LOCK` 串行化，避免并发访问同一数据目录。

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
3. **plugin-market-check**（ubuntu-latest）— 递归检出 `SoloSoul_plugin_market` 子模块，验证 `registry.json` 与 `plugins/` 一致，并检查子模块指针无未提交改动。
4. **build-macos**（macos-latest，仅 master push）— Tauri Release 构建、DMG 打包。
5. **build-windows**（windows-latest，仅 master push）— Tauri Release 构建、MSI 打包。
6. **release**（ubuntu-latest，仅 master push）— Draft Pre-release 发布。

### pr_check.yml（PR 快速反馈）

1. **frontend-check** — `tsc --noEmit` + `npm run lint` + `npm run test`。
2. **rust-check** — `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`。
3. **plugin-market-check** — 验证子模块 `registry.json` 一致性。

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
| Tauri Vault 服务（薄包装） | `tauri/src-tauri/src/services/vault_service.rs` |
| 共享核心库（Vault/模板/生物识别） | `tauri/crates/solosoul-core/` |
| Tauri SensitivityManager | `tauri/src-tauri/src/core/sensitivity_manager.rs` |
| Tauri Auth 命令 | `tauri/src-tauri/src/commands/auth.rs` |
| Tauri Profile 命令 | `tauri/src-tauri/src/commands/profile.rs` |
| Tauri 对象命令 | `tauri/src-tauri/src/commands/unified_object.rs` |
| Tauri 数据库模块 | `tauri/src-tauri/src/db/` |
| Tauri 本地 Embedding | `tauri/src-tauri/src/local_embed.rs` |
| Rust crypto crate | `tauri/crates/solosoul-crypto/` |
| Rust vault crate | `tauri/crates/solosoul-vault/` |
| Rust sync crate | `tauri/crates/solosoul-sync/` |
| Rust shared core crate | `tauri/crates/solosoul-core/` |
| CLI 入口 | `solosoul_cli/src/main.rs` |
| CLI 参数定义 | `solosoul_cli/src/cli.rs` |
| CLI 状态机与事件循环 | `solosoul_cli/src/app.rs` |
| CLI TUI 启动/渲染 | `solosoul_cli/src/tui.rs` |
| CLI 事件轮询 | `solosoul_cli/src/events.rs` |
| CLI 命令实现 | `solosoul_cli/src/commands/` |
| CLI 屏幕渲染 | `solosoul_cli/src/screens/` |
| CLI 可复用组件 | `solosoul_cli/src/widgets/` |
| CLI 密码输入框 | `solosoul_cli/src/widgets/password_input.rs` |
| CLI 登录/解锁屏幕 | `solosoul_cli/src/screens/unlock.rs` |
| CLI 首次启动创建账户 | `solosoul_cli/src/screens/onboarding.rs` |
| CLI 已登录首页 | `solosoul_cli/src/screens/home.rs` |
| CLI 对象列表/详情/统计 | `solosoul_cli/src/screens/object_list.rs`、`object_detail.rs`、`size.rs` |
| CLI 创建对象/编辑对象/回收站 | `solosoul_cli/src/screens/new_object.rs`、`edit_object.rs`、`trash_list.rs` |
| CLI 搜索结果/历史快照/审计日志 | `solosoul_cli/src/screens/search_results.rs`、`history_list.rs`、`operation_log.rs` |
| CLI 关于/帮助 | `solosoul_cli/src/screens/about.rs`、`help.rs` |
| CLI Vault 只读命令 | `solosoul_cli/src/commands/vault_read.rs` |
| CLI Vault 写入命令 | `solosoul_cli/src/commands/vault_write.rs` |
| CLI 搜索/历史/审计/系统命令 | `solosoul_cli/src/commands/search.rs`、`history.rs`、`log.rs`、`system.rs` |
| CLI 字段编辑器/模态提示 | `solosoul_cli/src/widgets/field_editor.rs`、`prompt.rs` |
| 共享进程锁 | `tauri/crates/solosoul-core/src/process_lock.rs` |
| 共享核心库安全解锁 | `tauri/crates/solosoul-core/src/vault_service.rs` (`unlock_secure`)
| 任务清单 | `docs/TODO.md` |
| 用户指南 | `docs/USER_GUIDE.md` |
| 技术路线图 | `docs/CLIENT_ROADMAP.md` |
| 事件日志 | `docs/WORKLOG.md` |
