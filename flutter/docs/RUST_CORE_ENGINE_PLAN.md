# Rust 核心引擎 + 可替换前端架构方案

> 最后更新：2026-05-01 (v2 — 吸收审查反馈)
> 状态：设计审查阶段
> 关联：LAN 同步方案的前置架构重构

## 1. 背景与动机

### 1.1 当前架构的问题

经过代码审查，SoloSoul 当前架构存在以下结构性问题：

| 问题 | 影响 | 严重度 |
|------|------|--------|
| **加密逻辑分裂** | Dart 层持有 `_encryptionKey` 并执行加密，Rust 只存已加密 blob | 高 |
| **密码验证重复** | `auth_storage.dart` 和 `account/manager.rs` 各自实现一套验证逻辑 | 高 |
| **Android/Windows 完整降级** | 使用 PBKDF2（非 Argon2id）+ JSON 文件（非 SQLite），与 iOS/macOS 不兼容 | 高 |
| **FFI 层无类型安全** | JSON relay 模式绕过 flutter_rust_bridge，无编译时检查 | 中 |
| **account 元数据双写** | Keychain 和 Rust config.json 各存一份，需手动合并 | 中 |
| **同步未实现** | Rust `SyncEngine` 是 stub，无实际逻辑 | 高 |

### 1.2 目标架构

```
┌──────────────────────────────────────────────────┐
│              前端壳 (Swappable)                    │
│  ┌────────────┐ ┌──────────┐ ┌────────────┐      │
│  │ Flutter UI │ │ SwiftUI  │ │ Compose    │      │
│  │ (当前)     │ │ (未来)   │ │ (未来)     │      │
│  └─────┬──────┘ └────┬─────┘ └──────┬─────┘      │
│        └─────────────┼──────────────┘             │
│                      │ FFI (C-ABI)                │
├──────────────────────┼────────────────────────────┤
│        Rust 核心引擎 (solosoul_core)               │
│  ┌───────────────────┴─────────────────────────┐  │
│  │ crypto/    加密引擎 (Argon2id, AES-256-GCM) │  │
│  │ vault/     数据存储 (SQLite, 迁移, CRUD)     │  │
│  │ account/   账户管理 (创建, 解锁, 密码变更)    │  │
│  │ sync/      同步引擎 (CRDT, Noise, 传输)      │  │
│  │ discovery/ 设备发现 (mDNS)                   │  │
│  │ security/  安全设置, 生物识别凭据管理        │  │
│  └─────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

核心原则：**所有机密计算、数据持久化、同步逻辑均在 Rust 内完成。前端只负责 UI 渲染和用户交互。**

---

## 2. 现状分析

### 2.1 Rust 层已实现的能力

| 模块 | 能力 | 成熟度 |
|------|------|--------|
| `crypto/argon2.rs` | Argon2id 密钥派生 + FFI | ✅ 生产就绪 |
| `crypto/aes.rs` | AES-256-GCM 加解密 + FFI | ✅ 生产就绪 |
| `crypto/utils.rs` | 随机字节、恒定时间比较 | ✅ 生产就绪 |
| `account/manager.rs` | 账户 CRUD、解锁/锁定、密码变更 | ✅ 基本完整 |
| `vault/store.rs` | SQLite 持久化 (profile/history/setting) | ✅ 基本完整 |
| `vault/processor.rs` | JSON relay 调度 (22 个 action) | ✅ 生产就绪 |
| `vault/profile.rs` | 完整数据 schema | ✅ 完整 |
| `vault/migration.rs` | Schema 迁移 (v1→v2→v3) | ✅ 完整 |
| `sync/engine.rs` | 同步引擎 | ❌ stub |
| `sync/protocol.rs` | 同步协议类型 | ❌ 仅类型定义 |
| `plugin/` | Wasm 插件沙箱 | ⚠️ feature-gated |

### 2.2 Dart 层仍承担的职责

| 职责 | 文件 | 应迁移到 Rust |
|------|------|:---:|
| Profile 加解密 | `rust_vault_service.dart:76-128` | ✅ |
| Android/Windows 密钥派生 (PBKDF2) | `native_crypto_service.dart` | ✅ 统一为 Argon2id |
| Android/Windows 文件存储 (JSON) | `native_vault_service.dart:479-1077` | ✅ 统一为 SQLite |
| 密码验证 | `auth_storage.dart:309-416` | ✅ |
| Account 元数据 (Keychain) | `auth_storage.dart` | ✅ 统一到 Rust |
| 备份加密 | `backup_service.dart` | ✅ |
| Field history 加密 | `field_history_service.dart` | ✅ |
| 生物识别凭据 | `biometric_credential_service.dart` | ⚠️ 部分 |
| 安全设置 | `security_service.dart` | ⚠️ 可选 |

### 2.3 FFI 现状

当前使用 **手动 C-ABI FFI**（4 个导出函数）+ **JSON relay 模式**（22 个 action）。

```rust
// 当前模式：单入口 JSON relay
#[no_mangle]
pub extern "C" fn vault_request_ffi(request_ptr: *const u8, request_len: usize) -> *mut c_char {
    // 解析 JSON → 匹配 action → 执行 → 返回 JSON
}
```

**问题**：无类型安全，JSON 序列化开销大，调试困难。

---

## 3. 目标 FFI 设计

### 3.1 设计原则

1. **类型安全优先**：使用 flutter_rust_bridge (FRB) 自动生成 Dart 绑定
2. **最小暴露面**：只暴露必要的公共 API，内部实现不泄露
3. **异步优先**：所有 I/O 操作返回 Future，不阻塞 UI 线程
4. **句柄模式**：Rust 对象通过 opaque handle 暴露，前端不直接操作内存
5. **回调通知**：Rust 通过 Stream 推送事件（同步进度、设备发现等）

### 3.2 FFI 接口分层

#### 层 1：核心引擎 API（FRB 生成）

```rust
// ===================== 账户生命周期 =====================

/// 初始化核心引擎，传入数据根目录
#[frb]
pub fn engine_init(data_root: String) -> Result<()>;

/// 创建账户，返回账户 ID
#[frb]
pub fn account_create(name: String, password: String) -> Result<String>;

/// 解锁账户（密码验证 + 密钥派生）
#[frb]
pub fn account_unlock(account_id: String, password: String) -> Result<()>;

/// 生物识别解锁（使用已缓存的 session key）
#[frb]
pub fn account_unlock_with_biometric(account_id: String, session_key: Vec<u8>) -> Result<()>;

/// 锁定当前账户（清零内存中的密钥）
#[frb]
pub fn account_lock() -> Result<()>;

