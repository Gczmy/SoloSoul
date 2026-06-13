//! 插件管理器
//!
//! 对外暴露安装、更新、卸载、运行等能力，供 Tauri Commands 调用。

use super::{
    compute_sha256, ConsentManager, FieldResolver, MarketPluginInfo, PluginAuditAction,
    PluginAuditLogger, PluginError, PluginEvent, PluginInstallResult, PluginManifest,
    PluginRegistry, PluginResult, PluginSessionInfo, PluginSessionManager, PluginStore, PluginTier,
    RateLimiter, WasmSandbox,
};
use semver::Version;
use serde::Deserialize;
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;

/// 市场 manifest 原始结构（与 `SoloSoul_plugin_market/plugins/*/manifest.json` 对应）
#[derive(Debug, Deserialize)]
struct MarketManifestRaw {
    plugin_id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    plugin_api_version: Option<String>,
    #[serde(default)]
    data_ttl_seconds: Option<u64>,
    #[serde(default)]
    required_fields: Vec<String>,
    #[serde(default)]
    optional_fields: Vec<String>,
    #[serde(default)]
    network_policy: super::PluginNetworkPolicy,
    #[serde(default)]
    require_user_confirmation: bool,
    #[serde(default)]
    tier: super::PluginTier,
    #[serde(default)]
    category: String,
}

/// 插件管理器
pub struct PluginManager {
    store: PluginStore,
    registry: PluginRegistry,
    market_dir: PathBuf,
    session_manager: PluginSessionManager,
    audit: Arc<PluginAuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    consent_manager: Arc<ConsentManager>,
    field_resolver: Arc<FieldResolver>,
    sandbox: WasmSandbox,
}

impl PluginManager {
    /// 创建插件管理器（开发模式，无 app_handle）
    pub fn new() -> Result<Self, PluginError> {
        let market_dir = super::paths::dev_market_dir();
        Ok(Self {
            store: PluginStore::new()?,
            registry: PluginRegistry::new(),
            market_dir,
            session_manager: PluginSessionManager::new(),
            audit: Arc::new(PluginAuditLogger::new()),
            rate_limiter: Arc::new(RateLimiter::new(60)),
            consent_manager: Arc::new(ConsentManager::new()),
            field_resolver: Arc::new(FieldResolver::new()),
            sandbox: WasmSandbox::new(),
        })
    }

    /// 创建插件管理器（Release 模式，使用 Tauri 资源目录）
    pub fn new_with_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, PluginError> {
        let market_dir = super::paths::resolve_market_dir(Some(app_handle))?;
        Ok(Self {
            store: PluginStore::new()?,
            registry: PluginRegistry::new_with_app_handle(app_handle)?,
            market_dir,
            session_manager: PluginSessionManager::new(),
            audit: Arc::new(PluginAuditLogger::new()),
            rate_limiter: Arc::new(RateLimiter::new(60)),
            consent_manager: Arc::new(ConsentManager::new()),
            field_resolver: Arc::new(FieldResolver::new()),
            sandbox: WasmSandbox::new(),
        })
    }

    /// 列出市场中所有插件，可按 tier 过滤
    pub fn list_all(
        &self,
        tier_filter: Option<PluginTier>,
    ) -> Result<Vec<MarketPluginInfo>, PluginError> {
        let installed = self.store.installed_manifests()?;
        let mut infos = self.registry.load(&installed)?;
        if let Some(tier) = tier_filter {
            infos.retain(|info| info.tier == tier);
        }
        Ok(infos)
    }

    /// 列出已安装插件
    pub fn list_installed(&self) -> Result<Vec<PluginManifest>, PluginError> {
        self.store.installed_manifests()
    }

