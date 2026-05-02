# Rust 核心引擎 — 后续实施计划

> 创建日期：2026-05-01
> 状态：待实施
> 前置：Phase 1-5 已完成，Phase 6 评估完成
> 分支：`feat/rust-core-engine`

## 背景

Phase 1-5 已在 `feat/rust-core-engine` 分支完成，核心成果：

- 统一加密层（Argon2id + AES-256-GCM）归于 Rust
- 账户管理统一在 `AccountManager`
- Dart fallback 已消除（~700 行删除）
- FRB 原型验证通过（enum-with-data、嵌套 HashMap）
- CRDT 同步引擎 + Noise 加密通道 + TCP 传输 + mDNS 发现

当前阻塞点：FRB 类型化 API 未替换 JSON relay pattern，导致 Dart 层仍持有废弃的 `_encryptionKey` 和 `NativeCryptoService`。

---

## P0: FRB 完整集成（解除所有阻塞）

**目标**：用 FRB 类型化函数替换 JSON relay pattern，为 P1 清理打通链路。

### 任务清单

#### 7.1 核心 FFI 函数（FRB 化）

将以下 JSON relay action 迁移为独立的 FRB 函数：

| 函数签名 | 对应 JSON action | 优先级 |
|----------|-----------------|--------|
| `frb_encrypt_bytes(data: Vec<u8>) -> Vec<u8>` | `encrypt_data` | 最高 |
| `frb_decrypt_bytes(data: Vec<u8>) -> Vec<u8>` | `decrypt_data` | 最高 |
| `frb_save_profile(account_id: String, profile_json: String) -> bool` | `save_profile` | 最高 |
| `frb_load_profile(account_id: String) -> Option<String>` | `load_profile` | 最高 |
| `frb_create_account(name: String, password: String) -> CreateAccountResult` | `create_account` | 高 |
| `frb_unlock_vault(password: String) -> UnlockVaultResult` | `unlock_vault` | 高 |
| `frb_lock_vault() -> bool` | `lock_vault` | 高 |
| `frb_list_accounts() -> Vec<AccountInfo>` | `list_accounts` | 高 |
| `frb_delete_account(account_id: String) -> bool` | `delete_account` | 中 |
| `frb_get_vault_stats() -> VaultStats` | `get_vault_stats` | 中 |
| `frb_change_password(old: String, new: String) -> ChangePasswordResult` | `change_password` | 中 |
| `frb_derive_key(password: String, salt: String, preset: KdfPreset) -> Vec<u8>` | 新增 | 高 |

#### 7.2 类型定义（`api.rs`）

已有类型（无需修改）：
- `SensitivityLevel`、`PropertyValue`、`FieldHistoryEntry`、`FormHistories`
- `VaultStats`、`AccountInfo`、`ProfileSummary`
- `CreateAccountResult`、`UnlockVaultResult`、`ChangePasswordResult`

需新增：
- `KdfPreset` 枚举（Fast/Balanced/Secure）— 已在 `crypto/argon2.rs` 定义，需在 `api.rs` 重新导出给 FRB
- `SyncResult`、`SyncDirection` — 已在 `sync/engine.rs` 定义

#### 7.3 Dart 侧适配

1. 运行 `flutter_rust_bridge_codegen generate` 生成新 Dart bindings
2. `native_vault_service.dart` — 将 `request()` 调用替换为 FRB 生成的函数
3. `profile_storage_service.dart` — 直接调用 FRB 函数，移除 JSON 序列化层
4. `auth_services.dart` — 使用 `frb_create_account` / `frb_unlock_vault`

#### 7.4 验证

- `cargo test --lib` 全部通过
- `dart analyze` 零问题
- `flutter test` 通过
- 手动测试：创建账户 → 解锁 → 保存/加载 profile → 锁定

**预估工时**：3-4 天

---

## P1: 安全与清理债务

**目标**：消除安全风险，清理废弃代码，实现"所有机密计算归于 Rust"。

### 7.5 独立 `derive_key` FFI 端点

**前置**：P0 FRB 集成

当前 `deriveKey()` 被捆绑在 `unlock_vault` 内部。Dart 认证流需要独立的密钥派生（用于生物识别、密码验证等场景）。

```rust
// api.rs
#[frb]
pub fn frb_derive_key(
    password: String,
    salt_hex: String,
    preset: KdfPreset,
) -> Vec<u8> { ... }
```

**影响的 Dart 文件**：
- `auth_storage.dart` — `deriveKey()` 调用
- `auth_notifier.dart` — `deriveKey()` 调用
- `biometric_credential_service.dart` — `deriveKey()` 调用

