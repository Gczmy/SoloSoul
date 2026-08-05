use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use crate::sync::auto_sync::AutoSyncManager;
use crate::sync::device_auto_sync::DeviceAutoSyncManager;
use solosoul_core::vault_file_system::{SafVaultFileSystem, VaultFileSystem};
use solosoul_core::vault_service::AccountSummary;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// `.solosoul_config` 文件名（存放在 SAF 目录根，用于重装后自动发现）。
const SAF_CONFIG_FILE: &str = ".solosoul_config";

#[derive(Clone)]
pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
    pub auto_sync: AutoSyncManager,
    /// 设备间自动同步调度器（前台/数据变更/定时）。
    pub device_auto_sync: DeviceAutoSyncManager,
    /// 标记是否已有后台过期回收站清理任务在运行，用于防止并发重复执行。
    pub trash_cleanup_running: Arc<AtomicBool>,
    /// 跨设备恢复主机状态（取消信号、后台线程、临时导出文件）。
    pub recovery_state: Arc<Mutex<RecoveryState>>,
    /// 生物识别因失败次数过多进入临时锁定后的预计解除时间。
    /// None 表示未锁定；用于在前端区分「不支持」和「暂时锁定」。
    pub biometric_lockout_until: Arc<Mutex<Option<Instant>>>,
}

