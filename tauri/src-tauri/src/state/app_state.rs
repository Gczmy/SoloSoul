use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use solosoul_core::vault_file_system::SafVaultFileSystem;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
}

/// Result of first-launch vault directory initialization.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeVaultResult {
    pub success: bool,
    pub needs_restart: bool,
    pub message: String,
}

impl AppState {
    fn app_config_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("app_config.json")
    }

    fn load_saved_saf_uri(data_dir: &std::path::Path) -> Option<String> {
        let path = Self::app_config_path(data_dir);
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;
        config
            .get("saf_tree_uri")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// 保存或删除 SAF tree URI 配置。
    fn save_saf_uri(data_dir: &std::path::Path, uri: Option<&str>) -> Result<(), String> {
        let path = Self::app_config_path(data_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
        }
        if let Some(uri) = uri {
            let config = serde_json::json!({ "saf_tree_uri": uri });
            std::fs::write(
                &path,
                serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("写入配置失败: {e}"))?;
        } else if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("删除配置失败: {e}"))?;
        }
        Ok(())
    }

    /// 尝试以 SAF 模式初始化 VaultService。
    fn try_init_saf_vault(
        handle: &tauri::AppHandle,
        data_dir: &std::path::Path,
        uri: &str,
    ) -> Result<VaultService, anyhow::Error> {
        tracing::info!("[AppState] SAF init: creating SafVaultFileSystem...");
        let temp_dir = data_dir.join("saf_vault_temp");
        std::fs::create_dir_all(&temp_dir)?;

        let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));

        let fs = Arc::new(SafVaultFileSystem::new(
            uri.to_string(),
            temp_dir.clone(),
            sync_driver,
        ));

        tracing::info!("[AppState] SAF init: creating VaultService...");
        let svc = VaultService::with_file_system(temp_dir, fs);

        tracing::info!("[AppState] SAF init: loading accounts...");
        svc.load_accounts();

        tracing::info!("[AppState] SAF init: success");
        Ok(svc)
    }

    /// 用本地 App-private 目录初始化 VaultService。
    fn try_init_local_vault(data_dir: &std::path::Path) -> Result<VaultService, anyhow::Error> {
        tracing::info!(
            "[AppState] local vault init: data_dir={}",
            data_dir.display()
        );
        let svc = VaultService::with_base_path(data_dir.to_path_buf());
        svc.load_accounts();
        tracing::info!(
            "[AppState] loaded accounts count: {}",
            svc.list_accounts().len()
        );
        Ok(svc)
    }

    /// 创建一个占位用的 VaultService（首次启动尚未选择目录时使用）。
    /// 占位目录使用应用私有目录下的 `.uninitialized_vault`。
    fn placeholder_vault(data_dir: &std::path::Path) -> Result<VaultService, anyhow::Error> {
        let placeholder_dir = data_dir.join(".uninitialized_vault");
        std::fs::create_dir_all(&placeholder_dir)?;
        let svc = VaultService::with_base_path(placeholder_dir);
        svc.load_accounts();
        Ok(svc)
    }

    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        // ── 移动端 VaultService 初始化 ──
        let vault_service = if cfg!(mobile) {
            let data_dir = normalize_path(
                &handle
                    .path()
                    .resolve(".", tauri::path::BaseDirectory::Data)
                    .map_err(|e| anyhow::anyhow!("无法解析应用数据目录: {e}"))?,
            );
            tracing::info!("[AppState] mobile data_dir: {}", data_dir.display());

            let saved_uri = Self::load_saved_saf_uri(&data_dir);
            if let Some(ref uri) = saved_uri {
                tracing::info!("[AppState] found saved SAF URI: {uri}");
                match Self::try_init_saf_vault(&handle, &data_dir, uri) {
                    Ok(svc) => {
                        tracing::info!("[AppState] SAF vault initialized successfully");
                        Arc::new(RwLock::new(svc))
                    }
                    Err(e) => {
                        tracing::error!(
                            "[AppState] SAF vault init FAILED (falling back to local): {:#}",
                            e
                        );
                        Arc::new(RwLock::new(Self::try_init_local_vault(&data_dir)?))
                    }
                }
            } else {
                // 首次启动：尚未选择目录，使用占位 VaultService，
                // 等 onboarding 调用 initialize_vault 后再热替换。
                tracing::info!(
                    "[AppState] first launch: using placeholder vault until directory is selected"
                );
                Arc::new(RwLock::new(Self::placeholder_vault(&data_dir)?))
            }
        } else {
            Arc::new(RwLock::new(VaultService::new()))
        };

        // ── SyncService ──
        let sync_service = Arc::new(SyncService::new(vault_service.clone()));

        // ── PluginManager（初始化失败不阻止应用启动） ──
        let plugin_manager = match PluginManager::new_with_app_handle(&handle) {
            Ok(pm) => Arc::new(pm),
            Err(e) => {
                tracing::warn!(
                    "[AppState] PluginManager 初始化失败，将以无插件模式运行: {:#}",
                    e
                );
                match PluginManager::new() {
                    Ok(pm) => Arc::new(pm),
                    Err(fallback_err) => {
                        tracing::error!(
                            "[AppState] PluginManager 回退构造也失败: {:#}（将继续无插件启动）",
                            fallback_err
                        );
                        match PluginManager::new() {
                            Ok(pm) => Arc::new(pm),
                            Err(_) => {
                                return Err(anyhow::anyhow!(
                                    "PluginManager 无法初始化（三次尝试均失败）"
                                ));
                            }
                        }
                    }
                }
            }
        };

        Ok(Self {
            handle,
            vault_service,
            sync_service,
            plugin_manager,
        })
    }

    /// 判断当前是否使用了 SAF 远程存储。
    pub fn has_saf_vault(&self) -> bool {
        self.vault_service
            .read()
            .map(|g| g.is_remote_storage())
            .unwrap_or(false)
    }

    /// 首次启动时初始化 VaultService（不重启）。
    /// 仅对 Android 有效；桌面端调用会返回错误。
    /// 用新的 VaultService 整体替换当前 vault_service 中的实例。
    ///
    /// 注意：当选择 SAF 目录时，本方法会等待首次同步完成后再返回，
    /// 以避免用户在同步完成前创建账户导致的数据竞态；同步失败会直
    /// 接返回错误，让前端可以提示用户。
    pub async fn initialize_vault(
        &self,
        saf_uri: Option<String>,
    ) -> Result<InitializeVaultResult, String> {
        if !cfg!(mobile) {
            return Err("仅在移动端支持初始化 Vault 目录".to_string());
        }

        let data_dir = normalize_path(
            &self
                .handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| format!("无法解析应用数据目录: {e}"))?,
        );

        let handle = self.handle.clone();
        let new_svc = if let Some(ref uri) = saf_uri {
            Self::try_init_saf_vault(&handle, &data_dir, uri)
                .map_err(|e| format!("初始化 SAF Vault 失败: {e}"))?
        } else {
            Self::try_init_local_vault(&data_dir)
                .map_err(|e| format!("初始化本地 Vault 失败: {e}"))?
        };

        {
            let mut guard = self
                .vault_service
                .write()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            *guard = new_svc;
        }

        // 清理占位目录，避免残留空数据。
        let placeholder_dir = data_dir.join(".uninitialized_vault");
        if placeholder_dir.exists() {
            let _ = std::fs::remove_dir_all(&placeholder_dir);
        }

        // 若启用了 SAF，同步等待首次同步完成。
        // 失败直接返回错误，前端会展示给用户；成功则保证用户在
        // 已有远程数据被拉取后才会继续。
        if self.has_saf_vault() {
            if let Err(e) = self.init_saf_sync().await {
                // 同步失败：回退到本地 vault，避免留下半初始化的 SAF 状态。
                // 同时不保存 SAF URI，下次启动仍走本地/占位路径。
                tracing::warn!(
                    "[initialize_vault] SAF initial sync failed, rolling back to local: {e}"
                );
                let local_svc = Self::try_init_local_vault(&data_dir)
                    .map_err(|e| format!("回退到本地 Vault 失败: {e}"))?;
                {
                    let mut guard = self
                        .vault_service
                        .write()
                        .map_err(|_| "Vault service lock poisoned".to_string())?;
                    *guard = local_svc;
                }
                return Err(format!("首次同步失败: {e}"));
            }
            // 同步成功后才持久化 SAF URI
            if let Some(ref uri) = saf_uri {
                Self::save_saf_uri(&data_dir, Some(uri))?;
            }
        }

        Ok(InitializeVaultResult {
            success: true,
            needs_restart: false,
            message: "Vault 目录已初始化".to_string(),
        })
    }

    pub async fn init_saf_sync(&self) -> Result<(), String> {
        let svc = self.vault_service.clone();
        let app_handle = self.handle.clone();

        let data_dir = normalize_path(
            &app_handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| format!("无法解析应用数据目录: {e}"))?,
        );
        let saved_uri = Self::load_saved_saf_uri(&data_dir);

        if let Some(ref uri) = saved_uri {
            let plugin_handle = app_handle.state::<AttachmentImportPluginHandle<tauri::Wry>>();
            let valid = plugin_handle.check_vault_dir_access(uri).unwrap_or(false);
            if !valid {
                tracing::error!(
                    "[AppState] SAF directory access revoked for {}, skipping initial sync",
                    uri
                );
                return Err(
                    "SAF 目录访问权限已被撤销，请前往「设置 > 保险库目录」重新选择目录。"
                        .to_string(),
                );
            }
        }

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let read_guard = svc
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let accounts_path = read_guard.base_path().join("accounts.json");
            if accounts_path.exists() {
                tracing::info!(
                    "[AppState] SAF temp dir already has accounts.json, skipping initial sync"
                );
                return Ok(());
            }
            read_guard.sync_from_remote()?;
            tracing::info!("[AppState] SAF initial sync completed");
            Ok(())
        })
        .await
        .map_err(|e| format!("SAF sync task panicked: {e}"))?
    }
}
