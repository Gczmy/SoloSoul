# Crate 拆分与 Rust 架构

> **文档定位**: Rust 后端的具体 crate 拆分方案、模块设计、代码组织规范。
>
> **阅读对象**: Rust 开发者、后端开发者。
>
> **前置知识**: 需先阅读 `ADR-003-Rust_Crate拆分与架构.md`。

---

## 目录

- [Workspace 配置](#workspace-配置)
- [核心 Crate: solosoul-crypto](#核心-crate-solosoul-crypto)
- [核心 Crate: solosoul-vault](#核心-crate-solosoul-vault)
- [核心 Crate: solosoul-sync](#核心-crate-solosoul-sync)
- [主 Crate: src-tauri](#主-crate-src-tauri)
- [模块依赖图](#模块依赖图)
- [错误处理规范](#错误处理规范)
- [从零实现顺序](#从零实现顺序)

---

## Workspace 配置

### 根目录 Cargo.toml

```toml
[workspace]
members = [
    "src-tauri",
    "crates/solosoul-crypto",
    "crates/solosoul-vault",
    "crates/solosoul-sync",
]
resolver = "2"

[workspace.package]
version = "2.0.0"
edition = "2021"
authors = ["SoloSoul Team"]
license = "MIT"
repository = "https://github.com/Gczmy/SoloSoul"

[workspace.dependencies]
# 密码学
argon2 = "0.5.3"
aes-gcm = "0.10.3"
zeroize = { version = "1.8", features = ["derive", "aarch64"] }
rand = "0.8"
sha2 = "0.10"

# 数据库
rusqlite = { version = "0.32", features = ["bundled", "backup", "chrono"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 异步
tokio = { version = "1.40", features = ["full"] }
tokio-util = "0.7"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# HTTP
tokio-tungstenite = "0.24"
reqwest = { version = "0.12", features = ["json", "stream"] }

# 时间
chrono = { version = "0.4", features = ["serde"] }

# 工具
uuid = { version = "1.10", features = ["v4", "serde"] }
regex = "1.11"
once_cell = "1.20"

# 测试
tempfile = "3"
```

---

## 核心 Crate: solosoul-crypto

### 目录结构

```
crates/solosoul-crypto/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── kdf.rs          # 密钥派生（Argon2id）
    ├── cipher.rs       # 对称加密（AES-256-GCM）
    ├── secure.rs       # 安全内存（Zeroize + SecretBox）
    └── ffi.rs          # C FFI 兼容层（供旧 已废弃）
```

### Cargo.toml

```toml
[package]
name = "solosoul-crypto"
version.workspace = true
edition.workspace = true
authors.workspace = true

[dependencies]
argon2 = { workspace = true }
aes-gcm = { workspace = true }
zeroize = { workspace = true }
rand = { workspace = true }
sha2 = { workspace = true }
thiserror = { workspace = true }
```

### lib.rs

```rust
//! SoloSoul 密码学核心库
//! 
//! 提供：
//! - Argon2id 密钥派生
//! - AES-256-GCM 加密/解密
//! - 安全内存管理（自动擦除）
//! - C FFI 接口（兼容旧系统）

pub mod kdf;
pub mod cipher;
pub mod secure;

// 条件编译：仅在需要 FFI 时启用
#[cfg(feature = "ffi")]
pub mod ffi;

pub use kdf::{derive_key, KdfConfig, KdfParams};
pub use cipher::{encrypt, decrypt, AesGcmError};
pub use secure::{SecureBytes, SecureString, secure_wipe};
```

### kdf.rs（Argon2id）

```rust
use argon2::{self, Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

/// KDF 配置
#[derive(Debug, Clone, Copy)]
pub struct KdfConfig {
    pub memory_kb: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl KdfConfig {
    /// 开发模式（默认）
    pub fn development() -> Self {
        Self {
            memory_kb: 8 * 1024,  // 8 MiB
            iterations: 2,
            parallelism: 4,
        }
    }

    /// 生产模式
    pub fn production() -> Self {
        Self {
            memory_kb: 64 * 1024,  // 64 MiB
            iterations: 3,
            parallelism: 4,
        }
    }
}

/// 派生 256-bit 密钥
pub fn derive_key(
    password: &str,
    salt: &[u8],
    config: &KdfConfig,
) -> Result<Zeroizing<Vec<u8>>, argon2::Error> {
    let params = Params::new(
        config.memory_kb,
        config.iterations,
        config.parallelism,
        Some(32),  // 256-bit
    )?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = Zeroizing::new(vec![0u8; 32]);
    argon2.hash_password_into(password.as_bytes(), salt, &mut output)?;
    
    Ok(output)
}
```

### cipher.rs（AES-256-GCM）

```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("加密失败")]
    EncryptionFailed,
    #[error("解密失败：密文可能已损坏")]
    DecryptionFailed,
    #[error("无效的密钥长度")]
    InvalidKeyLength,
}

/// AES-256-GCM 加密
pub fn encrypt(
    key: &[u8; 32],
    plaintext: &[u8],
) -> Result<Vec<u8>, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CipherError::InvalidKeyLength)?;
    
    let nonce_bytes = rand::random::<[u8; 12]>();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CipherError::EncryptionFailed)?;
    
    // nonce (12 bytes) + ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

/// AES-256-GCM 解密
pub fn decrypt(
    key: &[u8; 32],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if ciphertext.len() < 12 {
        return Err(CipherError::DecryptionFailed);
    }
    
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CipherError::InvalidKeyLength)?;
    
    let nonce = Nonce::from_slice(&ciphertext[..12]);
    let plaintext = cipher
        .decrypt(nonce, &ciphertext[12..])
        .map_err(|_| CipherError::DecryptionFailed)?;
    
    Ok(Zeroizing::new(plaintext))
}
```

### secure.rs（安全内存）

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::ops::{Deref, DerefMut};

/// 安全字节数组（自动擦除）
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecureBytes {
    #[zeroize(skip)]  // 不跳过，这里只是标记
    data: Vec<u8>,
}

impl SecureBytes {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
    
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Deref for SecureBytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for SecureBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// 安全字符串（自动擦除）
pub type SecureString = zeroize::Zeroizing<String>;

/// 手动擦除内存
pub fn secure_wipe<T: Zeroize>(data: &mut T) {
    data.zeroize();
}
```

---

## 核心 Crate: solosoul-vault

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

### Cargo.toml

```toml
[package]
name = "solosoul-vault"
version.workspace = true
edition.workspace = true

[dependencies]
solosoul-crypto = { path = "../solosoul-crypto" }

rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

### manager.rs

```rust
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vault 状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultState {
    Uninitialized,
    Locked,
    Unlocked,
}

/// Vault 管理器
pub struct VaultManager {
    state: Arc<RwLock<VaultState>>,
    storage: Box<dyn Storage>,
    encryption: VaultEncryption,
}

impl VaultManager {
    pub fn new(base_path: &Path) -> Result<Self, VaultError> {
        Ok(Self {
            state: Arc::new(RwLock::new(VaultState::Uninitialized)),
            storage: Box::new(FileStorage::new(base_path)?),
            encryption: VaultEncryption::new(),
        })
    }

    /// 初始化 Vault（首次使用）
    pub async fn initialize(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<(), VaultError> {
        let mut state = self.state.write().await;
        
        // 1. 生成 Salt
        let salt = generate_salt();
        
        // 2. 派生密钥
        let config = KdfConfig::development();  // 或通过环境变量配置
        let key = derive_key(password, &salt, &config)?;
        
        // 3. 创建目录结构
        self.storage.create_account_dir(account_id)?;
        
        // 4. 保存配置（Salt + 验证令牌）
        let config = AccountConfig {
            salt: base64::encode(&salt),
            verify_token: self.encryption.create_verify_token(&key)?,
        };
        self.storage.save_config(account_id, &config)?;
        
        // 5. 初始化数据库
        self.storage.init_database(account_id, &key)?;
        
        *state = VaultState::Unlocked;
        Ok(())
    }

    /// 解锁 Vault
    pub async fn unlock(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<(), VaultError> {
        let mut state = self.state.write().await;
        
        // 1. 读取 Salt
        let config = self.storage.load_config(account_id)?;
        let salt = base64::decode(&config.salt)?;
        
        // 2. 派生密钥
        let kdf_config = KdfConfig::development();
        let key = derive_key(password, &salt, &kdf_config)?;
        
        // 3. 验证密码（通过验证令牌）
        if !self.encryption.verify_token(&key, &config.verify_token)? {
            return Err(VaultError::InvalidPassword);
        }
        
        // 4. 解密数据库密钥
        let db_key = self.encryption.decrypt_db_key(account_id, &key)?;
        
        // 5. 打开数据库
        self.storage.open_database(account_id, &db_key)?;
        
        // 6. 保存派生密钥（内存中）
        self.encryption.set_active_key(key)?;
        
        *state = VaultState::Unlocked;
        Ok(())
    }

    /// 锁定 Vault
    pub async fn lock(&self) -> Result<(), VaultError> {
        let mut state = self.state.write().await;
        
        // 擦除内存中的密钥
        self.encryption.clear_active_key();
        
        // 关闭数据库连接
        self.storage.close_database()?;
        
        *state = VaultState::Locked;
        Ok(())
    }

    /// 检查状态
    pub async fn state(&self) -> VaultState {
        *self.state.read().await
    }
}

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

## 核心 Crate: solosoul-sync

### 目录结构

```
crates/solosoul-sync/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── crdt.rs           # CRDT 数据结构
    ├── transport.rs      # 网络传输（TCP + Noise）
    ├── noise.rs          # Noise 协议实现
    └── discovery.rs      # mDNS 服务发现
```

### Cargo.toml

```toml
[package]
name = "solosoul-sync"
version.workspace = true
edition.workspace = true

[dependencies]
solosoul-crypto = { path = "../solosoul-crypto" }
solosoul-vault = { path = "../solosoul-vault" }

tokio = { workspace = true }
tokio-util = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# mDNS
mdns-sd = "0.11"

# Noise
snow = "0.9"
```

---

## 主 Crate: src-tauri

### 目录结构

```
src-tauri/
├── Cargo.toml
├── build.rs
├── tauri.conf.json
├── capabilities/
│   └── default.json
├── icons/
└── src/
    ├── main.rs
    ├── lib.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── auth.rs
    │   ├── vault.rs
    │   ├── profile.rs
    │   ├── unified_object.rs
    │   ├── search.rs
    │   ├── settings.rs
    │   ├── export_import.rs
    │   ├── backup.rs
    │   ├── plugin.rs
    │   ├── ocr.rs
    │   ├── llm.rs
    │   ├── sync.rs
    │   ├── log.rs
    │   └── system.rs
    ├── services/
    │   ├── mod.rs
    │   ├── auth_service.rs
    │   ├── vault_service.rs
    │   ├── profile_service.rs
    │   ├── unified_object_service.rs
    │   ├── search_service.rs
    │   ├── settings_service.rs
    │   ├── export_import_service.rs
    │   ├── backup_service.rs
    │   ├── plugin_service.rs
    │   ├── ocr_service.rs
    │   ├── llm_service.rs
    │   └── sync_service.rs
    ├── core/
    │   ├── mod.rs
    │   ├── crypto/       # 临时，Phase 2 后删除
    │   ├── vault/        # 临时，Phase 3 后删除
    │   ├── profile/
    │   ├── plugin/
    │   ├── ocr/
    │   ├── sync/         # 临时，Phase 4 后删除
    │   └── utils/
    ├── db/
    │   ├── mod.rs
    │   ├── connection.rs
    │   ├── migrations.rs
    │   └── repositories/
    ├── state/
    │   ├── mod.rs
    │   ├── app_state.rs
    │   ├── vault_state.rs
    │   └── session_state.rs
    └── ipc/
        ├── mod.rs
        ├── events.rs
        └── streams.rs
```

### main.rs

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        // 插件
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        
        // 初始化状态
        .setup(|app| {
            let app_state = state::AppState::new(app.handle().clone())?;
            app.manage(app_state);
            Ok(())
        })
        
        // IPC 命令
        .invoke_handler(tauri::generate_handler![
            // 认证
            commands::auth::bootstrap,
            commands::auth::login,
            commands::auth::logout,
            
            // Vault
            commands::vault::unlock,
            commands::vault::lock,
            commands::vault::change_password,
            commands::vault::delete_account,
            commands::vault::list_accounts,
            
            // Profile
            commands::profile::get,
            commands::profile::update,
            commands::profile::get_section,
            commands::profile::update_field,
            
            // UnifiedObject
            commands::unified_object::list,
            commands::unified_object::get,
            commands::unified_object::create,
            commands::unified_object::update,
            commands::unified_object::delete,
            commands::unified_object::get_section_data,
            commands::unified_object::update_field,
            
            // Search
            commands::search::unified_search,
            commands::search::advanced_search,
            
            // Settings
            commands::settings::get_all,
            commands::settings::get,
            commands::settings::update,
            commands::settings::reset_to_default,
            
            // Export/Import
            commands::export_import::export_data,
            commands::export_import::import_data,
            
            // Backup
            commands::backup::list,
            commands::backup::create,
            commands::backup::restore,
            commands::backup::delete,
            
            // Plugin
            commands::plugin::list,
            commands::plugin::install,
            commands::plugin::uninstall,
            commands::plugin::run,
            commands::plugin::approve_consent,
            
            // OCR
            commands::ocr::initialize,
            commands::ocr::scan,
            commands::ocr::get_status,
            
            // LLM
            commands::llm::get_config,
            commands::llm::update_config,
            commands::llm::send_message,
            commands::llm::get_usage,
            
            // Sync
            commands::sync::get_local_info,
            commands::sync::discover,
            commands::sync::sync_with_device,
            commands::sync::get_logs,
            
            // Log
            commands::log::get_recent,
            commands::log::export,
            
            // System
            commands::system::get_app_info,
            commands::system::check_version,
        ])
        
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### commands/profile.rs

```rust
use tauri::State;
use crate::state::AppState;
use crate::services::profile_service::ProfileService;

#[tauri::command]
pub async fn profile_get(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<ProfileData, String> {
    let service = ProfileService::new(state.db_pool());
    service.get_profile(&account_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_update(
    state: State<'_, AppState>,
    account_id: String,
    data: ProfileData,
) -> Result<(), String> {
    let service = ProfileService::new(state.db_pool());
    service.update_profile(&account_id, data).await.map_err(|e| e.to_string())
}
```

### state/app_state.rs

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    handle: tauri::AppHandle,
    vault_state: Arc<RwLock<VaultRuntimeState>>,
    session_state: Arc<RwLock<SessionState>>,
}

impl AppState {
    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        Ok(Self {
            handle,
            vault_state: Arc::new(RwLock::new(VaultRuntimeState::default())),
            session_state: Arc::new(RwLock::new(SessionState::default())),
        })
    }
    
    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.handle
    }
    
    pub fn vault_state(&self) -> &Arc<RwLock<VaultRuntimeState>> {
        &self.vault_state
    }
    
    pub fn db_pool(&self) -> &DbPool {
        // ...
    }
}
```

---

## 模块依赖图

```
src-tauri
├── commands/       ──→ services/ ──→ core/ ──→ crates/
│                     │
│                     └── db/
│                         │
│                         └── repositories/
│
├── state/          ──→ 全局状态（Mutex/RwLock 保护）
│
├── ipc/            ──→ 事件定义、Channel 类型
│
└── core/           ──→ 业务逻辑（无 Tauri 依赖，可单元测试）
    ├── crypto/     ──→ 临时（Phase 2 迁移到 solosoul-crypto）
    ├── vault/      ──→ 临时（Phase 3 迁移到 solosoul-vault）
    ├── profile/    ──→ Profile 数据模型 + 验证
    ├── plugin/     ──→ Wasmtime 沙盒
    ├── ocr/        ──→ ONNX 引擎
    ├── sync/       ──→ 临时（Phase 4 迁移到 solosoul-sync）
    └── utils/      ──→ 错误、路径、时间工具

crates/
├── solosoul-crypto ──→ 密码学（无外部依赖）
├── solosoul-vault  ──→ 存储（依赖 crypto）
└── solosoul-sync   ──→ 同步（依赖 crypto + vault）
```

---

## 错误处理规范

### 原则

1. **库级错误**: 使用 `thiserror` 定义精确错误类型
2. **应用级错误**: 使用 `anyhow` 快速传播
3. **IPC 错误**: 统一转换为 `String` 返回前端

### 错误类型层次

```rust
// crates/solosoul-crypto/src/lib.rs
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("KDF 失败: {0}")]
    Kdf(#[from] argon2::Error),
    #[error("加密失败")]
    Encryption,
    #[error("解密失败")]
    Decryption,
    #[error("无效的密钥")]
    InvalidKey,
}

// src-tauri/src/core/utils/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("密码学错误: {0}")]
    Crypto(#[from] solosoul_crypto::CryptoError),
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
    #[error("{0}")]
    Other(String),
}

// IPC 命令返回
#[tauri::command]
pub async fn some_command() -> Result<SomeData, String> {
    let result = do_something().await?;
    Ok(result)
}
```

---

## 从零实现顺序

### Phase 1: 项目骨架（1-2 天）

1. 初始化 Tauri 项目
2. 配置 Workspace（单 crate 先跑通）
3. 添加依赖（tauri + tokio + rusqlite）
4. 配置 tauri-specta 类型生成
5. 测试 `invoke` / `listen` 基本调用

### Phase 2: 密码学核心（3-5 天）

6. 从 `flutter/native/src/crypto/` 迁移代码
7. 封装为 `solosoul-crypto` crate
8. 添加单元测试（Argon2id 派生 + AES-GCM 往返）
9. 验证与现有 Flutter 实现结果一致

### Phase 3: Vault 存储（3-5 天）

10. 从 `flutter/native/src/vault/` 迁移代码
11. 封装为 `solosoul-vault` crate
12. SQLite + SQLCipher 集成
13. 实现 Vault 生命周期（init/unlock/lock/changePassword）
14. 单元测试

### Phase 4: 业务服务（5-7 天）

15. 实现 IPC 命令骨架（所有命令签名 + 空实现）
16. 实现 services/ 层
17. 实现 core/ 层（profile, plugin, ocr）
18. 实现 db/ 层（连接 + 迁移 + 仓库）
19. 集成测试

### Phase 5: 提取独立 Crate（后续）

20. 提取 `solosoul-crypto`
21. 提取 `solosoul-vault`
22. 提取 `solosoul-sync`

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*对应重构阶段：Phase 1-2（项目初始化 + 核心库）*
