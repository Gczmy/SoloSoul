# ADR-003: Rust Crate 拆分与架构

> **状态**: 已采纳 ✅  
> **决策日期**: 2026-06-04  
> **决策人**: SoloSoul 架构组  
> **影响范围**: Rust 代码组织、编译速度、测试隔离、可复用性

---

## 背景

当前 SoloSoul 有一套 Rust 密码学实现（位于 flutter/native/src/crypto/）：
1. `flutter/native/src/` — 供 Flutter FFI 使用，含 crypto/vault/account/sync/plugin
2. `src/lib.rs` — 供 已废弃 FFI 使用，仅导出 Argon2id + AES-256-GCM

Tauri 迁移的目标是**合并为单一 Rust 代码库**，消除重复，同时保持良好的模块化和可测试性。

## 候选方案

### 方案 A: Workspace + 多 Crate（推荐）

**结构**:
```
Cargo.toml (workspace)
crates/
  ├── solosoul-crypto/     # 密码学核心
  ├── solosoul-vault/      # Vault 存储
  ├── solosoul-sync/       # 同步引擎
  └── solosoul-plugin/     # 插件系统（可选独立）
src-tauri/
  └── src/
      ├── commands/        # IPC 命令
      ├── services/        # 业务服务
      ├── core/            # 核心业务逻辑（无 Tauri 依赖）
      └── db/              # 数据库访问
```

**优势**:
- **编译增量**: 修改一个 crate 不需要重编译全部
- **测试隔离**: 每个 crate 可独立测试
- **可复用**: `solosoul-crypto` 可被 CLI 工具复用
- **依赖清晰**: 显式声明 crate 间依赖
- **发布灵活**: 可独立发布 crate 到 crates.io

**劣势**:
- **初始复杂度**: 需要配置 workspace
- **API 稳定性**: crate 间接口变更需协调
- **IDE 支持**: 某些 IDE 对 workspace 支持不如单 crate

---

### 方案 B: 单 Crate + 模块划分

**结构**:
```
src-tauri/
  └── src/
      ├── main.rs
      ├── lib.rs
      ├── crypto/
      ├── vault/
      ├── sync/
      ├── plugin/
      ├── commands/
      ├── services/
      └── db/
```

**优势**:
- **最简单**: 无需 workspace 配置
- **无 API 边界**: 模块间调用无限制
- **IDE 友好**: 单 crate 下 IDE 跳转最流畅

**劣势**:
- **编译慢**: 任何修改都触发全量编译
- **测试困难**: 无法单独测试某个模块
- **不可复用**: 整个应用绑定在一起，CLI 无法复用
- **循环依赖风险**: 模块间容易产生隐式循环依赖

---

### 方案 C: 混合方案（渐进拆分）

**初始阶段**: 单 crate（方案 B）
**稳定后**: 逐步提取独立 crate（方案 A）

**优势**:
- **渐进演进**: 不一开始就过度设计
- **风险最低**: 快速启动，后续优化

**劣势**:
- **技术债务**: 后期拆分成本高
- **重复工作**: 先写单 crate，再拆分为多 crate

---

## 决策

**采用方案 A：Workspace + 多 Crate**，但**渐进实施**。

### 实施策略

```
Phase 1（重构初期）: 所有代码在 src-tauri/src/ 中，按模块划分
  └─ 目标：快速验证架构可行性

Phase 2（功能稳定后）: 提取 solosoul-crypto
  └─ 目标：密码学库独立，CLI 可复用

Phase 3（Vault 稳定后）: 提取 solosoul-vault
  └─ 目标：存储层独立

Phase 4（同步稳定后）: 提取 solosoul-sync
  └─ 目标：同步引擎独立
```

### Crate 定义

#### 1. `solosoul-crypto`（Phase 2 提取）

**职责**: Argon2id KDF + AES-256-GCM + 安全内存擦除

**接口**:
```rust
// lib.rs
pub mod kdf;
pub mod cipher;
pub mod secure;

pub use kdf::{derive_key, KdfConfig, KdfParams};
pub use cipher::{encrypt, decrypt, AesGcmError};
pub use secure::{SecureBytes, secure_wipe};
```

**依赖**:
- `argon2`
- `aes-gcm`
- `zeroize`
- `rand`
- `thiserror`

**不依赖**: Tauri、SQLite、Tokio

---

#### 2. `solosoul-vault`（Phase 3 提取）

**职责**: 加密文件存储 + Vault 生命周期管理

**接口**:
```rust
// lib.rs
pub mod storage;
pub mod manager;
pub mod encryption;

pub use storage::{Storage, FileStorage};
pub use manager::{VaultManager, VaultState};
pub use encryption::{VaultEncryption, EncryptedRecord};
```

**依赖**:
- `solosoul-crypto`
- `rusqlite`
- `serde`
- `serde_json`
- `thiserror`
- `tokio`（异步 I/O）

