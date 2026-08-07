# 03 — 项目结构与 Rust Workspace

> **前置阅读**：`01_技术选型确认与架构决策.md`、`04_密码学库选型与核心实现.md`
> **Manifesto 对齐**：依赖最小化 | 最少惊喜 | 安全默认
> **源文档**：`tauri_refactor/项目结构规划.md` + `tauri_refactor/Crate拆分与Rust架构.md`
>
> **合并说明（2026-08）**：本文档由原「03_项目顶层结构规划」与「04_Rust_Crate拆分与后端架构」合并而来，
> 并已按当前代码修正：crates 实际 5 个（新增 core/plugin）、`unified_object` 已更名 `object`、
> `src-tauri/src/commands/` 实际 30+ 模块、样式组织无 Liquid Glass 依赖。

---

## 1. 顶层目录结构

```
SoloSoul/                          # 项目根目录
├── Cargo.toml                     # Workspace 定义
├── Cargo.lock
├── package.json                   # 前端依赖
├── vite.config.ts                 # Vite 构建配置
├── tsconfig.json
├── index.html                     # 入口 HTML
├── tauri.conf.json                # Tauri 配置
│
├── src/                           # 前端源代码（React 19 + TypeScript）
│   ├── main.tsx                   # 前端入口
│   ├── App.tsx                    # 根组件（路由 + 全局布局）
│   ├── styles/                    # 全局样式
│   │   ├── global.css             # 全局样式、字体、滚动条
│   │   ├── animations.css         # 关键帧动画 + interactive-* 工具类
│   │   └── themes.css             # 明暗主题变量
│   ├── components/                # 共享 UI 组件
│   │   ├── ui/                    # 基础 UI（Button, Input, Card, Dialog...）
│   │   ├── layout/                # 布局（AppShell, AppBar, SideNavigation）
│   │   ├── data-display/          # 数据展示（SensitiveValue, SensitivityBadge）
│   │   └── forms/                 # 表单（PasswordInput, DatePicker）
│   ├── pages/                     # 页面组件（按用户旅程组织）
│   │   ├── auth/                  # J1: 认证旅程
│   │   ├── home/                  # J2: 首页
│   │   ├── workspace/             # J2: 对象工作区
│   │   ├── editor/                # J2: 对象编辑器
│   │   ├── settings/              # J3/J7: 设置相关
│   │   ├── ai/                    # J5: AI 与自动化
│   │   ├── sync/                  # J6: 设备同步
│   │   ├── scan/                  # OCR 扫描
│   │   ├── search/                # 全局搜索
│   │   └── system/                # J7: 系统管理
│   ├── hooks/                     # 自定义 Hooks
│   ├── stores/                    # 全局状态（Zustand，17 个）
│   ├── lib/                       # 工具库（ipc, theme, masking, dialog, pageIcons...）
│   ├── locales/                   # i18n 词条文件（zh-CN / en-US，按领域拆分）
│   ├── types/                     # TypeScript 类型定义
│   └── assets/                    # 静态资源（字体、图标、图片）
│
├── src-tauri/                     # Tauri Rust 后端
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/              # 权限声明（Tauri v2）
│   └── src/
│       ├── main.rs                # 入口
│       ├── lib.rs                 # 库入口
│       ├── commands/              # IPC 命令处理器（30+ 个模块）
│       ├── state/                 # 应用状态管理（Tauri State）
│       ├── services/              # 业务服务层
│       ├── db/                    # 数据库访问层
│       └── sync/                  # 自动同步等（device_auto_sync 等）
│
├── crates/                        # 独立 Rust crates（Workspace 成员，共 5 个）
│   ├── solosoul-core/             # 共享核心逻辑（Vault 服务/模板/生物识别/OCR/进程锁）
│   ├── solosoul-crypto/           # 密码学库
│   ├── solosoul-vault/            # Vault 存储库
│   ├── solosoul-sync/             # 同步引擎库
│   └── solosoul-plugin/           # 插件运行时（WASI 沙盒 + Host）
│
├── solosoul_cli/                  # 独立 CLI（TUI，独立 Cargo 项目）
├── docs/                          # 项目文档
└── scripts/                       # 构建/开发脚本
```

---

## 2. Workspace 成员

| Crate | 路径 | 职责 | Tauri 依赖 |
|-------|------|------|-----------|
| `solosoul-core` | `crates/solosoul-core/` | 共享核心：Vault 服务/模板/生物识别/OCR/进程锁 | [错误] 无 |
| `solosoul-crypto` | `crates/solosoul-crypto/` | Argon2id KDF + AES-256-GCM + 安全内存 | [错误] 无 |
| `solosoul-vault` | `crates/solosoul-vault/` | Vault 管理 + SQLite 存储 + 加密层 | [错误] 无 |
| `solosoul-sync` | `crates/solosoul-sync/` | HLC 增量同步 + Noise 协议 + mDNS 发现 | [错误] 无 |
| `solosoul-plugin` | `crates/solosoul-plugin/` | 插件运行时（WASI 沙盒 + Host + 盖章） | [错误] 无 |
| `src-tauri` | `src-tauri/` | Tauri 应用 + IPC 命令 + 服务层 | [正确] tauri |

