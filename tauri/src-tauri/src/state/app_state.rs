use super::recovery::RecoveryState;
use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use crate::sync::auto_sync::AutoSyncManager;
use crate::sync::device_auto_sync::DeviceAutoSyncManager;
use solosoul_core::vault_service::AccountSummary;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

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
    /// SyncService + 入站回调 + 设备自动同步装配，并恢复两个持久化开关。
    fn init_sync_components(
        handle: &tauri::AppHandle,
        vault_service: &Arc<RwLock<VaultService>>,
    ) -> (Arc<SyncService>, DeviceAutoSyncManager) {
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
                        // B：响应方发回给发起方的记录条数——前端 toast/结果行据此
                        // 展示双向完整交换量（旧版只有入站方向，「检查 0 条」误导）。
                        "outboundRecords": info.outbound_records,
                    }),
                );
            })));
        }

        // ── DeviceAutoSyncManager（设备间自动同步，依赖 SyncService） ──
        let device_auto_sync =
            DeviceAutoSyncManager::new(sync_service.clone(), vault_service.clone(), handle.clone());

        // P0#1: 启动时恢复自动同步开关持久化状态（AtomicBool 默认 false，
        // 重启后恢复用户上次选择，消除"已打开但实际已失效"的感知断裂）。
        // ui_preferences.json 存于 Vault base 目录，Vault 未解锁亦可读。
        if let Ok(svc) = vault_service.read() {
            if let Some(enabled) = crate::commands::settings::read_auto_sync_pref(handle, &svc) {
                device_auto_sync.set_enabled(enabled);
                tracing::info!("[AppState] restored auto_sync_enabled={}", enabled);
            }
        }

        // 启动时恢复「账户设置偏好是否随设备同步」开关（默认 true，
        // 无持久化值时保持默认；与 auto_sync_enabled 同模式）。
        if let Ok(svc) = vault_service.read() {
            if let Some(enabled) = crate::commands::settings::read_ui_prefs_sync_pref(handle, &svc)
            {
                svc.set_ui_prefs_sync_enabled(enabled);
                tracing::info!("[AppState] restored ui_prefs_sync_enabled={}", enabled);
            }
        }

        (sync_service, device_auto_sync)
    }

    /// PluginManager 初始化：多级兜底（临时目录 → 当前目录），最终失败才中止启动。
    /// Android Release 构建使用 panic=abort，AppState::new 返回 Err 会导致 setup
    /// 失败直接闪退，故仅当文件系统级异常（所有目录均不可写）才返回 Err。
    fn init_plugin_manager(handle: &tauri::AppHandle) -> Result<Arc<PluginManager>, anyhow::Error> {
        match crate::plugin::new_plugin_manager(handle) {
            Ok(pm) => return Ok(Arc::new(pm)),
            Err(e) => {
                tracing::warn!(
                    "[AppState] PluginManager 初始化失败，将以无插件模式运行: {:#}",
                    e
                );
            }
        }
        match PluginManager::new() {
            Ok(pm) => return Ok(Arc::new(pm)),
            Err(fallback_err) => {
                tracing::error!(
                    "[AppState] PluginManager 回退构造也失败: {:#}（将继续无插件启动）",
                    fallback_err
                );
            }
        }
        // 最终兜底：使用系统临时目录构造空插件管理器。
        // 固定目录名复用：每次兜底不再新建 <pid> 后缀目录（避免残留堆积）。
        let fallback_dir = std::env::temp_dir().join("solosoul_plugin_fallback");
        let _ = std::fs::create_dir_all(&fallback_dir);
        match PluginManager::new_with_dirs(fallback_dir.clone(), fallback_dir.clone()) {
            Ok(pm) => return Ok(Arc::new(pm)),
            Err(final_err) => {
                tracing::error!(
                    "[AppState] PluginManager 最终兜底也失败: {:#}（继续无插件启动）",
                    final_err
                );
            }
        }
        // 极端情况（临时目录也不可写）下仍不中止启动，使用当前目录作为最后兜底。
        match PluginManager::new_with_dirs(
            std::env::current_dir().unwrap_or_else(|_| fallback_dir.clone()),
            fallback_dir,
        ) {
            Ok(pm) => Ok(Arc::new(pm)),
            Err(last_err) => {
                // 仅当临时目录与当前目录均不可写（文件系统级异常）
                // 才中止启动——此时任何目录都无法构造 PluginManager。
                // Android 上 temp_dir 指向可写的应用缓存目录，实际不可达。
                tracing::error!(
                    "[AppState] PluginManager 最后兜底失败: {:#}（无插件模式）",
                    last_err
                );
                Err(anyhow::anyhow!(
                    "PluginManager 无法初始化（多次兜底均失败）"
                ))
            }
        }
    }

    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        // ── 移动端 VaultService 初始化 ──
        let vault_service = Self::init_vault_service(&handle)?;

        // ── SyncService / 回调 / DeviceAutoSyncManager / 持久化开关恢复 ──
        let (sync_service, device_auto_sync) = Self::init_sync_components(&handle, &vault_service);

        // ── AutoSyncManager（在 VaultService 初始化之后启动） ──
        let auto_sync = AutoSyncManager::new_for_vault(vault_service.clone(), handle.clone());

        // ── PluginManager（初始化失败不阻止应用启动） ──
        let plugin_manager = Self::init_plugin_manager(&handle)?;

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

        // 热替换 VaultService 后重应用「同步设置偏好」开关（成功路径）
        self.replace_vault_service(new_svc)?;

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

            // 首次同步：失败回退本地（P044-6 抽取），成功走收尾（进度完成/重载缓存/写配置/调度兜底）
            if let Err(e) = self.init_saf_sync().await {
                return self.rollback_after_saf_sync_failure(&data_dir, &e);
            }
            self.after_saf_sync_success(&data_dir, saf_uri.as_deref())?;
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
    /// 热替换 VaultService 后重应用「同步设置偏好」开关：新实例默认 true，
    /// 若不重应用，用户关闭的偏好同步会在切换目录后静默重置为默认开启。
    fn replace_vault_service(
        &self,
        new_svc: solosoul_core::vault_service::VaultService,
    ) -> Result<(), String> {
        let mut guard = self
            .vault_service
            .write()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let ui_prefs_sync = guard.ui_prefs_sync_enabled();
        *guard = new_svc;
        guard.set_ui_prefs_sync_enabled(ui_prefs_sync);
        Ok(())
    }

    /// SAF 首次同步成功收尾：进度完成事件、重载账户缓存、写 .solosoul_config、
    /// 调度 WorkManager 兜底同步。
    fn after_saf_sync_success(
        &self,
        data_dir: &std::path::Path,
        saf_uri: Option<&str>,
    ) -> Result<(), String> {
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
        if let Some(uri) = saf_uri {
            let temp_dir = data_dir.join("saf_vault_temp");
            let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(self.handle.clone()));
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
        Ok(())
    }

    /// SAF 首次同步失败：清除提前写入的 URI、回退本地 vault（保留「失败不保存」语义）、
    /// 取消 WorkManager 兜底同步，返回「首次同步失败」错误。
    fn rollback_after_saf_sync_failure(
        &self,
        data_dir: &std::path::Path,
        err: &str,
    ) -> Result<InitializeVaultResult, String> {
        // 同步失败：回退到本地 vault，避免留下半初始化的 SAF 状态。
        // 同时不保存 SAF URI，下次启动仍走本地/占位路径。
        tracing::warn!("[initialize_vault] SAF initial sync failed, rolling back to local: {err}");
        // 清除本次提前写入的 URI，保持「失败不保存」的既有语义。
        let _ = Self::save_saf_uri(data_dir, None);
        let local_svc = Self::try_init_local_vault(data_dir)
            .map_err(|e| format!("回退到本地 Vault 失败: {e}"))?;
        self.replace_vault_service(local_svc)?;
        // 取消 WorkManager 兜底同步：首次同步失败说明 SAF 不可用，
        // 避免旧配置持续触发无效同步。
        if let Err(e) = self.cancel_saf_fallback_sync() {
            tracing::warn!("[initialize_vault] failed to cancel SAF fallback sync: {e}");
        }
        Err(format!("首次同步失败: {err}"))
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