/// 变更密码
#[frb]
pub fn account_change_password(old_password: String, new_password: String) -> Result<()>;

/// 列出所有账户
#[frb]
pub fn account_list() -> Result<Vec<AccountSummary>>;

/// 删除账户
#[frb]
pub fn account_delete(account_id: String) -> Result<()>;

/// 检查是否已解锁
#[frb]
pub fn account_is_unlocked() -> bool;

// ===================== 数据 CRUD =====================

/// 保存 profile（Rust 内部加密）
#[frb]
pub fn profile_save(account_id: String, profile: ProfileData) -> Result<()>;

/// 加载 profile（Rust 内部解密）
#[frb]
pub fn profile_load(account_id: String) -> Result<ProfileData>;

/// 列出所有 profile
#[frb]
pub fn profile_list(account_id: String) -> Result<Vec<ProfileSummary>>;

/// 删除 profile
#[frb]
pub fn profile_delete(account_id: String, profile_id: String) -> Result<()>;

/// 搜索 profile
#[frb]
pub fn profile_search(account_id: String, query: String) -> Result<Vec<ProfileSummary>>;

/// 保存 field histories
#[frb]
pub fn field_histories_save(account_id: String, histories: FormHistories) -> Result<()>;

/// 加载 field histories
#[frb]
pub fn field_histories_load(account_id: String) -> Result<FormHistories>;

/// 保存设置
#[frb]
pub fn setting_save(account_id: String, key: String, value: String) -> Result<()>;

/// 加载设置
#[frb]
pub fn setting_load(account_id: String, key: String) -> Result<Option<String>>;

// ===================== 备份 =====================

/// 创建加密备份
#[frb]
pub fn backup_create(account_id: String) -> Result<String>; // 返回备份文件路径

/// 恢复备份
#[frb]
pub fn backup_restore(backup_path: String, account_id: String) -> Result<()>;

/// 列出备份
#[frb]
pub fn backup_list(account_id: String) -> Result<Vec<BackupEntry>>;

// ===================== 同步 =====================

/// 开始设备发现
#[frb]
pub fn sync_start_discovery() -> Result<Stream<DiscoveryEvent>>;

/// 停止设备发现
#[frb]
pub fn sync_stop_discovery() -> Result<()>;

/// 发起同步连接
#[frb]
pub fn sync_connect(peer_id: String) -> Result<Stream<SyncEvent>>;

/// 执行同步
#[frb]
pub fn sync_execute(account_id: String) -> Result<SyncResult>;

/// 配对设备（输入配对码）
#[frb]
pub fn sync_pair(code: String) -> Result<()>;

// ===================== 安全设置 =====================

/// 保存安全设置
#[frb]
pub fn security_save_settings(settings: SecuritySettings) -> Result<()>;

/// 加载安全设置
#[frb]
pub fn security_load_settings() -> Result<SecuritySettings>;

/// 获取生物识别 session key（用于安全存储）
#[frb]
pub fn security_get_biometric_key(account_id: String) -> Result<Option<Vec<u8>>>;

/// 存储生物识别 session key
#[frb]
pub fn security_store_biometric_key(account_id: String, key: Vec<u8>) -> Result<()>;
```

#### 层 2：数据类型定义（FRB 自动生成 Dart 类）

```rust
/// 账户摘要（不含敏感信息）
#[frb(dart_metadata = ("freezed"))]
pub struct AccountSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_unlock_at: Option<String>,
    pub has_biometric: bool,
}

/// Profile 数据（与现有 Dart ProfileData 对齐）
#[frb(dart_metadata = ("freezed"))]
pub struct ProfileData {
    pub id: String,
    pub name: String,
    pub type_id: String,
    pub icon_name: String,
    pub properties: HashMap<String, PropertyValue>,
    pub children_ids: Vec<String>,
    pub parent_id: Option<String>,
    pub is_deleted: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub version: i32,
}

#[frb(dart_metadata = ("freezed"))]
pub enum PropertyValue {
    Text { text: String, sensitivity: SensitivityLevel },
    Number { value: f64 },
    Boolean { value: bool },
    Date { timestamp: i64 },
    Select { selected: String, options: Vec<String> },
    MultiSelect { selected: Vec<String>, options: Vec<String> },
}

#[frb(dart_metadata = ("freezed"))]
pub enum SensitivityLevel { Public, Private, Restricted }

/// Field Histories
#[frb(dart_metadata = ("freezed"))]
pub struct FormHistories {
    pub histories: HashMap<String, HashMap<String, FieldHistory>>,
}

#[frb(dart_metadata = ("freezed"))]
pub struct FieldHistory {
    pub entries: Vec<FieldHistoryEntry>,
}

#[frb(dart_metadata = ("freezed"))]
pub struct FieldHistoryEntry {
    pub values: HashMap<String, String>,
    pub timestamp: i64,
}

/// 备份条目
#[frb(dart_metadata = ("freezed"))]
pub struct BackupEntry {
    pub file_name: String,
    pub created_at: String,
    pub size_bytes: i64,
}

/// 安全设置
#[frb(dart_metadata = ("freezed"))]
pub struct SecuritySettings {
    pub auto_lock_seconds: i32,
    pub clipboard_clear_seconds: i32,
    pub biometric_enabled: bool,
    pub hide_sensitive_fields: bool,
}

/// 同步相关事件
#[frb(dart_metadata = ("freezed"))]
pub enum DiscoveryEvent {
    PeerFound { peer_id: String, name: String, device_type: String },
    PeerLost { peer_id: String },
}

#[frb(dart_metadata = ("freezed"))]
pub enum SyncEvent {
    Connected { peer_id: String },
    Authenticating,
    Syncing { progress: f64 },
    Completed { result: SyncResult },
    Error { message: String },
}

#[frb(dart_metadata = ("freezed"))]
pub struct SyncResult {
    pub success: bool,
    pub direction: SyncDirection,
    pub bytes_transferred: i64,
    pub error: Option<String>,
}

#[frb(dart_metadata = ("freezed"))]
pub enum SyncDirection {
    Upload,
    Download,
    NoChange,
    ConflictResolved { chosen: ConflictChoice },
}

#[frb(dart_metadata = ("freezed"))]
pub enum ConflictChoice { Local, Remote, Merged }