> **拆分原则**：单一职责、crates 不依赖 `tauri`（独立可测试）、核心逻辑可被 CLI 复用、渐进拆分（稳定后提取到独立 crate）。

### 依赖关系

```
src-tauri (Tauri App)
├── solosoul-core
├── solosoul-crypto
├── solosoul-vault
│   └── solosoul-crypto
├── solosoul-sync
│   ├── solosoul-crypto
│   └── solosoul-vault
├── solosoul-plugin
│   └── solosoul-core（进程锁等）
└── tauri（官方 crate）
```

---

## 3. 前后端边界定义

### 3.1 前端职责（`src/`）

| 职责 | 说明 |
|------|------|
| **UI 渲染** | React 组件树、页面布局、动画 |
| **用户交互** | 点击、输入、表单验证、导航 |
| **状态缓存** | Zustand 存储前端状态，减少 IPC 调用 |
| **IPC 调用** | 通过 `invoke()` 调用 Rust 命令（统一封装于 `src/lib/ipcClient.ts`） |
| **事件监听** | 通过 `listen()` 接收 Rust 事件 |
| **路由管理** | React Router 处理页面导航 |
| **本地化** | i18n 资源加载、语言切换 |

### 3.2 后端职责（`src-tauri/` + `crates/`）

| 职责 | 说明 |
|------|------|
| **业务逻辑** | 所有 CRUD、搜索、排序、过滤 |
| **数据持久化** | SQLite 读写、文件系统操作 |
| **密码学** | Argon2id、AES-GCM、密钥派生、安全内存 |
| **原生 API** | Keychain、生物识别、文件选择、系统通知 |
| **插件沙盒** | Wasmtime 执行环境 |
| **同步协议** | mDNS、Noise、HLC 增量同步 |
| **日志记录** | 结构化日志、文件轮转 |

### 3.3 禁止越界

| [错误] 前端禁止 | [错误] 后端禁止 |
|-----------|-----------|
| 直接访问文件系统 | 操作 DOM |
| 直接访问数据库 | 渲染 UI |
| 持有密钥 | 处理路由导航 |
| 密码学运算（推导除外） | 管理前端状态 |
| 插件执行 | 本地化字符串处理 |

---

## 4. 核心 Crate: `solosoul-crypto`

### 目录结构

```
crates/solosoul-crypto/
├── Cargo.toml
└── src/
    ├── lib.rs          # 公共导出
    ├── kdf.rs          # Argon2id 密钥派生
    ├── cipher.rs       # AES-256-GCM 加密/解密（分块格式）
    ├── aes.rs          # 分块加解密实现
    └── secure.rs       # 安全内存（Zeroize + SecureBytes）
```

### 公共 API

```rust
pub mod kdf;
pub mod cipher;
pub mod secure;

pub use kdf::{derive_key, KdfConfig};
pub use cipher::{encrypt, decrypt, EncryptedData, CipherError};
pub use secure::{SecureBytes, SecureString, secure_wipe, secure_compare};
```

### KdfConfig 预设值

```rust
impl KdfConfig {
    /// 开发模式：8 MiB / 2 iterations / 4 parallelism
    pub fn development() -> Self { ... }

    /// 生产模式：64 MiB / 3 iterations / 4 parallelism（OWASP 推荐）
    pub fn production() -> Self { ... }
}
```

### CipherError 枚举

```rust
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("无效的密钥长度（需要 32 字节）")]
    InvalidKeyLength,
    #[error("Nonce 生成失败")]
    NonceGenerationFailed,
    #[error("加密失败")]
    EncryptionFailed,
    #[error("解密失败：密文可能已损坏或被篡改")]
    DecryptionFailed,
    #[error("无效的密文格式")]
    InvalidCiphertext,
}
```

---

## 5. 核心 Crate: `solosoul-vault`

### 目录结构

```
crates/solosoul-vault/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── storage.rs       # SQLite 存储层（对象/模板/附件/回收站/审计日志）
    ├── manager.rs       # Vault 生命周期管理
    ├── migration.rs     # Schema 迁移（CURRENT_SCHEMA_VERSION = 25）
    ├── profile.rs       # Profile 数据
    └── template_hash.rs # 模板哈希（单一真理来源）
```

### VaultManager 核心 API

```rust
pub struct VaultManager { ... }

impl VaultManager {
    pub fn new(base_path: &Path) -> Result<Self, VaultError>;
    pub async fn initialize(&self, account_id: &str, password: &str) -> Result<(), VaultError>;
    pub async fn unlock(&self, account_id: &str, password: &str) -> Result<(), VaultError>;
    pub async fn lock(&self) -> Result<(), VaultError>;
    pub async fn state(&self) -> VaultState;
}
```

### VaultError 枚举

```rust
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Vault 已锁定")]
    Locked,
    #[error("密码不正确")]
    InvalidPassword,
    #[error("账户不存在")]
    AccountNotFound,
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("加密错误: {0}")]
    Crypto(String),
}
```

---