**预估工时**：1 天

### 7.6 Noise IK 握手（替换 Noise_XX）

**前置**：无（可与 P0 并行）

当前 `protocol.rs` 使用 Noise_XX（3 次消息交换），生产环境应改为 Noise_IK（2 次消息交换，双向认证）。

```rust
// 当前
const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

// 目标
const NOISE_PATTERN: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
```

变更点：
- `handshake()` — 从 3 消息减为 2 消息
- 需要持久化对端公钥（存储在账户配置中）
- 测试需要更新握手流程

**预估工时**：1 天

### 7.7 移除 `_encryptionKey`

**前置**：P0 FRB 集成 + 7.5 `derive_key`

Dart 层持有 `_encryptionKey: Uint8List?` 是安全风险。FRB 集成后，密钥管理完全在 Rust 内部。

**删除清单**：

| 文件 | 删除内容 |
|------|---------|
| `rust_vault_service.dart` | `_encryptionKey` 字段、`setEncryptionKey()`、`encryptionKey` getter、`clearEncryptionKey()` |
| `profile_storage_service.dart` | `setEncryptionKey()`、`encryptionKey` getter、`clearEncryptionKey()` |
| `user_preferences_service.dart` | `RustVaultService.instance.encryptionKey` 引用 |

**替代方案**：
- `user_preferences_service.dart` — 改用 `RustVaultService.instance.encryptBytes()` / `decryptBytes()`
- `profile_storage_service.dart` — 密钥管理移入 Rust，Dart 不再持有密钥

**预估工时**：1 天

### 7.8 删除 `native_crypto_service.dart`

**前置**：7.5 `derive_key` + 7.7 移除 `_encryptionKey`

这是消除 Dart 层加密的最后一步。

**迁移清单**：

| 消费者 | 当前使用 | 迁移目标 |
|--------|---------|---------|
| `auth_services.dart` | `generateSalt()`, `deriveKey()` | `Random.secure()`, `frb_derive_key()` |
| `auth_storage.dart` | `deriveKey()`, `generateSalt()` | `frb_derive_key()`, `Random.secure()` |
| `auth_notifier.dart` | `deriveKey()` | `frb_derive_key()` |
| `biometric_credential_service.dart` | `generateSalt()`, `deriveKey()`, `encrypt()`, `decrypt()` | `Random.secure()`, `frb_derive_key()`, `frb_encrypt_bytes()`, `frb_decrypt_bytes()` |
| `user_preferences_service.dart` | `encrypt()`, `decrypt()` | `frb_encrypt_bytes()`, `frb_decrypt_bytes()` |
| `operation_log_provider.dart` | `generateSalt()`, `encrypt()`, `decrypt()` | `Random.secure()`, `frb_encrypt_bytes()`, `frb_decrypt_bytes()` |

**删除文件**：
- `lib/core/services/native_crypto_service.dart` (486 行)
- `test/benchmark/crypto_benchmark.dart` — 更新或删除
- `test/unit/core/services/biometric_credential_service_test.dart` — 更新 mock

**预估工时**：2 天

**P1 总预估**：5 天

---

## P2: 验证与增强

**目标**：验证真实环境可行性，提升同步引擎健壮性。

### 7.9 多设备真实环境测试

**前置**：无

当前同步引擎仅通过 `MockTransport` 测试。需要在真实设备上验证完整链路：

**测试场景**：
1. 两台 macOS 设备同 WiFi → mDNS 互相发现
2. TCP 连接建立 → Noise 握手 → 同步完成
3. WiFi 断开 → 超时处理 → 不崩溃
4. 同步过程中一方退出 → 另一方优雅处理
5. 大数据量（1000+ 对象）同步性能

**测试工具**：
- 两台设备运行 `flutter run -d macos`
- 手动触发同步，验证数据一致性

**预估工时**：2 天（含调试）

### 7.10 冲突策略实现 (`conflict.rs`)

**前置**：P0 FRB 集成

当前 CRDT 使用 Yrs 默认的 last-writer-wins 策略。对于敏感数据（如密码、银行卡号），需要更智能的冲突处理：

```rust
// native/src/sync/conflict.rs
pub enum ConflictStrategy {
    LastWriterWins,           // 默认，自动合并
    KeepBoth,                 // 保留两个版本，用户选择
    PromptUser { versions: Vec<ConflictVersion> },  // UI 提示
}

pub struct ConflictVersion {
    pub value: String,
    pub device_id: String,
    pub timestamp: String,
}
```