/// Profile 摘要
#[frb(dart_metadata = ("freezed"))]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub type_id: String,
    pub updated_at: i64,
    pub version: i32,
}
```

### 3.3 内部 Rust 架构

```
native/src/
├── lib.rs                    # 模块声明 + FRB 入口
├── frb_generated.rs          # FRB 自动生成
├── engine/
│   ├── mod.rs                # Engine 单例，持有所有子系统
│   ├── state.rs              # 全局状态管理 (解锁状态, 当前账户)
│   └── event_bus.rs          # 内部事件总线 (同步事件, 发现事件)
├── crypto/
│   ├── mod.rs                # 现有
│   ├── argon2.rs             # 现有，增强：统一密钥派生
│   ├── aes.rs                # 现有，增强：profile 加解密移入
│   └── utils.rs              # 现有
├── vault/
│   ├── mod.rs                # 现有
│   ├── store.rs              # 现有，增强：Android/Windows 统一使用 SQLite
│   ├── processor.rs          # 移除（被 FRB 直接调用替代）
│   ├── profile.rs            # 现有
│   └── migration.rs          # 现有
├── account/
│   ├── mod.rs                # 现有
│   ├── manager.rs            # 现有，增强：统一密码验证 + 元数据
│   └── config.rs             # 新增：统一 account config（含 Keychain 元数据）
├── sync/
│   ├── mod.rs
│   ├── engine.rs             # 重写：实际同步逻辑
│   ├── protocol.rs           # 重写：Noise IK + 传输协议
│   ├── crdt.rs               # 新增：yrs 集成
│   ├── conflict.rs           # 新增：冲突解决
│   └── transport.rs          # 新增：TCP/QUIC 传输
├── discovery/
│   ├── mod.rs                # 新增
│   └── mdns.rs               # 新增：mDNS 服务发现
├── security/
│   ├── mod.rs                # 新增
│   ├── settings.rs           # 新增：安全设置持久化
│   └── biometric.rs          # 新增：生物识别凭据管理
├── backup/
│   ├── mod.rs                # 新增
│   └── service.rs            # 新增：备份创建/恢复/列表
└── platform/
    ├── mod.rs                # 新增
    ├── android.rs            # 新增：Android 特定逻辑
    ├── ios.rs                # 新增：iOS 特定逻辑
    ├── macos.rs              # 新增：macOS 特定逻辑
    └── windows.rs            # 新增：Windows 特定逻辑
```

### 3.4 Cargo.toml 变更

```toml
[package]
name = "solosoul_core"
version = "2.0.0"

[dependencies]
# 现有
argon2 = "0.5"
aes-gcm = "0.10"
rand = "0.8"
zeroize = "1.7"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
flutter_rust_bridge = "=2.12.0"
thiserror = "2"
anyhow = "1"

# 新增：CRDT
yrs = "0.21"                    # Yjs Rust 实现，CRDT 文档管理

# 新增：网络加密协议
snow = "0.9"                    # Noise Protocol Framework 实现

# 新增：mDNS 发现
mdns-sd = "0.11"                # 跨平台 mDNS/SD

# 新增：异步运行时
tokio = { version = "1", features = ["rt-multi-thread", "net", "sync", "macros"] }

# 新增：传输层
quinn = "0.11"                  # QUIC 传输（可选，后续升级）

# 新增：日志
tracing = "0.1"                 # 结构化日志（替代 log crate）
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"        # 日志文件轮转

[features]
default = []
sandbox = ["wasmtime", "wasmtime-wasi"]
```

---

## 4. 迁移计划

### Phase 1：统一加密层（最高优先级）

**目标**：所有加密操作统一在 Rust 内完成，消除 Dart 层加密。

#### 1.1 扩展 `vault/processor.rs` 新增 action

```rust
// 新增 action：
"encrypt_data"    → 直接在 Rust 内加密，返回 nonce(12B) + ciphertext
"decrypt_data"    → 直接在 Rust 内解密，返回明文
"verify_password" → 统一密码验证入口
"export_vault"    → 导出整个账户的加密 blob（用于同步/备份）
"import_vault"    → 导入加密 blob 到账户
"migrate_encryption" → 将旧格式数据重加密为新格式（见 1.5）
```

#### 1.5 加密格式向后兼容（关键）

Dart 层的加密结果与 Rust 的 **可能不一致**：
- iOS/macOS：Dart 调用 Rust FFI 做 AES-256-GCM → 密钥和算法一致，**可直接解密**
- Android/Windows：Dart 使用 PBKDF2 (600K iterations) 派生密钥，而 Rust 使用 Argon2id → **密钥不同，无法直接解密**

**迁移策略**：

```rust
/// 首次解锁时自动检测并迁移加密格式
pub fn migrate_encryption(account_id: &str) -> Result<MigrationReport> {
    // 1. 尝试用当前密钥解密每条 profile
    // 2. 解密失败 → 检测是否为旧格式 (PBKDF2 派生的密钥)
    //    a. 读取 config.json 中的 crypto_version
    //    b. 若 crypto_version == 1 (PBKDF2)，使用旧参数派生密钥尝试解密
    //    c. 解密成功 → 用新密钥 (Argon2id) 重新加密 → 写回
    // 3. 解密成功 → 已是新格式，跳过
    // 4. 更新 crypto_version 为 2
    // 5. 返回迁移报告 (已迁移 N 条, 跳过 M 条)
}
```

**触发时机**：
- `account_unlock()` 成功后自动检查 `crypto_version`
- 若 `crypto_version < 2`，后台执行 `migrate_encryption()`
- 迁移期间锁定写操作，防止数据不一致
- 迁移完成后更新 `crypto_version`

**数据安全保障**：
- 迁移前自动创建一次性备份（使用旧密钥加密）
- 迁移失败 → 回滚到旧格式，不丢失数据
- 迁移报告写入审计日志

#### 1.2 修改 `RustVaultService` (Dart)

```dart
// BEFORE: Dart 加密
Uint8List? _encryptData(Uint8List data) {
  final nonce = NativeCryptoService.instance.generateSalt();
  final nonce12 = Uint8List.fromList(nonce.sublist(0, 12));
  final encrypted = NativeCryptoService.instance.encrypt(data: data, key: _encryptionKey!, nonce: nonce12);
  // ...
}

