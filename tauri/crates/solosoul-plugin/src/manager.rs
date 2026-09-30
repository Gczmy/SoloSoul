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
use crate::install_progress::{InstallProgressReporter, PluginInstallPhase, PluginInstallProgress};
use crate::store::{validate_plugin_id, PreparedStoredPlugin, MAX_WASM_SIZE};
use serde::Deserialize;
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 随机私有工作区；Unix 创建时即收紧为 0700，Windows 使用用户临时目录 ACL。
fn create_plugin_workspace() -> Result<tempfile::TempDir, PluginError> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("solosoul-plugin-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder
        .tempdir()
        .map_err(|e| PluginError::ExecutionFailed(format!("创建插件工作区失败: {e}")))
}

/// 清理所有权跟随真正的阻塞 worker；外层异步任务取消不能提前删除正在使用的文件。
fn spawn_plugin_worker<T: Send + 'static>(
    workspace: tempfile::TempDir,
    execute: impl FnOnce() -> T + Send + 'static,
) -> tokio::task::JoinHandle<T> {
    tokio::task::spawn_blocking(move || {
        let _workspace = workspace;
        execute()
    })
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

/// 已验证、已暂存但尚未发布的安装。不可 Clone；丢弃即释放其临时文件。
/// 必须交回准备它的 PluginStore 发布，不能绕过发布时的原安装版本复核。
pub struct PreparedPluginInstall {
    stored: PreparedStoredPlugin,
    completion: PluginInstallProgress,
    audit_install: bool,
}

impl PreparedPluginInstall {
    /// 实际准备版本的只读元数据，包含 bundled 回退后的版本。
    pub fn manifest(&self) -> &PluginManifest {
        self.stored.manifest()
    }
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
    sandbox: WasmSandbox,
    #[cfg(test)]
    install_client: Option<reqwest::Client>,
}

/// P025: 共享插件市场 HTTP 客户端——连接复用，避免每次下载重建 TLS 握手。
/// 超时按请求级设置（manifest 30s / WASM 60s），客户端级仅设连接超时兜底。
fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

const MAX_MANIFEST_SIZE: usize = 1024 * 1024;
const INSTALL_READ_CHUNK: usize = 64 * 1024;

fn wasm_size_error(size: u64) -> PluginError {
    PluginError::WasmTooLarge(usize::try_from(size).unwrap_or(usize::MAX))
}

/// bundled 文件同样有界；先查长度，再限制实际读取，防止读取中增长突破上限。
fn read_bundled_file(path: &Path, wasm: bool) -> Result<Vec<u8>, PluginError> {
    let limit = if wasm {
        MAX_WASM_SIZE
    } else {
        MAX_MANIFEST_SIZE
    };
    let too_large = |size: u64| {
        if wasm {
            wasm_size_error(size)
        } else {
            PluginError::InvalidManifest("manifest 超过 1 MiB 限制".to_string())
        }
    };
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > limit as u64 {
        return Err(too_large(size));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(too_large(bytes.len() as u64));
    }
    Ok(bytes)
}

async fn read_manifest_response(mut response: reqwest::Response) -> Result<Vec<u8>, PluginError> {
    let oversized = || PluginError::InvalidManifest("manifest 超过 1 MiB 限制".to_string());
    if response
        .content_length()
        .is_some_and(|size| size > MAX_MANIFEST_SIZE as u64)
    {
        return Err(oversized());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| PluginError::NetworkError("读取 manifest 响应失败".to_string()))?
    {
        for part in chunk.chunks(INSTALL_READ_CHUNK) {
            if part.len() > MAX_MANIFEST_SIZE - bytes.len() {
                return Err(oversized());
            }
            bytes.extend_from_slice(part);
            tokio::task::yield_now().await;
        }
    }
    Ok(bytes)
}

/// 分块读取响应，限制先于 Vec 增长；进度来自实际字节，循环主动让出执行权。
async fn read_wasm_response(
    mut response: reqwest::Response,
    progress: &mut InstallProgressReporter<'_>,
) -> Result<Vec<u8>, PluginError> {
    let total = response.content_length();
    if let Some(size) = total.filter(|size| *size > MAX_WASM_SIZE as u64) {
        return Err(wasm_size_error(size));
    }
    progress.download(0, total);
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| PluginError::NetworkError("读取下载响应失败".to_string()))?
    {
        for part in chunk.chunks(INSTALL_READ_CHUNK) {
            if part.len() > MAX_WASM_SIZE - bytes.len() {
                return Err(PluginError::WasmTooLarge(
                    bytes.len().saturating_add(part.len()),
                ));
            }
            bytes.extend_from_slice(part);
            progress.download(bytes.len() as u64, total);
            tokio::task::yield_now().await;
        }
    }
    Ok(bytes)
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
            sandbox: WasmSandbox::new(),
            #[cfg(test)]
            install_client: None,
        })
    }

    fn install_http_client(&self) -> &reqwest::Client {
        // 回环回归显式注入无代理客户端，不修改共享环境或生产客户端策略。
        #[cfg(test)]
        if let Some(client) = &self.install_client {
            return client;
        }
        http_client()
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

    /// 快速复用须由 Store 在同一次成对读取中验证版本与 SHA，并绑定原 selector。
    /// 无法读取/校验时仍按旧行为走完整准备，修复损坏或缺失的安装。
    fn prepare_installed_reuse(
        &self,
        plugin_id: &str,
        version: &str,
        expected_hash: &str,
    ) -> Option<PreparedStoredPlugin> {
        self.store
            .prepare_reuse(plugin_id, version, expected_hash)
            .ok()
            .flatten()
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

    fn validate_manifest_identity(
        manifest: &MarketManifestRaw,
        plugin_id: &str,
        version: &str,
    ) -> Result<(), PluginError> {
        if manifest.plugin_id != plugin_id || manifest.version != version {
            return Err(PluginError::InvalidManifest(
                "manifest 的插件 ID 或版本与安装目标不一致".to_string(),
            ));
        }
        Ok(())
    }

    fn check_compatible(version: &str, info: &crate::RegistryVersion) -> Result<(), PluginError> {
        if !crate::version::is_version_compatible(info, &crate::version::current_app_version()?) {
            return Err(PluginError::IncompatibleVersion(version.to_string()));
        }
        Ok(())
    }

    fn bundled_manifest(&self, plugin_id: &str) -> Result<MarketManifestRaw, PluginError> {
        let path = self
            .market_dir
            .join("plugins")
            .join(plugin_id)
            .join("manifest.json");
        let bytes = read_bundled_file(&path, false)?;
        serde_json::from_slice(&bytes)
            .map_err(|_| PluginError::InvalidManifest("bundled manifest 解析失败".to_string()))
    }

    fn bundled_wasm(&self, plugin_id: &str) -> Result<Vec<u8>, PluginError> {
        let path = self
            .market_dir
            .join("plugins")
            .join(plugin_id)
            .join("plugin.wasm");
        read_bundled_file(&path, true)
    }

    fn prepare_verified(
        &self,
        raw: MarketManifestRaw,
        plugin_id: &str,
        version: &str,
        info: &crate::RegistryVersion,
        wasm: Vec<u8>,
        progress: &mut InstallProgressReporter<'_>,
    ) -> Result<PreparedPluginInstall, PluginError> {
        Self::validate_manifest_identity(&raw, plugin_id, version)?;
        Self::check_compatible(version, info)?;
        if wasm.len() > MAX_WASM_SIZE {
            return Err(PluginError::WasmTooLarge(wasm.len()));
        }
        let bytes = wasm.len() as u64;
        progress.download(bytes, Some(bytes));
        progress.phase(PluginInstallPhase::Verifying);
        // 注册表摘要始终必验；空摘要不会降低为跳过校验。
        if compute_sha256(&wasm) != info.sha256 {
            return Err(PluginError::ChecksumMismatch);
        }
        let manifest = Self::manifest_from_raw(raw, version.to_string(), Some(info.sha256.clone()));
        progress.phase(PluginInstallPhase::Installing);
        let stored = self.store.prepare_plugin(&manifest, &wasm)?;
        progress.phase(PluginInstallPhase::Finalizing);
        Ok(PreparedPluginInstall {
            stored,
            completion: PluginInstallProgress {
                percent: 100,
                phase: PluginInstallPhase::Completed,
                downloaded_bytes: bytes,
                total_bytes: Some(bytes),
            },
            audit_install: true,
        })
    }

    /// 只准备，不改变当前安装。HTTP、本地回退、校验和暂存均由调用 Future 拥有。
    pub async fn prepare_install_from_registry_with_progress(
        &self,
        plugin_id: &str,
        version: &str,
        on_progress: &(dyn Fn(PluginInstallProgress) + Send + Sync),
    ) -> Result<PreparedPluginInstall, PluginError> {
        validate_plugin_id(plugin_id)?;
        let mut progress = InstallProgressReporter::new(on_progress);
        let entry = self.registry.get_entry(plugin_id)?;
        let info = entry
            .versions
            .get(version)
            .ok_or_else(|| PluginError::NotFound(format!("版本 {} 不存在", version)))?;
        Self::check_compatible(version, info)?;

        if let Some(stored) = self.prepare_installed_reuse(plugin_id, version, &info.sha256) {
            progress.phase(PluginInstallPhase::Finalizing);
            return Ok(PreparedPluginInstall {
                stored,
                completion: PluginInstallProgress::completed(),
                // 保留原免下载路径：复用成功不重复写 PluginInstalled 审计。
                audit_install: false,
            });
        }

        let raw = match self.fetch_manifest(info).await {
            Ok(raw) => {
                Self::validate_manifest_identity(&raw, plugin_id, version)?;
                raw
            }
            Err(_) => {
                let local = self.bundled_manifest(plugin_id)?;
                // 离线回退可以使用注册表已知的 bundled 版本，但不能替换插件身份。
                if local.plugin_id != plugin_id {
                    return Err(PluginError::InvalidManifest(
                        "bundled 插件 ID 与安装目标不一致".into(),
                    ));
                }
                if local.version != version {
                    let bundled_version = local.version.clone();
                    let bundled_info = entry.versions.get(&bundled_version).ok_or_else(|| {
                        PluginError::NetworkError("bundled 版本不在注册表中，不能回退安装".into())
                    })?;
                    Self::check_compatible(&bundled_version, bundled_info)?;
                    progress.phase(PluginInstallPhase::Downloading);
                    let wasm = self.bundled_wasm(plugin_id)?;
                    tokio::task::yield_now().await;
                    return self.prepare_verified(
                        local,
                        plugin_id,
                        &bundled_version,
                        bundled_info,
                        wasm,
                        &mut progress,
                    );
                }
                local
            }
        };

        progress.phase(PluginInstallPhase::Downloading);
        let wasm = match self.fetch_wasm(info, &mut progress).await {
            Ok(wasm) => wasm,
            Err(PluginError::WasmTooLarge(size)) => return Err(PluginError::WasmTooLarge(size)),
            Err(_) => self.bundled_wasm(plugin_id)?,
        };
        tokio::task::yield_now().await;
        self.prepare_verified(raw, plugin_id, version, info, wasm, &mut progress)
    }

    /// 准备注册表最新版本；CLI install latest 与 update 共享同一流程。
    pub async fn prepare_update_with_progress(
        &self,
        plugin_id: &str,
        on_progress: &(dyn Fn(PluginInstallProgress) + Send + Sync),
    ) -> Result<PreparedPluginInstall, PluginError> {
        validate_plugin_id(plugin_id)?;
        let latest = self
            .registry
            .get_entry(plugin_id)?
            .latest_version
            .ok_or_else(|| PluginError::RegistryError("缺少最新版本信息".to_string()))?;
        self.prepare_install_from_registry_with_progress(plugin_id, &latest, on_progress)
            .await
    }

    /// 同步短提交：Store 复核原选择并切换完整版本，成功后才写成功审计。
    /// CLI 在 TaskContext::commit 的原会话门闩内调用；这里不下载、不再校验大文件。
    pub fn publish_install(
        &self,
        prepared: PreparedPluginInstall,
    ) -> Result<PluginInstallResult, PluginError> {
        let plugin_id = prepared.manifest().id.clone();
        let version = prepared.manifest().version.clone();
        self.store.publish_prepared(prepared.stored)?;
        if prepared.audit_install {
            self.audit.log(
                &plugin_id,
                None::<String>,
                PluginAuditAction::PluginInstalled {
                    version: version.clone(),
                },
            );
        }
        Ok(PluginInstallResult {
            plugin_id,
            version,
            installed_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    fn publish_with_progress(
        &self,
        prepared: PreparedPluginInstall,
        on_progress: &(dyn Fn(PluginInstallProgress) + Send + Sync),
    ) -> Result<PluginInstallResult, PluginError> {
        let completion = prepared.completion.clone();
        let result = self.publish_install(prepared)?;
        on_progress(completion);
        Ok(result)
    }

    /// 原 GUI/库 API 保持签名，完成通知只在真实发布成功后发送。
    pub async fn install_from_registry(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<PluginInstallResult, PluginError> {
        self.install_from_registry_with_progress(plugin_id, version, &|_| {})
            .await
    }

    pub async fn install_from_registry_with_progress(
        &self,
        plugin_id: &str,
        version: &str,
        on_progress: &(dyn Fn(PluginInstallProgress) + Send + Sync),
    ) -> Result<PluginInstallResult, PluginError> {
        let prepared = self
            .prepare_install_from_registry_with_progress(plugin_id, version, on_progress)
            .await?;
        self.publish_with_progress(prepared, on_progress)
    }

    pub async fn update(&self, plugin_id: &str) -> Result<PluginInstallResult, PluginError> {
        self.update_with_progress(plugin_id, &|_| {}).await
    }

    pub async fn update_with_progress(
        &self,
        plugin_id: &str,
        on_progress: &(dyn Fn(PluginInstallProgress) + Send + Sync),
    ) -> Result<PluginInstallResult, PluginError> {
        let prepared = self
            .prepare_update_with_progress(plugin_id, on_progress)
            .await?;
        self.publish_with_progress(prepared, on_progress)
    }

    async fn fetch_manifest(
        &self,
        info: &crate::RegistryVersion,
    ) -> Result<MarketManifestRaw, PluginError> {
        let url = info
            .raw_url
            .as_ref()
            .or(info.download_url.as_ref())
            .ok_or_else(|| {
                PluginError::NetworkError("注册表中缺少 download_url / raw_url".to_string())
            })?;
        let base = url.strip_suffix("plugin.wasm").ok_or_else(|| {
            PluginError::NetworkError("无法从 download_url 推导 manifest URL".to_string())
        })?;
        let response = self
            .install_http_client()
            .get(format!("{}manifest.json", base))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|_| PluginError::NetworkError("下载 manifest 失败".to_string()))?
            .error_for_status()
            .map_err(|_| PluginError::NetworkError("下载 manifest 失败".to_string()))?;
        let bytes = read_manifest_response(response).await?;
        serde_json::from_slice(&bytes)
            .map_err(|_| PluginError::InvalidManifest("manifest JSON 解析失败".to_string()))
    }

    async fn fetch_wasm(
        &self,
        info: &crate::RegistryVersion,
        progress: &mut InstallProgressReporter<'_>,
    ) -> Result<Vec<u8>, PluginError> {
        let url = info
            .download_url
            .as_ref()
            .or(info.raw_url.as_ref())
            .ok_or_else(|| {
                PluginError::NetworkError("注册表中缺少 download_url / raw_url".to_string())
            })?;
        let response = self
            .install_http_client()
            .get(url)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|_| PluginError::NetworkError("下载插件失败".to_string()))?
            .error_for_status()
            .map_err(|_| PluginError::NetworkError("下载插件失败".to_string()))?;
        read_wasm_response(response, progress).await
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
    /// `attachment_key`: P001 附件静态加密密钥（插件复制附件到工作区前解密）。
    pub async fn run(
        &self,
        plugin_id: &str,
        params: HashMap<String, String>,
        channel: std::sync::Arc<dyn PluginEventSink>,
        vault_store: Option<Arc<VaultStore>>,
        account_id: Option<String>,
        attachment_key: Option<[u8; 32]>,
    ) -> Result<PluginResult, PluginError> {
        let (manifest, wasm_bytes) = self.store.load_plugin(plugin_id)?;
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
                let mut resolver = super::FieldResolver::with_vault_and_contracts(
                    vault,
                    account,
                    manifest.permissions.clone(),
                    manifest.contracts.clone(),
                );
                if let Some(key) = attachment_key {
                    resolver = resolver.with_attachment_key(key);
                }
                Arc::new(resolver)
            }
            _ => Arc::new(FieldResolver::new()),
        };
        let field_resolver = Arc::new((*field_resolver).clone().with_session(&session));
        let channel: Arc<dyn PluginEventSink> = Arc::new(super::event::SessionEventSink {
            resolver: field_resolver.clone(),
            inner: channel,
        });

        let session_id = session.id.clone();
        let locale = params
            .get("locale")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "zh-CN".to_string());
        let workspace = create_plugin_workspace()?;
        let workspace_dir = workspace.path().to_path_buf();
        let host = super::SoloHostFunctions::new_with_workspace(
            plugin_id,
            &manifest.name,
            &session_id,
            manifest.clone(),
            params,
            self.audit.clone(),
            self.rate_limiter.clone(),
            self.consent_manager.clone(),
            field_resolver.clone(),
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
        let result = spawn_plugin_worker(workspace, move || {
            let module = sandbox.compile(&wasm_bytes)?;
            sandbox.execute(&module, host, &session_for_spawn, &consent)
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("任务 Join 失败: {}", e)))?;

        match result {
            Ok(r) => {
                field_resolver.ensure_live()?;
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

#[cfg(test)]
mod workspace_tests {
    use super::*;

    #[tokio::test]
    async fn workspace_is_removed_after_success_error_and_panic() {
        for outcome in 0..3 {
            let workspace = create_plugin_workspace().unwrap();
            let path = workspace.path().to_path_buf();
            let worker_path = path.clone();
            let result = spawn_plugin_worker(workspace, move || {
                std::fs::write(worker_path.join("plaintext"), b"private").unwrap();
                match outcome {
                    0 => Ok(()),
                    1 => Err("execution failed"),
                    _ => panic!("simulated worker panic"),
                }
            })
            .await;
            match outcome {
                0 => assert!(result.unwrap().is_ok()),
                1 => assert!(result.unwrap().is_err()),
                _ => assert!(result.unwrap_err().is_panic()),
            }
            assert!(!path.exists());
        }
    }

    #[tokio::test]
    async fn cancelling_started_worker_keeps_workspace_until_worker_exits() {
        let workspace = create_plugin_workspace().unwrap();
        let path = workspace.path().to_path_buf();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let worker = spawn_plugin_worker(workspace, move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            assert!(worker_path.exists());
            std::fs::write(worker_path.join("plaintext"), b"private").unwrap();
        });
        started_rx.await.unwrap();
        worker.abort();
        assert!(path.exists());
        release_tx.send(()).unwrap();
        worker.await.unwrap();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn workspace_is_private_at_creation() {
        use std::os::unix::fs::PermissionsExt;
        let workspace = create_plugin_workspace().unwrap();
        assert_eq!(
            std::fs::metadata(workspace.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}

#[cfg(test)]
mod install_progress_tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn reports_received_bytes_before_download_finishes() {
        let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/plugin.wasm", server.local_addr().unwrap());
        let (half_tx, half_rx) = tokio::sync::oneshot::channel();
        let half_tx = Mutex::new(Some(half_tx));
        let writer = tokio::spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();
            let mut request = [0; 2048];
            // 此处只等待请求开始；EOF 不能被当作已收到请求。
            assert_ne!(stream.read(&mut request).await.unwrap(), 0);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n12345",
                )
                .await
                .unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(5), half_rx)
                .await
                .unwrap()
                .unwrap();
            stream.write_all(b"67890").await.unwrap();
        });
        let events = Mutex::new(Vec::new());
        let emit = |event: PluginInstallProgress| {
            if event.downloaded_bytes == 5 {
                if let Some(tx) = half_tx.lock().unwrap().take() {
                    tx.send(()).unwrap();
                }
            }
            events.lock().unwrap().push(event);
        };
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        let bytes = read_wasm_response(response, &mut InstallProgressReporter::new(&emit))
            .await
            .unwrap();
        writer.await.unwrap();
        assert_eq!(bytes, b"1234567890");
        let events = events.lock().unwrap();
        assert!(events
            .iter()
            .any(|p| p.percent == 50 && p.downloaded_bytes == 5 && p.total_bytes == Some(10)));
        assert_eq!(events.last().unwrap().percent, 90);
    }

    #[tokio::test]
    async fn bundled_install_and_update_complete_only_after_verified_write() {
        for (bundled_version, valid_hash) in [("1.0.0", true), ("0.9.0", true), ("1.0.0", false)] {
            let directory = tempfile::tempdir().unwrap();
            let market = directory.path().join("market");
            let data = directory.path().join("installed");
            let plugin = "com.solosoul.test.progress";
            let bundled = market.join("plugins").join(plugin);
            std::fs::create_dir_all(&bundled).unwrap();
            let bytes = b"\0asm\x01\0\0\0";
            let hash = if valid_hash {
                compute_sha256(bytes)
            } else {
                "invalid-hash".to_string()
            };
            let version = serde_json::json!({ "sha256": hash, "min_app_version": "0.0.0", "max_app_version": "99.0.0" });
            std::fs::write(market.join("registry.json"), serde_json::json!({
                "plugins": { (plugin): { "name": "Test", "latest_version": "1.0.0", "versions": { "1.0.0": version, "0.9.0": version } } }
            }).to_string()).unwrap();
            std::fs::write(bundled.join("manifest.json"), serde_json::json!({ "plugin_id": plugin, "name": "Test", "version": bundled_version }).to_string()).unwrap();
            std::fs::write(bundled.join("plugin.wasm"), bytes).unwrap();
            let manager = PluginManager::new_with_dirs(market, data).unwrap();
            let events = Mutex::new(Vec::new());
            let emit = |event: PluginInstallProgress| {
                if event.phase == PluginInstallPhase::Completed {
                    // 100% 发出时包与 manifest 已可从安装目录读取，不能把下载完成冒充安装完成。
                    assert_eq!(manager.store.load_wasm(plugin).unwrap(), bytes);
                }
                events.lock().unwrap().push(event);
            };
            let result = manager.update_with_progress(plugin, &emit).await;
            if valid_hash {
                assert_eq!(result.unwrap().version, bundled_version);
                assert_eq!(events.lock().unwrap().last().unwrap().percent, 100);
                // 重装同版本走已验证的本地缓存，仍然报告完成。
                manager
                    .install_from_registry_with_progress(plugin, bundled_version, &emit)
                    .await
                    .unwrap();
            } else {
                assert!(matches!(result, Err(PluginError::ChecksumMismatch)));
                assert!(events.lock().unwrap().iter().all(|p| p.percent < 100));
                assert!(manager.list_installed().unwrap().is_empty());
            }
        }
    }
}

#[cfg(test)]
#[path = "manager/rf214_tests.rs"]
mod rf214_tests;
