//! Plugin Manager — 插件生命周期管理的统一入口
//!
//! 负责协调 PluginStore、WasmSandbox、PluginSessionManager 和 Consent 通道。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use std::sync::Mutex as StdMutex;

/// 写日志到 /tmp/solosoul_rust.log，用全局 Mutex 防止多线程竞争导致格式混乱
static LOG_FILE_MUTEX: StdMutex<()> = StdMutex::new(());

pub fn rust_log(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let _guard = LOG_FILE_MUTEX.lock();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/tmp/solosoul_rust.log") {
        let _ = writeln!(file, "[{}] {}", now, msg);
    }
}


#[cfg(feature = "sandbox")]
use crate::frb_generated::StreamSink;

use std::collections::HashSet;
use std::time::Duration;

use super::host::{AuditAction, AuditEntry, ConsentChannel, ConsentRequest, ConsentResult, RateLimiter, resolve_field_sensitivity};
use super::manifest::PluginManifest;
use super::sandbox::WasmSandbox;
use super::session::{PluginSessionManager, SessionInfo};
use super::store::PluginStore;

/// 插件事件流（Rust -> Dart）
#[derive(Debug, Clone)]
pub enum PluginEvent {
    ConsentRequest {
        request_id: String,
        plugin_id: String,
        plugin_name: String,
        field: String,
        sensitivity: String,
    },
    ConsentTimeout {
        request_id: String,
    },
    Log {
        level: String,
        message: String,
    },
    /// 结构化结果（Phase 2: solosoul_result 通道）
    Result {
        json_data: String,
    },
    Progress {
        percent: u8,
    },
    Completed {
        exit_code: i32,
    },
    Error {
        message: String,
    },
}

/// 插件管理器（线程安全）
pub struct PluginManager {
    store: PluginStore,
    sandbox: WasmSandbox,
    session_manager: PluginSessionManager,
    rate_limiter: Arc<RateLimiter>,
    /// request_id -> std::sync::mpsc::Sender<ConsentResult>
    pending_consents: Arc<Mutex<HashMap<String, std::sync::mpsc::Sender<ConsentResult>>>>,
}