## 6. 主 Crate: `src-tauri`

### 内部模块层次

```
src-tauri/src/
├── main.rs                 # 入口：注册 plugins、state、commands
├── lib.rs                  # 库入口（测试用）
├── commands/               # IPC 命令处理器（30+ 个模块，命令总数 200+）
│   ├── auth.rs, profile.rs, object/, template.rs, search.rs
│   ├── settings.rs, export_import/, backup.rs, attachment.rs
│   ├── plugin.rs, ocr.rs, llm/, sync.rs, log.rs, biometric.rs
│   ├── crypto.rs, embed_model.rs, update.rs, fs.rs, pin.rs ...
├── services/               # 业务服务层（vault_service, llm_context...）
├── db/                     # 数据库访问层
│   ├── connection.rs, migrations.rs
│   └── repositories/
├── state/                  # Tauri State（全局状态）
│   └── app_state.rs
├── sync/                   # 同步相关（device_auto_sync 等）
└── local_embed.rs          # 本地 Embedding（ONNX）
```

---

## 7. 错误处理规范

| 层级 | 使用 | 示例 |
|------|------|------|
| **Crate 级**（solosoul-crypto 等） | `thiserror` 精确错误类型 | `CipherError`, `VaultError` |
| **应用级**（src-tauri） | `anyhow` 快速传播 + `thiserror` 顶层包装 | `AppError` |
| **IPC 命令** | `Result<T, String>` | `Ok(data)` / `Err("密码错误".to_string())` |

---

## 8. 安全约束（Manifesto 信条落地）

| 约束 | 实现方式 |
|------|---------|
| **密钥绝不离开 Rust 内存** | 所有密钥在 Rust 侧管理，前端通过 IPC 获取加密数据 |
| **内存自动擦除** | 使用 `Zeroizing<T>` 包裹所有密钥和派生输出 |
| **不使用 `.unwrap()`** | 生产代码中全部用 `Result` 传播（审计确认仅 2 处有前置检查） |
| **无硬编码密钥或 IV** | 全部使用 CSPRNG（OsRng）生成 |
| **不降低 KDF 参数"优化性能"** | Argon2id 参数固定，不可配置降低 |

---

## 9. 样式组织原则

```
[正确] CSS Modules + 全局 CSS
   - 组件级样式：Button.module.css（局部作用域）
   - 全局样式：styles/*.css（主题变量、动画、interactive-* 工具类）

[错误] 不使用 CSS-in-JS（styled-components/emotion）
   理由：运行时开销

[错误] 不使用 Tailwind CSS
   理由：团队无经验
```

---

## 10. 与 Flutter 项目的对应关系（历史记录）

| 当前 Flutter | Tauri 重构 | 说明 |
|-------------|-----------|------|
| `lib/` (Dart) | `src/` (React) | UI 层迁移 |
| `lib/core/services/` | `src-tauri/src/services/` | 服务层下沉 Rust |
| `lib/core/repositories/` | `src-tauri/src/db/repositories/` | 仓库层迁移 |
| `lib/core/models/` | `src-tauri/src/core/` + `src/types/` | 模型双写 |
| `lib/presentation/pages/` | `src/pages/` | 页面对应 |
| `lib/presentation/providers/` | `src/stores/` (Zustand) | 状态管理替换 |
| `lib/presentation/widgets/` | `src/components/` | 组件对应 |
| `native/src/` (Rust FFI) | `src-tauri/src/core/` + `crates/` | 核心代码直接复用 |

---

## 11. 从零搭建顺序

1. `npm create tauri-app@latest solosoul -- --template react-ts`
2. 配置 Workspace（根 Cargo.toml + `crates/` 目录）
3. 配置 Vite + React 19
4. 配置 ESLint + Prettier + `cargo fmt` + `clippy`
5. 配置 IPC 类型手工维护（tauri-specta 评估后未采用，类型定义于 `src/lib/ipc.ts`）
6. 实现 `solosoul-crypto`：kdf.rs → cipher.rs → secure.rs → 单元测试
7. 实现 `solosoul-vault`：storage.rs → manager.rs → migration.rs → 单元测试
8. 配置 `src-tauri` 依赖 workspace crates
9. 实现 `src-tauri/state/`：AppState (Tauri State)
10. 逐步填充 `commands/`、`services/`、`db/`

---

## 12. 完成标准

- [x] `npm run tauri dev` 可以启动并显示 React 页面
- [x] Workspace 的 `cargo build` 通过
- [x] `cargo test --workspace` 通过
- [x] ESLint / Prettier / clippy / rustfmt 已配置并通过
- [x] 前后端边界清晰（无越界代码）
- [x] `solosoul-crypto` 的所有函数使用 `Zeroizing` 保护
- [x] 无 `.unwrap()` 或 `.expect()` 在生产代码中（审计确认）
- [x] 每个 public API 有 doc comment

---

*文档版本：v1.1（原 03 v1.0 + 04 v1.0 合并）*
*创建日期：2026-06-05*
*最后更新：2026-08-07（合并并修正）*
*对应开发阶段：Phase 1-2（项目初始化 + Rust 核心）*
