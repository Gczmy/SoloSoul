# AGENTS.md — SoloSoul Project Guide

> 本文件供 AI 编程助手阅读。读者应被假设对项目一无所知。
> 项目主要文档语言为**中文**（夹杂英文技术术语）。代码注释同样以中文为主。

---

## 项目概述

**SoloSoul（独灵）** 是一个本地优先、隐私优先的个人数字孪生与通用身份引擎。

- **核心理念**：「Centralized Schema definition, decentralized data storage」（集中式 Schema 定义，去中心化数据存储）
- **核心哲学**：「独奏生命数据，重塑数字原点」
- **架构定位**：所有敏感数据仅本地存储，绝不上传云端；采用零知识架构（Zero-Knowledge），服务端/开发者无法解密用户数据。

### 项目状态（截至 2026-04-27）

| 组件 | 状态 | 说明 |
|------|------|------|
| **Flutter 客户端** | ✅ 主项目，活跃开发 | macOS Release 已发布（DMG 安装包），Unified Object Model 已完成，iOS/Android/Windows 待适配 |
| **Rust 原生核心** | ✅ 完整 | Argon2id + AES-256-GCM，通过 FFI 供 Flutter/Go 调用 |
| **Go 后端** | ✅ 功能完整，维护模式 | HTTP API 服务器 + CLI 工具；最小依赖设计 |
| **Web UI** | ⚠️ 遗留项目，维护模式 | Next.js 15 实现，无测试覆盖，无 CI 集成 |
| **JS SDK** | ❌ 未开始 | `sdk/js/` 为空占位目录 |
| **Python SDK** | ❌ 未开始 | `sdk/python/` 为空占位目录 |

---

## 技术栈

| 组件 | 技术 | 备注 |
|------|------|------|
| Flutter 客户端 | Dart, `flutter_riverpod`, `go_router`, `liquid_glass_widgets` | 状态管理以 Riverpod + StateNotifier 为主；UI 采用 iOS 26 Liquid Glass 设计语言 |
| Rust 核心（Flutter） | Rust 2021, `argon2`, `aes-gcm`, `rusqlite`, `tokio`, `wasmtime` | 位于 `flutter/native/`，构建为 `staticlib/cdylib/rlib` |
| Rust 核心（Go） | Rust 2021, `argon2`, `aes-gcm`, `rand` | 位于 `crypto-argon2/`，构建为 `staticlib/cdylib` |
| Go 后端 | Go 1.26.1, 标准库 `net/http`（Go 1.22 pattern mux） | **注意：不使用 Gin 框架** |
| Web UI | Next.js 15.1.0, React 19, TypeScript 5.7, Zustand 5 | 不使用 Tailwind，使用纯 CSS Modules + 全局 CSS |
| 构建脚本 | Bash (`build_rust.sh`, `build_dmg.sh`, `validate_ffi.sh`) | 无 Makefile |

### 关键外部依赖

**Go（极度精简，仅 2 个直接依赖）：**
- `golang.org/x/term v0.41.0` — 安全密码输入
- `google.golang.org/protobuf v1.36.11` — 插件管理相关转换辅助

**Flutter 关键依赖：**
- `flutter_riverpod: ^2.6.1` — 状态管理
- `flutter_rust_bridge: ^2.0.0` — Rust FFI 绑定生成
- `local_auth: ^2.3.0` — 生物识别（Touch ID）
- `flutter_secure_storage: ^9.2.2` — macOS Keychain 封装
- `pointycastle`, `cryptography`, `encrypt` — Dart 端密码学回退（Android 使用）
- `liquid_glass_widgets: ^0.10.6` — iOS 26 Liquid Glass 玻璃质感 UI 组件库（36 个组件），替代原生 Material Card/AppBar/TextField 等

---

## 项目结构

