//! Host functions - Secure interface between Wasm and Rust Core
//!
//! Plugins can only access data through these strictly defined functions.
//! All access is logged, rate-limited, and subject to user consent.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use wasmtime::{Caller, Linker};

use super::manifest::PluginManifest;
use super::rust_log;
use crate::vault::{ProfileData, VersionedProfileData};

// ============================================================================
// Consent Channel
// ============================================================================

/// 请求 Flutter 层用户确认
#[derive(Debug, Clone)]
pub struct ConsentChannel {
    pub tx: mpsc::Sender<ConsentRequest>,
}

/// 单次字段访问的授权请求
#[derive(Debug)]
pub struct ConsentRequest {
    pub request_id: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub field: String,
    pub session_id: String,
    pub sensitivity: SensitivityLevel,
    pub response: std::sync::mpsc::Sender<ConsentResult>,
}

/// 用户授权结果
#[derive(Debug, Clone)]
pub enum ConsentResult {
    /// 用户授权，返回解密后的字段值
    Approved(String),
    /// 用户拒绝
    Denied,
    /// 超时（Rust 侧 60s 未收到响应）
    Expired,
}

// ============================================================================
// Sensitivity Level
// ============================================================================

/// 字段敏感度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityLevel {
    /// 公开数据，无需确认
    Public,
    /// 内部数据，无需确认
    Internal,
    /// 敏感数据，需要用户确认
    Sensitive,
    /// 关键数据，需要用户确认
    Critical,
}

impl SensitivityLevel {
    /// 是否需要用户显式确认
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, SensitivityLevel::Sensitive | SensitivityLevel::Critical)
    }
}

/// 根据字段路径解析敏感度（运行时映射表）
pub(crate) fn resolve_field_sensitivity(field_id: &str) -> SensitivityLevel {
    match field_id {
        "identity.full_name" => SensitivityLevel::Public,
        "identity.contact.emails" | "identity.contact.phones" => SensitivityLevel::Internal,
        "identity.id_card.number"
        | "travel.primary_passport.number"
        | "financial.primary_bank_account.number" => SensitivityLevel::Critical,
        // address: country/count 为公开，其余为敏感
        "address.country" | "address.count" => SensitivityLevel::Public,
        f if f.starts_with("address[") => {
            if f.ends_with("].country") || f.ends_with("].label") {
                SensitivityLevel::Public
            } else {
                SensitivityLevel::Sensitive
            }
        }
        _ => SensitivityLevel::Sensitive,
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

/// 字段访问频率限制器（10次/分钟/字段）
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

    /// 检查是否允许访问，同时增加计数
    pub fn check(&self, plugin_id: &str, field: &str) -> bool {
        let mut counters = self.counters.lock().unwrap();
        let plugin_map = counters.entry(plugin_id.to_string()).or_default();
        let entry = plugin_map
            .entry(field.to_string())
            .or_insert((Instant::now(), 0));

        // 每分钟重置计数器
        if entry.0.elapsed() > Duration::from_secs(60) {
            entry.0 = Instant::now();
            entry.1 = 0;
        }

        entry.1 += 1;
        entry.1 <= self.max_per_minute
    }
}

// ============================================================================
// Audit Log
// ============================================================================

/// 审计日志条目
#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub plugin_id: String,
    pub session_id: String,
    pub timestamp: Instant,
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
    NetworkAllowed {
        url: String,
    },
    RateLimitTriggered {
        field: String,
    },
    PluginCrashed {
        reason: String,
    },
    SessionCreated,
    SessionRevoked,
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
        timestamp: Instant::now(),
        action,
    });
}

// ============================================================================
// SoloHostFunctions
// ============================================================================

/// Host functions state for a single plugin session
pub struct SoloHostFunctions {
    pub plugin_id: String,
    pub plugin_name: String,
    pub session_id: String,
    pub manifest: PluginManifest,
    pub consent_tx: mpsc::Sender<ConsentRequest>,
    pub audit_tx: mpsc::Sender<AuditEntry>,
    pub log_tx: mpsc::Sender<(String, String)>,
    pub rate_limiter: Arc<RateLimiter>,
    pub session_expires_at: Instant,
    pub wasi: wasmtime_wasi::preview1::WasiP1Ctx,
    /// 预授权的字段集合（批量授权后填充）
    pub pre_approved_fields: HashSet<String>,
}

