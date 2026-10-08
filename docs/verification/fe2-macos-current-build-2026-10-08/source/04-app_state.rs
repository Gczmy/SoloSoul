use super::recovery::RecoveryState;
use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use crate::sync::auto_sync::AutoSyncManager;
use crate::sync::cloud_auto_sync::CloudAutoSyncManager;
use crate::sync::contracts::{
    SyncCompleted, SyncConflictsUpdated, SyncPairingRequest, SyncProgress,
};
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
    /// OCR 任务共用有限准入与单一执行位，实际 worker 结束后回收。
    pub ocr_jobs: Arc<crate::services::ocr_jobs::OcrJobs>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
    pub auto_sync: AutoSyncManager,
    /// 设备间自动同步调度器（前台/数据变更/定时）。
    pub device_auto_sync: DeviceAutoSyncManager,
    /// 云同步调度器（Phase 2：上行快照 + 下行检测，前台/数据变更/定时/手动）。
    pub cloud_auto_sync: CloudAutoSyncManager,
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
                    SyncPairingRequest {
                        node_id: info.node_id,
                        fingerprint: info.fingerprint,
                        addr: info.addr,
                        device_name: info.device_name,
                        sas_code: info.sas_code,
                    },
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
                        SyncConflictsUpdated {
                            count: info.conflicts,
                        },
                    );
                }
                let _ = emit_handle.emit(
                    "sync-completed",
                    SyncCompleted {
                        peer_node_id: info.peer_node_id,
                        examined: info.examined,
                        applied: info.applied,
                        skipped: info.skipped,
                        conflicts: info.conflicts,
                        outbound_records: info.outbound_records,
                    },
                );
            })));
        }

        // ── DeviceAutoSyncManager（设备间自动同步，依赖 SyncService） ──
        let device_auto_sync =
            DeviceAutoSyncManager::new(sync_service.clone(), vault_service.clone(), handle.clone());

        // 账户同步偏好在解锁后从保险库读取，启动时不再读取明文全局开关。

        (sync_service, device_auto_sync)
    }

    /// PluginManager 初始化：多级兜底（临时目录 → 当前目录），最终失败才中止启动。
    /// Android Release 构建使用 panic=abort，AppState::new 返回 Err 会导致 setup
    /// 失败直接闪退，故仅当文件系统级异常（所有目录均不可写）才返回 Err。
    #[cfg(any(feature = "native-perf", feature = "macos-ui-regression"))]
    fn init_plugin_manager(
        handle: &tauri::AppHandle,
        native_owner: Arc<solosoul_vault::root_owner::VaultRootOwner>,
    ) -> Result<Arc<PluginManager>, anyhow::Error> {
        // 隔离测量不允许失败后写入正常用户目录、共享临时目录或工作目录。
        crate::plugin::new_owned_plugin_manager(handle, native_owner)
            .map(Arc::new)
            .map_err(Into::into)
    }

    #[cfg(not(any(feature = "native-perf", feature = "macos-ui-regression")))]
    fn init_plugin_manager(
        handle: &tauri::AppHandle,
        native_owner: Arc<solosoul_vault::root_owner::VaultRootOwner>,
    ) -> Result<Arc<PluginManager>, anyhow::Error> {
        match crate::plugin::new_owned_plugin_manager(handle, native_owner.clone()) {
            Ok(pm) => return Ok(Arc::new(pm)),
            Err(e) => {
                tracing::warn!(
                    "[AppState] PluginManager 初始化失败，将以无插件模式运行: {:#}",
                    e
                );
            }
        }
        // 最终兜底：使用系统临时目录构造空插件管理器。
        // 固定目录名复用：每次兜底不再新建 <pid> 后缀目录（避免残留堆积）。
        let fallback_dir = std::env::temp_dir().join("solosoul_plugin_fallback");
        // 默认根失败后不再 raw 重试；降级根也须先取得独立 owner。
        match crate::plugin::new_owned_plugin_manager_with_dirs(
            fallback_dir.clone(),
            fallback_dir.clone(),
            native_owner.clone(),
        ) {
            Ok(pm) => return Ok(Arc::new(pm)),
            Err(final_err) => {
                tracing::error!(
                    "[AppState] PluginManager 最终兜底也失败: {:#}（继续无插件启动）",
                    final_err
                );
            }
        }
        // 极端情况（临时目录也不可写）下仍不中止启动，使用当前目录作为最后兜底。
        match crate::plugin::new_owned_plugin_manager_with_dirs(
            std::env::current_dir().unwrap_or_else(|_| fallback_dir.clone()),
            fallback_dir,
            native_owner,
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
        Self::new_with_vault_service(handle, vault_service)
    }

    pub(crate) fn new_with_vault_service(
        handle: tauri::AppHandle,
        vault_service: Arc<RwLock<VaultService>>,
    ) -> Result<Self, anyhow::Error> {
        #[cfg(feature = "native-perf")]
        {
            let expected = crate::native_perf::root()
                .map_err(anyhow::Error::msg)?
                .join("vault");
            let vault = vault_service
                .read()
                .map_err(|_| anyhow::anyhow!("native-perf Vault lock poisoned"))?;
            anyhow::ensure!(
                vault.base_path() == &expected.canonicalize()?,
                "native-perf Vault directory mismatch"
            );
        }

        // ── SyncService / 回调 / DeviceAutoSyncManager / 持久化开关恢复 ──
        let (sync_service, device_auto_sync) = Self::init_sync_components(&handle, &vault_service);
        let cloud_auto_sync = CloudAutoSyncManager::new(vault_service.clone(), handle.clone());

        // ── AutoSyncManager（在 VaultService 初始化之后启动） ──
        let auto_sync = AutoSyncManager::new_for_vault(vault_service.clone(), handle.clone());

        // ── PluginManager（初始化失败不阻止应用启动） ──
        let native_owner = vault_service
            .read()
            .map_err(|_| anyhow::anyhow!("Vault service lock poisoned"))?
            .root_owner();
        let plugin_manager = Self::init_plugin_manager(&handle, native_owner)?;

        let app_state = Self {
            handle: handle.clone(),
            vault_service,
            ocr_jobs: Arc::new(crate::services::ocr_jobs::OcrJobs::new()),
            sync_service,
            plugin_manager,
            auto_sync,
            device_auto_sync,
            cloud_auto_sync,
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

        // 首次入口不能绕过已有导入的目录准入。真实占位目录的排他准入覆盖初始化过程。
        let _initial_guard = {
            let svc = self
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned")?;
            solosoul_core::import_activity::begin_initial_import_setup(svc.base_path(), &data_dir)?
        };
        let handle = self.handle.clone();
        let new_svc = if let Some(ref uri) = saf_uri {
            Self::try_init_saf_vault(&handle, &data_dir, uri)
                .map_err(|e| format!("初始化 SAF Vault 失败: {e}"))?
        } else {
            Self::try_init_local_vault(&data_dir)
                .map_err(|e| format!("初始化本地 Vault 失败: {e}"))?
        };

        // 首次同步期间新 root 同样不接受 RF022 导入。
        let target_import_guard = Arc::new(
            solosoul_core::import_activity::begin_import_maintenance(new_svc.base_path())?,
        );
        // 热替换 VaultService 后重应用「同步设置偏好」开关（成功路径）
        self.replace_vault_service(new_svc).await?;

        // 清理占位目录，避免残留空数据。
        let placeholder_dir = data_dir.join(".uninitialized_vault");
        if placeholder_dir.exists() {
            // 尚有原句柄时保留；不能删除其他 owner 仍使用的锁文件。
            if let Ok(owner) = solosoul_vault::root_owner::VaultRootOwner::acquire(&placeholder_dir)
            {
                let _ = crate::commands::vault_directory::clear_target_dir(owner.root());
            }
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
            let _ = self
                .handle
                .emit("sync-progress", SyncProgress::counters("sync_start", 0, 1));

            // 首次同步：失败回退本地（P044-6 抽取），成功走收尾（进度完成/重载缓存/写配置/调度兜底）
            if let Err(e) = self
                .init_saf_sync_with_import_guard(Some(Arc::clone(&target_import_guard)))
                .await
            {
                return self.rollback_after_saf_sync_failure(&data_dir, &e).await;
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
    async fn replace_vault_service(
        &self,
        new_svc: solosoul_core::vault_service::VaultService,
    ) -> Result<(), String> {
        // listener 的原 root pin 和所有真实 session 完全退出后才替换。
        self.sync_service.disable_and_wait().await?;
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
            SyncProgress::counters("sync_complete", 1, 1),
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
    async fn rollback_after_saf_sync_failure(
        &self,
        data_dir: &std::path::Path,
        err: &str,
    ) -> Result<InitializeVaultResult, String> {
        // 同步失败：回退到本地 vault，避免留下半初始化的 SAF 状态。
        // 同时不保存 SAF URI，下次启动仍走本地/占位路径。
        tracing::warn!("[initialize_vault] SAF initial sync failed, rolling back to local: {err}");
        // 清除本次提前写入的 URI，保持「失败不保存」的既有语义。
        let _ = Self::save_saf_uri(data_dir, None);
        // 首次失败恢复占位状态，使原入口可再次选择；保留 SAF cache，不搬走任务文件。
        let local_svc =
            Self::placeholder_vault(data_dir).map_err(|e| format!("回退到占位 Vault 失败: {e}"))?;
        self.replace_vault_service(local_svc).await?;
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
        self.init_saf_sync_with_import_guard(None).await
    }

    async fn init_saf_sync_with_import_guard(
        &self,
        import_guard: Option<Arc<solosoul_core::import_activity::ImportMaintenanceGuard>>,
    ) -> Result<(), String> {
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

        // 在派发 worker 前冻结文件系统句柄，guard 与实际同步的 root 保持一致。
        let (fs, activity) = {
            let read_guard = svc
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let activity = if import_guard.is_none() {
                Some(solosoul_core::import_activity::begin_owned_root_activity(
                    read_guard.root_owner(),
                )?)
            } else {
                None
            };
            (read_guard.file_system(), activity)
        };
        sync_import_root_from_remote(fs, import_guard, activity).await
    }
}

pub(crate) async fn sync_import_root_from_remote(
    fs: Arc<dyn solosoul_core::VaultFileSystem>,
    import_guard: Option<Arc<solosoul_core::import_activity::ImportMaintenanceGuard>>,
    activity: Option<solosoul_core::import_activity::RootActivityGuard>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // 取消等待后仍由实际 worker 保持目录维护准入。
        let _import_guard = import_guard;
        let _activity = activity;
        fs.sync_from_remote()?;
        tracing::info!("[AppState] SAF initial sync completed");
        Ok(())
    })
    .await
    .map_err(|e| format!("SAF sync task panicked: {e}"))?
}

#[cfg(test)]
mod rf022_saf_guard_tests {
    use super::sync_import_root_from_remote;
    use solosoul_core::import_activity::{begin_import_activity, begin_import_maintenance};
    use solosoul_core::{SafSyncDriver, SafVaultFileSystem, VaultFileSystem};
    use std::path::Path;
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::oneshot;

    const WAIT_LIMIT: Duration = Duration::from_secs(10);

    // 任何提前断言/超时退出都会唤醒实际 blocking worker，避免测试永久挂起。
    struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
    impl ReleaseOnDrop {
        fn release(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.release();
        }
    }

    // 提示 driver 的同步方法已结束；随后还要观察 worker 的 guard 实际销毁。
    struct NotifyDriverEnd(Option<oneshot::Sender<()>>);
    impl Drop for NotifyDriverEnd {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    struct BlockingSafSyncDriver {
        entered: Mutex<Option<oneshot::Sender<()>>>,
        release: Mutex<mpsc::Receiver<()>>,
        finished: Mutex<Option<oneshot::Sender<()>>>,
    }
    impl SafSyncDriver for BlockingSafSyncDriver {
        fn sync_to_remote(&self, _local_dir: &Path, _tree_uri: &str) -> Result<(), String> {
            Err("unexpected upload".to_string())
        }

        fn sync_from_remote(&self, local_dir: &Path, tree_uri: &str) -> Result<(), String> {
            let _finish = NotifyDriverEnd(
                self.finished
                    .lock()
                    .map_err(|_| "finished lock poisoned")?
                    .take(),
            );
            if tree_uri != "content://rf022-controlled-saf" {
                return Err("unexpected SAF root".to_string());
            }
            if let Some(tx) = self
                .entered
                .lock()
                .map_err(|_| "entered lock poisoned")?
                .take()
            {
                let _ = tx.send(());
            }
            self.release
                .lock()
                .map_err(|_| "release lock poisoned")?
                .recv_timeout(WAIT_LIMIT)
                .map_err(|_| "controlled SAF release timed out")?;
            // awaiter 取消后真实 worker 仍会完成写入。
            std::fs::write(local_dir.join("saf-worker-completed"), b"worker finished")
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rf022_saf_worker_keeps_maintenance_after_awaiter_cancellation() {
        let root = tempfile::tempdir().unwrap();
        let (entered_tx, entered_rx) = oneshot::channel();
        let (finished_tx, finished_rx) = oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut release = ReleaseOnDrop(Some(release_tx));
        let driver = Arc::new(BlockingSafSyncDriver {
            entered: Mutex::new(Some(entered_tx)),
            release: Mutex::new(release_rx),
            finished: Mutex::new(Some(finished_tx)),
        });
        let fs: Arc<dyn VaultFileSystem> = Arc::new(SafVaultFileSystem::new(
            "content://rf022-controlled-saf".to_string(),
            root.path().to_path_buf(),
            driver,
        ));
        let outer_guard = Arc::new(begin_import_maintenance(root.path()).unwrap());
        let worker_guard = Arc::downgrade(&outer_guard);
        let awaiter = tokio::spawn(sync_import_root_from_remote(
            fs,
            Some(Arc::clone(&outer_guard)),
            None,
        ));
        tokio::time::timeout(WAIT_LIMIT, entered_rx)
            .await
            .expect("SAF worker never entered")
            .expect("SAF worker lost entered notification");
        awaiter.abort();
        let cancellation = awaiter.await;
        drop(outer_guard);

        // 先采集观察值，释放/等待实际 worker 后再断言；失败不留下堵塞 worker。
        let denial_while_worker_runs = begin_import_activity(root.path()).err();
        let still_owned_by_worker = worker_guard.upgrade().is_some();
        let wrote_before_release = root.path().join("saf-worker-completed").exists();
        release.release();
        let driver_finished = tokio::time::timeout(WAIT_LIMIT, finished_rx).await;
        let worker_ended = tokio::time::timeout(WAIT_LIMIT, async {
            while worker_guard.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await;

        assert!(cancellation.err().is_some_and(|err| err.is_cancelled()));
        assert!(driver_finished.is_ok_and(|result| result.is_ok()));
        assert!(worker_ended.is_ok(), "actual worker retained the guard");
        assert_eq!(
            std::fs::read(root.path().join("saf-worker-completed")).unwrap(),
            b"worker finished"
        );
        assert_eq!(
            denial_while_worker_runs.as_deref(),
            Some("IMPORT_DIRECTORY_BUSY")
        );
        assert!(still_owned_by_worker);
        assert!(!wrote_before_release);
        assert!(begin_import_activity(root.path()).is_ok());
    }
}