```
SoloSoul/
├── flutter/                    # 主项目：Flutter 跨平台客户端
│   ├── lib/
│   │   ├── core/
│   │   │   ├── services/       # 核心服务（18个）：native_crypto, rust_vault, profile_storage, unified_object, keychain, biometric 等
│   │   │   ├── repositories/   # 数据仓库（Base + 各业务域）
│   │   │   ├── models/         # 基础模型、字段历史配置、UnifiedObject 模型
│   │   │   └── utils/          # 全局错误处理等
│   │   ├── presentation/
│   │   │   ├── pages/          # 13个页面：login, home, object_workspace, object_editor, profile, travel, financial, professional, settings, security_settings, sensitivity_settings, operation_log, trash, splash
│   │   │   ├── providers/      # Riverpod Notifiers（auth, profile, unified_object, sensitivity, account_style 等）
│   │   │   ├── widgets/        # 共享 UI 组件（敏感数据遮罩、对话框、侧边栏、图标选择器等）
│   │   │   └── theme/          # Material 3 主题配置
│   │   ├── frb/                # flutter_rust_bridge 生成的绑定代码
│   │   ├── data/               # Clean Architecture 脚手架（ lightly used ）
│   │   ├── domain/             # Clean Architecture 脚手架（ lightly used ）
│   │   └── main.dart           # 应用入口、生命周期监听、自动锁定
│   ├── native/                 # Rust 原生库（Flutter 专用）
│   │   └── src/
│   │       ├── crypto/         # Argon2id + AES-256-GCM
│   │       ├── vault/          # 加密存储（SQLite + AES-GCM 应用层加密）
│   │       ├── account/        # 账户管理
│   │       ├── sync/           # 同步引擎（预留）
│   │       └── plugin/         # Wasmtime 插件沙盒（预留）
│   ├── test/
│   │   ├── unit/               # Dart 单元测试
│   │   ├── widget/             # Flutter Widget 测试
│   │   └── integration_test/   # 集成测试（含 FFI 端到端测试）
│   └── integration_test/       # 应用级集成测试
│
├── web/                        # 遗留项目：Next.js Web UI（维护模式）
│   ├── app/                    # App Router
│   │   ├── (auth)/             # 受保护路由组：dashboard, home, ocr, plugins, profile, settings, vault
│   │   ├── (public)/           # 公开路由组：login, setup
│   │   └── api/                # Next.js API Routes（代理层，部分实现）
│   ├── components/             # 共享组件（大量空目录：ui/, plugin/, profile/, vault/）
│   ├── hooks/                  # 空目录
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
│   ├── TODO.md                 # 开发任务清单（P0–P7 优先级系统，44% 完成度）
│   ├── USER_GUIDE.md           # 终端用户指南
│   ├── CLIENT_ROADMAP.md       # 客户端技术架构路线图
│   ├── CLIENT_USER_GUIDE.md    # 客户端专用指南
│   ├── CHANGELOG.md            # 版本历史（SemVer）
│   ├── WORKLOG.md              # 事件日志（如 Vault 初始化挂起问题排查）
│   ├── PRIVACY_POLICY.md       # 隐私政策
│   └── TERMS_OF_SERVICE.md     # 服务条款
│
└── .github/workflows/          # GitHub Actions CI/CD
    ├── ci_cd.yml               # 完整流水线：Rust 测试 / Dart 单元测试 / Widget 测试 / 集成测试 / Release
    └── pr_check.yml            # PR 快速反馈：Format + Clippy + Dart Analyze + 测试
```

---

## 构建命令

### Flutter 客户端（主项目）

```bash
cd flutter

# 安装依赖
flutter pub get

# 代码分析
dart analyze --fatal-infos --fatal-warnings

# 运行（开发模式）
flutter run

# Release 构建（macOS）
flutter build macos --release --obfuscate --split-debug-info=./debug_info/macos

# DMG 安装包（需先完成 Release 构建）
cd ..
./build_dmg.sh
# 输出: flutter/build/macos/SoloSoul-v1.0.dmg
```

**注意事项：**
- macOS 构建需要 Xcode 和 CocoaPods（`macos/Podfile`）。
- Android 上 Vault 操作被临时 stub 掉，依赖 Dart 端密码学回退。
- Debug 符号保存在 `flutter/debug_info/`，已加入 `.gitignore`。

### Rust 原生库（Flutter）

```bash
cd flutter/native

# 构建 Release（供 Flutter 调用）
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

### Flutter 测试

```bash
cd flutter

# 单元测试
flutter test test/unit/

# Widget 测试
flutter test test/widget/

