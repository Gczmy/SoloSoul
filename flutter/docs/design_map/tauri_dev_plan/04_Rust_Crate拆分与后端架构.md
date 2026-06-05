# 04 — Rust Crate 拆分与后端架构

> **前置阅读**：`03_项目顶层结构规划.md`
> **Manifesto 对齐**：📐 依赖最小化 | 🛡️ 安全默认
> **源文档**：`tauri_refactor/Crate拆分与Rust架构.md`

---

## 1. Workspace 成员

| Crate | 路径 | 职责 | Tauri 依赖 |
|-------|------|------|-----------|
| `solosoul-crypto` | `crates/solosoul-crypto/` | Argon2id KDF + AES-256-GCM + 安全内存 | ❌ 无 |
| `solosoul-vault` | `crates/solosoul-vault/` | Vault 管理 + 文件存储 + 加密层 | ❌ 无 |
| `solosoul-sync` | `crates/solosoul-sync/` | CRDT + Noise 协议 + mDNS 发现 | ❌ 无 |
| `src-tauri` | `src-tauri/` | Tauri 应用 + IPC 命令 + 服务层 | ✅ tauri |

---

## 2. 核心 Crate: `solosoul-crypto`

### 目录结构

```
crates/solosoul-crypto/
├── Cargo.toml
└── src/
    ├── lib.rs          # 公共导出
    ├── kdf.rs          # Argon2id 密钥派生
    ├── cipher.rs       # AES-256-GCM 加密/解密
    ├── secure.rs       # 安全内存（Zeroize + SecureBytes）
    └── ffi.rs          # C FFI 兼容层（可选）
```

### Cargo.toml

```toml
[package]
name = "solosoul-crypto"
version.workspace = true
edition.workspace = true

[dependencies]
argon2 = { workspace = true }
aes-gcm = { workspace = true }
zeroize = { workspace = true }
rand = { workspace = true }
sha2 = { workspace = true }
thiserror = { workspace = true }
```

### lib.rs 公共 API

```rust
pub mod kdf;
pub mod cipher;
pub mod secure;

#[cfg(feature = "ffi")]
pub mod ffi;

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

## 3. 核心 Crate: `solosoul-vault`

### 目录结构

```
crates/solosoul-vault/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── storage.rs       # 文件存储接口
    ├── manager.rs       # Vault 生命周期管理
    ├── encryption.rs    # Vault 层加密逻辑
    └── schema.rs        # Vault Schema 定义
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

### VaultState 枚举

```rust
pub enum VaultState {
    Uninitialized,
    Locked,
    Unlocked,
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

## 4. 主 Crate: `src-tauri`

### 内部模块层次

```
src-tauri/src/
├── main.rs                 # 入口：注册 plugins、state、commands
├── lib.rs                  # 库入口（测试用）
├── commands/               # IPC 命令处理器（14 个模块）
│   ├── auth.rs, vault.rs, profile.rs, unified_object.rs
│   ├── search.rs, settings.rs, export_import.rs, backup.rs
│   ├── plugin.rs, ocr.rs, llm.rs, sync.rs, log.rs, system.rs
├── services/               # 业务服务层
├── core/                   # 核心业务逻辑（无 Tauri 依赖，可单元测试）
│   ├── crypto/             # 临时（Phase 2 后迁移到 solosoul-crypto）
│   ├── vault/              # 临时（Phase 3 后迁移到 solosoul-vault）
│   ├── profile/, plugin/, ocr/, sync/
│   └── utils/              # 错误、路径、时间工具
├── db/                     # 数据库访问层
│   ├── connection.rs, migrations.rs
│   └── repositories/
├── state/                  # Tauri State（全局状态）
│   ├── app_state.rs, vault_state.rs, session_state.rs
└── ipc/                    # IPC 辅助
    ├── events.rs           # 自定义事件定义
    └── streams.rs          # 流式响应
```

### main.rs 核心结构

```rust
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_state = state::AppState::new(app.handle().clone())?;
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 详见文档 07_IPC命令接口完整规范.md
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 5. 错误处理规范

### 分层错误策略

| 层级 | 使用 | 示例 |
|------|------|------|
| **Crate 级**（solosoul-crypto 等） | `thiserror` 精确错误类型 | `CipherError`, `VaultError` |
| **应用级**（src-tauri） | `anyhow` 快速传播 + `thiserror` 顶层包装 | `AppError` |
| **IPC 命令** | `Result<T, String>` | `Ok(data)` / `Err("密码错误".to_string())` |

### AppError 定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("密码学错误: {0}")]
    Crypto(#[from] solosoul_crypto::CipherError),
    #[error("Vault 错误: {0}")]
    Vault(#[from] solosoul_vault::VaultError),
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("无效参数: {0}")]
    InvalidParameter(String),
    #[error("未授权")]
    Unauthorized,
}
```

---

## 6. 安全约束（Manifesto 信条落地）

| 约束 | 实现方式 |
|------|---------|
| **密钥绝不离开 Rust 内存** | 所有密钥在 Rust 侧管理，前端通过 IPC 获取加密数据 |
| **内存自动擦除** | 使用 `Zeroizing<T>` 包裹所有密钥和派生输出 |
| **不使用 `.unwrap()`** | 生产代码中全部用 `Result` 传播 |
| **无硬编码密钥或 IV** | 全部使用 CSPRNG（OsRng）生成 |
| **不降低 KDF 参数"优化性能"** | Argon2id 参数固定，不可配置降低 |

---

## 7. 从零实现顺序

1. 初始化 Workspace（根 Cargo.toml）
2. 实现 `solosoul-crypto`：kdf.rs → cipher.rs → secure.rs → 单元测试
3. 实现 `solosoul-vault`：storage.rs → manager.rs → encryption.rs → 单元测试
4. 配置 `src-tauri` 依赖 workspace crates
5. 实现 `src-tauri/state/`：AppState (Tauri State)
6. 逐步填充 `commands/`、`services/`、`db/`

---

## 8. 完成标准

- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `solosoul-crypto` 的所有函数使用 `Zeroizing` 保护
- [ ] 无 `.unwrap()` 或 `.expect()` 在生产代码中
- [ ] 每个 public API 有 doc comment
- [ ] 错误类型清晰，包含中文错误消息

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 1-2（Rust 核心）*
