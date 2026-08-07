//! 插件管理器
//!
//! 对外暴露安装、更新、卸载、运行等能力，供 Tauri Commands 调用。

use super::{
    compute_sha256, ConsentManager, FieldResolver, MarketPluginInfo, PluginAuditAction,
    PluginAuditLogger, PluginError, PluginEvent, PluginInstallResult, PluginManifest,
    PluginRegistry, PluginResult, PluginSession, PluginSessionManager, PluginStore, PluginTier,
    RateLimiter, WasmSandbox,
};
use crate::event::PluginEventSink;
use serde::Deserialize;
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 插件 ID 允许字符集，防止通过 ID 构造路径遍历。
/// 将插件 ID 转换为可安全用于文件路径的名称。
fn sanitize_plugin_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 根据 locale 生成插件启动日志文本。
fn plugin_start_message(locale: &str, plugin_name: &str) -> String {
    let is_en = locale == "en-US" || locale.starts_with("en");
    if is_en {
        format!("Starting plugin: {}", plugin_name)
    } else {
        format!("启动插件: {}", plugin_name)
    }
}

fn validate_plugin_id(id: &str) -> Result<(), PluginError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(PluginError::StoreError(format!(
            "Invalid plugin id: {}",
            id
        )));
    }
    Ok(())
}

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
    #[serde(default)]
    params: Vec<super::manifest::PluginParam>,
    /// Stage 4 typed-lookup 契约绑定（市场 manifest 可选字段）
    #[serde(default)]
    contracts: Vec<super::manifest::PluginContractBinding>,
    /// Stage 4 typed-lookup 字段绑定
    #[serde(default)]
    field_bindings: Vec<super::manifest::PluginFieldBinding>,
    #[serde(default)]
    pub custom_ui: Option<String>,
    #[serde(default)]
    pub i18n: Option<HashMap<String, HashMap<String, String>>>,
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
        let market_dir = super::paths::default_market_dir();
        Self::new_with_dirs(market_dir, PluginStore::data_dir()?)
    }

    /// 创建插件管理器（Release 模式，使用 Tauri 资源目录）
    pub fn new_with_resource_dir(resource_dir: &std::path::PathBuf) -> Result<Self, PluginError> {
        let market_dir = super::paths::resolve_market_dir(Some(resource_dir))?;
        Self::new_with_dirs(market_dir, PluginStore::data_dir()?)
    }

    /// 显式注入市场目录与数据目录（由调用方负责解析，crate 不反向依赖 tauri）
    pub fn new_with_dirs(market_dir: PathBuf, data_dir: PathBuf) -> Result<Self, PluginError> {
        let audit_path = data_dir.join("plugin_audit.jsonl");
        Ok(Self {
            store: PluginStore::new_with_data_dir(data_dir.clone())?,
            registry: PluginRegistry::new_with_dirs(market_dir.clone(), data_dir),
            market_dir,
            session_manager: PluginSessionManager::new(),
            audit: Arc::new(PluginAuditLogger::new(Some(audit_path))),
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
    ///
    /// 分离原则：已安装插件的 manifest/WASM 只从应用数据目录（`PluginStore`）读写；
    /// `market_dir`（bundled 资源目录）仅在远程不可达时作为离线回退。
    ///
    /// 流程：
    /// 1. 读取注册表获取目标版本元数据
    /// 2. 检查应用数据目录中是否已安装该版本且 SHA-256 匹配 → 直接返回
    /// 3. 未安装或 hash 不匹配 → 优先从远程下载 manifest.json + plugin.wasm
    /// 4. 远程失败 → 回退到 bundled `market_dir`
    /// 5. 校验通过后保存到 PluginStore
    ///
    /// 应用数据目录中是否已安装目标版本且 WASM hash 匹配。
    /// 返回 true 表示可直接复用（免下载、免校验）。
    fn is_installed_ok(&self, plugin_id: &str, version: &str, expected_hash: &str) -> bool {
        let installed = match self.store.load_manifest(plugin_id) {
            Ok(m) => m,
            Err(_) => return false,
        };
        if installed.version != version {
            tracing::info!(
                "Installed plugin {} version {} does not match target {}, will re-install",
                plugin_id,
                installed.version,
                version
            );
            return false;
        }
        let wasm = match self.store.load_wasm(plugin_id) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let actual_hash = compute_sha256(&wasm);
        if actual_hash != expected_hash {
            tracing::warn!(
                "Installed plugin {} hash mismatch (expected {}, got {}), will re-download",
                plugin_id,
                expected_hash,
                actual_hash
            );
            return false;
        }
        true
    }

    /// 从 `MarketManifestRaw` 构造 `PluginManifest`（远程与 bundled 两路径共用；
    /// 为旧插件推导 effective roles）。
    fn manifest_from_raw(
        raw: MarketManifestRaw,
        version: String,
        wasm_hash_sha256: Option<String>,
    ) -> PluginManifest {
        let mut permissions = raw.required_fields.clone();
        permissions.extend(raw.optional_fields.clone());
        let contracts: Vec<_> = raw
            .contracts
            .iter()
            .map(|c| {
                let mut c2 = c.clone();
                c2.roles = c.effective_roles(&raw.field_bindings);
                c2
            })
            .collect();
        PluginManifest {
            id: raw.plugin_id,
            name: raw.name,
            version,
            description: raw.description,
            author: raw.publisher,
            homepage: raw.homepage,
            permissions,
            required_core_version: raw.plugin_api_version,
            wasm_hash_sha256,
            data_ttl_seconds: raw.data_ttl_seconds.unwrap_or(300),
            network_policy: raw.network_policy,
            require_user_confirmation: raw.require_user_confirmation,
            tier: raw.tier,
            category: raw.category,
            params: raw.params,
            contracts,
            field_bindings: raw.field_bindings,
            i18n: raw.i18n.clone(),
            custom_ui: raw.custom_ui,
        }
    }

    /// bundled 资源回退安装：读取本地 manifest/wasm，hash 校验后安装。
    ///
    /// 处理 bundled 版本与目标版本不一致的情况：尝试按 bundled 版本安装
    /// （从注册表查该版本 hash 并校验 WASM），保证离线可用。
    fn install_bundled_fallback(
        &self,
        plugin_id: &str,
        target_version: &str,
        entry: &crate::RegistryEntry,
    ) -> Result<Option<PluginInstallResult>, PluginError> {
        let bundled_dir = self.market_dir.join("plugins").join(plugin_id);
        let bundled_manifest = bundled_dir.join("manifest.json");
        let text = std::fs::read_to_string(&bundled_manifest).map_err(|_| {
            PluginError::NetworkError(format!(
                "无法下载 manifest 且 bundled 资源不存在: {}",
                target_version
            ))
        })?;
        let local: MarketManifestRaw = serde_json::from_str(&text).map_err(|e| {
            PluginError::InvalidManifest(format!("bundled manifest 解析失败: {}", e))
        })?;

        if local.version == target_version {
            return Ok(None); // 调用方继续按目标版本路径处理
        }

        // Bundled version mismatches the target — try installing the bundled version
        // by looking it up in the registry. This handles the common case where
        // the registry has been updated but the bundled WASM is still the old version.
        tracing::warn!(
            "Bundled version {} != target {}, attempting fallback install of bundled version",
            local.version,
            target_version
        );
        let bundled_version = local.version.clone();
        let bundled_version_info = entry.versions.get(&bundled_version).ok_or_else(|| {
            PluginError::NetworkError(format!(
                "远程 manifest 下载失败且 bundled 版本 {} 在注册表中不存在，无法降级安装",
                bundled_version
            ))
        })?;
        let bundled_wasm = bundled_dir.join("plugin.wasm");
        let wasm_bytes = std::fs::read(&bundled_wasm).map_err(|_| {
            PluginError::NetworkError("无法下载 WASM 且 bundled 资源不存在".to_string())
        })?;
        let actual_hash = compute_sha256(&wasm_bytes);
        if actual_hash != bundled_version_info.sha256 {
            return Err(PluginError::ChecksumMismatch);
        }
        let manifest = Self::manifest_from_raw(
            local,
            bundled_version.clone(),
            Some(bundled_version_info.sha256.clone()),
        );
        self.store.save_plugin(&manifest, &wasm_bytes)?;
        self.audit.log(
            plugin_id,
            None::<String>,
            PluginAuditAction::PluginInstalled {
                version: bundled_version.clone(),
            },
        );
        Ok(Some(PluginInstallResult {
            plugin_id: plugin_id.to_string(),
            version: bundled_version,
            installed_at: chrono::Utc::now().timestamp_millis(),
        }))
    }

    pub async fn install_from_registry(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<PluginInstallResult, PluginError> {
        let entry = self.registry.get_entry(plugin_id)?;
        let version_info = entry
            .versions
            .get(version)
            .ok_or_else(|| PluginError::NotFound(format!("版本 {} 不存在", version)))?;

        if !crate::version::is_version_compatible(
            version_info,
            &crate::version::current_app_version()?,
        ) {
            return Err(PluginError::IncompatibleVersion(version.to_string()));
        }

        validate_plugin_id(plugin_id)?;

        // ── 2. 检查应用数据目录中是否已安装且版本/hash 完全匹配 ──
        if self.is_installed_ok(plugin_id, version, &version_info.sha256) {
            tracing::info!(
                "Plugin {} {} already installed and hash matches",
                plugin_id,
                version
            );
            return Ok(PluginInstallResult {
                plugin_id: plugin_id.to_string(),
                version: version.to_string(),
                installed_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        // ── 3. 优先从远程下载 manifest，失败回退 bundled ──
        let manifest_raw = match self.fetch_manifest(version_info).await {
            Ok(m) => m,
            Err(remote_err) => {
                tracing::warn!(
                    "Remote manifest download failed: {}, falling back to bundled market_dir",
                    remote_err
                );
                if let Some(result) = self.install_bundled_fallback(plugin_id, version, &entry)? {
                    return Ok(result);
                }
                // bundled 版本与目标一致：读回 bundled manifest 继续按目标版本安装
                let bundled_dir = self.market_dir.join("plugins").join(plugin_id);
                let bundled_manifest = bundled_dir.join("manifest.json");
                let text = std::fs::read_to_string(&bundled_manifest).map_err(|_| {
                    PluginError::NetworkError(format!(
                        "无法下载 manifest 且 bundled 资源不存在: {}",
                        remote_err
                    ))
                })?;
                serde_json::from_str(&text).map_err(|e| {
                    PluginError::InvalidManifest(format!("bundled manifest 解析失败: {}", e))
                })?
            }
        };

        // ── 4. 优先从远程下载 WASM，失败回退 bundled ──
        let wasm_bytes = match self.fetch_wasm(version_info).await {
            Ok(bytes) => bytes,
            Err(remote_err) => {
                tracing::warn!(
                    "Remote WASM download failed: {}, falling back to bundled market_dir",
                    remote_err
                );
                let bundled_dir = self.market_dir.join("plugins").join(plugin_id);
                let bundled_wasm = bundled_dir.join("plugin.wasm");
                std::fs::read(&bundled_wasm).map_err(|_| {
                    PluginError::NetworkError(format!(
                        "无法下载 WASM 且 bundled 资源不存在: {}",
                        remote_err
                    ))
                })?
            }
        };

        // ── 5. 校验 ──
        let actual_hash = compute_sha256(&wasm_bytes);
        if actual_hash != version_info.sha256 {
            return Err(PluginError::ChecksumMismatch);
        }

        let manifest = Self::manifest_from_raw(
            manifest_raw,
            version.to_string(),
            Some(version_info.sha256.clone()),
        );
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
    pub async fn update(&self, plugin_id: &str) -> Result<PluginInstallResult, PluginError> {
        let entry = self.registry.get_entry(plugin_id)?;
        let latest = entry
            .latest_version
            .ok_or_else(|| PluginError::RegistryError("缺少最新版本信息".to_string()))?;
        self.install_from_registry(plugin_id, &latest).await
    }

    /// 从远程 URL 下载插件 manifest
    async fn fetch_manifest(
        &self,
        version_info: &crate::RegistryVersion,
    ) -> Result<MarketManifestRaw, PluginError> {
        let url = version_info
            .raw_url
            .as_ref()
            .or(version_info.download_url.as_ref())
            .ok_or_else(|| {
                PluginError::NetworkError("注册表中缺少 download_url / raw_url".to_string())
            })?;

        let manifest_url = if url.ends_with("plugin.wasm") {
            let base = &url[..url.len() - "plugin.wasm".len()];
            format!("{}manifest.json", base)
        } else {
            return Err(PluginError::NetworkError(
                "无法从 download_url 推导 manifest URL".to_string(),
            ));
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| PluginError::NetworkError(format!("HTTP 客户端创建失败: {}", e)))?;

        let text = client
            .get(&manifest_url)
            .send()
            .await
            .map_err(|e| PluginError::NetworkError(format!("下载 manifest 失败: {}", e)))?
            .text()
            .await
            .map_err(|e| PluginError::NetworkError(format!("读取 manifest 响应失败: {}", e)))?;

        let manifest: MarketManifestRaw = serde_json::from_str(&text)
            .map_err(|e| PluginError::InvalidManifest(format!("manifest JSON 解析失败: {}", e)))?;

        Ok(manifest)
    }

    /// 从远程 URL 下载插件 WASM 二进制
    async fn fetch_wasm(
        &self,
        version_info: &crate::RegistryVersion,
    ) -> Result<Vec<u8>, PluginError> {
        let url = version_info
            .download_url
            .as_ref()
            .or(version_info.raw_url.as_ref())
            .ok_or_else(|| {
                PluginError::NetworkError("注册表中缺少 download_url / raw_url".to_string())
            })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| PluginError::NetworkError(format!("HTTP 客户端创建失败: {}", e)))?;

        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(|e| PluginError::NetworkError(format!("下载插件失败: {}", e)))?
            .bytes()
            .await
            .map_err(|e| PluginError::NetworkError(format!("读取下载响应失败: {}", e)))?;

        Ok(bytes.to_vec())
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
        channel: std::sync::Arc<dyn PluginEventSink>,
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
            (Some(vault), Some(account)) => {
                Arc::new(super::FieldResolver::with_vault_and_contracts(
                    vault,
                    account,
                    manifest.permissions.clone(),
                    manifest.contracts.clone(),
                ))
            }
            _ => self.field_resolver.clone(),
        };

        let session_id = session.id.clone();
        let locale = params
            .get("locale")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "zh-CN".to_string());
        let workspace_dir = std::env::temp_dir()
            .join("solosoul-plugin")
            .join(sanitize_plugin_id(plugin_id))
            .join(&session_id);
        std::fs::create_dir_all(&workspace_dir)
            .map_err(|e| PluginError::ExecutionFailed(format!("创建插件工作区失败: {}", e)))?;
        let host = super::SoloHostFunctions::new_with_workspace(
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
            Some(workspace_dir.clone()),
        );

        let _ = channel.send(PluginEvent::log(
            "info",
            plugin_start_message(&locale, &manifest.name),
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

        let cleanup = || {
            let _ = std::fs::remove_dir_all(&workspace_dir);
        };

        match result {
            Ok(r) => {
                cleanup();
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
                cleanup();
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

    /// 响应对话框请求
    pub async fn dialog_response(
        &self,
        request_id: &str,
        value: Option<String>,
    ) -> Result<(), PluginError> {
        self.consent_manager
            .respond(request_id, value)
            .await
            .map_err(|e| PluginError::ExecutionFailed(format!("对话框响应失败: {}", e)))?;
        Ok(())
    }

    /// 列出活跃会话
    pub fn list_sessions(&self) -> Result<Vec<PluginSession>, PluginError> {
        Ok(self.session_manager.list_active())
    }

    /// 获取审计日志
    pub fn audit_log(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<super::PluginAuditEntry>, PluginError> {
        Ok(self.audit.recent(limit.unwrap_or(50)))
    }

    /// 刷新注册表（从远程拉取并验证签名）
    pub async fn update_registry(&self) -> Result<(), PluginError> {
        self.registry.update_from_remote().await
    }
}
