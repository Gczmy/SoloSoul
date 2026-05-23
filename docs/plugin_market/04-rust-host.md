## 5. Rust Host 侧实现

### 5.1 PluginStore（插件独立目录管理）

```rust
//! PluginStore - 管理插件的独立安装目录
//!
//! 插件存储在 ~/.solosoul/plugins/ 下，与 Vault 数据完全隔离。

use std::path::PathBuf;

pub struct PluginStore {
    base_dir: PathBuf,
}

impl PluginStore {
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("No home directory")?;
        let base_dir = home.join(".solosoul").join("plugins");
        std::fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;
        
        // 设置目录权限 0700
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&base_dir, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
        
        Ok(Self { base_dir })
    }

    pub fn plugin_dir(&self, plugin_id: &str) -> PathBuf {
        self.base_dir.join(plugin_id)
    }

    pub fn load_wasm(&self, plugin_id: &str) -> Result<Vec<u8>, String> {
        let path = self.plugin_dir(plugin_id).join("plugin.wasm");
        std::fs::read(&path)
            .map_err(|e| format!("Failed to read wasm: {}", e))
    }

    pub fn load_manifest(&self, plugin_id: &str) -> Result<PluginManifest, String> {
        let path = self.plugin_dir(plugin_id).join("manifest.json");
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    pub fn save_plugin(&self, plugin_id: &str, wasm: &[u8], manifest: &PluginManifest) -> Result<(), String> {
        let dir = self.plugin_dir(plugin_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;
        
        std::fs::write(dir.join("plugin.wasm"), wasm)
            .map_err(|e| format!("Failed to write wasm: {}", e))?;
        
        let manifest_json = serde_json::to_string_pretty(manifest)
            .map_err(|e| e.to_string())?;
        std::fs::write(dir.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;
        
        Ok(())
    }

    pub fn remove_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let dir = self.plugin_dir(plugin_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to remove plugin: {}", e))?;
        }
        Ok(())
    }

    pub fn list_installed(&self) -> Result<Vec<String>, String> {
        let mut plugins = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir)
            .map_err(|e| format!("Failed to read plugin dir: {}", e))? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // 排除系统文件/目录，并校验目录下包含合法的 manifest + wasm
                    if !name.starts_with('.') && self.is_valid_plugin_dir(name) {
                        plugins.push(name.to_string());
                    }
                }
            }
        }
        Ok(plugins)
    }

    fn is_valid_plugin_dir(&self, plugin_id: &str) -> bool {
        let dir = self.plugin_dir(plugin_id);
        dir.join("manifest.json").is_file() && dir.join("plugin.wasm").is_file()
    }
}
```

### 5.2 核心数据结构