**不依赖**: Tauri

---

#### 3. `solosoul-sync`（Phase 4 提取）

**职责**: P2P 同步引擎（CRDT + Noise + mDNS）

**接口**:
```rust
// lib.rs
pub mod crdt;
pub mod transport;
pub mod noise;
pub mod discovery;

pub use crdt::{CrdtDocument, StateVector, merge};
pub use transport::{SyncTransport, SyncResult};
pub use noise::{NoiseHandshake, NoiseConfig};
pub use discovery::{MdnsDiscovery, DiscoveredDevice};
```

**依赖**:
- `solosoul-crypto`
- `solosoul-vault`
- `tokio`
- `tokio-util`
- `serde`

**不依赖**: Tauri

---

#### 4. `src-tauri`（主应用 Crate）

**职责**: Tauri 应用、IPC 命令、业务服务、数据库访问

**模块**:
```
src-tauri/src/
├── main.rs              # 入口
├── lib.rs               # 库入口（测试用）
├── commands/            # IPC 命令（tauri::command）
│   ├── auth.rs
│   ├── vault.rs
│   ├── profile.rs
│   └── ...
├── services/            # 业务服务
│   ├── auth_service.rs
│   ├── vault_service.rs
│   └── ...
├── core/                # 核心业务逻辑（无 Tauri 依赖）
│   ├── crypto/
│   ├── vault/
│   ├── profile/
│   ├── plugin/
│   ├── ocr/
│   ├── sync/
│   └── utils/
├── db/                  # 数据库访问
│   ├── connection.rs
│   ├── migrations.rs
│   └── repositories/
├── state/               # Tauri State 管理
│   ├── app_state.rs
│   ├── vault_state.rs
│   └── session_state.rs
└── ipc/                 # IPC 辅助
    ├── events.rs
    └── streams.rs
```

**依赖**:
- `solosoul-crypto`（Phase 2+）
- `solosoul-vault`（Phase 3+）
- `solosoul-sync`（Phase 4+）
- `tauri`
- `rusqlite`
- `tokio`
- `serde`

---

## Crate 间依赖图

```
┌─────────────────────────────────────────────┐
│              src-tauri (Tauri App)            │
│  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │
│  │commands │  │services │  │     db      │  │
│  └────┬────┘  └────┬────┘  └──────┬──────┘  │
│       └─────────────┴──────────────┘         │
│                      │                        │
│  ┌───────────────────┴───────────────────┐   │
│  │              core/                     │   │
│  │  ┌──────┐ ┌───────┐ ┌──────┐ ┌──────┐ │   │
│  │  │crypto│ │ vault │ │plugin│ │ sync │ │   │
│  │  └──┬───┘ └───┬───┘ └──┬───┘ └──┬───┘ │   │
│  └─────┼─────────┼────────┼────────┼─────┘   │
│        │         │        │        │         │
└────────┼─────────┼────────┼────────┼─────────┘
         │         │        │        │
    ┌────┴─────────┴────────┴────────┘
    │      solosoul-crypto (Phase 2)     │
    └─────────────────────────────────────┘
         ▲
    ┌────┴──────────────────────────────┐
    │     solosoul-vault (Phase 3)       │
    └─────────────────────────────────────┘
         ▲
    ┌────┴──────────────────────────────┐
    │     solosoul-sync (Phase 4)        │
    └─────────────────────────────────────┘
```

## 模块可见性规则

```rust
// solosoul-crypto/src/lib.rs
pub mod kdf;           // 公开
pub mod cipher;        // 公开
mod internal;          // 私有（crate 内可见）

// solosoul-vault/src/lib.rs
pub mod storage;       // 公开
pub(crate) mod encryption; // crate 内可见
```

---

## 与当前代码的对照

| 当前代码 | 迁移目标 | 备注 |
|---------|---------|------|
| `flutter/native/src/crypto/` | `crates/solosoul-crypto/src/` | 直接提取 |
| `flutter/native/src/vault/` | `crates/solosoul-vault/src/` | 直接提取 |
| `flutter/native/src/sync/` | `crates/solosoul-sync/src/` | 直接提取 |
| `flutter/native/src/plugin/` | `src-tauri/src/core/plugin/` | 暂不独立 |
| `flutter/native/src/account/` | `src-tauri/src/core/profile/` | 合并到 profile |
| `src/lib.rs` | `crates/solosoul-crypto/src/ffi.rs` | C FFI 兼容层 |
| 已废弃的 Rust 调用 | 删除 | 已废弃被替代 |

---

## 相关文档

- `tauri_refactor/Crate拆分与Rust架构.md` — 具体实施细节
- `tauri_refactor/项目结构规划.md` — 顶层目录结构

---

*文档版本：v1.0*  
*创建日期：2026-06-04*