impl SoloHostFunctions {
    pub fn new(
        plugin_id: &str,
        plugin_name: &str,
        session_id: &str,
        manifest: PluginManifest,
        consent_tx: mpsc::Sender<ConsentRequest>,
        audit_tx: mpsc::Sender<AuditEntry>,
        log_tx: mpsc::Sender<(String, String)>,
        rate_limiter: Arc<RateLimiter>,
        ttl_seconds: u64,
        pre_approved_fields: HashSet<String>,
    ) -> Self {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .build_p1();
        Self {
            plugin_id: plugin_id.to_string(),
            plugin_name: plugin_name.to_string(),
            session_id: session_id.to_string(),
            manifest,
            consent_tx,
            audit_tx,
            log_tx,
            rate_limiter,
            session_expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
            wasi,
            pre_approved_fields,
        }
    }

    /// 注册所有 Host Functions 到 Linker
    pub fn register(linker: &mut Linker<Self>) -> Result<(), String> {
        // solosoul_request_field(field_id_ptr, field_id_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "env",
                "solosoul_request_field",
                |mut caller: Caller<'_, Self>,
                 field_id_ptr: i32,
                 field_id_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let field_id =
                        read_memory(&mut caller, field_id_ptr as usize, field_id_len as usize);

                    // 使用独立作用域获取 funcs 数据，避免与后续 write_memory 的 mutable borrow 冲突
                    let (plugin_id, session_id, manifest, rate_limiter, consent_tx, audit_tx, plugin_name, session_expires_at, pre_approved_fields) = {
                        let funcs = caller.data();
                        (
                            funcs.plugin_id.clone(),
                            funcs.session_id.clone(),
                            funcs.manifest.clone(),
                            Arc::clone(&funcs.rate_limiter),
                            funcs.consent_tx.clone(),
                            funcs.audit_tx.clone(),
                            funcs.plugin_name.clone(),
                            funcs.session_expires_at,
                            funcs.pre_approved_fields.clone(),
                        )
                    };

                    // 1. 速率限制检查
                    if !rate_limiter.check(&plugin_id, &field_id) {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::RateLimitTriggered {
                                field: field_id.clone(),
                            },
                        );
                        return -8; // RateLimited
                    }