```rust
//! Host functions - Secure interface between Wasm and Rust Core

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use wasmtime::{Caller, Linker};
use zeroize::Zeroizing;

use super::manifest::PluginManifest;

/// 请求 Flutter 层用户确认
pub struct ConsentChannel {
    pub tx: mpsc::Sender<ConsentRequest>,
}

pub struct ConsentRequest {
    pub plugin_id: String,
    pub field: String,
    pub session_id: String,
    pub sensitivity: SensitivityLevel,
    pub response: oneshot::Sender<ConsentResult>,
}

pub enum ConsentResult {
    Approved(String),
    Denied,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityLevel {
    Public,
    Internal,
    Sensitive,
    Critical,
}

/// 字段访问频率限制器
pub struct RateLimiter {
    /// plugin_id -> field -> (last_reset, count)
    counters: Mutex<HashMap<String, HashMap<String, (Instant, u32)>>>,
    max_per_minute: u32,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    pub fn check(&self, plugin_id: &str, field: &str) -> bool {
        let mut counters = self.counters.lock().unwrap();
        let plugin_map = counters.entry(plugin_id.to_string()).or_default();
        let entry = plugin_map.entry(field.to_string()).or_insert((Instant::now(), 0));

        // 每分钟重置计数器
        if entry.0.elapsed() > Duration::from_secs(60) {
            entry.0 = Instant::now();
            entry.1 = 0;
        }

        entry.1 += 1;
        entry.1 <= self.max_per_minute
    }
}

pub struct SoloHostFunctions {
    pub plugin_id: String,
    pub session_id: String,
    pub manifest: PluginManifest,
    pub consent_tx: mpsc::Sender<ConsentRequest>,
    pub audit_tx: mpsc::Sender<AuditEntry>,
    pub rate_limiter: Arc<RateLimiter>,
    pub session_expires_at: Instant,
    pub wasi: wasmtime_wasi::WasiCtx,
}

impl SoloHostFunctions {
    pub fn new(
        plugin_id: &str,
        session_id: &str,
        manifest: PluginManifest,
        consent_tx: mpsc::Sender<ConsentRequest>,
        audit_tx: mpsc::Sender<AuditEntry>,
        rate_limiter: Arc<RateLimiter>,
        ttl_seconds: u64,
    ) -> Self {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().inherit_stdio().build();
        Self {
            plugin_id: plugin_id.to_string(),
            session_id: session_id.to_string(),
            manifest,
            consent_tx,
            audit_tx,
            rate_limiter,
            session_expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
            wasi,
        }
    }

    /// 注册所有 Host Functions 到 Linker
    pub fn register(linker: &mut Linker<Self>) -> Result<(), String> {
        // solosoul_request_field(field_id_ptr, field_id_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "solosoul",
                "request_field",
                |mut caller: Caller<'_, Self>,
                 field_id_ptr: i32,
                 field_id_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let funcs = caller.data();
                    let field_id =
                        read_memory(&caller, field_id_ptr as usize, field_id_len as usize);

                    // 1. 速率限制检查
                    if !funcs.rate_limiter.check(&funcs.plugin_id, &field_id) {
                        log_audit(
                            &funcs.audit_tx,
                            &funcs.plugin_id,
                            &funcs.session_id,
                            AuditAction::RateLimitTriggered {
                                field: field_id.clone(),
                            },
                        );
                        return -8; // RateLimited
                    }

                    // 2. 校验字段是否在 manifest 声明范围内
                    if !funcs.manifest.is_field_requested(&field_id) {
                        log_audit(
                            &funcs.audit_tx,
                            &funcs.plugin_id,
                            &funcs.session_id,
                            AuditAction::FieldAccessDenied {
                                field: field_id.clone(),
                            },
                        );
                        return -1; // PermissionDenied
                    }

                    // 3. 校验 Session 未过期
                    if Instant::now() > funcs.session_expires_at {
                        return -3; // TtlExpired
                    }

                    // 4. 根据敏感度决定是否需要用户确认
                    let sensitivity = resolve_field_sensitivity(&field_id);
                    let needs_confirmation = match sensitivity {
                        SensitivityLevel::Public => false,
                        SensitivityLevel::Internal => false,
                        SensitivityLevel::Sensitive | SensitivityLevel::Critical => true,
                    };

                    if !needs_confirmation {
                        match funcs.decrypt_field_sync(&field_id) {
                            Ok(value) => {
                                if value.len() >= out_cap as usize {
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, &value);
                                log_audit(
                                    &funcs.audit_tx,
                                    &funcs.plugin_id,
                                    &funcs.session_id,
                                    AuditAction::FieldAccessGranted {
                                        field: field_id,
                                        confirmed_by_user: false,
                                    },
                                );
                                return 0;
                            }
                            Err(_) => return -5,
                        }
                    }

                    // 5. 敏感字段：通过 Flutter 通道请求用户确认
                    let (tx, rx) = oneshot::channel();
                    let request = ConsentRequest {
                        plugin_id: funcs.plugin_id.clone(),
                        field: field_id.clone(),
                        session_id: funcs.session_id.clone(),
                        sensitivity,
                        response: tx,
                    };

                    if funcs.consent_tx.try_send(request).is_err() {
                        return -1;
                    }

                    // 6. 阻塞等待 Flutter 用户响应（超时 60s）
                    // 注意：若超时，Rust 侧的 oneshot::Receiver 被 drop，但 Flutter 端弹窗可能仍然存在。
                    // 应通过 PluginEvent::ConsentTimeout 通知 Dart 关闭弹窗，防止状态泄漏。
                    match rx.blocking_recv() {
                        Ok(ConsentResult::Approved(value)) => {
                            if value.len() >= out_cap as usize {
                                return -4;
                            }
                            write_memory(&mut caller, out_ptr as usize, &value);
                            log_audit(
                                &funcs.audit_tx,
                                &funcs.plugin_id,
                                &funcs.session_id,
                                AuditAction::FieldAccessGranted {
                                    field: field_id,
                                    confirmed_by_user: true,
                                },
                            );
                            0
                        }
                        Ok(ConsentResult::Denied) => -2,
                        Ok(ConsentResult::Expired) | Err(_) => -3,
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_post_data(...) -> i32
        linker
            .func_wrap(
                "solosoul",
                "post_data",
                |mut caller: Caller<'_, Self>,
                 url_ptr: i32,
                 url_len: i32,
                 body_ptr: i32,
                 body_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let funcs = caller.data();
                    let url = read_memory(&caller, url_ptr as usize, url_len as usize);
                    let body = read_memory(&caller, body_ptr as usize, body_len as usize);

                    if !funcs.is_network_allowed(&url) {
                        log_audit(
                            &funcs.audit_tx,
                            &funcs.plugin_id,
                            &funcs.session_id,
                            AuditAction::NetworkBlocked { url: url.clone() },
                        );
                        return -10;
                    }

                    let rt = tokio::runtime::Handle::try_current();
                    let response = match rt {
                        Ok(handle) => handle.block_on(async {
                            funcs.proxy_http_post(&url, &body).await
                        }),
                        Err(_) => return -1,
                    };

                    match response {
                        Ok(data) => {
                            if data.len() >= out_cap as usize {
                                return -4;
                            }
                            write_memory(&mut caller, out_ptr as usize, &data);
                            0
                        }
                        Err(HttpError::Timeout) => -6,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn is_network_allowed(&self, url: &str) -> bool {
        let Some(ref policy) = self.manifest.network_policy else {
            return false; // 默认拒绝所有出站访问
        };
        if policy.block_all_outbound {
            return false; // 全部阻止，仅白名单例外
        }
        // 解析域名并匹配白名单（支持通配符如 *.solosoul.dev）
        let host = extract_host(url);
        policy.allowed_domains.iter().any(|pattern| match_domain(pattern, host))
    }

    fn decrypt_field_sync(&self, field_id: &str) -> Result<String, String> {
        let query = resolve_field_to_vault_query(field_id)?;
        Ok(String::new()) // vault::get_and_decrypt(query)
    }

    async fn proxy_http_post(&self, _url: &str, _body: &str) -> Result<Vec<u8>, HttpError> {
        Ok(vec![])
    }
}

fn read_memory(caller: &Caller<'_, SoloHostFunctions>, ptr: usize, len: usize) -> String {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else {
        return String::new();
    };
    let mut buf = vec![0u8; len];
    memory.read(&caller, ptr, &mut buf).unwrap_or(());
    String::from_utf8_lossy(&buf).to_string()
}

fn write_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, value: &str) {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else { return };
    memory.write(caller, ptr, value.as_bytes()).unwrap_or(());
}

fn resolve_field_sensitivity(field_id: &str) -> SensitivityLevel {
    match field_id {
        "identity.full_name" => SensitivityLevel::Public,
        "identity.contact.emails" | "identity.contact.phones" => SensitivityLevel::Internal,
        "identity.id_card.number" | "travel.primary_passport.number" => SensitivityLevel::Critical,
        _ => SensitivityLevel::Sensitive,
    }
}

use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    /// 字段路径到 VaultQuery 的运行时映射表
    /// 新增字段时仅需修改此表，无需重新编译核心逻辑
    static ref FIELD_MAP: HashMap<&'static str, VaultQuery> = {
        let mut m = HashMap::new();
        m.insert("identity.full_name", VaultQuery::Property {
            object_type: "identity".to_string(),
            property_key: "full_name".to_string(),
            tag: None,
        });
        m.insert("travel.primary_passport.number", VaultQuery::Property {
            object_type: "passport".to_string(),
            property_key: "number".to_string(),
            tag: Some("primary".to_string()),
        });
        // TODO: 从 ~/.solosoul/field_mapping.json 热加载自定义映射
        m
    };
}

fn resolve_field_to_vault_query(field_id: &str) -> Result<VaultQuery, String> {
    FIELD_MAP.get(field_id)
        .cloned()
        .ok_or_else(|| format!("unknown field: {}", field_id))
}

#[derive(Debug)]
pub enum HttpError {
    Timeout,
    Network,
}

#[derive(Debug, Clone)]
pub struct VaultQuery {
    pub object_type: String,
    pub property_key: String,
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub plugin_id: String,
    pub session_id: String,
    pub action: AuditAction,
}

#[derive(Clone, Debug)]
pub enum AuditAction {
    FieldAccessGranted {
        field: String,
        confirmed_by_user: bool,
    },
    FieldAccessDenied {
        field: String,
    },
    NetworkBlocked {
        url: String,
    },
    RateLimitTriggered {
        field: String,
    },
    PluginCrashed {
        reason: String,
    },
}

fn log_audit(
    tx: &mpsc::Sender<AuditEntry>,
    plugin_id: &str,
    session_id: &str,
    action: AuditAction,
) {
    let _ = tx.try_send(AuditEntry {
        plugin_id: plugin_id.to_string(),
        session_id: session_id.to_string(),
        action,
    });
}
```