// AFTER: 委托 Rust 加密
Future<Uint8List?> _encryptData(Uint8List data) async {
  final result = await NativeVaultService.instance.request('encrypt_data', {'data': base64Encode(data)});
  return result != null ? base64Decode(result['encrypted']) : null;
}
```

#### 1.3 删除 Dart 层加密代码

- `NativeCryptoService` 中的 Dart PBKDF2/AES fallback → 删除
- `RustVaultService._encryptionKey` → 移到 Rust 内部
- `auth_storage.dart` 中的 `verifyPassword()` → 委托 Rust

#### 1.4 统一 Android/Windows

- Android/Windows 也通过 Rust FFI 调用，不再使用 Dart fallback
- 需要确保 Rust 库在 Android (NDK) 和 Windows (MSVC) 上正确编译
- 验证 Argon2id 在 Android NDK 上的性能（16 MiB 内存是否合适）

**产出文件**：
- `native/src/vault/processor.rs` — 新增 encrypt/decrypt/verify actions
- `native/src/crypto/mod.rs` — 新增 `encrypt_profile_data()` / `decrypt_profile_data()` 公共 API
- `lib/core/services/rust_vault_service.dart` — 移除 Dart 加密，委托 Rust
- `lib/core/services/native_crypto_service.dart` — 精简为仅 FFI 包装（或删除）
- `lib/core/services/native_vault_service.dart` — 删除 Android/Windows fallback

---

### Phase 2：统一账户管理

**目标**：消除 Keychain 双写，Rust 成为账户数据唯一来源。

#### 2.1 扩展 Rust `AccountConfig`

```rust
pub struct AccountConfig {
    // 现有
    pub salt: String,
    pub verify_hash: String,
    pub crypto_version: i32,
    // 新增（从 Keychain 迁移）
    pub password_hint: Option<String>,
    pub last_unlock_at: Option<String>,
    pub recent_devices: Vec<DeviceInfo>,
    pub biometric_enabled: bool,
    pub biometric_session_key_hash: Option<String>,
}
```

#### 2.2 修改 `auth_storage.dart`

```dart
// BEFORE: 双写 Keychain + Rust
await _secureStorage.write(key: 'solosoul_account_$id', value: jsonEncode(config));
await NativeVaultService.instance.request('save_setting', ...);

// AFTER: 仅写 Rust
await NativeVaultService.instance.request('save_account_config', config.toJson());
```

#### 2.3 生物识别凭据

生物识别 session key 仍需存储在平台安全存储（Keychain / EncryptedSharedPrefs）中，因为：
- 它是设备绑定的，不应随 vault 同步
- 它需要在 Rust 未解锁时也能读取

方案：Rust 提供 `security_store_biometric_key()` / `security_get_biometric_key()`，内部调用平台安全存储。通过 **FFI 回调** 让前端层执行实际的 secure storage 操作。

**回调签名定义**：

```rust
// Rust 侧：注册平台回调
pub type SecureStoreFn = unsafe extern "C" fn(
    key: *const c_char,     // 存储键名
    value: *const u8,       // 数据指针
    len: usize,             // 数据长度
) -> bool;                  // 返回是否成功

pub type SecureLoadFn = unsafe extern "C" fn(
    key: *const c_char,     // 存储键名
    out_len: *mut usize,    // 输出：数据长度
) -> *mut u8;               // 返回数据指针（前端分配，Rust 通过 free 回调释放）

pub type SecureDeleteFn = unsafe extern "C" fn(
    key: *const c_char,     // 存储键名
) -> bool;

/// 注册平台安全存储回调（在 engine_init 时调用）
#[frb]
pub fn platform_register_callbacks(
    store: SecureStoreFn,
    load: SecureLoadFn,
    delete: SecureDeleteFn,
);
```

```dart
// Dart 侧：注册回调
final storeFn = NativeFunction<Bool Function(Pointer<Utf8>, Pointer<Utf8>, IntPtr)>.isolateLookup('dartSecureStore');
final loadFn  = NativeFunction<Pointer<Utf8> Function(Pointer<Utf8>, Pointer<IntPtr>)>.isolateLookup('dartSecureLoad');

SolosoulCore.instance.platformRegisterCallbacks(
  store: storeFn,
  load: loadFn,
  delete: deleteFn,
);
```

**线程安全注意事项**：
- Dart FFI 回调必须在 **Dart isolate 线程** 上执行，不能在 Rust 的 tokio 异步线程上调用
- Rust 侧需要通过 `Isolate.spawn` 或 Dart 的 `NativeCallable.listener` 机制将回调调度到正确的线程
- 推荐使用 `flutter_rust_bridge` 的 `StreamSink` 机制替代原始 C 回调，由 FRB 处理线程调度

**产出文件**：
- `native/src/account/config.rs` — 新增，统一 AccountConfig
- `native/src/account/manager.rs` — 扩展，新增元数据字段
- `lib/presentation/providers/auth/auth_storage.dart` — 精简，移除 Keychain 双写
- `lib/presentation/providers/auth/auth_services.dart` — 精简，移除合并逻辑

---

### Phase 3：消除 Dart Fallback

**目标**：所有平台统一使用 Rust 核心引擎。

#### 3.1 Android NDK 编译验证

```bash
# 验证 Rust 库在 Android 上编译
cd native
cargo build --target aarch64-linux-android --release
cargo build --target x86_64-linux-android --release
```

关键验证点：
- Argon2id 16 MiB 内存在 Android 上的性能（目标：< 2 秒）
- rusqlite bundled SQLite 编译无问题
- 所有依赖的 Android 兼容性

#### 3.2 Argon2id 参数可配置

16 MiB 内存可能使某些低端 Android 设备解锁时间超过 2 秒。参考 Bitwarden 的做法，提供可配置参数：

```rust
/// 密钥派生参数（存储在 account config.json 中）
pub struct KdfParams {
    pub algorithm: KdfAlgorithm,    // Argon2id (默认) 或 PBKDF2 (兼容旧数据)
    pub memory_kib: u32,            // 默认 65536 (64 MiB for desktop), 16384 (16 MiB for mobile)
    pub iterations: u32,            // Argon2id: 3, PBKDF2: 600000
    pub parallelism: u32,           // Argon2id: 4
}

impl KdfParams {
    /// 根据平台自动选择合适的默认参数
    pub fn platform_defaults() -> Self {
        match Platform::detect() {
            Platform::Desktop => KdfParams {
                algorithm: KdfAlgorithm::Argon2id,
                memory_kib: 65536,  // 64 MiB
                iterations: 3,
                parallelism: 4,
            },
            Platform::Mobile => KdfParams {
                algorithm: KdfAlgorithm::Argon2id,
                memory_kib: 16384,  // 16 MiB
                iterations: 3,
                parallelism: 4,
            },
        }
    }
}
```

在 `SecuritySettings` 中暴露给用户：

```rust
pub struct SecuritySettings {
    // ... 现有字段
    /// KDF 参数级别：Fast / Balanced / Secure
    pub kdf_preset: KdfPreset,
}

