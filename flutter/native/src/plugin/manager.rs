//! Plugin Manager — 插件生命周期管理的统一入口
//!
//! 负责协调 PluginStore、WasmSandbox、PluginSessionManager 和 Consent 通道。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

// StreamSink 用于 FRB Stream，待代码生成时启用
// use flutter_rust_bridge::StreamSink;

use super::host::{AuditEntry, ConsentChannel, ConsentRequest, ConsentResult, RateLimiter};
use super::manifest::PluginManifest;
use super::sandbox::{PluginError, WasmSandbox};
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
    /// request_id -> oneshot::Sender<ConsentResult>
    pending_consents: Arc<Mutex<HashMap<String, oneshot::Sender<ConsentResult>>>>,
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

    /// 执行插件（核心方法，返回事件流）
    pub fn execute_plugin(
        &self,
        plugin_id: String,
        session_ttl_seconds: u64,
        // sink: StreamSink<PluginEvent>, // TODO: 启用 FRB Stream 后恢复
    ) -> Result<i32, String> {
        // 1. 加载 manifest 和 wasm
        let manifest = self.store.load_manifest(&plugin_id)?;
        let wasm_bytes = self.store.load_wasm(&plugin_id)?;

        // 2. 编译模块
        let module = self
            .sandbox
            .compile_module(&wasm_bytes)
            .map_err(|e| format!("{:?}", e))?;

        // 3. 创建通道
        let (consent_tx, mut consent_rx) = tokio::sync::mpsc::channel::<ConsentRequest>(16);
        let (audit_tx, mut audit_rx) = tokio::sync::mpsc::channel::<AuditEntry>(64);

        let pending_consents = Arc::clone(&self.pending_consents);
        let session_manager = self.session_manager.clone();
        let sandbox = WasmSandbox::new().map_err(|e| format!("{:?}", e))?;
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let plugin_name = manifest.name.clone();
        let manifest_clone = manifest.clone();
        let plugin_id_clone = plugin_id.clone();

        // 4. 创建 tokio runtime 处理异步事件
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        // 5. 启动 Consent 处理任务
        // TODO: 启用 FRB Stream 后恢复 Consent 处理任务和 Audit 处理任务
        // 当前简化实现：直接执行，不通过 Stream 发送事件
        let result = sandbox.execute(
            &module,
            &plugin_id_clone,
            &plugin_name,
            &manifest_clone,
            &ConsentChannel { tx: consent_tx },
            audit_tx,
            rate_limiter,
            session_ttl_seconds,
        );

        // 8. 注册 Session
        let session_id = uuid::Uuid::new_v4().to_string();
        session_manager.register(&plugin_id, &plugin_name, &session_id, session_ttl_seconds);

        match result {
            Ok(r) => Ok(r.exit_code),
            Err(e) => Err(format!("{:?}", e)),
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
            sender
                .send(result)
                .map_err(|_| "Consent receiver dropped".to_string())?;
            Ok(())
        } else {
            Err("Consent request not found or expired".to_string())
        }
    }
}

// ============================================================================
// 全局单例
// ============================================================================

lazy_static::lazy_static! {
    static ref PLUGIN_MANAGER: Mutex<Option<PluginManager>> = Mutex::new(None);
}

fn get_plugin_manager() -> Result<std::sync::MutexGuard<'static, Option<PluginManager>>, String> {
    PLUGIN_MANAGER
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))
}

fn init_plugin_manager() -> Result<(), String> {
    let mut guard = get_plugin_manager()?;
    if guard.is_none() {
        *guard = Some(PluginManager::new()?);
    }
    Ok(())
}

pub fn with_manager<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&PluginManager) -> Result<R, String>,
{
    init_plugin_manager()?;
    let guard = get_plugin_manager()?;
    let manager = guard.as_ref().ok_or("PluginManager not initialized")?;
    f(manager)
}