### 5.3 沙盒执行与 TTL 管理

**TTL 清零方案：Store 级隔离（推荐）**

Wasm 线性内存由 Plugin 完全控制，Host 无法可靠地"清零某个区域"而不被 Plugin 绕过。因此采用**Store 级隔离**：每个 Plugin 拥有独立的 `wasmtime::Store`，敏感数据仅在 Store 存活期间可访问，TTL 到期后整个 Store 被 drop，Plugin 的所有内存（包括可能的副本）随 Store 一起销毁。

```rust
impl WasmSandbox {
    /// 执行插件，返回结果并自动清理敏感内存
    pub async fn execute(
        &self,
        module: &Module,
        host: SoloHostFunctions,
        ttl_seconds: u64,
    ) -> Result<PluginResult, PluginError> {
        let mut linker = Linker::new(&self.engine);
        SoloHostFunctions::register(&mut linker)?;

        // WASI 基础环境（标准输入输出，无文件系统访问）
        wasmtime_wasi::add_to_linker(&mut linker, |host: &mut SoloHostFunctions| {
            &mut host.wasi
        })?;

        let mut store = Store::new(&self.engine, host);
        store.add_fuel(10_000_000)?; // 防死循环

        let instance = linker.instantiate(&mut store, module)?;
        let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;

        // 捕获 Wasm Trap（插件 panic / 内存越界 / 除零等）
        let result = match run.call(&mut store, ()) {
            Ok(code) => Ok(PluginResult { exit_code: code }),
            Err(trap) => {
                log_audit(
                    &store.data().audit_tx,
                    &store.data().plugin_id,
                    &store.data().session_id,
                    AuditAction::PluginCrashed {
                        reason: trap.to_string(),
                    },
                );
                Err(PluginError::ExecutionFailed(trap.to_string()))
            }
        };

        // Store 离开作用域后被 drop，所有 Wasm 内存（含可能的敏感数据副本）清零
        drop(store);

        result
    }
}
```