pub enum KdfPreset {
    Fast,       // 8 MiB, 2 iterations — 低端设备
    Balanced,   // 16 MiB, 3 iterations — 默认
    Secure,     // 64 MiB, 3 iterations — 高安全需求
}
```

#### 3.3 Windows MSVC 编译验证

```bash
cargo build --target x86_64-pc-windows-msvc --release
```

#### 3.4 验证 Dart 依赖无隐式使用

删除 Dart 加密库前，确认无其他地方隐式依赖：

```bash
# 检查依赖树，确认 crypto/encrypt/pointycastle 无传递依赖
flutter pub deps -- --style=compact | grep -E "crypto|encrypt|pointycastle|cryptography"

# 运行全量测试确认无断裂
flutter test
```

#### 3.5 删除 Dart Fallback 代码

删除 `native_vault_service.dart` 中约 600 行的 Android/Windows fallback 代码：
- `_androidListProfiles()`, `_androidSaveProfile()`, `_androidLoadProfile()`, `_androidDeleteProfile()`
- `_windowsListProfiles()`, `_windowsSaveProfile()`, etc.
- `createAccountAsync()`, `unlockVaultAsync()`, etc.
- `FallbackSecureStorage` 的文件 fallback

**产出文件**：
- `native/.cargo/config.toml` — 确认 Android NDK targets
- `lib/core/services/native_vault_service.dart` — 删除 ~600 行 fallback 代码
- `lib/core/services/fallback_secure_storage.dart` — 删除或精简为仅 Keychain 包装

---

### Phase 4：迁移至 FRB 代码生成

**目标**：从手动 JSON relay 迁移到 flutter_rust_bridge 自动生成的类型安全 API。

#### 4.1 配置 FRB

```yaml
# flutter_rust_bridge.yaml
rust_input: crate::lib
rust_root: native
dart_output: lib/frb/
dart_enums_style: true
```

#### 4.1.1 FRB 原型验证（必须先于大规模迁移）

FRB 对复杂类型（嵌套泛型、枚举带数据）的支持有时需要额外配置，且版本升级可能引入破坏性变化。在全面迁移前，先验证最复杂的类型：

```rust
// 原型测试 1：枚举带数据（最可能出问题的类型）
#[frb(dart_metadata = ("freezed"))]
pub enum PropertyValue {
    Text { text: String, sensitivity: SensitivityLevel },
    Number { value: f64 },
    Boolean { value: bool },
    // ...
}

// 原型测试 2：嵌套 HashMap<String, HashMap<String, T>>
#[frb(dart_metadata = ("freezed"))]
pub struct FormHistories {
    pub histories: HashMap<String, HashMap<String, FieldHistory>>,
}

// 原型测试 3：Stream 返回（用于同步事件推送）
#[frb]
pub fn sync_events() -> Stream<SyncEvent>;
```

验证清单：
- [ ] `PropertyValue` 枚举的 Dart freezed 类是否正确生成
- [ ] 嵌套 `HashMap` 的序列化/反序列化是否工作
- [ ] `Stream<T>` 的 Dart 端是否生成正确的 `Stream` 类型
- [ ] `Option<Vec<u8>>` 等嵌套可选类型是否正确映射
- [ ] `Result<T, E>` 的 Dart 错误处理是否符合预期

**若 FRB 无法处理某些类型**：保留少量 JSON relay 通道作为 fallback，但必须：
- 明确标记哪些 action 走 JSON relay（添加 `#[legacy_json_relay]` 注解）
- 在 Dart 侧添加类型包装层，对外 API 保持一致
- 设置移除计划：FRB 后续版本支持后立即迁移

```rust
// BEFORE: JSON relay
pub fn vault_request(json: String) -> String { ... }

// AFTER: 类型化 FRB 函数
#[frb]
pub fn profile_save(account_id: String, profile: ProfileData) -> Result<()> {
    let engine = get_engine()?;
    engine.vault.save_profile(&account_id, &profile)
}
```

#### 4.3 Dart 侧适配

```dart
// BEFORE: JSON 手动构造
final response = await NativeVaultService.instance.request('save_profile', {
  'name': name,
  'data': base64Encode(encryptedData),
});

// AFTER: 类型安全调用
await SolosoulCore.instance.profileSave(
  accountId: accountId,
  profile: profile.toRustModel(),
);
```

**产出文件**：
- `lib/frb/` — FRB 自动生成的 Dart 绑定
- `native/src/lib.rs` — 移除 `vault_request_ffi`，改为 FRB 入口
- `lib/core/services/native_vault_service.dart` — 重写为 FRB 包装
- 所有调用 `NativeVaultService.instance.request()` 的文件 — 迁移到新 API

---

### Phase 5：同步引擎（分 4 个子阶段）

**目标**：在 Rust 核心内实现完整的同步能力。
**风险提示**：CRDT + Noise + 增量同步的完整实现工作量可能被高估。拆分为 4 个可独立验证的子阶段，降低风险。

#### 5a：本地 CRDT 文档与 ProfileData 互转（2-3 天）

**目标**：实现 ProfileData ↔ CRDT 文档的双向转换，纯本地逻辑，无网络。

```rust
// native/src/sync/crdt.rs
use yrs::{Doc, Transact, Text, Map};

pub struct SoloDoc {
    doc: Doc,
    profile_map: Map,
    history_map: Map,
}

impl SoloDoc {
    /// 从 ProfileData 创建 CRDT 文档
    pub fn from_profile(profile: &ProfileData) -> Self { ... }

    /// 应用远端更新
    pub fn apply_update(&mut self, update: &[u8]) { ... }

    /// 生成本地更新
    pub fn encode_state_as_update(&self, state_vector: &[u8]) -> Vec<u8> { ... }

    /// 转换回 ProfileData
    pub fn to_profile(&self) -> ProfileData { ... }
}
```

**验证**：
- `from_profile()` → `to_profile()` roundtrip 数据一致
- 两个独立 SoloDoc 模拟双端编辑 → 交换 update → 合并后结果正确
- 嵌套属性（HashMap 内 HashMap）的 CRDT 映射正确
- 删除/软删除操作的 CRDT 语义正确

**产出**：`native/src/sync/crdt.rs` + 单元测试

#### 5b：Noise 握手与加密通道测试（2-3 天）

**目标**：实现 Noise IK 握手，验证加密通道在本地两个进程间工作。

```rust
// native/src/sync/protocol.rs
use snow::Builder;

pub struct SecureChannel {
    session: snow::TransportState,
}

impl SecureChannel {
    /// 作为 initiator 握手
    pub fn initiate(pairing_key: &[u8]) -> Result<(Self, Vec<u8>)> { ... }

    /// 作为 responder 握手
    pub fn respond(pairing_key: &[u8], handshake_msg: &[u8]) -> Result<(Self, Vec<u8>)> { ... }

    /// 加密传输数据
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> { ... }

    /// 解密接收数据
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> { ... }
}
```