impl PluginManager {
    pub fn new() -> Result<Self, String> {
        let store = PluginStore::new()?;
        let sandbox = WasmSandbox::new().map_err(|e| format!("{:?}", e))?;
        let session_manager = PluginSessionManager::new();
        let rate_limiter = Arc::new(RateLimiter::new(if cfg!(debug_assertions) {
            100
        } else {
            10
        }));
        Ok(Self {
            store,
            sandbox,
            session_manager,
            rate_limiter,
            pending_consents: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 从文件路径安装插件
    pub fn install_plugin(
        &self,
        wasm_path: String,
        manifest_path: String,
    ) -> Result<String, String> {
        let wasm_bytes = std::fs::read(&wasm_path)
            .map_err(|e| format!("Failed to read wasm: {}", e))?;
        let manifest_json = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;
        manifest.validate()?;

        self.store
            .save_plugin(&manifest.plugin_id, &wasm_bytes, &manifest)?;
        Ok(manifest.plugin_id)
    }

    /// 加载插件清单
    pub fn load_manifest(&self, plugin_id: &str) -> Result<PluginManifest, String> {
        self.store.load_manifest(plugin_id)
    }

    /// 获取插件基础目录
    pub fn get_base_dir(&self) -> String {
        self.store.base_dir().to_string_lossy().to_string()
    }

    /// 列出已安装插件
    pub fn list_installed(&self) -> Result<Vec<String>, String> {
        self.store.list_installed()
    }

    /// 列出所有活跃 Session
    pub fn list_active_sessions(&self) -> Vec<SessionInfo> {
        self.session_manager.list_active()
    }

    /// 强制卸载插件（撤销 Session + 删除目录）
    pub fn force_unload(&self, plugin_id: &str) -> Result<(), String> {
        self.session_manager.revoke(plugin_id);
        self.store.remove_plugin(plugin_id)?;
        Ok(())
    }

    /// 执行插件（核心方法，返回 exit code）
    ///
    /// 执行流程：
    /// 1. 加载 manifest 和 wasm
    /// 2. 编译 wasm 模块
    /// 3. 生成 session_id 并注册 Session
    /// 4. 创建 Consent / Audit 通道
    /// 5. 启动 Consent 后台处理线程（将请求存入 pending_consents）
    /// 6. 调用 WasmSandbox.execute() 运行插件
    /// 7. 插件返回后清理 Session（若执行成功则保留至 TTL 过期）
    pub fn execute_plugin(
        &self,
        plugin_id: String,
        session_ttl_seconds: u64,
        sink: StreamSink<PluginEvent>,
    ) -> Result<i32, String> {
        // 1. 加载 manifest 和 wasm
        let manifest = self.store.load_manifest(&plugin_id)?;
        let wasm_bytes = self.store.load_wasm(&plugin_id)?;

        // 2. 编译模块
        let module = self
            .sandbox
            .compile_module(&wasm_bytes)
            .map_err(|e| format!("{:?}", e))?;

        // 3. 生成 Session ID 并预先注册（执行期间 Host Functions 可查询 Session 状态）
        let session_id = uuid::Uuid::new_v4().to_string();
        let plugin_name = manifest.name.clone();
        self.session_manager
            .register(&plugin_id, &plugin_name, &session_id, session_ttl_seconds);

        // 4. 创建 Consent、Audit、Log 和 Result 通道
        let (consent_tx, mut consent_rx) = tokio::sync::mpsc::channel::<ConsentRequest>(16);
        let (audit_tx, mut audit_rx) = tokio::sync::mpsc::channel::<AuditEntry>(64);
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<(String, String)>(256);
        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<String>(64);

        let pending_consents = Arc::clone(&self.pending_consents);
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let manifest_clone = manifest.clone();

        // 5. 启动 Consent 后台处理线程
        //    消费 consent_rx，将每个请求的 request_id + oneshot sender 存入 pending_consents
        //    同时通过 StreamSink 推送 ConsentRequest 事件到 Dart 端
        //    当 consent_rx 被 drop（execute_plugin 返回）时，线程自然退出
        let sink_consent = sink.clone();
        std::thread::spawn(move || {
            while let Some(request) = consent_rx.blocking_recv() {
                let request_id = request.request_id.clone();
                let sender = request.response;
                if let Ok(mut guard) = pending_consents.lock() {
                    guard.insert(request_id.clone(), sender);
                }
                // 通过 StreamSink 推送 ConsentRequest 事件到 Dart 端
                let _ = sink_consent.add(PluginEvent::ConsentRequest {
                    request_id,
                    plugin_id: request.plugin_id,
                    plugin_name: request.plugin_name,
                    field: request.field,
                    sensitivity: format!("{:?}", request.sensitivity),
                });
            }
        });

        // 5b. 启动 Audit 后台处理线程
        //     消费 audit_rx，将审计日志追加写入 ~/.solosoul/audit/plugin_audit.log
        std::thread::spawn(move || {
            use std::fs::{create_dir_all, OpenOptions};
            use std::io::Write;
            use std::path::PathBuf;

            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| "/tmp".to_string());
            let audit_dir = PathBuf::from(home).join(".solosoul").join("audit");
            let audit_file = audit_dir.join("plugin_audit.log");

            let _ = create_dir_all(&audit_dir);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&audit_dir, std::fs::Permissions::from_mode(0o700));
            }

            while let Some(entry) = audit_rx.blocking_recv() {
                let line = format_audit_entry(&entry);
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&audit_file)
                {
                    let _ = writeln!(file, "{}", line);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&audit_file, std::fs::Permissions::from_mode(0o600));
                    }
                }
            }
        });

        // 5c. 启动 Log 后台处理线程
        //     消费 log_rx，将插件日志通过 StreamSink 推送到 Dart 端
        let sink_log = sink.clone();
        let log_thread = std::thread::spawn(move || {
            while let Some((level, message)) = log_rx.blocking_recv() {
                let _ = sink_log.add(PluginEvent::Log {
                    level,
                    message,
                });
            }
        });

        // 5d. 启动 Result 后台处理线程（Phase 2: 结构化结果通道）
        //     消费 result_rx，将结构化结果通过 StreamSink 推送到 Dart 端
        let sink_result = sink.clone();
        let result_thread = std::thread::spawn(move || {
            while let Some(json_data) = result_rx.blocking_recv() {
                let _ = sink_result.add(PluginEvent::Result { json_data });
            }
        });

        // 6. 批量授权：收集 manifest 中所有声明字段，一次性请求用户确认
        //    显示所有字段的敏感度等级，用户有权拒绝任何字段
        let mut pre_approved_fields: HashSet<String> = HashSet::new();
        let all_fields: Vec<String> = manifest_clone.required_fields.iter()
            .chain(manifest_clone.optional_fields.iter())
            .cloned()
            .collect();
        
        if !all_fields.is_empty() {
            let mut pending_rxs: Vec<(String, std::sync::mpsc::Receiver<ConsentResult>)> = Vec::new();
            
            // 为每个声明字段发送 ConsentRequest（包括 Public/Internal/Sensitive/Critical）
            for field in &all_fields {
                let request_id = uuid::Uuid::new_v4().to_string();
                let (tx, rx) = std::sync::mpsc::channel();
                
                // 将 sender 存入 pending_consents（供 Dart 响应时查找）
                if let Ok(mut guard) = self.pending_consents.lock() {
                    guard.insert(request_id.clone(), tx);
                }
                
                // 通过 StreamSink 推送 ConsentRequest 事件到 Dart 端
                let _ = sink.add(PluginEvent::ConsentRequest {
                    request_id: request_id.clone(),
                    plugin_id: plugin_id.clone(),
                    plugin_name: plugin_name.clone(),
                    field: field.clone(),
                    sensitivity: format!("{:?}", resolve_field_sensitivity(field)),
                });
                
                pending_rxs.push((field.clone(), rx));
            }
            
            // 发送批量结束标志，通知 Dart 显示批量对话框
            let _ = sink.add(PluginEvent::Log {
                level: "batch_end".to_string(),
                message: format!("pre-consent|{}", plugin_id),
            });
            
            // 等待所有用户响应（超时 60s）
            let mut any_denied = false;
            for (field, rx) in pending_rxs {
                match rx.recv_timeout(Duration::from_secs(60)) {
                    Ok(ConsentResult::Approved(_)) => {
                        pre_approved_fields.insert(field);
                    }
                    Ok(ConsentResult::Denied) => {
                        any_denied = true;
                        break;
                    }
                    Ok(ConsentResult::Expired) | Err(_) => {
                        any_denied = true;
                        break;
                    }
                }
            }
            
            if any_denied {
                // 用户拒绝或超时，立即撤销 Session 并通过 Stream 通知 Dart
                // ⚠️ 不可返回 Err，否则 FRB 的 unhandled Future 会抛出 Unhandled Exception
                self.session_manager.revoke(&plugin_id);
                let _ = sink.add(PluginEvent::Error {
                    message: "User denied or timed out field access".to_string(),
                });
                return Ok(0);
            }
        }

        // 7. 执行插件（在独立线程中运行，避免阻塞 Dart UI）
        let sink_execute = sink.clone();

        let result = self.sandbox.execute(
            &module,
            &plugin_id,
            &plugin_name,
            &session_id,
            &manifest_clone,
            &ConsentChannel { tx: consent_tx },
            audit_tx,
            log_tx,
            result_tx,
            rate_limiter,
            session_ttl_seconds,
            pre_approved_fields,
        );

        // 7. 根据执行结果处理 Session，并通过 StreamSink 推送 Completed/Error 事件
        //    ⚠️ 必须先等待 log_thread 和 result_thread 结束，确保所有插件事件已推送到 Dart，
        //       然后再发送 Completed/Error，否则 StreamSink 关闭后事件会丢失，
        //       或 Completed 先于 Result/Log 到达 Dart 端导致对话框提前弹出。
        match result {
            Ok(r) => {
                // 先等待所有后台线程结束，确保 Result/Log 事件已全部进入 StreamSink
                let _ = log_thread.join();
                let _ = result_thread.join();
                // 然后发送 Completed，保证 Dart 端最后收到 Completed
                let _ = sink_execute.add(PluginEvent::Completed {
                    exit_code: r.exit_code,
                });
                // 执行完成后清理 Session（成功或失败均清理）
                self.session_manager.revoke(&plugin_id);
                Ok(r.exit_code)
            }
            Err(e) => {
                // 先等待所有后台线程结束，确保 Error/Log 事件已全部进入 StreamSink
                let _ = log_thread.join();
                let _ = result_thread.join();
                // 执行失败时撤销 Session 并通知 Dart
                self.session_manager.revoke(&plugin_id);
                let _ = sink_execute.add(PluginEvent::Error {
                    message: format!("{:?}", e),
                });
                Err(format!("{:?}", e))
            }
        }
    }

    /// 响应用户授权（Dart -> Rust）
    pub fn consent_response(
        &self,
        request_id: String,
        approved: bool,
        value: Option<String>,
    ) -> Result<(), String> {
        let sender = {
            let mut guard = self.pending_consents.lock().unwrap();
            guard.remove(&request_id)
        };

        if let Some(sender) = sender {
            let result = if approved {
                ConsentResult::Approved(value.unwrap_or_default())
            } else {
                ConsentResult::Denied
            };
            // 忽略发送错误：receiver 可能已超时关闭
            let _ = sender.send(result);
            Ok(())
        } else {
            Err("Plugin consent request not found or expired".to_string())
        }
    }
}