                    // 2. 校验字段是否在 manifest 声明范围内
                    if !manifest.is_field_requested(&field_id) {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::FieldAccessDenied {
                                field: field_id.clone(),
                            },
                        );
                        return -1; // PermissionDenied
                    }

                    // 3. 校验 Session 未过期
                    if Instant::now() > session_expires_at {
                        return -3; // TtlExpired
                    }

                    // 4. 检查是否为预授权字段（批量授权模式，支持通配符匹配）
                    let is_pre_approved = pre_approved_fields.iter().any(|f| crate::plugin::manifest::PluginManifest::field_matches(f, &field_id));
                    if is_pre_approved {
                        match decrypt_field_value(&field_id) {
                            Ok(value) => {
                                if value.len() >= out_cap as usize {
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, &value);
                                log_audit(
                                    &audit_tx,
                                    &plugin_id,
                                    &session_id,
                                    AuditAction::FieldAccessGranted {
                                        field: field_id,
                                        confirmed_by_user: true,
                                    },
                                );
                                return 0;
                            }
                            Err(_e) => {
                                return -5; // InvalidField
                            }
                        }
                    }

                    // 5. 根据敏感度决定是否需要用户确认
                    let sensitivity = resolve_field_sensitivity(&field_id);
                    let needs_confirmation = sensitivity.needs_confirmation();

                    if !needs_confirmation {
                        match decrypt_field_value(&field_id) {
                            Ok(value) => {
                                if value.len() >= out_cap as usize {
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, &value);
                                log_audit(
                                    &audit_tx,
                                    &plugin_id,
                                    &session_id,
                                    AuditAction::FieldAccessGranted {
                                        field: field_id,
                                        confirmed_by_user: false,
                                    },
                                );
                                return 0;
                            }
                            Err(_e) => {
                                return -5; // InvalidField
                            }
                        }
                    }

                    // 5. 敏感字段：通过 Flutter 通道请求用户确认
                    let request_id = uuid::Uuid::new_v4().to_string();
                    let (tx, rx) = std::sync::mpsc::channel();
                    let request = ConsentRequest {
                        request_id: request_id.clone(),
                        plugin_id: plugin_id.clone(),
                        plugin_name: plugin_name.clone(),
                        field: field_id.clone(),
                        session_id: session_id.clone(),
                        sensitivity,
                        response: tx,
                    };

                    if consent_tx.try_send(request).is_err() {
                        return -1; // Consent channel closed
                    }

                    // 6. 阻塞等待 Flutter 用户响应（超时 60s）
                    // 使用 std::sync::mpsc::recv_timeout 防止 Dart 端因死锁或异常永远无法响应
                    {
                        use std::fs::OpenOptions;
                        use std::io::Write;
                        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/tmp/solosoul_rust.log") {
                            let _ = writeln!(file, "[{}] request_field waiting consent: field={}, request_id={}", now, field_id, request_id);
                        }
                    }
                    match rx.recv_timeout(Duration::from_secs(60)) {
                        Ok(ConsentResult::Approved(value)) => {
                            if value.len() >= out_cap as usize {
                                return -4;
                            }
                            write_memory(&mut caller, out_ptr as usize, &value);
                            log_audit(
                                &audit_tx,
                                &plugin_id,
                                &session_id,
                                AuditAction::FieldAccessGranted {
                                    field: field_id,
                                    confirmed_by_user: true,
                                },
                            );
                            0
                        }
                        Ok(ConsentResult::Denied) => -2,        // UserDenied
                        Ok(ConsentResult::Expired) => -3,
                        Err(_) => -3, // Timeout / Disconnected
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_post_data(url_ptr, url_len, body_ptr, body_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "env",
                "solosoul_post_data",
                |mut caller: Caller<'_, Self>,
                 url_ptr: i32,
                 url_len: i32,
                 body_ptr: i32,
                 body_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let url = read_memory(&mut caller, url_ptr as usize, url_len as usize);
                    let body = read_memory(&mut caller, body_ptr as usize, body_len as usize);

                    let (plugin_id, session_id, manifest, audit_tx) = {
                        let funcs = caller.data();
                        (
                            funcs.plugin_id.clone(),
                            funcs.session_id.clone(),
                            funcs.manifest.clone(),
                            funcs.audit_tx.clone(),
                        )
                    };

                    if !is_network_allowed(&manifest, &url) {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::NetworkBlocked { url: url.clone() },
                        );
                        return -10; // DomainNotAllowed
                    }

                    // 执行 HTTP POST（在 tokio runtime 中阻塞执行）
                    let response = match block_on_http(proxy_http_post(&url, &body)) {
                        Ok(data) => {
                            log_audit(
                                &audit_tx,
                                &plugin_id,
                                &session_id,
                                AuditAction::NetworkAllowed { url: url.clone() },
                            );
                            Ok(data)
                        }
                        Err(e) => Err(e),
                    };

                    match response {
                        Ok(data) => {
                            if data.len() >= out_cap as usize {
                                return -4; // BufferTooSmall
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

        // solosoul_log(level_ptr, level_len, msg_ptr, msg_len)
        linker
            .func_wrap(
                "env",
                "solosoul_log",
                |mut caller: Caller<'_, Self>,
                 level_ptr: i32,
                 level_len: i32,
                 msg_ptr: i32,
                 msg_len: i32| {
                    let level = read_memory(&mut caller, level_ptr as usize, level_len as usize);
                    let message = read_memory(&mut caller, msg_ptr as usize, msg_len as usize);

                    let funcs = caller.data();
                    let _ = funcs.audit_tx.try_send(AuditEntry {
                        plugin_id: funcs.plugin_id.clone(),
                        session_id: funcs.session_id.clone(),
                        timestamp: Instant::now(),
                        action: AuditAction::NetworkAllowed {
                            url: format!("[LOG:{}] {}", level, message),
                        },
                    });
                    let _ = funcs.log_tx.try_send((level, message));
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_get_timestamp() -> i64
        linker
            .func_wrap(
                "env",
                "solosoul_get_timestamp",
                |_caller: Caller<'_, Self>| -> i64 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

fn is_network_allowed(manifest: &PluginManifest, url: &str) -> bool {
    let Some(ref policy) = manifest.network_policy else {
        return false; // 默认拒绝所有出站访问
    };
    if policy.block_all_outbound {
        return false; // 全部阻止，仅白名单例外
    }
    // 解析域名并匹配白名单
    let host = extract_host(url);
    policy.allows_domain(&host)
}

/// 从 Vault 中同步解密字段值
///
/// 执行流程：
/// 1. 获取 AccountManager → session_key + vault_store
/// 2. 加载第一个 Profile 的加密 data
/// 3. 用 session_key 解密（支持 SOLO blob 和 Legacy Dart 格式）
/// 4. 反序列化为 ProfileData，按字段路径提取值
fn decrypt_field_value(field_id: &str) -> Result<String, String> {
    // 1. 获取 AccountManager
    let manager_guard = crate::get_account_manager()
        .map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    // 2. 获取 session_key
    let session_key = manager
        .get_session_key()
        .ok_or("Vault not unlocked")?;

    // 3. 获取 vault_store
    let vault_guard = manager
        .get_vault_store()
        .ok_or("Vault store not available")?;
    let vault_store = vault_guard
        .as_ref()
        .ok_or("Vault store not open")?;

    // 4. 列出 profiles 并取第一个
    let profiles = vault_store
        .list_profiles()
        .map_err(|e| format!("Failed to list profiles: {}", e))?;
    let profile_summary = profiles.first().ok_or("No profiles found")?;

    // 5. 加载 profile 的加密 data
    let profile = vault_store
        .load_profile(&profile_summary.id)
        .map_err(|e| format!("Failed to load profile: {}", e))?
        .ok_or("Profile not found")?;

    // 6. 解密
    let plaintext = crate::crypto::decrypt_profile_data(&session_key, &profile.data)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    // 7. 反序列化（支持 VersionedProfileData 和旧格式 ProfileData）
    let json_str = String::from_utf8_lossy(&plaintext);
    let data: ProfileData = match serde_json::from_str::<VersionedProfileData>(&json_str) {
        Ok(versioned) => versioned.data,
        Err(_) => {
            // 向后兼容：旧格式没有 version 包装层
            serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed to parse profile data: {}", e))?
        }
    };

    // 8. 按字段路径取值（旧格式）
    let result = extract_field_value(field_id, &data);
    if result.is_some() {
        return result.ok_or_else(|| format!("Field '{}' not found or empty", field_id));
    }

    // 9. 尝试从 Unified Object Model 提取（Flutter 新格式）
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse as JSON: {}", e))?;
    let uom_result = extract_from_unified_object_model(field_id, &json_value);
    uom_result.ok_or_else(|| format!("Field '{}' not found or empty", field_id))
}

/// 从 ProfileData 中按字段路径提取值
fn extract_field_value(field_id: &str, data: &ProfileData) -> Option<String> {
    // 支持 address 数组索引语法，如 address[0].street
    if let Some(addr_field) = field_id.strip_prefix("address[") {
        if let Some((idx_str, rest)) = addr_field.split_once("].") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                let identity = data.identity.as_ref()?;
                let addrs: Vec<_> = identity.addresses.iter().filter(|a| !a.is_deleted).collect();
                let addr = addrs.get(idx)?;
                return match rest {
                    "street" => addr.street.clone(),
                    "city" => addr.city.clone(),
                    "state" => addr.state.clone(),
                    "postalCode" => addr.postal_code.clone(),
                    "country" => addr.country.clone(),
                    "label" => addr.label.clone(),
                    _ => None,
                };
            }
        }
    }

    // address.count 返回未删除地址数量
    if field_id == "address.count" {
        let count = data.identity.as_ref()?.addresses.iter().filter(|a| !a.is_deleted).count();
        return Some(count.to_string());
    }

    // address.xxx 简写路径：默认映射到第一个未删除地址（主地址）
    if let Some(addr_key) = field_id.strip_prefix("address.") {
        let identity = data.identity.as_ref()?;
        let addr = identity.addresses.iter().find(|a| !a.is_deleted)?;
        return match addr_key {
            "street" => addr.street.clone(),
            "city" => addr.city.clone(),
            "state" => addr.state.clone(),
            "postalCode" => addr.postal_code.clone(),
            "country" => addr.country.clone(),
            "label" => addr.label.clone(),
            // district 在 AddressData 中不存在，返回 None
            _ => None,
        };
    }

    match field_id {
        "identity.full_name" => data.identity.as_ref()?.full_name.clone(),
        "identity.contact.emails" => {
            let emails: Vec<String> = data
                .identity
                .as_ref()?
                .contact
                .as_ref()?
                .entries
                .iter()
                .filter(|e| e.entry_type == "email")
                .map(|e| e.value.clone())
                .collect();
            if emails.is_empty() {
                None
            } else {
                Some(emails.join(", "))
            }
        }
        "identity.contact.phones" => {
            let phones: Vec<String> = data
                .identity
                .as_ref()?
                .contact
                .as_ref()?
                .entries
                .iter()
                .filter(|e| e.entry_type == "phone")
                .map(|e| e.value.clone())
                .collect();
            if phones.is_empty() {
                None
            } else {
                Some(phones.join(", "))
            }
        }
        "identity.id_card.number" => data
            .identity
            .as_ref()?
            .id_cards
            .iter()
            .find(|c| !c.is_deleted)
            .and_then(|c| c.number.clone()),
        "travel.primary_passport.number" => data
            .travel
            .as_ref()?
            .passports
            .iter()
            .find(|p| !p.is_deleted)
            .and_then(|p| p.number.clone()),
        "financial.primary_bank_account.number" => data
            .financial
            .as_ref()?
            .bank_accounts
            .iter()
            .find(|b| !b.is_deleted)
            .and_then(|b| b.account_number.clone()),
        _ => None,
    }
}

/// 从 Unified Object Model（Flutter 新格式）中提取字段值
///
/// UOM JSON 结构：
/// {
///   "unified_objects": {
///     "objects": [
///       {
///         "name": "Home Address",
///         "properties": {
///           "street": {"type": "text", "text": "...", "sensitivity": "..."},
///           ...
///         }
///       }
///     ]
///   }
/// }
/// 获取所有非删除的 profile_address 对象（按数组索引顺序）
fn get_address_objects(objects: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    let mut addrs: Vec<&serde_json::Value> = Vec::new();
    for obj in objects {
        if obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let type_id = obj.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
        if type_id == "profile_address" {
            addrs.push(obj);
        }
    }
    addrs
}

fn extract_from_unified_object_model(field_id: &str, json_value: &serde_json::Value) -> Option<String> {
    let objects = json_value
        .get("unified_objects")?
        .get("objects")?
        .as_array()?;

    // 1. 处理 address.count
    if field_id == "address.count" {
        let addrs = get_address_objects(objects);
        return Some(addrs.len().to_string());
    }

    // 2. 处理 address[N].xxx 数组索引语法
    if let Some(addr_field) = field_id.strip_prefix("address[") {
        if let Some((idx_str, rest)) = addr_field.split_once("].") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                let addrs = get_address_objects(objects);
                let addr = addrs.get(idx)?;
                let properties = addr.get("properties")?;
                let prop_key = match rest {
                    "street" => "street",
                    "city" => "city",
                    "state" => "state",
                    "postalCode" => "postalCode",
                    "country" => "country",
                    "district" => "district",
                    "label" => "label",
                    _ => rest,
                };
                let prop = properties.get(prop_key)?;
                return prop.get("text").and_then(|t| t.as_str()).map(|s| s.to_string());
            }
        }
    }

    // 3. 处理 address.xxx 简写路径（返回第一个非空地址的对应字段）
    if let Some(addr_key) = field_id.strip_prefix("address.") {
        let prop_key = match addr_key {
            "street" => "street",
            "city" => "city",
            "state" => "state",
            "postalCode" => "postalCode",
            "country" => "country",
            "district" => "district",
            "label" => "label",
            _ => addr_key,
        };
        let addrs = get_address_objects(objects);
        let mut first_empty: Option<String> = None;
        for addr in addrs {
            let properties = match addr.get("properties") {
                Some(p) => p,
                None => continue,
            };
            let prop = match properties.get(prop_key) {
                Some(p) => p,
                None => continue,
            };
            if let Some(text) = prop.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
                if first_empty.is_none() {
                    first_empty = Some(text.to_string());
                }
            }
        }
        return first_empty;
    }

    // 4. 通用字段路径映射（identity 等）
    let property_key = match field_id {
        "identity.full_name" => "fullName",
        "identity.given_name" => "givenName",
        "identity.family_name" => "familyName",
        "identity.date_of_birth" => "dateOfBirth",
        "identity.gender" => "gender",
        "identity.nationality" => "nationality",
        _ => {
            field_id.split('.').last().unwrap_or(field_id)
        }
    };

    let mut first_empty: Option<String> = None;
    for obj in objects {
        if obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let type_id = obj.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
        if type_id == "page" || type_id == "collection" {
            continue;
        }
        let properties = match obj.get("properties") {
            Some(p) => p,
            None => continue,
        };
        let prop = match properties.get(property_key) {
            Some(p) => p,
            None => continue,
        };
        if let Some(text) = prop.get("text").and_then(|t| t.as_str()) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(text.to_string());
            }
            continue;
        }
        if let Some(value) = prop.get("value").and_then(|v| v.as_f64()) {
            return Some(value.to_string());
        }
        if let Some(iso_date) = prop.get("isoDate").and_then(|d| d.as_str()) {
            if !iso_date.is_empty() {
                return Some(iso_date.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(iso_date.to_string());
            }
            continue;
        }
        if let Some(checked) = prop.get("checked").and_then(|c| c.as_bool()) {
            return Some(checked.to_string());
        }
        if let Some(selected_id) = prop.get("selectedId").and_then(|s| s.as_str()) {
            if !selected_id.is_empty() {
                return Some(selected_id.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(selected_id.to_string());
            }
            continue;
        }
        if let Some(selected_ids) = prop.get("selectedIds").and_then(|s| s.as_array()) {
            let ids: Vec<String> = selected_ids.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            let joined = ids.join(", ");
            if !joined.is_empty() {
                return Some(joined);
            }
            if first_empty.is_none() {
                first_empty = Some(joined);
            }
            continue;
        }
    }

    first_empty
}

async fn proxy_http_post(url: &str, body: &str) -> Result<String, HttpError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| HttpError::Network)?;

    let response = client
        .post(url)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                HttpError::Timeout
            } else {
                HttpError::Network
            }
        })?;

    let text = response.text().await.map_err(|_| HttpError::Network)?;
    Ok(text)
}

// ============================================================================
// HTTP Error
// ============================================================================

#[derive(Debug)]
pub enum HttpError {
    Timeout,
    Network,
}

// ============================================================================
// Async helper
// ============================================================================

/// 在当前或新创建的 tokio runtime 中阻塞执行 async 代码
fn block_on_http<F>(fut: F) -> Result<String, HttpError>
where
    F: std::future::Future<Output = Result<String, HttpError>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(fut),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().map_err(|_| HttpError::Network)?;
            rt.block_on(fut)
        }
    }
}

// ============================================================================
// URL / Domain helpers
// ============================================================================

fn extract_host(url: &str) -> String {
    url.split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').next())
        .unwrap_or("")
        .to_string()
}

// ============================================================================
// Wasm memory helpers
// ============================================================================

fn read_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, len: usize) -> String {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else {
        return String::new();
    };
    let mut buf = vec![0u8; len];
    memory.read(caller, ptr, &mut buf).unwrap_or(());
    String::from_utf8_lossy(&buf).to_string()
}

fn write_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, value: &str) {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else { return };
    memory.write(caller, ptr, value.as_bytes()).unwrap_or(());
}