**验证**：
- Noise IK 握手完成（initiator + responder 两端）
- 握手后双向加密/解密 roundtrip
- 篡改密文 → 解密失败
- 重放旧消息 → 检测并拒绝
- 配对码错误 → 握手失败

**产出**：`native/src/sync/protocol.rs` + 单元测试

#### 5c：基于 state vector 的增量同步逻辑（2-3 天）

**目标**：实现完整的同步流程，使用内存中的 mock transport 验证逻辑正确性。

```rust
// native/src/sync/engine.rs
pub struct SyncEngine {
    crdt: SoloDoc,
    channel: Option<SecureChannel>,
    transport: Box<dyn Transport>,  // trait，便于 mock
}

impl SyncEngine {
    /// 执行同步
    pub async fn sync(&mut self, account_id: &str) -> Result<SyncResult> {
        // 1. 交换状态向量
        let local_sv = self.crdt.state_vector();
        let remote_sv = self.exchange_state_vector(&local_sv).await?;

        // 2. 计算差量
        let update = self.crdt.encode_state_as_update(&remote_sv);

        // 3. 通过 Noise 信道传输
        let encrypted = self.channel.as_mut().unwrap().encrypt(&update);
        self.transport.send(&encrypted).await?;

        // 4. 接收远端差量
        let remote_encrypted = self.transport.recv().await?;
        let remote_update = self.channel.as_mut().unwrap().decrypt(&remote_encrypted)?;
        self.crdt.apply_update(&remote_update);

        // 5. 返回结果
        Ok(SyncResult { success: true, direction: SyncDirection::Merged, ... })
    }
}
```

**同步协议消息**：

```rust
enum SyncMessage {
    /// 交换状态向量
    StateVectorRequest { account_id: String },
    StateVectorResponse { state_vector: Vec<u8> },

    /// 传输更新
    UpdateRequest { encrypted_update: Vec<u8> },
    UpdateAck { success: bool },

    /// 冲突检测
    ConflictDetected { local_version: u32, remote_version: u32 },
    ConflictResolution { choice: ConflictChoice },
}
```

**验证**：
- 两端各有不同修改 → 同步后两端数据一致
- 一端无修改 → 同步方向为 NoChange
- 两端修改同一字段 → CRDT 自动合并（last-writer-wins on scalar）
- 大数据量（1000+ 对象）同步性能测试

**产出**：`native/src/sync/engine.rs` + `native/src/sync/conflict.rs` + 集成测试

#### 5d：整合到 mDNS 发现与 TCP 传输（1-2 天）

**目标**：将 mock transport 替换为真实 TCP 传输，接入 mDNS 设备发现。

```rust
// native/src/sync/transport.rs
pub struct TcpTransport {
    stream: tokio::net::TcpStream,
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> { ... }
    async fn recv(&mut self) -> Result<Vec<u8>> { ... }
}

// native/src/discovery/mdns.rs
pub struct MdnsDiscovery {
    service: mdns_sd::ServiceDaemon,
}

impl MdnsDiscovery {
    pub fn start_advertise(&self, device_name: &str, port: u16) -> Result<()> { ... }
    pub fn browse(&self) -> Result<Stream<DiscoveryEvent>> { ... }
    pub fn stop(&self) -> Result<()> { ... }
}
```

**验证**：
- 两台设备同 WiFi → mDNS 互相发现
- TCP 连接建立 → Noise 握手 → 同步完成
- WiFi 断开 → 超时处理 → 不崩溃
- 同步过程中一方退出 → 另一方优雅处理

**产出**：`native/src/sync/transport.rs` + `native/src/discovery/mdns.rs` + 手动测试

**Phase 5 总产出文件**：
- `native/src/sync/crdt.rs` — yrs CRDT 集成
- `native/src/sync/protocol.rs` — Noise IK 握手 + 传输协议
- `native/src/sync/engine.rs` — 重写同步引擎
- `native/src/sync/conflict.rs` — 冲突解决策略
- `native/src/sync/transport.rs` — TCP 传输层
- `native/src/discovery/mdns.rs` — mDNS 设备发现

---

### Phase 6：前端薄壳化

**目标**：Flutter UI 层精简为纯展示 + 用户交互。

#### 6.1 Dart 层删除清单

| 文件 | 操作 |
|------|------|
| `native_crypto_service.dart` | 删除（Rust 内部处理） |
| `native_vault_service.dart` | 重写为 FRB 薄包装 |
| `rust_vault_service.dart` | 删除（加密在 Rust 内） |
| `fallback_secure_storage.dart` | 删除（统一 Rust） |
| `profile_storage_service.dart` | 精简为 FRB 调用包装 |
| `backup_service.dart` | 精简为 FRB 调用包装 |
| `field_history_service.dart` | 精简为 FRB 调用包装 |
| `security_service.dart` | 精简为 FRB 调用包装 |
| `auth_storage.dart` | 精简，移除加密/验证逻辑 |
| `auth_services.dart` | 精简，移除迁移/合并逻辑 |

#### 6.2 保留的 Dart 职责

| 职责 | 原因 |
|------|------|
| UI 渲染 (Widget) | 平台特定 |
| 路由 (GoRouter) | Flutter 特定 |
| Riverpod 状态管理 | Flutter 特定 |
| 本地通知 UI | Flutter 特定 |
| 生物识别 prompt (local_auth) | 平台 UI |
| 平台 MethodChannel | 锁屏事件等 |

#### 6.3 Riverpod Provider 适配

```dart
// 现有模式保留，但底层调用改为 FRB
@riverpod
class ProfileNotifier extends _$ProfileNotifier {
  @override
  Future<ProfileData> build(String accountId) async {
    return await SolosoulCore.instance.profileLoad(accountId: accountId);
  }

  Future<void> save(ProfileData profile) async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      await SolosoulCore.instance.profileSave(accountId: accountId, profile: profile);
      return profile;
    });
  }
}
```

---

## 5. 未来平台迁移路径

### 5.1 SwiftUI (iOS/macOS 原生)

```
SoloSoul.xcodeproj
├── SoloSoulCore.xcframework     # Rust 编译的 XCFramework
│   ├── ios-arm64/
│   ├── ios-arm64-simulator/
│   └── macos-arm64/
├── SoloSoul/
│   ├── Bridging-Header.h       # C-ABI 桥接
│   ├── Views/                  # SwiftUI 视图
│   ├── ViewModels/             # ObservableObject
│   └── Services/               # Swift 包装层
└── Package.swift
```