// ============================================================================
// 全局单例
// ============================================================================

lazy_static::lazy_static! {
    static ref PLUGIN_MANAGER: Mutex<Option<Arc<PluginManager>>> = Mutex::new(None);
}

fn get_plugin_manager() -> Result<std::sync::MutexGuard<'static, Option<Arc<PluginManager>>>, String> {
    PLUGIN_MANAGER
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))
}

fn init_plugin_manager() -> Result<(), String> {
    let mut guard = get_plugin_manager()?;
    if guard.is_none() {
        *guard = Some(Arc::new(PluginManager::new()?));
    }
    Ok(())
}

pub fn with_manager<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&PluginManager) -> Result<R, String>,
{
    init_plugin_manager()?;
    let manager = {
        let guard = get_plugin_manager()?;
        guard.as_ref().ok_or("PluginManager not initialized")?.clone()
    };
    let result = f(&manager);
    result
}

// ============================================================================
// Audit log formatter
// ============================================================================

/// 将审计条目格式化为 JSON Lines 格式
fn format_audit_entry(entry: &AuditEntry) -> String {
    let action_json = match &entry.action {
        AuditAction::FieldAccessGranted {
            field,
            confirmed_by_user,
        } => {
            format!(
                r#"{{"type":"FieldAccessGranted","field":"{}","confirmed_by_user":{}}}"#,
                field, confirmed_by_user
            )
        }
        AuditAction::FieldAccessDenied { field } => {
            format!(r#"{{"type":"FieldAccessDenied","field":"{}"}}"#, field)
        }
        AuditAction::NetworkBlocked { url } => {
            format!(r#"{{"type":"NetworkBlocked","url":"{}"}}"#, url)
        }
        AuditAction::NetworkAllowed { url } => {
            format!(r#"{{"type":"NetworkAllowed","url":"{}"}}"#, url)
        }
        AuditAction::RateLimitTriggered { field } => {
            format!(r#"{{"type":"RateLimitTriggered","field":"{}"}}"#, field)
        }
        AuditAction::PluginCrashed { reason } => {
            format!(r#"{{"type":"PluginCrashed","reason":"{}"}}"#, reason)
        }
        AuditAction::SessionCreated => r#"{"type":"SessionCreated"}"#.to_string(),
        AuditAction::SessionRevoked => r#"{"type":"SessionRevoked"}"#.to_string(),
    };

    let timestamp = chrono::Utc::now().to_rfc3339();
    format!(
        r#"{{"timestamp":"{}","plugin_id":"{}","session_id":"{}","action":{}}}"#,
        timestamp, entry.plugin_id, entry.session_id, action_json
    )
}