    /// 从市场注册表安装指定版本插件
    pub fn install_from_registry(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<PluginInstallResult, PluginError> {
        let entry = self.registry.get_entry(plugin_id)?;
        let version_info = entry
            .versions
            .get(version)
            .ok_or_else(|| PluginError::NotFound(format!("版本 {} 不存在", version)))?;

        if !is_version_compatible(version_info, &current_app_version()?) {
            return Err(PluginError::IncompatibleVersion(version.to_string()));
        }

        let manifest_path = self.market_dir.join(plugin_id).join("manifest.json");
        let wasm_path = self.market_dir.join(plugin_id).join("plugin.wasm");

        let manifest_raw: MarketManifestRaw =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let wasm_bytes = std::fs::read(&wasm_path)?;

        let actual_hash = compute_sha256(&wasm_bytes);
        if actual_hash != version_info.sha256 {
            return Err(PluginError::ChecksumMismatch);
        }

        let mut permissions = manifest_raw.required_fields;
        permissions.extend(manifest_raw.optional_fields);

        let manifest = PluginManifest {
            id: manifest_raw.plugin_id,
            name: manifest_raw.name,
            version: manifest_raw.version,
            description: manifest_raw.description,
            author: manifest_raw.publisher,
            homepage: manifest_raw.homepage,
            permissions,
            required_core_version: manifest_raw.plugin_api_version,
            wasm_hash_sha256: Some(version_info.sha256.clone()),
            data_ttl_seconds: manifest_raw.data_ttl_seconds.unwrap_or(300),
            network_policy: manifest_raw.network_policy,
            require_user_confirmation: manifest_raw.require_user_confirmation,
            tier: manifest_raw.tier,
            category: manifest_raw.category,
        };

        self.store.save_plugin(&manifest, &wasm_bytes)?;
        self.audit.log(
            plugin_id,
            None::<String>,
            PluginAuditAction::PluginInstalled {
                version: version.to_string(),
            },
        );

        Ok(PluginInstallResult {
            plugin_id: plugin_id.to_string(),
            version: version.to_string(),
            installed_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    /// 更新插件到注册表最新版本
    pub fn update(&self, plugin_id: &str) -> Result<PluginInstallResult, PluginError> {
        let entry = self.registry.get_entry(plugin_id)?;
        let latest = entry
            .latest_version
            .ok_or_else(|| PluginError::RegistryError("缺少最新版本信息".to_string()))?;
        self.install_from_registry(plugin_id, &latest)
    }

    /// 卸载插件
    pub fn uninstall(&self, plugin_id: &str) -> Result<(), PluginError> {
        self.store.delete_plugin(plugin_id)?;
        self.audit.log(
            plugin_id,
            None::<String>,
            PluginAuditAction::PluginUninstalled,
        );
        Ok(())
    }

    /// 运行插件
    pub async fn run(
        &self,
        plugin_id: &str,
        params: HashMap<String, String>,
        channel: Channel<PluginEvent>,
        vault_store: Option<Arc<VaultStore>>,
        account_id: Option<String>,
    ) -> Result<PluginResult, PluginError> {
        let wasm_bytes = self.store.load_wasm(plugin_id)?;
        let manifest = self.store.load_manifest(plugin_id)?;
        let session = self
            .session_manager
            .create(plugin_id, manifest.data_ttl_seconds);

        self.audit.log(
            plugin_id,
            Some(&session.id),
            PluginAuditAction::PluginRunStarted,
        );

        let field_resolver = match (vault_store, account_id) {
            (Some(vault), Some(account)) => Arc::new(super::FieldResolver::with_vault(
                vault,
                account,
                manifest.permissions.clone(),
            )),
            _ => self.field_resolver.clone(),
        };

        let session_id = session.id.clone();
        let host = super::SoloHostFunctions::new(
            plugin_id,
            &manifest.name,
            &session_id,
            manifest.clone(),
            params,
            self.audit.clone(),
            self.rate_limiter.clone(),
            self.consent_manager.clone(),
            field_resolver,
            channel.clone(),
        );

        let _ = channel.send(PluginEvent::log(
            "info",
            format!("开始运行插件: {}", manifest.name),
        ));

        let sandbox = self.sandbox;
        let consent = self.consent_manager.clone();
        let session_for_spawn = session.clone();
        let result = tokio::task::spawn_blocking(move || {
            let module = sandbox.compile(&wasm_bytes)?;
            sandbox.execute(&module, host, &session_for_spawn, &consent)
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("任务 Join 失败: {}", e)))?;

        match result {
            Ok(r) => {
                self.audit.log(
                    plugin_id,
                    Some(&session.id),
                    PluginAuditAction::PluginRunCompleted {
                        exit_code: r.exit_code,
                    },
                );
                Ok(r)
            }
            Err(e) => {
                let _ = channel.send(PluginEvent::error(plugin_id, e.to_string()));
                self.audit.log(
                    plugin_id,
                    Some(&session.id),
                    PluginAuditAction::PluginRunFailed {
                        reason: e.to_string(),
                    },
                );
                Err(e)
            }
        }
    }

    /// 响应授权请求
    pub async fn consent_response(
        &self,
        request_id: &str,
        approved: bool,
        value: Option<String>,
    ) -> Result<(), PluginError> {
        let response_value = if approved { value } else { None };
        self.consent_manager
            .respond(request_id, response_value)
            .await
            .map_err(|_| PluginError::ConsentDenied)?;
        Ok(())
    }

    /// 列出活跃会话
    pub fn list_sessions(&self) -> Result<Vec<PluginSessionInfo>, PluginError> {
        Ok(self
            .session_manager
            .list_active()
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// 获取审计日志
    pub fn audit_log(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<super::PluginAuditEntry>, PluginError> {
        Ok(self.audit.recent(limit.unwrap_or(50)))
    }

    /// 刷新注册表（当前实现无缓存，直接返回成功）
    pub fn update_registry(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// 当前应用版本
fn current_app_version() -> Result<Version, PluginError> {
    let s = env!("CARGO_PKG_VERSION");
    Version::parse(s).map_err(|e| PluginError::RegistryError(format!("版本解析失败: {}", e)))
}

/// 解析注册表版本兼容性
fn is_version_compatible(version: &super::RegistryVersion, app_version: &Version) -> bool {
    let min = match Version::parse(&version.min_app_version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let max = match Version::parse(&version.max_app_version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    app_version >= &min && app_version <= &max
}