Swift 调用示例：

```swift
// 通过 C 桥接调用 Rust
let result = solosoul_profile_load(accountId)
let profile = try JSONDecoder().decode(ProfileData.self, from: result!)
```

### 5.2 Jetpack Compose (Android 原生)

```
app/
├── src/main/
│   ├── cpp/
│   │   └── solosoul_bridge.cpp  # JNI 桥接
│   ├── java/com/solosoul/
│   │   ├── core/
│   │   │   └── SoloSoulCore.kt  # Rust FFI 包装
│   │   ├── ui/                  # Compose UI
│   │   └── viewmodel/           # ViewModel
│   └── jniLibs/
│       └── arm64-v8a/
│           └── libsolosoul_core.so
```

### 5.3 跨平台复用率

| 组件 | 复用率 | 说明 |
|------|--------|------|
| 加密引擎 | 100% | Rust 静态库，平台无关 |
| Vault 存储 | 100% | SQLite，平台无关 |
| CRDT 同步 | 100% | yrs，平台无关 |
| Noise 信道 | 100% | snow，平台无关 |
| mDNS 发现 | 100% | mdns-sd，平台无关 |
| FFI 绑定 | 需重写 | 每平台约 200-500 行桥接代码 |
| UI 层 | 需重写 | 但架构清晰，可并行开发 |

---

## 6. 依赖变更汇总

### 6.1 Rust 新增依赖

```toml
yrs = "0.21"                    # CRDT
snow = "0.9"                    # Noise Protocol
mdns-sd = "0.11"                # mDNS 服务发现
tokio = { version = "1", features = ["rt-multi-thread", "net", "sync", "macros"] }
tracing = "0.1"                 # 结构化日志
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"        # 日志文件轮转
```

### 6.2 Rust 移除依赖

```toml
# 移除（不再需要，同步用 Noise + TCP）
reqwest = "0.12"
tokio-tungstenite = "0.21"
```

### 6.3 Dart 新增依赖

```yaml
# 无新增。flutter_rust_bridge 已存在。
# bonsoir 不再需要（mDNS 在 Rust 内处理）
```

### 6.4 Dart 移除依赖

```yaml
# Phase 3 完成后可移除：
crypto: ^3.0.3                  # PBKDF2 fallback 不再需要
encrypt: ^5.0.3                 # Dart AES fallback 不再需要
pointycastle: ^3.9.1            # Dart 加密库不再需要
cryptography: ^2.9.0            # Dart 加密库不再需要
```

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Argon2id 在低端 Android 设备上太慢 | 用户体验 | 可配置参数（Fast/Balanced/Secure），见 3.2 |
| FRB 代码生成的局限性 | 某些复杂类型无法映射 | Phase 4.1.1 原型验证先行；保留少量 JSON relay 作为 fallback |
| FRB 版本升级破坏性变更 | 构建失败 | 锁定 FRB 版本 (=2.12.0)，升级前在分支验证 |
| yrs 在移动端的兼容性 | 编译问题 | yrs 是纯 Rust，无 C 依赖，兼容性好 |
| Noise 协议实现正确性 | 安全风险 | 使用 `snow`（经过审计的 Noise 实现），不自研 |
| 迁移期间加密格式不兼容 | 用户数据丢失 | Phase 1.5 自动检测 + 迁移 + 备份，见详细方案 |
| Android NDK 编译问题 | 构建失败 | Phase 3 前先验证，已有 `.cargo/config.toml` 配置 |
| Dart 回调线程安全 | 崩溃/死锁 | 使用 FRB StreamSink 替代原始 C 回调，见 2.3 |
| 同步中途崩溃 | 数据不一致 | WAL 模式 + 两阶段提交，见 11.2 |

---

## 8. 验证计划

### 8.1 单元测试（Rust）

```bash
cd native && cargo test
```

- 加密/解密 roundtrip
- 密码验证正确性
- Vault CRUD 操作
- CRDT 合正确性
- Noise 握手完成
- Schema 迁移

### 8.2 集成测试（Dart）

```bash
flutter test
```

- FFI 调用完整流程
- 账户创建 → 解锁 → CRUD → 锁定
- 跨账户数据隔离

### 8.3 手动测试

- macOS: 全流程（创建账户 → 编辑 profile → 备份 → 恢复）
- iOS: 全流程 + 生物识别
- Android: 全流程（验证 Argon2id 性能）
- Windows: 全流程

### 8.4 安全审计清单

- [ ] 密钥仅存在于 Rust 内存，Dart 层不持有
- [ ] `zeroize` 用于所有密钥清零
- [ ] 密码验证使用恒定时间比较
- [ ] 无敏感数据写入日志（password, key, salt, nonce, 明文数据）
- [ ] 日志自动脱敏（`sanitize_sensitive()` 宏）
- [ ] 备份文件格式与 vault 加密格式一致
- [ ] 同步传输经过 Noise 加密
- [ ] 配对码使用后销毁
- [ ] 旧格式加密数据迁移后，旧密钥材料已清零
- [ ] 生物识别 session key 不随 vault 同步（设备绑定）
- [ ] 数据库使用 WAL 模式，崩溃后自动恢复
- [ ] 同步中断后数据一致性验证（两阶段提交）

---

## 9. 时间线估算

| Phase | 内容 | 预计工时 | 前置条件 |
|-------|------|----------|----------|
| Phase 1 | 统一加密层（含迁移兼容） | 4-6 天 | 无 |
| Phase 2 | 统一账户管理 | 2-3 天 | Phase 1 |
| Phase 3 | 消除 Dart fallback | 3-5 天 | Phase 1, 2 |
| Phase 4 | FRB 代码生成（含原型验证） | 4-6 天 | Phase 3 |
| Phase 5a | CRDT ↔ ProfileData 互转 | 2-3 天 | Phase 4 |
| Phase 5b | Noise 握手与加密通道 | 2-3 天 | Phase 5a |
| Phase 5c | 增量同步逻辑 (mock transport) | 2-3 天 | Phase 5b |
| Phase 5d | mDNS 发现 + TCP 传输 | 1-2 天 | Phase 5c |
| Phase 6 | 前端薄壳化 | 2-3 天 | Phase 4 |
| **总计** | | **22-34 天** | |

Phase 5a-5d 与 Phase 6 可并行开发。

---

## 10. 关键文件索引

### Rust 侧