/// 跨设备恢复主机的运行时状态。
pub struct RecoveryState {
    /// 正向恢复（主机发送数据）的取消信号。
    pub host_cancel: Arc<AtomicBool>,
    pub host_thread: Option<std::thread::JoinHandle<()>>,
    pub export_path: Option<PathBuf>,
    /// 恢复主机注册的 mDNS 服务实例名（用于清理）。
    pub mdns_instance_name: Option<String>,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            host_cancel: Arc::new(AtomicBool::new(false)),
            host_thread: None,
            export_path: None,
            mdns_instance_name: None,
        }
    }
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of first-launch vault directory initialization.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeVaultResult {
    pub success: bool,
    pub needs_restart: bool,
    pub message: String,
    /// 初始化后检测到的已有账户数量（0 = 新用户需创建，>0 = 直接登录）。
    #[serde(default)]
    pub account_count: u32,
    /// 初始化后检测到的已有账户列表，用于引导页展示账户名称。
    #[serde(default)]
    pub accounts: Vec<AccountSummary>,
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

    /// 写入 .solosoul_config 到本地临时目录，并通过 SAF 同步写入远端。
    /// 使卸载重装后用户选择相同目录时能自动恢复 SAF URI。
    pub(crate) fn write_saf_config_to_remote(
        temp_dir: &Path,
        saf_uri: &str,
        sync_driver: Arc<dyn solosoul_core::vault_file_system::SafSyncDriver>,
    ) -> Result<(), String> {
        let config_path = temp_dir.join(SAF_CONFIG_FILE);
        let config = serde_json::json!({
            "version": 1,
            "saf_tree_uri": saf_uri,
            "created_at": Self::now_rfc3339(),
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("写入 .solosoul_config 失败: {e}"))?;

        // 通过完整的 SAF 文件系统同步，确保 .solosoul_config 被上传到远端
        let fs = SafVaultFileSystem::new(saf_uri.to_string(), temp_dir.to_path_buf(), sync_driver);
        fs.sync_to_remote()?;
        tracing::info!("[AppState] .solosoul_config written and synced to SAF");
        Ok(())
    }

    /// 从本地临时目录读取 .solosoul_config，提取 saf_tree_uri。
    pub(crate) fn read_saf_config_uri(temp_dir: &Path) -> Option<String> {
        let config_path = temp_dir.join(SAF_CONFIG_FILE);
        if !config_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;
        config
            .get("saf_tree_uri")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// ISO 8601 格式时间戳，替代 chrono crate 依赖。
    fn now_rfc3339() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let hours = time_secs / 3600;
        let minutes = (time_secs % 3600) / 60;
        let seconds = time_secs % 60;

        let mut y = 1970i64;
        let mut remaining_days = days as i64;
        loop {
            let days_in_year = if Self::is_leap_year(y) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            y += 1;
        }
        let month_days = if Self::is_leap_year(y) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut m = 1usize;
        for days_in_month in month_days {
            if remaining_days < days_in_month {
                break;
            }
            remaining_days -= days_in_month;
            m += 1;
        }
        let d = remaining_days + 1;

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
            y, m, d, hours, minutes, seconds
        )
    }

    fn is_leap_year(y: i64) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
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

                // 先检查 SAF URI 是否仍然可访问（防止用户手动删除外部目录后以失效状态启动）。
                // check_vault_dir_access 在 Android 上通过 Kotlin 插件查询 ContentResolver
                // 判断目录是否存在；非 Android 平台返回 false，由下层 cfg 守卫。
                let is_valid = {
                    let plugin_handle = handle.state::<AttachmentImportPluginHandle<tauri::Wry>>();
                    plugin_handle.check_vault_dir_access(uri).unwrap_or(false)
                };

                if !is_valid {
                    tracing::warn!(
                        "[AppState] saved SAF URI is no longer accessible (dir deleted or revoked), falling back to local vault"
                    );
                    // 取消可能已调度的 WorkManager 兜底同步，避免旧 URI 持续重试。
                    let _ = handle
                        .state::<AttachmentImportPluginHandle<tauri::Wry>>()
                        .cancel_fallback_sync();
                    // 注意：故意【不】清除已失效的 SAF URI——
                    // 前端登录后会调用 vault_check_directory 检测失效并弹窗 + 横幅提示
                    // 用户重新选择目录（每次启动都应提醒）；auto-sync 每 30s 也会发射
                    // saf-auth-revoked 事件维持横幅。若在此清空 URI，前端将无从得知
                    // 外部目录已丢失，表现为「目录被删除后重启无任何提示」的回归。
                    // 用户重新选择目录（vault_set_directory 保存新 URI）或主动切回本地
                    // （保存 None）后，此路径自然退出。

                    // 迁移 SAF temp cache 到本地目录，保全用户缓存数据。
                    // 迁移失败不阻止降级（仅打日志）。
                    //
                    // 注意：这里必须用合并模式（clear_dst=false）！
                    // src（saf_vault_temp）位于 dst（data_dir）内部，若按默认
                    // 模式先清空 dst，会连带删除源目录本身以及 logs / app_resources
                    // / models 等应用级目录——用户数据被毁、插件市场目录被删，
                    // 首次启动直接闪退（插件管理器初始化失败导致 AppState::new 报错）。
                    let temp_cache = data_dir.join("saf_vault_temp");
                    if temp_cache.exists() {
                        tracing::info!("[AppState] migrating SAF temp cache to local vault");
                        match crate::commands::vault_directory::migrate_vault_data(
                            &temp_cache,
                            &data_dir,
                            false,
                        ) {
                            Ok(()) => {
                                // 合并迁移是 copy 而非 move，成功后删除残留副本，
                                // 避免加密数据双份占用磁盘；失败仅打日志，不影响降级。
                                if let Err(e) = std::fs::remove_dir_all(&temp_cache) {
                                    tracing::warn!(
                                        "[AppState] failed to clean up SAF temp cache: {e}"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[AppState] temp cache migration failed (non-fatal): {e}"
                                );
                            }
                        }
                    }
                    // 降级到本地 vault
                    match Self::try_init_local_vault(&data_dir) {
                        Ok(svc) => {
                            tracing::info!("[AppState] local vault init after SAF fallback: OK");
                            Arc::new(RwLock::new(svc))
                        }
                        Err(e) => {
                            tracing::error!(
                                "[AppState] local vault init after SAF fallback FAILED: {e}"
                            );
                            return Err(e);
                        }
                    }
                } else {
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

        // 装配入站新 peer 回调 → 全局事件 sync-pairing-request。
        // 响应方在入站 Hello 落库一条新的未信任 peer 记录时触发，前端任意页面
        // （AppShell 全局挂载）都能弹出配对确认对话框，无需用户停留在同步页。
        {
            use solosoul_sync::types::NewPeerInfo;
            let emit_handle = handle.clone();
            sync_service.set_peer_callback(Some(Arc::new(move |info: NewPeerInfo| {
                let _ = emit_handle.emit(
                    "sync-pairing-request",
                    serde_json::json!({
                        "nodeId": info.node_id,
                        "fingerprint": info.fingerprint,
                        "addr": info.addr,
                        "deviceName": info.device_name,
                        // SAS 配对验证码：两侧展示同一 6 位数字供目视比对。
                        "sasCode": info.sas_code,
                    }),
                );
            })));
        }

        // 装配入站会话完成回调 → 全局事件 sync-completed。
        // 响应方成功完成一次同步会话时触发，前端任意页面（AppShell 全局挂载）
        // 都能收到完成提醒并刷新结果——与发起方侧「同步完成」toast 对称，
        // 让两侧同时展示同步完成与具体条数。
        {
            use solosoul_sync::types::SessionCompletedInfo;
            let emit_handle = handle.clone();
            sync_service.set_session_callback(Some(Arc::new(move |info: SessionCompletedInfo| {
                // 响应方产生新冲突时也推送冲突徽章事件（与发起方 sync_with_device 对齐）。
                if info.conflicts > 0 {
                    let _ = emit_handle.emit(
                        "sync-conflicts-updated",
                        serde_json::json!({ "count": info.conflicts }),
                    );
                }
                let _ = emit_handle.emit(
                    "sync-completed",
                    serde_json::json!({
                        "peerNodeId": info.peer_node_id,
                        "examined": info.examined,
                        "applied": info.applied,
                        "skipped": info.skipped,
                        "conflicts": info.conflicts,
                    }),
                );
            })));
        }

        // ── DeviceAutoSyncManager（设备间自动同步，依赖 SyncService） ──
        let device_auto_sync =
            DeviceAutoSyncManager::new(sync_service.clone(), vault_service.clone(), handle.clone());

        // ── AutoSyncManager（在 VaultService 初始化之后启动） ──
        let auto_sync = AutoSyncManager::new_for_vault(vault_service.clone(), handle.clone());

        // ── PluginManager（初始化失败不阻止应用启动） ──
        let plugin_manager = match crate::plugin::new_plugin_manager(&handle) {
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
                        // 最终兜底：使用系统临时目录构造空插件管理器。
                        // 插件初始化失败绝不中止应用启动——Android Release 构建
                        // 使用 panic=abort，AppState::new 返回 Err 会导致 setup 失败
                        // 直接闪退（曾因迁移误删 app_resources 触发此路径）。
                        // 固定目录名复用：每次兜底不再新建 <pid> 后缀目录（避免残留堆积）。
                        let fallback_dir = std::env::temp_dir().join("solosoul_plugin_fallback");
                        let _ = std::fs::create_dir_all(&fallback_dir);
                        match PluginManager::new_with_dirs(
                            fallback_dir.clone(),
                            fallback_dir.clone(),
                        ) {
                            Ok(pm) => Arc::new(pm),
                            Err(final_err) => {
                                tracing::error!(
                                    "[AppState] PluginManager 最终兜底也失败: {:#}（继续无插件启动）",
                                    final_err
                                );
                                // 极端情况（临时目录也不可写）下仍不中止启动，
                                // 使用当前目录作为最后兜底；若仍失败仅打日志。
                                match PluginManager::new_with_dirs(
                                    std::env::current_dir()
                                        .unwrap_or_else(|_| fallback_dir.clone()),
                                    fallback_dir,
                                ) {
                                    Ok(pm) => Arc::new(pm),
                                    Err(last_err) => {
                                        // 仅当临时目录与当前目录均不可写（文件系统级异常）
                                        // 才中止启动——此时任何目录都无法构造 PluginManager，
                                        // 保留错误仅作为最后防线，Android 上 temp_dir 指向
                                        // 可写的应用缓存目录，实际不可达。
                                        tracing::error!(
                                            "[AppState] PluginManager 最后兜底失败: {:#}（无插件模式）",
                                            last_err
                                        );
                                        return Err(anyhow::anyhow!(
                                            "PluginManager 无法初始化（多次兜底均失败）"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        let app_state = Self {
            handle: handle.clone(),
            vault_service,
            sync_service,
            plugin_manager,
            auto_sync,
            device_auto_sync,
            trash_cleanup_running: Arc::new(AtomicBool::new(false)),
            recovery_state: Arc::new(Mutex::new(RecoveryState::new())),
            biometric_lockout_until: Arc::new(Mutex::new(None)),
        };

        // 若当前使用 SAF 远程 Vault，调度 WorkManager 兜底同步，
        // 确保应用被系统回收后仍能定期同步到 SAF。
        if app_state.has_saf_vault() {
            if let Err(e) = app_state.schedule_saf_fallback_sync() {
                tracing::warn!("[AppState] failed to schedule SAF fallback sync: {e}");
            }
        }

        Ok(app_state)
    }

    /// 判断当前是否使用了 SAF 远程存储。
    pub fn has_saf_vault(&self) -> bool {
        self.vault_service
            .read()
            .map(|g| g.is_remote_storage())
            .unwrap_or(false)
    }

    /// 调度 WorkManager 后台 SAF 同步兜底任务。
    ///
    /// - 仅 Android 平台生效，其他平台直接返回 Ok(())。
    /// - 仅在当前 Vault 为 SAF 远程存储且已保存 SAF URI 时执行。
    /// - 失败仅记录日志并返回错误，不阻塞主流程。
    pub(crate) fn schedule_saf_fallback_sync(&self) -> Result<(), String> {
        if !cfg!(target_os = "android") {
            return Ok(());
        }
        if !self.has_saf_vault() {
            return Ok(());
        }

        let data_dir = normalize_path(
            &self
                .handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| format!("无法解析应用数据目录: {e}"))?,
        );
        let saved_uri = Self::load_saved_saf_uri(&data_dir);
        if let Some(tree_uri) = saved_uri {
            let local_dir = data_dir.join("saf_vault_temp");
            let plugin_handle = self
                .handle
                .state::<AttachmentImportPluginHandle<tauri::Wry>>();
            plugin_handle
                .schedule_fallback_sync(local_dir.to_string_lossy().as_ref(), &tree_uri)?;
            tracing::info!("[AppState] scheduled SAF fallback sync via WorkManager");
        }
        Ok(())
    }

    /// 取消 WorkManager 后台 SAF 同步兜底任务。
    ///
    /// - 仅 Android 平台生效，其他平台直接返回 Ok(())。
    /// - 失败仅记录日志并返回错误，不阻塞主流程。
    pub(crate) fn cancel_saf_fallback_sync(&self) -> Result<(), String> {
        if !cfg!(target_os = "android") {
            return Ok(());
        }

        let plugin_handle = self
            .handle
            .state::<AttachmentImportPluginHandle<tauri::Wry>>();
        plugin_handle.cancel_fallback_sync()?;
        tracing::info!("[AppState] cancelled SAF fallback sync via WorkManager");
        Ok(())
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

        // 用户明确选择本地目录（saf_uri=None）时，清除可能残留的失效 SAF URI
        // （目录被删除后 AppState::new 降级本地时有意保留用于提醒），避免下次
        // 启动重复进入降级/提醒路径。
        if saf_uri.is_none() {
            let _ = Self::save_saf_uri(&data_dir, None);
        }

        // 若启用了 SAF，同步等待首次同步完成。
        // 失败直接返回错误，前端会展示给用户；成功则保证用户在
        // 已有远程数据被拉取后才会继续。
        if self.has_saf_vault() {
            // 先把本次要初始化的 SAF URI 持久化到磁盘，再执行首次同步——
            // init_saf_sync 会读取磁盘配置做有效性校验；若磁盘仍残留旧的失效
            // URI（目录被删除后 AppState::new 降级本地时有意保留，用于提醒），
            // 会错误地拒绝本次全新目录的初始化。
            if let Some(ref uri) = saf_uri {
                Self::save_saf_uri(&data_dir, Some(uri))?;
            }

            // 同步开始：通知前端显示进度条
            let _ = self.handle.emit(
                "sync-progress",
                serde_json::json!({"phase": "sync_start", "current": 0, "total": 1}),
            );

            if let Err(e) = self.init_saf_sync().await {
                // 同步失败：回退到本地 vault，避免留下半初始化的 SAF 状态。
                // 同时不保存 SAF URI，下次启动仍走本地/占位路径。
                tracing::warn!(
                    "[initialize_vault] SAF initial sync failed, rolling back to local: {e}"
                );
                // 清除本次提前写入的 URI，保持「失败不保存」的既有语义。
                let _ = Self::save_saf_uri(&data_dir, None);
                let local_svc = Self::try_init_local_vault(&data_dir)
                    .map_err(|e| format!("回退到本地 Vault 失败: {e}"))?;
                {
                    let mut guard = self
                        .vault_service
                        .write()
                        .map_err(|_| "Vault service lock poisoned".to_string())?;
                    *guard = local_svc;
                }
                // 取消 WorkManager 兜底同步：首次同步失败说明 SAF 不可用，
                // 避免旧配置持续触发无效同步。
                if let Err(e) = self.cancel_saf_fallback_sync() {
                    tracing::warn!("[initialize_vault] failed to cancel SAF fallback sync: {e}");
                }
                return Err(format!("首次同步失败: {e}"));
            }
            // 同步成功：通知前端进度完成
            let _ = self.handle.emit(
                "sync-progress",
                serde_json::json!({"phase": "sync_complete", "current": 1, "total": 1}),
            );

            // 同步后重载账户缓存，使前端能感知已有账户
            {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                svc.load_accounts();
            }

            // 写入 .solosoul_config 到 SAF 目录（含 saf_tree_uri 元数据），
            // 使卸载重装后用户选择相同目录时能自动恢复配置。
            if let Some(ref uri) = saf_uri {
                let temp_dir = data_dir.join("saf_vault_temp");
                let sync_driver =
                    Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(self.handle.clone()));
                Self::write_saf_config_to_remote(&temp_dir, uri, sync_driver).ok();

                // 检测：同步后检查 .solosoul_config 是否写入成功
                if let Some(config_uri) = Self::read_saf_config_uri(&temp_dir) {
                    tracing::info!(
                        "[AppState] .solosoul_config detected after sync, URI matches: {}",
                        config_uri == *uri
                    );
                } else {
                    tracing::warn!(
                        "[AppState] .solosoul_config not found after writing (sync may be pending)"
                    );
                }
                // 写入/检测 .solosoul_config 失败不影响主流程，仅打日志
            }

            // 调度 WorkManager 兜底同步，确保应用被系统回收后仍能同步到 SAF。
            if let Err(e) = self.schedule_saf_fallback_sync() {
                tracing::warn!("[AppState] failed to schedule SAF fallback sync: {e}");
            }
        }

        let accounts: Vec<AccountSummary> = self
            .vault_service
            .read()
            .map(|g| g.list_accounts())
            .unwrap_or_default();
        let account_count = accounts.len() as u32;

        Ok(InitializeVaultResult {
            success: true,
            needs_restart: false,
            message: "Vault 目录已初始化".to_string(),
            account_count,
            accounts,
        })
    }

    /// 设置生物识别临时锁定的到期时间（覆盖已有时间）。
    /// 用于 Android 指纹/人脸失败次数过多后，前端可据此显示锁定状态。
    pub fn set_biometric_lockout(&self, duration: Duration) {
        let mut guard = self
            .biometric_lockout_until
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = Some(Instant::now() + duration);
    }

    /// 检查当前是否仍处于生物识别临时锁定状态。
    /// 若已过期则自动清除。
    pub fn is_biometric_locked_out(&self) -> bool {
        let mut guard = self
            .biometric_lockout_until
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(until) = *guard {
            if Instant::now() < until {
                return true;
            }
            *guard = None;
        }
        false
    }

    /// 返回生物识别锁定的剩余秒数（若已锁定）。
    pub fn biometric_lockout_remaining(&self) -> Option<u64> {
        let guard = self
            .biometric_lockout_until
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.map(|until| {
            let now = Instant::now();
            if now < until {
                until.duration_since(now).as_secs()
            } else {
                0
            }
        })
    }

    /// 返回生物识别锁定的预计解除时间（Unix 秒）。
    /// 用于向前端展示「多久后可重试」。
    pub fn biometric_lockout_until_ts(&self) -> Option<i64> {
        let remaining = self.biometric_lockout_remaining()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Some(now + remaining as i64)
    }

    /// 手动清除生物识别锁定状态（成功验证或用户手动重试前调用）。
    pub fn clear_biometric_lockout(&self) {
        let mut guard = self
            .biometric_lockout_until
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = None;
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
                    "SAF 目录访问权限已被撤销，请前往「设置 > 数据管理」重新选择目录。".to_string(),
                );
            }
        }

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let read_guard = svc
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            // 始终执行首次同步：
            // 1. 防止本地 temp 中仅有 accounts.json 而账户目录未同步的半拉状态；
            // 2. sync_from_remote 内部按 mtime/size 跳过已一致文件，不会重复大下载。
            read_guard.sync_from_remote()?;
            tracing::info!("[AppState] SAF initial sync completed");
            Ok(())
        })
        .await
        .map_err(|e| format!("SAF sync task panicked: {e}"))?
    }
}
