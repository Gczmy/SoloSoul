# SoloSoul Native Client Development Roadmap

> 创建日期: 2026-04-14
> 最后更新: 2026-04-18
> 版本: 5.0
>
> **技术栈更新 (v2.0)**: Go Core → **Rust Core** + flutter_rust_bridge
> **安全架构更新 (v3.0)**: 新增 Wasm 沙盒、内存锁定、JIT 解密、审计日志
> **工程细节更新 (v4.0)**: 新增握手协议、异步隔离、核弹开关
> **存储安全更新 (v4.1)**: 新增 SQLCipher 双重加密
> **性能优化更新 (v5.0)**: 新增 UX 设计、缓存策略、前台服务
> **混合执行更新 (v5.1)**: 新增 Local Runner + Cloud Coordinator 混合模式

---

## 目录

1. [项目概述](#1-项目概述)
2. [技术栈选型](#2-技术栈选型)
3. [安全架构设计](#3-安全架构设计)
4. [云同步协议设计](#4-云同步协议设计)
   - [4.6 混合执行模式](#46-混合执行模式-hybrid-execution)
5. [离线编辑与冲突解决](#5-离线编辑与冲突解决)
6. [开发阶段规划](#6-开发阶段规划)
7. [TODO 清单](#7-todo-清单)
8. [技术细节参考](#8-技术细节参考)

---

## 1. 项目概述

### 1.1 目标
将 SoloSoul Web UI 替换为原生客户端应用，支持 macOS、Android、Windows 三大平台，并实现端到端加密的云同步功能。

### 1.2 核心原则
- **安全优先**: 银行级安全架构，零知识设计
- **本地优先**: 数据本地加密存储，云端仅存加密 blob
- **单点写入**: 互斥登录，防止加密数据的合并冲突

### 1.3 现有资产复用

| 资产类型 | 可复用程度 | 说明 |
|----------|------------|------|
| Rust Core (crypto-argon2) | ✅ 完全复用 | 已有 `crypto-argon2` crate，可直接编译为 C 库 |
| API 接口定义 | ✅ 可复用 | HTTP/gRPC API 保持一致 |
| 设计规范 | ⚠️ 部分复用 | 颜色、间距、字体等可参考 |
| React UI 代码 | ❌ 不可复用 | 需要完全重写 |
| Go Core (vault) | ⚠️ 需移植 | vault 逻辑需用 Rust 重写 (使用 rusqlite + SQLCipher) |

---

## 2. 技术栈选型

### 2.1 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                      Flutter UI Layer                        │
│          (macOS / Android / Windows)                        │
├─────────────────────────────────────────────────────────────┤
│                    Rust Core Library                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │
│  │ Argon2id│  │ AES-256 │  │ Vault   │  │ Sync Engine │  │
│  │   KDF   │  │   GCM   │  │ Manager │  │   (E2EE)    │  │
│  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │
│                      │                                      │
│              flutter_rust_bridge                            │
│                  (FFI 桥接)                                 │
├─────────────────────────────────────────────────────────────┤
│                   Cloud Storage Layer                       │
│          (S3 / B2 / Cloudflare R2)                         │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术选型详情

| 组件 | 技术选型 | 理由 |
|------|----------|------|
| **跨平台 UI** | Flutter 3.x | 单一代码库，一致视觉表现，性能优于 Electron |
| **核心逻辑层** | Rust compiled as C library (.a/.so/.dylib) | 零成本抽象，接近 C 的 FFI 性能，内存安全 |
| **FFI 桥接** | flutter_rust_bridge | 自动生成 Dart ↔ Rust 胶水代码，支持异步 |
| **加密库** | RustCrypto (aes-gcm, argon2) | 社区审查严格，安全性极高 |
| **本地存储** | rusqlite + SQLCipher | 双重加密：应用层 AES-256-GCM + 存储层 SQLCipher |
| **云存储** | S3 / Backblaze B2 / Cloudflare R2 | 成本低，支持加密 API，S3 兼容性好 |
| **实时通道** | WebSocket | 推送下线信号，处理实时同步 |

### 2.3 Rust vs Go 性能对比

| 指标 | Go (CGO) | Rust | 结论 |
|------|----------|------|------|
| FFI 调用开销 | 微秒级堆栈切换 | 接近零 (C ABI 兼容) | Rust 胜 |
| 运行时开销 | 有 GC 停顿 | 无运行时 | Rust 胜 |
| 内存安全 | 依赖 GC | 所有权模型 + 编译器检查 | Rust 胜 |
| 加密性能 | 依赖汇编优化 | 接近 C (使用相同指令) | 相当 |
| 现有资产 | crypto-argon2 已完成 | 可直接复用 | Rust 胜 |

### 2.4 各平台支持情况

| 平台 | Flutter 支持 | Rust Core 支持 | 特殊考虑 |
|------|--------------|----------------|----------|
| macOS | ✅ 完整 | ✅ .a / .dylib | 使用 Keychain 存储密钥 |
| Android | ✅ 完整 | ✅ .so (NDK) | 使用 Android Keystore |
| Windows | ✅ 完整 | ✅ .dll | 使用 Windows DPAPI |

---

## 3. 安全架构设计

### 3.1 端到端加密流程

```
┌──────────────────────────────────────────────────────────────────┐
│                         密钥派生流程                               │
│                                                                  │
│  ┌─────────────┐    Argon2id     ┌─────────────────────────┐   │
│  │ Master      │ ──────────────▶ │ Master Key (256-bit)    │   │
│  │ Password    │   + Device     │                         │   │
│  └─────────────┘   Salt         └─────────────────────────┘   │
│         │                                  │                    │
│         │                                  ▼                    │
│         │                        ┌─────────────────┐           │
│         │                        │  Session Key    │           │
│         │                        │  (派生用于会话) │           │
│         │                        └─────────────────┘           │
│         │                                  │                    │
│         │                                  ▼                    │
│         │                        ┌─────────────────┐           │
│         │                        │  Data Key       │           │
│         │                        │  (加密实际数据)  │           │
│         │                        └─────────────────┘           │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 加密参数

| 参数 | 值 | 说明 |
|------|-----|------|
| KDF | Argon2id | 内存密集型，防止 GPU 暴力破解 |
| Memory | 64 MB | M 系列芯片有硬件加速 |
| Iterations | 3 | 平衡安全性与性能 |
| Parallelism | 4 | 4 核并行 |
| Salt | 32 bytes | 设备唯一，随设备注册生成 |
| Cipher | AES-256-GCM | 认证加密，防篡改 |
| Nonce | 12 bytes | 每次加密随机生成 |

### 3.2.1 双重锁架构 (Dual-Lock Architecture)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        数据加密层级                                   │
│                                                                     │
│  Layer 1: 应用层加密 (AES-256-GCM)                                  │
│  ├── 用户数据先经过 AES-256-GCM 加密                                 │
│  └── 防止：云端看到明文、传输过程泄露                                 │
│                                                                     │
│  Layer 2: 存储层加密 (SQLCipher)                                    │
│  ├── 加密后的数据存入 .db 文件                                      │
│  └── 防止：物理存储被盗、磁盘镜像被提取                               │
│                                                                     │
│  结论：即使黑客同时拿到磁盘镜像 + 主密码，仍需破解两层加密             │
└─────────────────────────────────────────────────────────────────────┘
```

**为什么需要双重加密？**

| 攻击场景 | 单层 AES-256-GCM | 双重加密 (AES-256-GCM + SQLCipher) |
|----------|-------------------|-------------------------------------|
| 云端数据泄露 | ❌ 明文可见 | ✅ 密文 |
| 传输过程被抓包 | ❌ 明文可见 | ✅ 密文 |
| 物理存储被盗 | ❌ 数据库可直接打开 | ✅ SQLCipher 需要密钥 |
| 内存 Dump | ✅ 可能泄露 | ✅ 可能泄露 |
| 冷启动攻击 | ✅ 可能泄露 | ✅ 可能泄露 |

**SQLCipher 配置**:
- 算法: AES-256-CBC (SQLCipher 默认)
- 密钥: 由 Argon2id 派生的 Key 注入
- PBKDF2 迭代: 256,000 次 (SQLCipher 默认)

| 平台 | 密钥存储方案 |
|------|--------------|
| macOS | Keychain Services |
| Android | Android Keystore (StrongBox if available) |
| Windows | DPAPI + Microsoft Pluton (可选) |

### 3.4 安全内存处理

Rust Core 使用 `zeroize` crate 实现安全内存清理：

```rust
use zeroize::Zeroize;

fn process_sensitive_data(data: &mut [u8]) {
    // 使用后自动清零
    data.zeroize();
}

// 或者使用 Zeroizing 包装器
use zeroize::Zeroizing;

fn encrypt_with_cleanup(key: &[u8], plaintext: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut result = Zeroizing::new(Vec::new());
    // 加密逻辑...
    result // 离开作用域时自动清零
}
```

Rust 的优势：
- 编译器强制清零，无遗漏
- `Zeroize` trait 提供统一接口
- 极端情况下 (panic) 也不会泄露

### 3.5 插件沙盒化架构 (Wasm Sandbox)

采用 **"宿主-沙盒"架构**，插件限制为编译好的 `.wasm` 文件，通过 Rust 定义的严格 Host Functions 交互。

#### 3.5.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Flutter UI Layer                            │
│                  (用户交互、授权弹窗、审计日志)                       │
├─────────────────────────────────────────────────────────────────────┤
│                        Rust Core (Host)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────────────┐  │
│  │ Host        │  │ 内存安全    │  │ 插件管理器                │  │
│  │ Functions   │  │ (mlock,    │  │ (Wasmtime/Wasmer)        │  │
│  │ (严格接口)  │  │ Zeroize)   │  │                           │  │
│  └─────────────┘  └─────────────┘  └───────────────────────────┘  │
│         ▲                                          │                │
│         │                                          │                │
│  ┌──────┴──────┐                                   │                │
│  │ Plugin      │                                   │                │
│  │ (.wasm)    │ ◀──只能通过 Host Functions 访问数据──┘                │
│  │ 隔离执行    │                                                   │
│  └────────────┘                                                   │
├─────────────────────────────────────────────────────────────────────┤
│                      数据访问控制                                    │
│  插件请求 get_id_card() ──▶ Host 检测权限 ──▶ 弹出用户授权 ──▶ 返回   │
└─────────────────────────────────────────────────────────────────────┘
```

#### 3.5.2 Host Functions 接口定义

```rust
use wasmtime::{Engine, Linker, Module, Store};

pub struct SoloHostFunctions {
    // 插件只能通过这些函数获取数据
    // 每个函数都必须经过用户授权
}

impl SoloHostFunctions {
    /// 获取用户姓名
    /// 触发 Flutter 弹窗请求授权
    fn get_user_name(&mut self) -> Result<String, Trap> {
        // 1. 检查权限范围
        // 2. 通过 Flutter 通道弹出授权请求
        // 3. 用户确认后返回数据
    }

    /// 获取身份证号 (敏感)
    /// 触发更严格的授权流程
    fn get_id_card_number(&mut self) -> Result<String, Trap> {
        // 1. 权限检查
        // 2. Flutter 弹窗 (显示"即将获取身份证号")
        // 3. 延迟加载 + 阅后即焚
    }

    /// 获取护照信息 (敏感)
    fn get_passport(&mut self) -> Result<String, Trap> {
        // 同上
    }
}

/// Wasmtime 配置示例
fn create_plugin_engine() -> Engine {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);              // 消耗燃料，防止无限循环
    config.crw_functions(true);             // 启用cref/crefnull 支持

    // 网络隔离 - 禁止插件建立网络连接
    config::Wasmtime::new()
}
```

#### 3.5.3 插件权限声明 (manifest.json)

```json
{
  "plugin_id": "com.example.visa-booking",
  "name": "Visa Booking Plugin",
  "version": "1.0.0",
  "required_fields": [
    "identity.full_name.full_name",
    "travel.primary_passport.number"
  ],
  "optional_fields": [
    "identity.contact.phones"
  ],
  "network_policy": {
    "allowed_domains": [
      "*.visaservices.com",
      "api.booking.com"
    ],
    "block_all_outbound": true
  },
  "data_ttl_seconds": 300,
  "require_user_confirmation": true
}
```

### 3.6 内存生存周期管理 (TTL & Memory Pinning)

#### 3.6.1 内存锁定 (Memory Pinning)

防止操作系统将敏感数据交换到磁盘：

```rust
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::ptr::null_mut;

fn pin_memory(data: &mut [u8]) -> Result<(), std::io::Error> {
    // mlock - 防止内存被交换到磁盘
    let ret = unsafe {
        libc::mlock(
            data.as_ptr() as *const libc::c_void,
            data.len() as libc::size_t
        )
    };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn unpin_memory(data: &mut [u8]) -> Result<(), std::io::Error> {
    // munlock - 解锁内存
    let ret = unsafe {
        libc::munlock(
            data.as_ptr() as *const libc::c_void,
            data.len() as libc::size_t
        )
    };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
```

#### 3.6.2 即时解密 (Just-in-Time Decryption)

对于长时间排队的预约任务：

```
┌────────────────────────────────────────────────────────────────────┐
│                        任务状态机                                    │
│                                                                     │
│  ┌──────────┐    提交成功     ┌──────────┐   触发解密   ┌──────────┐│
│  │ QUEUED   │ ───────────▶ │ READY    │ ──────────▶ │SUBMITTED││
│  │ (仅存ID) │               │(即将提交) │             │          ││
│  └──────────┘               └──────────┘             └──────────┘│
│                                                                     │
│  解密时机：只在 READY 状态下才触发解密                               │
│  排队期间：沙盒只持有 Task ID，不持有任何明文数据                     │
└────────────────────────────────────────────────────────────────────┘
```

```rust
pub enum TaskState {
    Queued {
        task_id: String,
        queued_at: DateTime<Utc>,
    },
    Ready {
        task_id: String,
        decrypted_data: Zeroizing<Vec<u8>>,  // JIT 解密
        expires_at: DateTime<Utc>,          // TTL
    },
    Submitted,
    Failed,
}

impl TaskState {
    fn transition_to_ready(&mut self, vault: &Vault) -> Result<(), Error> {
        if let TaskState::Queued { task_id } = self {
            // 仅在即将提交时才解密
            let data = vault.get_and_decrypt(task_id)?;
            *self = TaskState::Ready {
                task_id: task_id.clone(),
                decrypted_data: Zeroizing::new(data),
                expires_at: Utc::now() + Duration::seconds(30),
            };
        }
        Ok(())
    }
}
```

### 3.7 网络隔离策略

插件只能访问白名单域名，禁止带外传输：

```rust
use std::net::IpAddr;

pub struct NetworkPolicy {
    allowed_domains: Vec<String>,
    blocked_ips: Vec<IpAddr>,
}

impl NetworkPolicy {
    pub fn is_allowed(&self, host: &str) -> bool {
        // 检查是否在白名单中
        for domain in &self.allowed_domains {
            if domain.contains('*') {
                // 处理通配符 *.visaservices.com -> visaservices.com
                if host.ends_with(&domain[2..]) {
                    return true;
                }
            } else if host == domain {
                return true;
            }
        }
        false
    }
}

pub struct IsolatedNetwork;

impl IsolatedNetwork {
    /// 插件网络请求拦截
    pub fn intercept_request(&self, url: &str) -> Result<(), NetworkError> {
        let parsed = url::Url::parse(url)
            .map_err(|_| NetworkError::InvalidUrl)?;

        let host = parsed.host_str()
            .ok_or(NetworkError::NoHost)?;

        if !self.policy.is_allowed(host) {
            return Err(NetworkError::DomainBlocked(host.to_string()));
        }

        Ok(())
    }
}
```

### 3.8 审计日志与异常监控

#### 3.8.1 不可篡改的本地审计日志

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: uuid::Uuid,
    pub timestamp: DateTime<Utc>,
    pub plugin_id: String,
    pub action: AuditAction,
    pub field_accessed: Option<String>,
    pub user_confirmation: bool,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    FieldAccess { field: String },
    DataDecrypted { size_bytes: usize },
    PluginInstalled,
    PluginUninstalled,
    SessionStarted,
    SessionEnded,
    RateLimitTriggered,
}

pub struct AuditLogger {
    file: std::fs::OpenOptions,
}

impl AuditLogger {
    /// 追加写入审计日志 (Append-only)
    pub fn append(&mut self, entry: &AuditLogEntry) -> Result<(), Error> {
        let json = serde_json::to_string(entry)?;
        let line = format!("{}\n", json);
        self.file.write_all(line.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    /// 验证日志完整性 (Chain Hash)
    pub fn verify_integrity(&self) -> Result<bool, Error> {
        // 每一行包含上一行的 hash，形成链式验证
    }
}
```

#### 3.8.2 频率限制 (Rate Limiting)

```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct RateLimiter {
    requests: Arc<HashMap<String, AtomicU64>>,
    window_secs: u64,
    max_requests: u64,
}

impl RateLimiter {
    pub fn check(&self, plugin_id: &str) -> Result<(), RateLimitError> {
        let count = self.requests
            .get(plugin_id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);

        if count >= self.max_requests {
            return Err(RateLimitError::Exceeded {
                plugin_id: plugin_id.to_string(),
                count,
                window_secs: self.window_secs,
            });
        }

        Ok(())
    }

    pub fn record(&self, plugin_id: &str) {
        self.requests
            .entry(plugin_id.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }
}

/// 熔断机制 - 高频请求自动熔断
pub struct CircuitBreaker {
    failure_count: AtomicU64,
    last_failure: std::sync::Mutex<Option<DateTime<Utc>>>,
}

impl CircuitBreaker {
    pub fn is_open(&self) -> bool {
        let failures = self.failure_count.load(Ordering::Relaxed);
        if failures > 100 {
            // 100 次失败后熔断
            return true;
        }
        false
    }
}
```

### 3.9 安全架构综合对比

| 安全维度 | 传统方案 | 本架构方案 |
|----------|----------|------------|
| 存储安全 | 云端存明文或弱加密 | E2EE，云端只存加密 blob |
| 内存安全 | 普通内存分配 | mlock + Zeroize，防止 swap |
| 执行环境 | 直接访问数据库 | Wasm 沙盒，Host Functions 隔离 |
| 数据授权 | 一次性授权 | 单次交互确认，按字段授权 |
| 网络隔离 | 无限制 | 白名单域名，禁止带外传输 |
| 销毁机制 | 可能残留内存 | 阅后即焚 + TTL |
| 审计追溯 | 无或弱日志 | 不可篡改链式审计日志 |

### 3.10 插件握手协议 (Plugin Handshake)

Wasm 沙盒启动时验证插件完整性，防止木马篡改：

```rust
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct PluginRegistry {
    /// 官方插件签名白名单: plugin_id -> (version -> sha256_hash)
    whitelist: HashMap<String, HashMap<String, String>>,
}

impl PluginRegistry {
    /// 验证插件 Hash
    pub fn verify_plugin(&self, wasm_bytes: &[u8], plugin_id: &str, version: &str) -> Result<(), SecurityError> {
        // 1. 计算插件文件的 SHA-256
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let hash = format!("{:x}", hasher.finalize());

        // 2. 与白名单比对
        if let Some(version_hashes) = self.whitelist.get(plugin_id) {
            if let Some(expected_hash) = version_hashes.get(version) {
                if hash == *expected_hash {
                    return Ok(()); // 验证通过
                }
            }
        }

        Err(SecurityError::PluginNotWhitelisted {
            plugin_id: plugin_id.to_string(),
            hash,
        })
    }
}

/// 插件加载时的完整握手流程
pub fn load_plugin_with_handshake(
    registry: &PluginRegistry,
    wasm_path: &Path,
    plugin_id: &str,
    version: &str,
) -> Result<wasmtime::Module, Error> {
    // 1. 读取 wasm 文件
    let wasm_bytes = std::fs::read(wasm_path)?;

    // 2. 完整性校验
    registry.verify_plugin(&wasm_bytes, plugin_id, version)?;

    // 3. 编译模块
    let engine = create_plugin_engine()?;
    let module = wasmtime::Module::new(&engine, &wasm_bytes)?;

    // 4. 记录审计日志
    audit_logger::record(&AuditLogEntry {
        action: AuditAction::PluginVerified {
            plugin_id: plugin_id.to_string(),
            version: version.to_string(),
        },
    });

    Ok(module)
}
```

**白名单存储格式** (manifest.json):

```json
{
  "plugin_id": "com.example.visa-booking",
  "version": "1.0.0",
  "sha256": "a3b5c8d7e9f0123456789abcdef0123456789abcdef0123456789abcdef0123"
}
```

### 3.11 异步隔离 (Flutter 线程安全)

UI 线程与加密/Wasm 线程隔离，保证界面流畅：

```
┌────────────────────────────────────────────────────────────────────┐
│                     Flutter Main Isolate                             │
│  (UI 渲染、用户交互、状态管理)                                        │
│                                                                     │
│  StreamBuilder<T> ◀─── Stream<T>                                    │
│       │                     │                                        │
│       │              ┌─────┴──────┐                                │
│       │              │ Rust Core   │                                │
│       │              │ ThreadPool │                                │
│       │              │            │                                │
│       │              │ ┌────────┐ │                                │
│       │              │ │ Argon2 │ │ ◀── 计算密集型                │
│       │              │ └────────┘ │                                │
│       │              │ ┌────────┐ │                                │
│       │              │ │ AES-GCM│ │ ◀── 计算密集型                │
│       │              │ └────────┘ │                                │
│       │              │ ┌────────┐ │                                │
│       │              │ │ Wasm   │ │ ◀── 计算密集型                │
│       │              │ └────────┘ │                                │
└───────┼──────────────┴─────────────┴────────────────────────────────┘
        │                     ▲
        │                     │
        │              flutter_rust_bridge
        │                 Stream
```

**Rust 线程池实现**:

```rust
use tokio::runtime::Builder;
use tokio::sync::mpsc;

pub struct CryptoThreadPool {
    inner: tokio::runtime::Runtime,
}

impl CryptoThreadPool {
    pub fn new() -> Self {
        let inner = Builder::new_multi_thread()
            .worker_threads(4)                    // 4 个工作线程
            .thread_name("crypto-worker")
            .enable_io()
            .build()
            .unwrap();
        Self { inner }
    }

    /// 异步执行加密任务，通过 Stream 返回进度
    pub fn spawn_encrypt_task(
        &self,
        data: Vec<u8>,
        key: Vec<u8>,
    ) -> impl Stream<Item = CryptoProgress> {
        let (tx, rx) = mpsc::channel(100);

        self.inner.spawn(async move {
            // 1. 状态: 开始
            let _ = tx.send(CryptoProgress::Started).await;

            // 2. 状态: 派生密钥
            let derived_key = derive_key_with_argon2(&key).await;
            let _ = tx.send(CryptoProgress::KeyDerived).await;

            // 3. 状态: 加密中
            let encrypted = aes_gcm_encrypt(&data, &derived_key).await;
            let _ = tx.send(CryptoProgress::Encrypted { bytes: encrypted.len() }).await;

            // 4. 状态: 完成
            let _ = tx.send(CryptoProgress::Completed).await;
        });

        rx
    }

    /// 异步执行 Wasm 插件
    pub fn spawn_wasm_task(
        &self,
        module: wasmtime::Module,
    ) -> impl Stream<Item = WasmProgress> {
        let (tx, rx) = mpsc::channel(100);
        self.inner.spawn(async move {
            let _ = tx.send(WasmProgress::Executing).await;
            // Wasm 执行...
            let _ = tx.send(WasmProgress::Finished).await;
        });
        rx
    }
}

#[derive(Debug, Clone)]
pub enum CryptoProgress {
    Started,
    KeyDerived,
    Encrypted { bytes: usize },
    Completed,
    Error(String),
}
```

**Flutter 端监听**:

```dart
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

class VaultStore {
  final _cryptoPool = CryptoThreadPool();

  /// 加密数据（不阻塞 UI）
  Stream<CryptoProgress> encryptData(Uint8List data, Uint8List key) {
    return _cryptoPool.spawnEncryptTask(data, key);
  }

  /// UI 监听进度
  Widget buildEncryptButton() {
    return StreamBuilder(
      stream: encryptData(_data, _key),
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          final progress = snapshot.data;
          if (progress is CryptoProgress.Encrypted) {
            return Text('已加密 ${progress.bytes} bytes');
          }
        }
        return CircularProgressIndicator();
      },
    );
  }
}
```

### 3.12 紧急核弹开关 (Global Kill Switch)

发现异常时立即销毁所有敏感数据：

```
┌────────────────────────────────────────────────────────────────────┐
│                      触发条件                                        │
│                                                                     │
│  用户点击"紧急终止"                                                 │
│  或                                                                  │
│  检测到异常行为 (高频请求/未知漏洞)                                  │
│  或                                                                  │
│  收到服务器 SESSION_REVOKED                                          │
└────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│                      销毁流程 (按序执行)                            │
│                                                                     │
│  Step 1: 关闭 Wasm 引擎 ───▶ 停止所有插件执行                       │
│  Step 2: 释放敏感内存 ───▶ munlock + zeroize                       │
│  Step 3: 清除密钥缓存 ───▶ 内存中的 Key Material                   │
│  Step 4: 关闭网络连接 ───▶ 断开所有云端连接                         │
│  Step 5: 锁定本地存储 ───▶ vault 进入 locked 状态                   │
│  Step 6: 跳转登录页 ───▶ 要求用户重新认证                          │
└────────────────────────────────────────────────────────────────────┘
```

**Rust 实现**:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub static KILL_SWITCH: AtomicBool = AtomicBool::new(false);

pub struct VaultManager {
    // ... 其他字段
    wasm_engine: Option<wasmtime::Engine>,
    sensitive_buffers: Vec<Zeroizing<Vec<u8>>>,
}

impl VaultManager {
    /// 全局终止开关
    pub fn trigger_kill_switch(&mut self) {
        // 1. 设置全局标志
        KILL_SWITCH.store(true, Ordering::SeqCst);

        // 2. 关闭 Wasm 引擎 (终止所有插件)
        if let Some(engine) = self.wasm_engine.take() {
            drop(engine);
        }

        // 3. 释放所有敏感缓冲区
        for mut buffer in self.sensitive_buffers.drain(..) {
            buffer.zeroize();
        }

        // 4. 清除密钥缓存
        self.session_key.zeroize();
        self.master_key.zeroize();

        // 5. 记录审计日志
        audit_logger::record(&AuditLogEntry {
            action: AuditAction::KillSwitchTriggered,
            reason: "user_requested", // or "rate_limit", "session_revoked"
        });

        // 6. 强制刷新 UI 状态
        self.state = VaultState::Locked;
    }
}

/// Flutter 层调用
class VaultStore {
  static void emergencyLock() {
    // 通过 Flutter 通道调用 Rust 的 kill_switch
    _bridge.invoke('triggerKillSwitch');
    // Flutter 端跳转到登录页
    Get.offAllNamed('/login');
  }
}
```

**Flutter UI**:

```dart
class SecuritySettings extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // ... 其他设置

        // 紧急终止按钮
        ListTile(
          leading: Icon(Icons.warning_amber, color: Colors.red),
          title: Text('紧急终止所有任务'),
          subtitle: Text('立即销毁内存中的敏感数据，锁定 vault'),
          onTap: () async {
            final confirmed = await showDialog<bool>(
              context: context,
              builder: (context) => AlertDialog(
                title: Text('确认紧急终止?'),
                content: Text('所有运行中的任务将被强制结束。'),
                actions: [
                  TextButton('取消', onPressed: () => Navigator.pop(false)),
                  TextButton('确认终止', style: TextButton.styleFrom(
                    foregroundColor: Colors.red,
                  ), onPressed: () => Navigator.pop(true)),
                ],
              ),
            );

            if (confirmed == true) {
              await VaultStore.emergencyLock();
            }
          },
        ),
      ],
    );
  }
}
```

---

## 4. 云同步协议设计

### 4.1 排他性登录机制 (Exclusive Session)

**核心原则**: 同一时间只能有一个活跃会话

```
设备 A 登录成功 ──▶ 云端记录 SessionID_A 为 Active
      │
      │  设备 B 登录
      ▼
云端检测到已有活跃会话 ──▶ 生成新 SessionToken
      │                        │
      │                        ▼
      │               向设备 A 发送 LOGOUT_SIGNAL
      │                        │
      ▼                        ▼
设备 A 收到信号 ──▶ 清理内存，跳转登录页
      │
      ▼
设备 B 登录成功
```

### 4.2 序列号机制 (Sequence Number)

云端为每个用户维护一个自增版本号 V：

| 操作 | 携带版本 | 云端版本 | 结果 |
|------|----------|----------|------|
| 读取 | - | V | 返回当前数据，更新本地 V |
| 写入 | V | V | 写入成功，V++ |
| 写入 | V | V' (V' > V) | ❌ 拒绝，需要重新同步 |

### 4.3 Session Token 结构

```json
{
  "session_id": "uuid-v4",
  "user_id": "user-uuid",
  "device_id": "device-uuid",
  "created_at": "2026-04-14T00:00:00Z",
  "expires_at": "2026-04-15T00:00:00Z",
  "sequence_number": 42,
  "encryption_public_key": "base64-encoded-key"
}
```

### 4.4 WebSocket 信号协议

| 信号 | 方向 | 载荷 | 处理 |
|------|------|------|------|
| `SESSION_REVOKED` | Server → Client | `{ reason: "new_login" }` | 清理内存，显示登录页 |
| `DATA_CHANGED` | Server → Client | `{ new_sequence: 43 }` | 拉取最新数据 |
| `KEEPALIVE` | Client ↔ Server | `{ ts: timestamp }` | 保活检测 |

### 4.5 API 端点设计

```
POST   /api/v1/auth/register        # 设备注册
POST   /api/v1/auth/login           # 登录 (获取 session)
POST   /api/v1/auth/logout          # 登出
POST   /api/v1/auth/refresh         # 刷新 session

GET    /api/v1/vault                # 获取加密 blob + 版本号
PUT    /api/v1/vault                # 上传加密 blob (需携带版本号)
GET    /api/v1/vault/metadata       # 获取元数据 (版本号、上次修改时间)

WS     /api/v1/sync                 # 实时同步通道
```

### 4.6 混合执行模式 (Hybrid Execution)

针对预约类任务（如签证预约SlotGo），设计"本地执行为主、云端协作为辅"的混合模式：

#### 4.6.1 架构角色

| 角色 | 职责 | 运行位置 |
|------|------|----------|
| **Local Runner** | 1. 定时刷新请求（防会话过期）<br>2. 数据解密与填充<br>3. 预约任务执行（浏览器指纹模拟）<br>4. 结果回写本地 | 用户设备（手机/电脑） |
| **Cloud Coordinator** | 1. 插件版本更新检查<br>2. 预约规则/配额分发<br>3. 配置同步（加密下发）<br>4. 通知推送（钉钉/邮件） | 云端服务器 |

#### 4.6.2 数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                      混合模式数据流                               │
│                                                                 │
│   手机端                                         电脑端          │
│   ┌─────────┐                                   ┌─────────┐    │
│   │ Local   │                                   │ Local   │    │
│   │ Runner  │                                   │ Runner  │    │
│   └────┬────┘                                   └────┬────┘    │
│        │ 加密云端通信                                 │         │
│        ▼                                              ▼         │
│   ┌─────────────────────────────────────────────────────┐       │
│   │              Cloud Coordinator (云端)               │       │
│   │  · 配置加密分发  · 规则更新  · 通知推送             │       │
│   └─────────────────────────────────────────────────────┘       │
│                          │                                      │
│                          ▼                                      │
│                   ┌──────────┐                                 │
│                   │ 加密云存 │ ← E2EE Blob (手机/电脑都上传)   │
│                   └──────────┘                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 4.6.3 典型任务流程

```
1. 用户在手机端创建预约任务（目标网站、频率、参数）
2. Local Runner 定期在后台执行刷新请求（防会话过期）
3. 用户在电脑端打开 SoloSoul，主动解密数据并执行预约
4. 预约成功后，结果通过 Cloud Coordinator 推送通知
5. 两端均保持数据同步（加密 blob）
```

#### 4.6.4 为什么需要"本地挂起"

| 问题 | 解决方案 | 理由 |
|------|----------|------|
| **反爬虫检测** | 浏览器指纹在本地生成 | 云端无法模拟真实用户环境 |
| **安全屏障** | 零知识：云端只有加密blob | 即使服务器被攻破，数据仍安全 |
| **网络隔离** | 预约请求直接从用户设备发出 | 避免 IP 段集中（如 1000 用户 = 1000 IP） |
| **会话管理** | 本地维护登录状态 | 避免频繁 SSO 导致账号风险 |

#### 4.6.5 安全考量

1. **Local Runner 隔离**：Runner 运行在独立的沙盒进程中，即使被攻破也无法直接访问 vault
2. **最小权限原则**：Runner 只能访问任务所需的特定字段，无法获取整个 profile
3. **操作审计**：所有预约操作记录审计日志（时间、操作者、目标网站）
4. **紧急停止**：用户提供"核弹开关"，一键终止所有运行中的任务

---

## 5. 离线编辑与冲突解决

### 5.1 状态机

```
                    ┌─────────────┐
                    │   INITIAL   │
                    └──────┬──────┘
                           │ 登录成功
                           ▼
┌────────────────────────────────────────────────────────┐
│                      SYNCED                             │
│  本地数据 = 云端数据                                    │
└────────────────────────────────────────────────────────┘
           │                           ▲
           │ 本地修改                   │ 同步成功
           ▼                           │
┌────────────────────────────────────────────────────────┐
│                      DIRTY                             │
│  本地已修改，尚未同步到云端                             │
│  数据保存在本地加密缓冲区                               │
└────────────────────────────────────────────────────────┘
           │                           ▲
           │ 联网检测到版本冲突        │ 用户选择
           ▼                           │
┌────────────────────────────────────────────────────────┐
│                    CONFLICT                            │
│  需要用户仲裁：保留本地 / 拉取云端 / 手动合并           │
└────────────────────────────────────────────────────────┘
```

### 5.2 冲突解决 UI

当检测到冲突时，呈现给用户三个选项：

| 选项 | 说明 | 风险 |
|------|------|------|
| **保留本地** | 离线修改覆盖云端 | 可能丢失云端的新数据 |
| **使用云端** | 放弃离线修改，拉取云端 | 离线修改会丢失 |
| **手动合并** | 对比界面，用户选择性保留 | 需要较好的 UI 设计 |

### 5.3 离线队列设计

```dart
class OfflineQueue {
  String id;                    // 队列项唯一 ID
  DateTime timestamp;           // 修改时间
  EncryptedBlob localBlob;      // 加密后的本地数据
  EncryptedBlob? cloudBlob;     // 冲突时的云端数据 (可选)
  SyncStatus status;            // pending / conflict / resolved
  ConflictResolution? resolution; // resolved 时记录用户选择
}
```

### 5.4 强制下线后的处理流程

```
检测到 SESSION_REVOKED 信号
        │
        ▼
┌───────────────────┐
│ 进入只读/锁定模式   │
│ 提示用户重新登录   │
└───────────────────┘
        │
        ▼
用户重新输入密码
        │
        ▼
重新派生密钥 ──▶ 解密本地 Dirty 数据
        │
        ▼
检测版本号冲突
        │
        ├─── 版本一致 ──▶ 同步成功
        │
        └─── 版本不一致 ──▶ 显示冲突解决 UI
```

---

## 6. 开发阶段规划

### Phase 1: 基础设施 ✅ 完成

| 任务 | 状态 | 交付物 |
|------|------|--------|
| Rust Core 封装为 C 库 | ✅ 完成 | libsolo_core.a / .so / .dylib |
| flutter_rust_bridge 集成 | ✅ 完成 | 自动生成 bindings |
| 复用 crypto-argon2 crate | ✅ 完成 | Argon2id 实现 (64MB, 3 iterations) |
| Rust Vault 实现 | ✅ 完成 | AES-256-GCM 加密/解密 |
| macOS Keychain 集成 | ✅ 完成 | macos/Runner/AppDelegate.swift |
| iOS Keychain 集成 | 🔄 待完成 | 需实现 iOS Keychain method handler |

### Phase 2: 认证与会话 ✅ 完成

| 任务 | 状态 | 交付物 |
|------|------|--------|
| 设备注册流程 | ✅ 完成 | 注册 API + Flutter UI |
| 登录/登出 | ✅ 完成 | 登录 UI + 会话管理 |
| 账户创建/解锁 | ✅ 完成 | 密码提示词、账户列表 |
| Session Token 管理 | ✅ 完成 | 刷新、过期处理 |

### Phase 3: 数据同步 🔄 进行中

| 任务 | 状态 | 交付物 |
|------|------|--------|
| Profile CRUD | ✅ 完成 | Profile/Travel/Financial/Professional |
| 云端存储格式 | 🔄 待开发 | Go 后端 solosould |
| 加密 blob 上传/下载 | 🔄 待开发 | 同步 API |
| 版本号机制 | 🔄 待开发 | 冲突检测 |
| 实时同步通道 | 🔄 待开发 | WebSocket 集成 |
| 冲突解决 UI | 🔄 待开发 | 三选项对话框 |

### Phase 4: 离线支持 ❌ 未开始

| 任务 | 状态 | 交付物 |
|------|------|--------|
| 离线队列 | ❌ 待开发 | 本地修改暂存 |
| 联网恢复流程 | ❌ 待开发 | 自动同步 |
| 草稿/脏数据管理 | ❌ 待开发 | 状态标记 |

### Phase 5: 平台特性 🔄 部分完成

| 任务 | macOS | iOS | Android | Windows |
|------|-------|-----|---------|---------|
| Keychain/Keystore 集成 | ✅ | 🔄 | 🔄 | 🔄 |
| TouchID/FaceID | 🔄 待集成 | 🔄 | 🔄 | 🔄 |
| 原生导航 | ✅ | 🔄 | 🔄 | 🔄 |
| 系统通知 | 🔄 | 🔄 | 🔄 | 🔄 |
| DMG 分发 | ✅ | 🔄 | 🔄 | 🔄 |

> **注意**: macOS DMG 构建使用 `build_dmg.sh` 自动化脚本，位于项目根目录

### Phase 6: 功能完善 🔄 部分完成

| 任务 | 状态 | 说明 |
|------|------|------|
| Profile 编辑 | ✅ 完成 | Identity/Travel/Financial/Professional/Preferences |
| OCR 集成 | 🔄 待开发 | PaddleOCR stub 存在 |
| Plugin 系统 | 🔄 预留接口 | 尚未实现 Wasm 沙盒 |
| 多语言支持 | 🔄 待开发 | i18n 框架未集成 |
| Riverpod 3.x 升级 | 🔄 待开发 | 当前 2.6.1，需迁移到 3.x |
| macOS Release Build | ✅ 完成 | macOS Release 构建已完成 |

---

## 7. TODO 清单

### 7.1 Phase 1: 基础设施

- [x] 设计 Rust Core C ABI 接口
- [x] 编译 macOS 版 Rust Core (.a/.dylib)
- [ ] 编译 Android 版 Rust Core (NDK)
- [ ] 编译 Windows 版 Rust Core (.dll)
- [x] 集成 flutter_rust_bridge，生成 Dart bindings
- [x] 复用现有 crypto-argon2 crate
- [x] 实现 Rust Vault (AES-256-GCM)
- [x] 实现 flutter_secure_storage 封装 (macOS ✅, iOS 🔄)
- [x] 实现基本 Vault 加密/解密
- [ ] 编写集成测试

### 7.2 Phase 2: 认证与会话

- [x] 设计设备注册 API
- [x] 实现设备注册流程 UI
- [x] 实现登录/登出 UI
- [x] 实现 Keychain 存储 Session (macOS ✅, iOS 🔄)
- [ ] 实现 WebSocket 客户端
- [ ] 实现排他登录逻辑
- [ ] 实现强制下线 UI
- [x] 实现 Session 刷新机制

### 7.3 Phase 3: 数据同步

- [ ] 设计云端存储格式
- [ ] 实现加密 blob 上传
- [ ] 实现加密 blob 下载
- [ ] 实现版本号检测
- [ ] 实现冲突检测
- [ ] 实现冲突解决 UI (三选项)
- [ ] 实现 WebSocket 实时同步

### 7.4 Phase 4: 离线支持

- [ ] 实现本地修改暂存
- [ ] 实现离线队列管理
- [ ] 实现联网自动同步
- [ ] 实现草稿状态管理
- [ ] 处理极端情况 (换设备/卸载)

### 7.5 Phase 5: 平台特性

- [x] macOS: Keychain 集成
- [ ] macOS: TouchID 解锁
- [ ] macOS: 原生菜单栏
- [ ] iOS: Keychain 集成 (需实现 iOS method handler)
- [ ] iOS: Face ID 解锁
- [ ] Android: Android Keystore 集成
- [ ] Android: Fingerprint 解锁
- [ ] Android: Material Design 3
- [ ] Windows: DPAPI 集成
- [ ] Windows: Windows Hello 解锁
- [ ] Windows: WinUI 3 适配

### 7.6 Phase 6: 功能完善

- [x] Profile CRUD (Identity/Travel/Financial/Professional/Preferences)
- [ ] 文档上传与 OCR
- [ ] Plugin 预留接口
- [ ] 多语言支持 (i18n)
- [ ] Riverpod 3.x 升级

### 7.7 Phase 7: 安全增强 (与 Phase 5 并行)

- [ ] Wasmtime/Wasmer 集成
- [ ] Host Functions 接口定义与实现
- [ ] 插件权限 manifest.json 解析
- [ ] 用户交互授权弹窗 (Flutter)
- [ ] mlock 内存锁定实现
- [ ] Zeroize 敏感数据清理
- [ ] JIT 即时解密机制
- [ ] 网络白名单策略实现
- [x] 链式审计日志 (OperationLogger)
- [ ] Rate Limiting 频率限制
- [ ] Circuit Breaker 熔断机制
- [ ] **插件握手协议** - SHA-256 白名单校验
- [ ] **异步线程池** - Tokio 多线程隔离
- [ ] **全局核弹开关** - EmergencyLock

### 7.8 Phase 8: 性能优化 (与 Phase 5 并行)

- [x] **Argon2id 异步计算** - 后台线程执行，骨架屏动画
- [ ] **生物识别绕过** - TouchID/FaceID 解锁临时密钥
- [ ] **Wasm AOT 预编译** - 模块序列化缓存
- [ ] **Wasm 预热机制** - 用户进入详情页时静默初始化
- [x] **Flutter 缓存层** - ProfileProvider 缓存非敏感数据
- [ ] **软着陆强制下线** - 只读模式而非直接跳转
- [ ] **SQLCipher WAL 模式** - 索引优化
- [ ] **Android 前台服务** - 常驻通知栏
- [ ] **低频轮询机制** - 排队时降低 Wasm 执行频率

---

## 8. 技术细节参考

### 8.1 Rust Core C ABI 导出示例

使用 `cbindgen` 自动生成 C 头文件：

```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use zeroize::Zeroize;

#[no_mangle]
pub extern "C" fn solo_vault_unlock(
    passphrase: *const c_char,
    salt: *const u8,
    salt_len: usize,
) -> i32 {
    // 获取 passphrase
    let passphrase = unsafe { std::ffi::CStr::from_ptr(passphrase) };
    let passphrase = match passphrase.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Argon2id 密钥派生
    let salt_slice = unsafe { std::slice::from_raw_parts(salt, salt_len) };
    let mut salt_bytes = salt_slice.to_vec();

    let argon2 = Argon2::default();
    let mut key = Vec::new();

    match argon2.hash_password(passphrase.as_bytes(), &salt_bytes) {
        Ok(hash) => {
            key = hash.hash.unwrap().as_bytes().to_vec();
        }
        Err(_) => return -1,
    }

    // 清理敏感数据
    salt_bytes.zeroize();
    key.zeroize();

    0 // 成功
}

#[no_mangle]
pub extern "C" fn solo_vault_encrypt(
    data: *const u8,
    data_len: usize,
    key: *const u8,
) -> *mut u8 {
    // 返回加密后的数据指针 (调用方需释放)
}
```

### 8.2 flutter_rust_bridge 使用示例

**Rust 端 (src/lib.rs)**：

```rust
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultData {
    pub profile_id: String,
    pub encrypted_blob: Vec<u8>,
}

#[frb]
pub struct UnlockResult {
    pub success: bool,
    pub error: Option<String>,
}

#[frb]
pub async fn unlock_vault(passphrase: String, salt: Vec<u8>) -> UnlockResult {
    // 解锁逻辑
    // 返回结果
}

#[frb]
pub async fn encrypt_blob(data: Vec<u8>, key: Vec<u8>) -> Vec<u8> {
    // AES-256-GCM 加密
}

#[frb]
pub async fn decrypt_blob(encrypted: Vec<u8>, key: Vec<u8>) -> Vec<u8> {
    // AES-256-GCM 解密
}

#[frb]
pub async fn sync_to_cloud(blob: Vec<u8>, endpoint: String) -> Result<(), String> {
    // 上传到 S3/B2
}
```

**Flutter 端 (直接调用)**：

```dart
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

class SoloVault {
  final vault = VaultImpl();

  Future<UnlockResult> unlock(String passphrase, Uint8List salt) async {
    return await vault.unlockVault(passphrase, salt);
  }

  Future<Uint8List> encrypt(Uint8List data, Uint8List key) async {
    return await vault.encryptBlob(data, key);
  }

  Future<Uint8List> decrypt(Uint8List encrypted, Uint8List key) async {
    return await vault.decryptBlob(encrypted, key);
  }
}
```

### 8.3 加密 Blob 格式

```
┌─────────────────────────────────────────────────────────┐
│ Magic (4 bytes)    │ 0x53 4F 4C 4F ("SOLO")            │
├─────────────────────────────────────────────────────────┤
│ Version (1 byte)   │ 0x02 (Rust 重构版本)              │
├─────────────────────────────────────────────────────────┤
│ Nonce (12 bytes)   │ 随机数 (aes-gcm)                 │
├─────────────────────────────────────────────────────────┤
│ Ciphertext (*)     │ AES-256-GCM 加密后的数据           │
├─────────────────────────────────────────────────────────┤
│ Auth Tag (16 bytes)│ 认证标签                          │
├─────────────────────────────────────────────────────────┤
│ Checksum (32 bytes)│ BLAKE3(data) 用于完整性验证       │
└─────────────────────────────────────────────────────────┘
```

### 8.4 Rust Crates 依赖

```toml
[dependencies]
# 加密
aes-gcm = "0.10"
argon2 = "0.5"
blake3 = "1.0"
zeroize = "1.7"

# 存储 (双重加密：应用层 + 存储层)
rusqlite = { version = "0.31", default-features = false, features = ["bundled"] }
sqlcipher = "4.5"

# 异步
tokio = { version = "1", features = ["full"] }

# FFI
flutter_rust_bridge = "2"
cbindgen = "0.24"

# HTTP/云存储
reqwest = { version = "0.12", features = ["json"] }
aws-config = "1"
aws-sdk-s3 = "1"

# WebSocket
tokio-tungstenite = "0.21"

# 错误处理
thiserror = "1"
anyhow = "1"
```

### 8.5 编译目标

```bash
# macOS
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Android (NDK)
cargo build --release --target aarch64-linux-android --lib

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

### 8.7 性能优化与 UX 设计

**核心原则**: "计算任务全部异步化" + "UI 优先展示缓存"

| 性能杀手 | 严重程度 | 解决方案 |
|----------|----------|----------|
| Argon2id 启动延迟 | ⭐⭐⭐⭐⭐ | 生物识别 + 临时 Key 绕过 |
| FFI 通信阻塞 | ⭐⭐⭐ | 异步 Isolate 运行 Rust |
| Wasm 冷启动 | ⭐⭐⭐ | 预加载 + AOT 模块序列化 |
| SQLCipher I/O | ⭐⭐ | WAL 模式 + 局部解密 |
| 移动端电池/散热 | ⭐⭐⭐⭐ | 前台服务 + 低频轮询 |

#### 8.7.1 Argon2id 登录优化

**问题**: Argon2id 计算需要 1-2 秒，阻塞 UI 线程

**解决方案**:

```dart
class LoginPage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<UnlockResult>(
      future: _performUnlock(context), // 在后台 Isolate 执行
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.waiting) {
          return SkeletonLoader(); // 骨架屏动画
        }
        // ... 登录结果
      },
    );
  }

  Future<UnlockResult> _performUnlock(BuildContext context) async {
    // 严禁在 UI 线程执行 Argon2id
    return await RustCore.unlockVault(masterPassword);
  }
}
```

**生物识别绕过**:
```dart
class VaultStore {
  /// 平时使用 TouchID/FaceID 解锁临时密钥
  Future<bool> biometricUnlock() async {
    final tempKey = await flutter_secure_storage
        .read(key: 'temp_session_key');
    if (tempKey != null) {
      // 无需 Argon2id，直接用临时密钥解锁
      return await RustCore.unlockWithKey(tempKey);
    }
    return false;
  }
}
```

#### 8.7.2 Wasm 预热与 AOT 编译

**问题**: Wasmtime 冷启动需要数秒

**解决方案**:

```rust
pub struct WasmEngine {
    engine: wasmtime::Engine,
    compiled_modules: HashMap<String, Arc<CompiledModule>>,
}

impl WasmEngine {
    /// 预热：在用户进入详情页时静默初始化
    pub fn prewarm(&self, plugin_id: &str, wasm_bytes: &[u8]) {
        // AOT 编译并缓存
        let compiled = wasmtime::Module::compile(
            &self.engine,
            wasm_bytes,
        )?;

        // 序列化保存
        let serialized = compiled.serialize()?;
        self.compiled_modules.insert(
            plugin_id.to_string(),
            Arc::new(serialized),
        );
    }

    /// 即时加载（从缓存）
    pub fn load_instant(&self, plugin_id: &str) -> Result<wasmtime::Module, Error> {
        if let Some(compiled) = self.compiled_modules.get(plugin_id) {
            return Ok(wasmtime::Module::deserialize(
                &self.engine,
                compiled.as_ref(),
            )?);
        }
        Err(Error::PluginNotPrewarmed)
    }
}
```

**Flutter 端预热时机**:
```dart
// 用户进入任务详情页时，静默预热 Wasm
class TaskDetailPage extends StatefulWidget {
  @override
  void initState() {
    super.initState();
    // 预热下一个可能用到的插件
    WasmEngine.prewarm('visa-booking-plugin');
  }
}
```

#### 8.7.3 FFI 通信优化

**问题**: 频繁跨语言调用导致 UI 粘滞

**解决方案**:

```dart
class ProfileViewModel {
  // Rust 端一次性推送常用数据，Flutter 端缓存
  final _cache = HashMap<String, dynamic>();

  /// 批量获取（非敏感）UI 数据
  Future<ProfileSummary> loadProfileSummary() async {
    if (_cache.containsKey('profile_summary')) {
      return _cache['profile_summary']; // 直接从缓存返回
    }
    final summary = await RustCore.getProfileSummary();
    _cache['profile_summary'] = summary;
    return summary;
  }

  /// 数据变更时通知缓存失效
  void onDataChanged() {
    _cache.remove('profile_summary');
  }
}
```

#### 8.7.4 软着陆强制下线

**问题**: 突然弹窗强制下线导致离线数据丢失

**解决方案**:

```dart
class VaultState {
  ConnectionState state = ConnectionState.connected;

  void onSessionRevoked() {
    // 不直接跳转登录，而是切换到只读模式
    state = ConnectionState.readOnly;
    notifyListeners();
  }
}

class ProfileEditPage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return BlocBuilder<VaultCubit, VaultState>(
      builder: (context, state) {
        if (state == VaultState.readOnly) {
          return Column(
            children: [
              // 半透明遮罩提示
              Container(
                color: Colors.black54,
                child: AlertDialog(
                  title: Text('已失去连接'),
                  content: Text('当前为只读模式，可查看数据但无法保存'),
                  actions: [
                    TextButton('离线保存', onPressed: () {
                      // 保存到本地队列
                      context.read<VaultCubit>().saveOffline();
                    }),
                    TextButton('重新登录'),
                  ],
                ),
              ),
              // 允许继续查看，但禁止同步
              _buildReadOnlyContent(),
            ],
          );
        }
        return _buildEditableContent();
      },
    );
  }
}
```

#### 8.7.5 SQLCipher 性能优化

**解决方案**:

```rust
// 初始化时配置 SQLite
fn init_database(path: &Path, key: &[u8]) -> Result<Connection, Error> {
    let conn = Connection::open(path)?;

    // 1. 开启 WAL 模式，提高并发读写
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    // 2. 设置同步模式为 NORMAL，平衡安全与性能
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;

    // 3. 合理设置缓存大小
    conn.execute_batch("PRAGMA cache_size=-64000;")?; // 64MB

    // 4. 对查询字段建立索引
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_profile_id ON profiles(profile_id);"
    )?;

    // 5. 使用参数化查询，避免 SQL 注入
    let mut stmt = conn.prepare(
        "SELECT * FROM profiles WHERE profile_id = ?1"
    )?;

    Ok(conn)
}
```

#### 8.7.6 移动端电池与散热

**问题**: 长时间排队任务导致手机发烫、进程被杀

**解决方案**:

```dart
// Flutter 端：使用 flutter_local_notifications 实现前台服务
class BackgroundTaskService {
  static Future<void> startReservationTask(Task task) async {
    // 1. 开启前台服务，显示常驻通知
    await flutter_local_notifications.show(
      id: task.id,
      title: '预约任务运行中',
      body: '点击查看详情',
      payload: task.id,
    );

    // 2. 通知 Rust 端开始任务
    await RustCore.startTask(task);
  }

  // 3. 收到推送时唤醒
  static void onPushReceived(RemoteMessage message) {
    if (message.data['action'] == 'wake_wasm') {
      RustCore.wakeWasmEngine();
    }
  }
}
```

**Rust 端低频轮询**:
```rust
pub struct TaskScheduler {
    is_running: AtomicBool,
}

impl TaskScheduler {
    /// 排队期间降低频率，收到信号才唤醒
    pub fn start_polling(&self, task_id: &str) {
        self.is_running.store(true, Ordering::SeqCst);

        tokio::spawn(async move {
            while self.is_running.load(Ordering::SeqCst) {
                // 检查是否收到唤醒信号
                if self.check_wake_signal(task_id).await? {
                    // 唤醒并执行
                    self.wake_and_execute(task_id).await?;
                }

                // 低频轮询：10 秒检查一次
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }
}
```

---

### 8.6 参考资料

- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) - Dart ↔ Rust 桥接
- [RustCrypto](https://github.com/RustCrypto) - 加密算法集合
- [Argon2 RFC](https://datatracker.ietf.org/doc/html/rfc9106) - Argon2 规范
- [AES-GCM NIST](https://csrc.nist.gov/publications/detail/sp/800-38d/final) - AES-GCM 规范
- [OWASP Mobile](https://github.com/OWASP/owasp-mstg) - 移动安全测试指南
- [zeroize](https://docs.rs/zeroize/) - 安全内存清理

---

---

## 9. 多平台客户端开发路线图 (Flutter 扩展)

> 创建日期: 2026-04-16
> 版本: 1.0

### 9.1 项目概述

**目标**: 将现有的 macOS Flutter 客户端扩展到 Android、iOS 和 Windows 平台，保持一致的 UI/UX体验。

**现有资产**:
- Flutter 项目位于 `flutter/` 目录
- macOS 客户端已完全开发完成
- 共享代码: 所有页面、主题、状态管理、业务逻辑

### 9.2 平台开发顺序与工作目录

| 优先级 | 平台 | 工作目录 | 预计工作量 |
|--------|------|----------|------------|
| P1 | iOS | `flutter_ios/` | 2-3 周 |
| P2 | Android | `flutter_android/` | 2-3 周 |
| P3 | Windows | `flutter_windows/` | 2-3 周 |

### 9.3 iOS 客户端开发计划

**前置条件**:
- macOS + Xcode 环境
- Apple Developer Program 账户 ($99/year)
- Rust 工具链 (用于编译 crypto-argon2)

**里程碑**:

| 阶段 | 任务 | 交付物 |
|------|------|--------|
| M1 | 准备 iOS 工作目录 | `flutter_ios/` 完整副本 |
| M2 | 编译 Rust 库为 iOS Framework | `libsolosoul_core.framework` |
| M3 | 配置 iOS 项目 | `ios/Runner.xcworkspace` |
| M4 | 集成 iOS Keychain | SecureStorage 使用 iOS Keychain |
| M5 | 集成 Face ID / Touch ID | Biometric authentication |
| M6 | 配置 App Icon 和发布资源 | Assets.xcassets |
| M7 | 测试与构建验证 | `flutter build ios --simulator --no-codesign` |

**技术要点**:
- Rust 库需编译为 iOS Simulator (x86_64, arm64) 和真机 (arm64)
- 使用 `cargo build --release --target aarch64-apple-ios` 交叉编译
- iOS Keychain 存储 session token 和 salt
- 支持 Face ID / Touch ID 解锁

**文件结构**:
```
flutter_ios/
├── lib/                    # 共享 Flutter 代码 (符号链接或复制)
├── ios/                    # iOS 原生项目
│   └── Runner/
│       └── libsolosoul_core.framework  # Rust 库
├── pubspec.yaml            # Flutter 依赖
└── .gitignore
```

### 9.4 Android 客户端开发计划

**前置条件**:
- Android Studio
- Android SDK (API 24+)
- Rust 工具链 + Android NDK
- Google Play Developer Console 账户

**里程碑**:

| 阶段 | 任务 | 交付物 |
|------|------|--------|
| M1 | 准备 Android 工作目录 | `flutter_android/` 完整副本 |
| M2 | 编译 Rust 库为 Android SO | `libsolosoul_core.so` (多架构) |
| M3 | 配置 Android 项目 | `android/app/build.gradle` |
| M4 | 集成 Android Keystore | SecureStorage 使用 Keystore |
| M5 | 集成 BiometricPrompt | Fingerprint / Face authentication |
| M6 | 配置 App Icon 和发布资源 | `android/app/src/main/res/` |
| M7 | 测试与构建验证 | `flutter build apk --debug` |

**技术要点**:
- Rust 库需编译为 arm64-v8a, armeabi-v7a, x86_64, x86 架构
- 使用 `cargo build --release --target aarch64-linux-android` 交叉编译
- Android Keystore 存储加密密钥
- 支持指纹/面容识别

**文件结构**:
```
flutter_android/
├── lib/                    # 共享 Flutter 代码
├── android/                # Android 原生项目
│   └── app/
│       └── src/main/jniLibs/  # Rust SO 库
├── pubspec.yaml
└── .gitignore
```

### 9.5 Windows 客户端开发计划

**前置条件**:
- Windows PC (或 macOS + Parallels/VMware)
- Visual Studio 2022 + C++ 工具链
- Rust 工具链
- Microsoft Store Developer 账户

**里程碑**:

| 阶段 | 任务 | 交付物 |
|------|------|--------|
| M1 | 准备 Windows 工作目录 | `flutter_windows/` 完整副本 |
| M2 | 编译 Rust 库为 Windows DLL | `solosoul_core.dll` |
| M3 | 配置 Windows 项目 | `windows/` CMake 项目 |
| M4 | 集成 Windows Credential Manager | SecureStorage 使用 Credential Manager |
| M5 | 集成 Windows Hello | Biometric authentication |
| M6 | 系统托盘和快捷键 | System tray + global shortcuts |
| M7 | 配置 App Icon 和发布资源 | Windows icon assets |
| M8 | 测试与构建验证 | `flutter build windows` |

**技术要点**:
- Rust 库使用 `x86_64-pc-windows-msvc` 目标编译
- Windows 存储使用 Credential Manager (类似 Keychain)
- 支持 Windows Hello (指纹/面容/PIN)
- 可选: 系统托盘图标和全局快捷键

**文件结构**:
```
flutter_windows/
├── lib/                    # 共享 Flutter 代码
├── windows/                # Windows 原生项目
│   └── solosoul_core.dll   # Rust 库
├── pubspec.yaml
└── .gitignore
```

### 9.6 跨平台通用任务

**所有平台共享**:

| 任务 | 说明 |
|------|------|
| Flutter 依赖同步 | 确保 `pubspec.yaml` 在所有平台一致 |
| Rust FFI 编译 | 为每个平台编译原生库 |
| 安全存储抽象 | 统一 `SecureStorageService` 接口 |
| 生物识别抽象 | 统一 `BiometricService` 接口 |
| 应用签名 | 配置各平台签名证书 |
| 图标资源 | 统一设计，应用图标适配 |
| 隐私政策 | 准备各平台隐私政策文档 |

### 9.7 Rust FFI 编译脚本

```bash
#!/bin/bash
# build_rust_ios.sh

# iOS
cargo build --release --target aarch64-apple-ios
cargo build --release --target x86_64-apple-ios --features macos

# Android
cargo build --release --target aarch64-linux-android
cargo build --release --target armv7-linux-androideabi
cargo build --release --target x86_64-linux-android

# Windows (需要 x86_64-pc-windows-msvc 目标)
cargo build --release --target x86_64-pc-windows-msvc
```

### 9.8 风险与注意事项

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| iOS 发布审核拒绝 | 高 | 提前检查 App Store Guidelines |
| Rust 交叉编译失败 | 中 | 使用 Docker 或专用构建机器 |
| Android 碎片化 | 中 | 测试主流设备 (Pixel, Samsung) |
| Windows Defender 误报 | 低 | 申请白名单，代码签名 |
| 各平台 Keychain 差异 | 低 | 抽象统一接口 |

---

## 附录 A: 术语表

| 术语 | 说明 |
|------|------|
| KDF | Key Derivation Function，密钥派生函数 |
| E2EE | End-to-End Encryption，端到端加密 |
| Blob | Binary Large Object，二进制大对象 |
| DPAPI | Data Protection API，Windows 数据保护 API |
| Sequence Number | 序列号，用于版本控制 |
| Dirty Flag | 脏标记，表示数据已修改但未同步 |
| mlock | 系统调用，防止内存被交换到磁盘 |
| JIT | Just-in-Time，仅在需要时才解密 |
| Zeroize | Rust trait，强制内存清零 |
| Wasm | WebAssembly，插件沙盒运行时 |
| Host Functions | 宿主提供给 Wasm 调用的安全接口 |
| Rate Limiting | 频率限制，防止高频攻击 |
| Circuit Breaker | 熔断机制，异常时自动中断 |
| Memory Pinning | 内存钉扎，防止换页到磁盘 |

## 附录 B: 决策记录

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-04-14 | 选择 Flutter + Go Core (v1.0) | 单一代码库，复用 Go crypto，平衡效率与安全 |
| 2026-04-14 | 排他登录 + 序列号 | 避免加密数据合并冲突 |
| 2026-04-14 | 本地优先 + 冲突解决 UI | 平衡离线可用性与数据一致性 |
| 2026-04-14 | S3/B2/R2 作为云存储 | 成本低，S3 兼容，支持加密 API |
| 2026-04-14 | **升级为 Rust Core (v2.0)** | Go CGO 有 FFI 开销，Rust 零成本抽象，内存安全更强，已有 crypto-argon2 经验 |
| 2026-04-14 | **Wasm 沙盒架构** | 插件隔离执行，Host Functions 严格接口 |
| 2026-04-14 | **内存锁定 (mlock)** | 防止明文进入 swap，避免冷启动攻击 |
| 2026-04-14 | **JIT 即时解密** | 排队期间只存 Task ID，提交时才解密 |
| 2026-04-14 | **Zeroize 阅后即焚** | 任务结束后强制覆写内存 |
| 2026-04-14 | **网络白名单隔离** | 插件只能访问白名单域名，禁止带外传输 |
| 2026-04-14 | **链式审计日志** | 不可篡改，记录所有敏感操作 |
| 2026-04-14 | **Rate Limiting + 熔断** | 防止高频请求攻击被劫持的插件 |
| 2026-04-14 | **插件握手协议** | SHA-256 白名单校验，防止木马篡改 |
| 2026-04-14 | **异步线程池 (Tokio)** | UI 与加密/Wasm 隔离，界面不卡顿 |
| 2026-04-14 | **全局核弹开关** | 紧急情况下立即销毁所有敏感数据 |
| 2026-04-14 | **SQLCipher 双重加密** | 应用层 AES-256-GCM + 存储层 SQLCipher，即使物理存储被拿走也无法打开 |
| 2026-04-14 | **性能优化原则** | "计算任务全部异步化" + "UI 优先展示缓存" |
| 2026-04-14 | **生物识别解锁** | 平时使用 TouchID/FaceID 解锁临时密钥，避免每次都跑 Argon2id |
| 2026-04-14 | **软着陆强制下线** | 只读模式而非直接跳转，允许离线保存 |