| 文件 | 职责 | 变更类型 |
|------|------|----------|
| `native/src/lib.rs` | FFI 入口 | 重构 → FRB 入口 |
| `native/src/crypto/argon2.rs` | Argon2id 密钥派生 | 增强 |
| `native/src/crypto/aes.rs` | AES-256-GCM | 增强：新增 profile 加解密封装 |
| `native/src/vault/store.rs` | SQLite 持久化 | 增强：Android/Windows 统一 |
| `native/src/vault/processor.rs` | JSON relay | 移除（Phase 4） |
| `native/src/vault/profile.rs` | 数据 schema | 保留 |
| `native/src/account/manager.rs` | 账户管理 | 增强：统一密码验证 + 元数据 |
| `native/src/account/config.rs` | 账户配置 | 新增 |
| `native/src/sync/engine.rs` | 同步引擎 | 重写 |
| `native/src/sync/protocol.rs` | 同步协议 | 重写 |
| `native/src/sync/crdt.rs` | CRDT 集成 | 新增 |
| `native/src/sync/transport.rs` | TCP 传输层 | 新增 |
| `native/src/sync/conflict.rs` | 冲突解决策略 | 新增 |
| `native/src/discovery/mdns.rs` | mDNS 设备发现 | 新增 |
| `native/src/security/settings.rs` | 安全设置 | 新增 |
| `native/src/backup/service.rs` | 备份服务 | 新增 |
| `native/src/engine/logger.rs` | 统一日志系统 | 新增 |
| `native/Cargo.toml` | 依赖配置 | 更新 |
| `docs/FFI_REFERENCE.md` | FFI 接口文档 | 新增 |

### Dart 侧

| 文件 | 职责 | 变更类型 |
|------|------|----------|
| `lib/core/services/native_crypto_service.dart` | 加密 FFI | 删除 |
| `lib/core/services/native_vault_service.dart` | Vault FFI | 重写 |
| `lib/core/services/rust_vault_service.dart` | Vault 包装 | 删除 |
| `lib/core/services/fallback_secure_storage.dart` | Keychain 包装 | 删除 |
| `lib/core/services/profile_storage_service.dart` | Profile CRUD | 精简 |
| `lib/core/services/backup_service.dart` | 备份 | 精简 |
| `lib/core/services/field_history_service.dart` | History CRUD | 精简 |
| `lib/core/services/security_service.dart` | 安全设置 | 精简 |
| `lib/presentation/providers/auth/auth_storage.dart` | 账户存储 | 精简 |
| `lib/presentation/providers/auth/auth_services.dart` | 账户服务 | 精简 |
| `lib/frb/` | FRB 生成的绑定 | 新增 |
| `pubspec.yaml` | Dart 依赖 | 移除加密包 |
| `flutter_rust_bridge.yaml` | FRB 配置 | 更新 |

---

## 11. 运维与工程实践

### 11.1 统一日志系统

Rust 引擎内部实现日志宏，统一输出到文件，避免 Dart 侧碎片化捕捉：

```rust
// native/src/engine/logger.rs
use log::{info, warn, error};

/// 初始化日志系统（在 engine_init 时调用）
pub fn init_logger(log_dir: &str, level: LogLevel) {
    // 使用 env_logger 或 tracing 输出到文件
    // 日志文件：{data_root}/logs/solosoul_{date}.log
    // 自动轮转：保留最近 7 天
}

/// 安全日志宏：自动过滤敏感数据
macro_rules! safe_log {
    (info, $($arg:tt)*) => {
        // 检查并替换敏感字段（password, key, salt, token）
        log::info!("{}", sanitize_sensitive(format!($($arg)*)));
    };
}
```

日志级别：
- `ERROR`：加密失败、数据库损坏、同步协议错误
- `WARN`：解密失败（可能是旧格式）、同步冲突
- `INFO`：账户解锁/锁定、同步开始/完成、备份创建
- `DEBUG`：FFI 调用详情、CRDT 操作细节（仅 Debug 构建）

**安全规则**：日志中 **绝不** 包含密码、密钥、salt、nonce、明文数据。

### 11.2 崩溃恢复与数据完整性

```rust
// native/src/vault/store.rs

impl VaultStore {
    /// 初始化时使用 WAL 模式
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
        // ...
    }

    /// 每次 unlock 后自动检查数据库完整性
    pub fn integrity_check(&self) -> Result<bool> {
        let result: String = self.conn
            .query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
        Ok(result == "ok")
    }
}
```

**同步中断恢复**：
- 同步操作使用 **两阶段提交**：先写入临时表，确认后移入主表
- 若同步中途崩溃 → 重启后检查临时表 → 回滚未完成的写入
- 每次同步前自动创建检查点（WAL checkpoint）

**定期备份**：
- 每次 unlock 后检查距上次自动备份是否超过 24 小时
- 若超过 → 后台异步创建备份（不阻塞 UI）
- 备份保留策略：最近 5 份常规 + 3 份特殊（手动触发）

### 11.3 多语言 / 国际化

**原则**：Rust 核心引擎 **不包含任何面向用户的字符串**。

```rust
// Rust 侧：只返回错误码和结构化数据
pub enum VaultError {
    AccountNotFound { account_id: String },
    InvalidPassword,
    DecryptionFailed { reason: DecryptionFailReason },
    // ... 不含人类可读消息
}

// Dart 侧：根据错误码查找本地化字符串
String localizeError(VaultError error) {
  return switch (error) {
    AccountNotFound() => S.of(context).errorAccountNotFound(error.accountId),
    InvalidPassword() => S.of(context).errorInvalidPassword,
    DecryptionFailed() => S.of(context).errorDecryptionFailed,
    // ...
  };
}
```

UI 字符串仍由 Flutter 层的 `intl` / `flutter_localizations` 管理，Rust 核心只提供结构化错误码。

### 11.4 FFI 接口文档

为所有公开的 FFI 函数编写独立文档，以便未来原生 UI 开发者参考：

```markdown
# SoloSoul Core FFI Reference

## account_create

创建新账户。

**签名**: `fn account_create(name: String, password: String) -> Result<String>`

**参数**:
| 参数 | 类型 | 说明 |
|------|------|------|
| name | String | 账户显示名称，1-64 字符 |
| password | String | 主密码，需满足复杂度要求 |

**返回**: 账户 ID（UUID v4 格式）

**错误**:
- `PasswordTooWeak` — 密码不满足复杂度要求
- `DuplicateAccount` — 同名账户已存在

**示例** (Swift):
\`\`\`swift
let accountId = try solosoul_account_create("My Vault", "correct-horse-battery-staple")
\`\`\`
```

此文档应：
- 与 Rust 代码同步维护（CI 中检查文档覆盖率）
- 包含 Dart / Swift / Kotlin 三种语言的调用示例
- 作为 `docs/FFI_REFERENCE.md` 放在仓库中