# 集成测试（需要先构建 Rust native lib）
cd native && cargo build --release && cd ..
flutter test integration_test/
```

**Flutter 测试结构：**
- `test/unit/` — Provider 逻辑、迁移指纹、版本检测、Vault Service 单元测试。
- `test/widget/` — 页面渲染、敏感标签组件、旅行页面交互。
- `integration_test/` — 应用启动导航、OCR 对话框、FFI 端到端（创建账户/保存/加载/删除 Profile）。

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

Flutter 端所有字段均有 `SensitivityLevel`：
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

### Flutter / Dart

- **状态管理**：`flutter_riverpod` + `StateNotifier`。避免直接混用 `setState` 进行跨页面状态共享。
- **密码验证对话框**：统一使用 `lib/presentation/widgets/password_verification_dialog.dart`，禁止多处复制对话框代码。
- **防抖保存**：Profile 修改采用 500ms debounce，关键操作可强制立即保存。
- **操作日志**：每次 CRUD 生成 `OperationEntry`，含 before→after 差异描述，支持 30 天软删除后永久清理。
- **自动锁定**：`main.dart` 监听 App 生命周期，后台/非活跃时启动倒计时，超时锁定 Vault 并擦除敏感状态。

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

GitHub Actions 仅覆盖 **Flutter + flutter/native Rust**，不覆盖 Go 后端和 `crypto-argon2/`。

### ci_cd.yml（Push 到 master/main 或 PR）

1. **rust-test**（ubuntu-latest）— `flutter/native` 的 `cargo test`。
2. **dart-unit-test**（ubuntu-latest）— `flutter test test/unit/`。
3. **widget-test**（ubuntu-latest）— `flutter test test/widget/`。
4. **integration-test**（macos-latest）— 构建 Rust native lib 后运行集成测试。
5. **release**（macos-latest，仅 master push）— Release 构建、DMG 打包、Draft Pre-release 发布、推送产物到独立仓库。

### pr_check.yml（PR 快速反馈）

1. **rust-check** — `cargo fmt --check` + `cargo clippy -- -D warnings`。
2. **dart-check** — `dart analyze --fatal-infos --fatal-warnings`。
3. **test** — Rust 测试 + Dart 单元测试 + Widget 测试。

### CI 版本号

- Flutter: `3.41.6`
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

### 3. Rust 密码学库重复
存在**两套独立的 Rust 密码学实现**：
- `crypto-argon2/` — 供 Go 后端 FFI 使用。
- `flutter/native/` — 供 Flutter 使用，功能几乎完全重复（均导出 Argon2id + AES-256-GCM）。
修改密码学逻辑时，需确认目标平台，避免改错仓库。

### 4. Web UI 与 Go 后端的状态不一致
Web UI 的部分 Next.js API Routes 为存根（stub）或返回 mock 数据，客户端代码（`lib/api.ts`）通常**直接调用** Go 后端 `localhost:8080`。调试时需确认请求实际落点。

### 5. SDK 占位
`sdk/js/` 和 `sdk/python/` 为空目录，任何 SDK 相关需求需从零开始实现。

### 6. iOS Keychain Handler
`ios/Runner/AppDelegate.swift` 缺少 Keychain method handler，`flutter_secure_storage` 在 iOS Release 下需要额外适配。当前临时回退到文件存储（P0 问题，见 `docs/TODO.md`）。

---

## 常用文件速查

| 目的 | 路径 |
|------|------|
| Flutter 入口 | `flutter/lib/main.dart` |
| Flutter 核心加密 FFI | `flutter/lib/core/services/native_crypto_service.dart` |
| Flutter Vault 服务 | `flutter/lib/core/services/rust_vault_service.dart` |
| Flutter Profile 存储/模型 | `flutter/lib/core/services/profile_storage_service.dart` |
| Flutter UnifiedObject 模型 | `flutter/lib/core/models/unified_object_model.dart` |
| Flutter UnifiedObject 服务 | `flutter/lib/core/services/unified_object_service.dart` |
| Flutter Auth 状态（1000+ 行） | `flutter/lib/presentation/providers/auth_provider.dart` |
| Flutter Profile 状态（2000+ 行） | `flutter/lib/presentation/providers/profile_provider.dart` |
| Flutter 对象状态管理 | `flutter/lib/presentation/providers/unified_object_provider.dart` |
| Flutter 对象工作区页面 | `flutter/lib/presentation/pages/object_workspace_page.dart` |
| Flutter 对象编辑器页面 | `flutter/lib/presentation/pages/object_editor_page.dart` |
| Flutter 常驻侧边栏 | `flutter/lib/presentation/widgets/app_sidebar.dart` |
| Go HTTP API 服务器 | `core/api/server.go` |
| Go Vault 文件存储 | `core/vault/file_store.go` |
| Go 密码学（KDF 参数） | `core/crypto/kdf_common.go` |
| Go CLI 入口 | `cmd/solosoul/main.go` |
| Go Daemon 入口 | `cmd/solosould/main.go` |
| Rust（Go 专用）FFI | `crypto-argon2/src/lib.rs` |
| Rust（Flutter 专用） | `flutter/native/src/lib.rs` |
| Web API 客户端 | `web/lib/api.ts` |
| Web 全局状态 | `web/lib/store.ts` |
| 任务清单 | `docs/TODO.md` |
| 用户指南 | `docs/USER_GUIDE.md` |
| 技术路线图 | `docs/CLIENT_ROADMAP.md` |
| 事件日志 | `docs/WORKLOG.md` |