**实现步骤**：
1. 定义 `ConflictStrategy` trait
2. 为每个 profile section 配置策略（identity → PromptUser, preferences → LastWriterWins）
3. Dart 侧 UI 弹窗让用户选择冲突版本

**预估工时**：2 天

### 7.11 代码警告与死代码清理

**前置**：无

| 问题 | 位置 | 修复 |
|------|------|------|
| `hex_encode` 未使用 | `vault/store.rs:454` | 删除 |
| `mut profile` 不需要 mut | `account/manager.rs:948` | 移除 mut |
| `api.rs` 重复类型警告 | `VaultStats`、`AccountInfo`、`ProfileSummary` | 使用 `#[frb(ignore)]` 或统一到 `api.rs` |
| FRB `frb_expand` cfg 警告 | `api.rs` 所有 `#[frb]` 注解 | 等待 FRB 更新或添加 `#[allow(unexpected_cfgs)]` |

**预估工时**：0.5 天

**P2 总预估**：4.5 天

---

## P3: 云同步（未来规划）

**目标**：将局域网同步扩展到广域网，支持跨网络设备同步。

### 7.12 WebSocket Transport

**前置**：P0 FRB 集成

当前 `Transport` trait 只有 TCP 实现。云同步需要 WebSocket transport 连接到后端：

```rust
// native/src/sync/transport.rs
pub struct WebSocketTransport {
    // 连接到 wss://sync.solosoul.app
}

impl Transport for WebSocketTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), String> { ... }
    fn recv(&mut self) -> Result<Vec<u8>, String> { ... }
}
```

**架构**：

```
设备 A ──WebSocket──► 云端中继 ◄──WebSocket── 设备 B
         (Noise 加密)           (Noise 加密)
```

- 云端只存储加密 blob，零知识
- 使用已有的 Noise 加密通道
- CRDT 增量同步减少传输量

**后端需求**：
- WebSocket 中继服务器（Go 或 Rust）
- 加密 blob 存储（S3 或 PostgreSQL）
- 账户认证（JWT token）

**预估工时**：5-7 天（含后端）

### 7.13 离线队列与重试

**前置**：7.12

设备离线时的同步队列：
1. 本地修改记录到 WAL (Write-Ahead Log)
2. 网络恢复后自动重试
3. 冲突检测与解决

**预估工时**：3 天

---

## 执行顺序与依赖图

```
P0 (FRB 集成) ─────────────────────────────► P1.5 (derive_key)
    │                                            │
    ├────────────────────────────────────────────┤
    │                                            ▼
    │                                    P1.7 (移除 _encryptionKey)
    │                                            │
    │                                            ▼
    │                                    P1.8 (删除 native_crypto_service)
    │
    ├──────────────────────────────────────────► P2.10 (冲突策略)
    │
    ▼
P1.6 (Noise IK) ──► P2.9 (真实环境测试)
                         │
                         ▼
                    P3.12 (WebSocket transport)
```

## 总预估

| 阶段 | 工时 | 累计 |
|------|------|------|
| P0: FRB 完整集成 | 3-4 天 | 3-4 天 |
| P1: 安全与清理 | 5 天 | 8-9 天 |
| P2: 验证与增强 | 4.5 天 | 12.5-13.5 天 |
| P3: 云同步 | 8-10 天 | 20.5-23.5 天 |

## 验收标准

### P0 完成标准
- [ ] `frb_encrypt_bytes` / `frb_decrypt_bytes` 替换 JSON relay
- [ ] `frb_save_profile` / `frb_load_profile` 替换 JSON relay
- [ ] `frb_create_account` / `frb_unlock_vault` 替换 JSON relay
- [ ] `frb_derive_key` 新增并可用
- [ ] `cargo test --lib` 全部通过（排除已知 pre-existing 失败）
- [ ] `dart analyze` 零问题
- [ ] `flutter test` 通过
- [ ] 手动测试：创建账户 → 解锁 → 保存/加载 → 锁定

### P1 完成标准
- [ ] `native_crypto_service.dart` 已删除
- [ ] `_encryptionKey` 从所有 Dart 文件移除
- [ ] Noise_IK 握手实现并测试通过
- [ ] `dart analyze` 零问题
- [ ] 所有 Dart 消费者使用 FRB 函数

### P2 完成标准
- [ ] 两台设备 mDNS 发现 + 同步成功
- [ ] `conflict.rs` 实现并有单元测试
- [ ] 代码警告清零
- [ ] 1000+ 对象同步性能测试通过
