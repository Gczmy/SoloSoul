//! 云同步自动调度器（Phase 2 · 步骤 4-5）。
//!
//! 复用 `auto_sync_core` 三态内核（与 `device_auto_sync.rs` 同构），三类触发：
//! - 应用切回前台（Foreground，立即执行）
//! - 本地数据变更（DataChange，防抖 10s）
//! - 周期轮询（Periodic；实际间隔由 `CloudSyncConfig.interval_secs` 门控，
//!   内核 tick 固定 60s，避免配置变更时重建循环）
//! - 手动（Manual，「立即同步」按钮，立即执行）
//!
//! 动作流程（每轮）：
//! 1. 仅 Vault 解锁态执行；读取 `CloudSyncConfig`（未配置/未启用 → 静默跳过）。
//! 2. **上行**：全量导出加密快照（口令 = `snapshot_password`）→ 分片上传至
//!    `{root}{account}/snapshots/{device_id}/{hlc}.solosoul` → 更新 `latest.json`
//!    索引 → 按保留策略清理本设备旧快照。
//! 3. **下行检测**：拉取 `latest.json`，发现其他设备有新水线 → 下载到
//!    `{data_dir}/cloud_sync_incoming/{account_id}/{device_id}/` 并 emit `cloud-sync-incoming`
//!    事件，由前端引导用户一键导入（复用既有 import 命令 + 冲突 UI）。
//!
//! 锁纪律：所有跨 await 的阶段均不持有 `vault_service` 读锁——导出走
//! `spawn_blocking`（锁在阻塞线程内获取/释放），其余仅短暂内联取数。

use futures::future::BoxFuture;
use solosoul_core::cloud_sync::{
    build_latest_index_path, build_snapshot_remote_path, CloudConnector,
};
use solosoul_core::{VaultService, VaultSession};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use solosoul_core::export_import::export::{
    execute_encrypted_export, EncryptedExportRequest, EncryptedExportScope,
};

/// 快照文件名后缀（与 build_snapshot_remote_path 保持一致）。
const SNAPSHOT_EXT: &str = ".solosoul";
/// 下行待导入目录名（位于 vault base 目录下）。
const INCOMING_DIR: &str = "cloud_sync_incoming";
/// 已应用远端水线的 sys_config 键前缀：`{prefix}{device_id}` = hlc 字符串。
const APPLIED_KEY_PREFIX: &str = "cloud_applied:";

// ── 调度器外壳（与 device_auto_sync 同构）───────────────────

/// 云同步触发来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudSyncSource {
    Foreground,
    DataChange,
    Periodic,
    Manual,
}

impl CloudSyncSource {
    fn as_str(&self) -> &'static str {
        match self {
            CloudSyncSource::Foreground => "foreground",
            CloudSyncSource::DataChange => "data_change",
            CloudSyncSource::Periodic => "periodic",
            CloudSyncSource::Manual => "manual",
        }
    }
}

/// 外部可触发的云同步事件（Periodic 由内核 interval 直接触发）。
pub enum CloudSyncEvent {
    Foreground,
    DataChange,
    Manual,
}

/// 调度器固定参数（用户可变项 `interval_secs` 在动作内门控）。
#[derive(Clone)]
pub struct CloudAutoSyncConfig {
    pub debounce_delay: Duration,
    /// 内核周期 tick 固定 60s；是否真正同步由动作内检查 `interval_secs` 决定。
    pub periodic_interval: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl Default for CloudAutoSyncConfig {
    fn default() -> Self {
        Self {
            debounce_delay: Duration::from_secs(10),
            periodic_interval: Duration::from_secs(60),
            max_retries: 2,
            retry_delay: Duration::from_secs(2),
        }
    }
}

/// 可注入的云同步动作，便于单元测试。
pub trait CloudSyncActionTrait: Send + Sync + 'static {
    fn run(&self, source: CloudSyncSource) -> BoxFuture<'static, Result<(), String>>;
}

/// 云同步自动调度管理器。
#[derive(Clone)]
pub struct CloudAutoSyncManager {
    tx: mpsc::Sender<CloudSyncEvent>,
}

impl CloudAutoSyncManager {
    /// 创建并启动云同步调度任务（生产入口）。
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>, app_handle: AppHandle) -> Self {
        let action = Arc::new(CloudSyncActionImpl {
            vault_service,
            app_handle,
            running: Arc::new(AtomicBool::new(false)),
        });
        Self::new_with_action(action, CloudAutoSyncConfig::default())
    }

    /// 创建并启动（可注入动作与配置，测试用）。
    pub fn new_with_action(
        action: Arc<dyn CloudSyncActionTrait>,
        config: CloudAutoSyncConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let manager = Self { tx };
        super::auto_sync_core::spawn_scheduler::<CloudSyncEvent, dyn CloudSyncActionTrait, _>(
            rx,
            action,
            super::auto_sync_core::SchedulerConfig {
                debounce_delay: config.debounce_delay,
                periodic_interval: config.periodic_interval,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            },
            || true, // 周期 tick 恒开；间隔门控在动作内做（需读运行时配置）
        );
        manager
    }

    /// 触发一次前台同步（立即执行）。
    pub fn trigger_foreground(&self) {
        let _ = self.tx.try_send(CloudSyncEvent::Foreground);
    }

    /// 触发一次数据变更同步（防抖）。
    pub fn trigger_data_change(&self) {
        let _ = self.tx.try_send(CloudSyncEvent::DataChange);
    }

    /// 手动「立即同步」（立即执行）。
    pub fn trigger_manual(&self) {
        let _ = self.tx.try_send(CloudSyncEvent::Manual);
    }
}

impl super::auto_sync_core::SchedulerEvent for CloudSyncEvent {
    type Source = CloudSyncSource;

    fn is_immediate(&self) -> bool {
        matches!(self, CloudSyncEvent::Foreground | CloudSyncEvent::Manual)
    }

    fn source(&self) -> CloudSyncSource {
        match self {
            CloudSyncEvent::Foreground => CloudSyncSource::Foreground,
            CloudSyncEvent::DataChange => CloudSyncSource::DataChange,
            CloudSyncEvent::Manual => CloudSyncSource::Manual,
        }
    }

    fn debounce_source() -> CloudSyncSource {
        CloudSyncSource::DataChange
    }

    fn periodic_source() -> CloudSyncSource {
        CloudSyncSource::Periodic
    }
}

impl super::auto_sync_core::SchedulerAction for dyn CloudSyncActionTrait {
    type Source = CloudSyncSource;

    fn run(&self, source: CloudSyncSource) -> BoxFuture<'static, Result<(), String>> {
        CloudSyncActionTrait::run(self, source)
    }
}

// ── 生产动作实现 ────────────────────────────────────────────

struct CloudSyncActionImpl {
    vault_service: Arc<std::sync::RwLock<VaultService>>,
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
}

impl CloudSyncActionTrait for CloudSyncActionImpl {
    fn run(&self, source: CloudSyncSource) -> BoxFuture<'static, Result<(), String>> {
        // 防重入：上一轮未结束时跳过本轮触发
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Box::pin(async { Ok(()) });
        }

        let vault_service = self.vault_service.clone();
        let app_handle = self.app_handle.clone();
        let running = self.running.clone();

        Box::pin(async move {
            let result = run_cloud_sync_round(&vault_service, &app_handle, source).await;
            running.store(false, Ordering::SeqCst);
            result
        })
    }
}

/// 一轮完整云同步。返回 Err 触发内核退避重试。
async fn run_cloud_sync_round(
    vault_service: &Arc<std::sync::RwLock<VaultService>>,
    app_handle: &AppHandle,
    source: CloudSyncSource,
) -> Result<(), String> {
    let source_str = source.as_str();

    // ── 1. 解锁态检查 + 读配置（短暂持锁，无跨 await）─────────────
    let pre = {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let account_id = match svc.get_current_account() {
            Some(id) => id,
            None => {
                tracing::debug!("[CloudSync] vault locked, skipping ({})", source_str);
                return Ok(());
            }
        };
        let session = svc.capture_session(&account_id)?;
        let vault = session.vault();
        let cfg = match vault.get_cloud_sync_config(&account_id)? {
            Some(c) if c.enabled => c,
            _ => return Ok(()), // 未配置或未启用：静默跳过
        };
        if cfg.snapshot_password.is_empty() {
            tracing::warn!("[CloudSync] snapshot_password 未设置，跳过");
            return Ok(());
        }
        // B-01：Wi-Fi only 门控（仅移动端有实际语义；桌面 is_on_wifi 恒 true）
        if cfg.wifi_only && !crate::network_status_plugin::is_on_wifi(app_handle) {
            tracing::debug!(
                "[CloudSync] wifi_only 开启且当前非 Wi-Fi，跳过本轮 ({})",
                source_str
            );
            return Ok(());
        }
        // 周期触发的时间间隔门控
        if source == CloudSyncSource::Periodic {
            if let Some(last) = cfg.last_sync_at {
                let elapsed = chrono::Utc::now().signed_duration_since(last);
                if elapsed.num_seconds() < cfg.interval_secs as i64 {
                    return Ok(());
                }
            }
        }
        // device_id 复用设备同步 node_id，缺失则生成并落库
        let device_id = match vault.get_sync_node_id()? {
            Some(id) => id,
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                svc.with_session(&session, |vault| vault.set_sync_node_id(&id))?;
                id
            }
        };
        CloudPreContext {
            service: vault_service.clone(),
            session,
            account_id,
            config: cfg,
            base_path: svc.base_path().to_path_buf(),
            device_id,
            emit_event: {
                let app = app_handle.clone();
                Arc::new(move |name, payload| app.emit(name, payload).map_err(|e| e.to_string()))
            },
            #[cfg(test)]
            barrier: None,
        }
    };

    pre.emit(
        "cloud-sync-status",
        serde_json::json!({ "phase": "sync_start", "source": source_str }),
    )?;

    let connector = solosoul_core::cloud_sync::create_connector(&to_core_config(&pre.config))
        .map_err(|e| format!("创建连接器失败: {}", e))?;

    let result = run_sync_inner(&connector, &pre).await;

    match &result {
        Ok(()) => {
            pre.finish(source_str)?;
        }
        Err(e) => {
            tracing::warn!("[CloudSync] round failed ({}): {}", source_str, e);
            let _ = pre.emit(
                "cloud-sync-status",
                serde_json::json!({ "phase": "error", "source": source_str, "message": e }),
            );
        }
    }
    result
}

/// 一轮同步的预取上下文（解锁态下一次性收集，避免跨 await 持锁）。
struct CloudPreContext {
    service: Arc<std::sync::RwLock<VaultService>>,
    session: VaultSession,
    account_id: String,
    config: solosoul_vault::CloudSyncConfig,
    base_path: PathBuf,
    device_id: String,
    emit_event: CloudEventSink,
    #[cfg(test)]
    barrier: Option<CloudTestBarrier>,
}

type CloudEventSink = Arc<dyn Fn(&str, serde_json::Value) -> Result<(), String> + Send + Sync>;

impl CloudPreContext {
    fn finish(&self, source: &str) -> Result<(), String> {
        // 只更新原会话当前配置的时间戳，保留网络等待期间的设置变更。
        self.with_vault(|vault| {
            if let Some(mut cfg) = vault.get_cloud_sync_config(&self.account_id)? {
                cfg.last_sync_at = Some(chrono::Utc::now());
                vault.set_cloud_sync_config(&self.account_id, cfg)?;
            }
            Ok(())
        })?;
        self.emit(
            "cloud-sync-status",
            serde_json::json!({"phase":"sync_complete", "source":source}),
        )
    }

    fn with_vault<T>(
        &self,
        action: impl FnOnce(&solosoul_vault::VaultStore) -> Result<T, String>,
    ) -> Result<T, String> {
        self.service
            .read()
            .map_err(|_| "Vault service lock poisoned")?
            .with_session(&self.session, action)
    }

    fn check(&self) -> Result<(), String> {
        self.with_vault(|_| Ok(()))
    }

    fn emit(&self, name: &str, mut payload: serde_json::Value) -> Result<(), String> {
        payload["accountId"] = self.account_id.clone().into();
        payload["sessionGeneration"] = self.session.generation().into();
        self.with_vault(|_| {
            if let Err(e) = (self.emit_event)(name, payload) {
                tracing::warn!("[CloudSync] event delivery failed: {e}");
            }
            Ok(())
        })
    }

    #[cfg(test)]
    async fn checkpoint(&self, stage: CloudTestStage) {
        if let Some(barrier) = &self.barrier {
            barrier(stage).await;
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum CloudTestStage {
    Downloaded,
    BeforeImport,
    BeforeWaterline,
}
#[cfg(test)]
type CloudTestBarrier = Arc<dyn Fn(CloudTestStage) -> BoxFuture<'static, ()> + Send + Sync>;

async fn run_sync_inner(
    connector: &Arc<dyn CloudConnector>,
    pre: &CloudPreContext,
) -> Result<(), String> {
    pre.check()?;
    let root = root_prefix(&pre.config);

    // ── 2. 上行：导出 → 上传 ──────────────────────────────────
    let hlc = format!("{}-0", chrono::Utc::now().timestamp_millis());
    let remote_path = build_snapshot_remote_path(&root, &pre.account_id, &pre.device_id, &hlc);

    let temp_dir = pre.base_path.join("cloud_sync_tmp").join(&pre.account_id);
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("创建临时目录失败: {e}"))?;
    // N-002：清扫历史残留（崩溃/上传中断留下的陈旧快照，>24h 即清理）
    sweep_stale_temp_snapshots(&temp_dir).await;
    let temp_path = temp_dir.join(format!("snapshot_{hlc}{}", SNAPSHOT_EXT));

    if let Err(error) = export_full_snapshot(pre, &temp_path).await {
        tokio::fs::remove_file(&temp_path).await.ok();
        return Err(error);
    }
    let file_size = tokio::fs::metadata(&temp_path)
        .await
        .map_err(|e| e.to_string())?
        .len();

    // N-002：无论上传成败均清理本次临时快照，避免 `?` 提前传播导致残留累积
    let upload_result = async {
        pre.check()?;
        let file = tokio::fs::File::open(&temp_path)
            .await
            .map_err(|e| e.to_string())?;
        let reader = tokio::io::BufReader::new(file);
        connector
            .upload(&remote_path, Box::pin(reader), file_size)
            .await
            .map_err(|e| format!("上传快照失败: {}", e))
    }
    .await;
    tokio::fs::remove_file(&temp_path).await.ok();
    let (_, etag) = upload_result?;
    pre.check()?;

    // ── 3. 更新 latest.json 索引 ──────────────────────────────
    update_latest_index(
        connector.as_ref(),
        &root,
        &pre.account_id,
        &pre.device_id,
        &hlc,
        &remote_path,
        file_size,
        &etag,
    )
    .await?;
    pre.check()?;

    // ── 4. 保留策略清理（仅本设备目录）─────────────────────────
    apply_retention(
        connector.as_ref(),
        &pre.config,
        &root,
        &pre.account_id,
        &pre.device_id,
    )
    .await?;
    pre.check()?;

    // ── 5. 下行检测：其他设备新水线 → 下载待导入 → 通知前端 ────
    detect_and_fetch_incoming(connector.as_ref(), pre).await?;

    Ok(())
}

/// 全量导出加密快照到指定路径。
///
/// 导出为纯同步 CPU/IO 密集操作，放 `spawn_blocking` 执行并在闭包内部
/// 获取读锁（不跨 await 持锁）。口令校验（≠ 主密码）在核心函数内执行。
async fn export_full_snapshot(pre: &CloudPreContext, dest: &Path) -> Result<(), String> {
    let vs = pre.service.clone();
    let session = pre.session.clone();
    let password = zeroize::Zeroizing::new(pre.config.snapshot_password.clone());
    let dest = dest.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        let svc = vs
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.with_session(&session, |_| Ok(()))?;
        let scope = EncryptedExportScope::full_snapshot();
        let req = EncryptedExportRequest {
            scope: &scope,
            password: &password,
            password_hint: &None,
            app_version: env!("CARGO_PKG_VERSION"),
        };
        execute_encrypted_export(&svc, &session, &req, &dest)
            .map(|_| ())
            .map_err(crate::services::encrypted_export::map_export_failure)
    })
    .await
    .map_err(|e| format!("导出任务 join 失败: {e}"))?
}

/// 读取既有 latest.json（不存在视为空索引）→ 写入本设备条目 → 条件回传云端。
///
/// B-05 并发保护：PUT 附带 If-Match（当前 ETag），其他设备并发更新时服务器返回
/// 412 → 映射为 Conflict → 重拉最新索引、合并本设备条目后重试（最多 3 次）。
/// 重试耗尽仍冲突则报错，交由内核退避重试整轮同步。
#[allow(clippy::too_many_arguments)]
async fn update_latest_index(
    connector: &dyn CloudConnector,
    root: &str,
    account_id: &str,
    device_id: &str,
    hlc: &str,
    remote_path: &str,
    size: u64,
    etag: &str,
) -> Result<(), String> {
    use solosoul_core::cloud_sync::{DeviceSnapshotMeta, LatestIndex};

    let index_remote = build_latest_index_path(root, account_id);
    if let Some(parent) = remote_parent(&index_remote) {
        connector
            .ensure_dir(&parent)
            .await
            .map_err(|e| format!("ensure_dir 索引目录失败: {}", e))?;
    }

    const MAX_MERGE_RETRIES: usize = 3;
    for attempt in 0..MAX_MERGE_RETRIES {
        // 拉取当前索引与 ETag（404 视为首次创建：无条件写）
        let (mut index, current_etag) = match download_to_vec(connector, &index_remote).await {
            Ok(bytes) => {
                let parsed: LatestIndex = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("解析 latest.json 失败: {e}"))?;
                let cur = connector
                    .head(&index_remote)
                    .await
                    .ok()
                    .and_then(|m| m.etag);
                (parsed, cur)
            }
            Err(solosoul_core::cloud_sync::CloudSyncError::NotFound(_)) => {
                (LatestIndex::default(), None)
            }
            Err(e) => return Err(format!("拉取索引失败: {}", e)),
        };

        // 若远端已有本设备条目且水线更新（异常残留），跳过覆盖避免回退
        if let Some(existing) = index.devices.get(device_id) {
            if existing.hlc.as_str() > hlc {
                tracing::info!(
                    "[CloudSync] 远端已有本设备更新水线 {} >= {}，跳过索引写入",
                    existing.hlc,
                    hlc
                );
                return Ok(());
            }
        }

        index.devices.insert(
            device_id.to_string(),
            DeviceSnapshotMeta {
                device_id: device_id.to_string(),
                device_name: None,
                hlc: hlc.to_string(),
                remote_path: remote_path.to_string(),
                size,
                etag: etag.to_string(),
                uploaded_at: chrono::Utc::now(),
            },
        );
        index.updated_at = chrono::Utc::now();

        let bytes = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
        let payload_len = bytes.len() as u64;
        let cursor = std::io::Cursor::new(bytes);

        match connector
            .upload_if_match(
                &index_remote,
                Box::pin(cursor),
                payload_len,
                current_etag.as_deref(),
            )
            .await
        {
            Ok(_) => return Ok(()),
            Err(solosoul_core::cloud_sync::CloudSyncError::Conflict(_)) => {
                tracing::info!(
                    "[CloudSync] latest.json 并发冲突（第 {}/{} 次重试）",
                    attempt + 1,
                    MAX_MERGE_RETRIES
                );
                continue;
            }
            Err(e) => return Err(format!("上传索引失败: {}", e)),
        }
    }
    Err("latest.json 并发冲突重试耗尽".to_string())
}

/// GFS 保留清理：保留最近 N 份全量 + 每日/周/月各桶最新一份，删除其余。
async fn apply_retention(
    connector: &dyn CloudConnector,
    config: &solosoul_vault::CloudSyncConfig,
    root: &str,
    account_id: &str,
    device_id: &str,
) -> Result<(), String> {
    let dir = format!(
        "{}/{}/snapshots/{}",
        root.trim_end_matches('/'),
        account_id,
        device_id
    );
    let metas = connector
        .list(&dir)
        .await
        .map_err(|e| format!("列取快照列表失败: {}", e))?;

    // 解析 (millis, path)，按时间降序
    let mut entries: Vec<(i64, String)> = metas
        .iter()
        .filter_map(|m| {
            let name = m.path.rsplit('/').next()?;
            let stem = name.strip_suffix(SNAPSHOT_EXT)?;
            let millis = stem.split('-').next()?.parse::<i64>().ok()?;
            Some((millis, m.path.clone()))
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));

    let retention = &config.retention;
    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_daily = std::collections::HashSet::<String>::new();
    let mut seen_weekly = std::collections::HashSet::<String>::new();
    let mut seen_monthly = std::collections::HashSet::<String>::new();

    for (idx, (millis, path)) in entries.iter().enumerate() {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(*millis);
        let keep_this = idx < retention.recent_full || {
            let mut hit = false;
            if let Some(dt) = dt {
                if retention.daily && seen_daily.insert(dt.format("%Y-%m-%d").to_string()) {
                    hit = true;
                }
                if !hit && retention.weekly {
                    // ISO 周：%G-W%V
                    if seen_weekly.insert(dt.format("%G-W%V").to_string()) {
                        hit = true;
                    }
                }
                if !hit && retention.monthly && seen_monthly.insert(dt.format("%Y-%m").to_string())
                {
                    hit = true;
                }
            }
            hit
        };
        if keep_this {
            keep.insert(path.clone());
        }
    }

    let mut removed = 0usize;
    for (_, path) in &entries {
        if !keep.contains(path) && connector.delete(path).await.is_ok() {
            removed += 1;
        }
    }
    if removed > 0 {
        tracing::info!(
            "[CloudSync] retention removed {} old snapshots of {}",
            removed,
            device_id
        );
    }
    Ok(())
}

/// 下行检测：其他设备水线新于本地已记录值 → 下载到 incoming 目录 → 通知前端。
async fn detect_and_fetch_incoming(
    connector: &dyn CloudConnector,
    pre: &CloudPreContext,
) -> Result<(), String> {
    pre.check()?;
    let root = root_prefix(&pre.config);
    let index_remote = build_latest_index_path(&root, &pre.account_id);
    let bytes = match download_to_vec(connector, &index_remote).await {
        Ok(b) => b,
        Err(solosoul_core::cloud_sync::CloudSyncError::NotFound(_)) => return Ok(()),
        Err(e) => return Err(format!("下行拉取索引失败: {}", e)),
    };
    let index: solosoul_core::cloud_sync::LatestIndex =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析 latest.json 失败: {e}"))?;
    pre.check()?;

    let mut incoming_files: Vec<String> = Vec::new();
    for (device_id, meta) in &index.devices {
        if *device_id == pre.device_id {
            continue; // 跳过自己刚上传的
        }
        // 本地已记录的该设备水线（短暂持锁读 sys_config）
        let applied_key = format!("{}{}", APPLIED_KEY_PREFIX, device_id);
        let applied = pre.with_vault(|vault| vault.get_sys_config(&applied_key))?;
        if applied.as_deref() == Some(meta.hlc.as_str()) {
            continue; // 已应用
        }

        // 不允许远端索引把文件写到其他账户或目录。
        validate_cloud_path_component(device_id)?;
        validate_cloud_path_component(&meta.hlc)?;
        let incoming_dir = pre
            .base_path
            .join(INCOMING_DIR)
            .join(&pre.account_id)
            .join(device_id);
        tokio::fs::create_dir_all(&incoming_dir)
            .await
            .map_err(|e| e.to_string())?;
        let dest = incoming_dir.join(format!("{}{}", meta.hlc, SNAPSHOT_EXT));
        if !dest.exists() {
            // 未下载完整的文件不发布为待导入包；BufWriter 必须显式 flush。
            let staging =
                tempfile::NamedTempFile::new_in(&incoming_dir).map_err(|e| e.to_string())?;
            let file = tokio::fs::File::create(staging.path())
                .await
                .map_err(|e| e.to_string())?;
            let mut writer = tokio::io::BufWriter::new(file);
            let pinned: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)> =
                Pin::new(&mut writer);
            connector
                .download(&meta.remote_path, pinned)
                .await
                .map_err(|e| format!("下载快照 {} 失败: {}", meta.hlc, e))?;
            writer.flush().await.map_err(|e| e.to_string())?;
            drop(writer);
            staging.persist(&dest).map_err(|e| e.to_string())?;
        }
        #[cfg(test)]
        pre.checkpoint(CloudTestStage::Downloaded).await;
        pre.check()?;
        incoming_files.push(dest.to_string_lossy().to_string());
    }

    if incoming_files.is_empty() {
        return Ok(());
    }
    tracing::info!(
        "[CloudSync] {} incoming snapshot(s) detected",
        incoming_files.len()
    );

    // B-06：auto_import 开启 → 静默自动导入（skipExisting 安全策略）并标记水线；
    // 关闭 → 维持 v1 行为（emit 事件由前端引导一键导入）。导入失败的单个文件保留
    // 在 incoming 目录，下轮继续尝试/等待手动处理。
    let mut pending_manual: Vec<String> = Vec::new();
    for file in &incoming_files {
        #[cfg(test)]
        pre.checkpoint(CloudTestStage::BeforeImport).await;
        pre.check()?;
        let imported = if pre.config.auto_import {
            auto_import_one(pre, file).await.unwrap_or_else(|e| {
                tracing::warn!("[CloudSync] 自动导入 {:?} 失败: {}", file, e);
                false
            })
        } else {
            false
        };
        pre.check()?;
        if !imported {
            pending_manual.push(file.clone());
        }
    }

    if !pending_manual.is_empty() {
        pre.emit(
            "cloud-sync-incoming",
            serde_json::json!({
                "files": pending_manual,
                "hint": "使用导入功能并输入云同步快照口令即可合并其他设备的数据",
            }),
        )?;
    }
    Ok(())
}

/// B-06：静默导入单个云端快照。成功返回 true（并记录已应用水线）。
///
/// 复用 `import_execute_resumable_for_session`（skipExisting + 全量选择），读锁在 spawn_blocking
/// 内获取；仅原会话完整导入后提交水线与清理源包。
async fn auto_import_one(pre: &CloudPreContext, file: &str) -> Result<bool, String> {
    pre.check()?;
    // 从文件路径推导 device_id / hlc：
    //   {base}/cloud_sync_incoming/{account_id}/{device_id}/{hlc}.solosoul
    let path = Path::new(file);
    let hlc = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("非法快照文件名")?
        .to_string();
    let device_id = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .ok_or("非法快照目录结构")?
        .to_string();
    validate_cloud_path_component(&device_id)?;
    validate_cloud_path_component(&hlc)?;

    let vs = pre.service.clone();
    let session = pre.session.clone();
    let pw = pre.config.snapshot_password.clone();
    let file_owned = file.to_string();
    let locale = "en-US".to_string();
    let operation_id = uuid::Uuid::new_v4().to_string();
    let activity = solosoul_core::import_activity::begin_import_activity(&pre.base_path)?;

    // 导入为同步密集操作且需持锁：spawn_blocking 内获取读锁
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _activity = activity;
        let svc = vs
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let request = solosoul_core::export_import::import::EncryptedImportRequest {
            source_path: file_owned,
            password: zeroize::Zeroizing::new(pw),
            options: solosoul_core::export_import::import::ImportOptions {
                locale,
                ..Default::default()
            },
            operation: Some((operation_id, solosoul_vault::ImportSourceKind::Cloud)),
        };
        solosoul_core::export_import::import::execute_encrypted_import(
            &svc, &session, request, None,
        )
        .map_err(crate::services::encrypted_import::map_import_failure)
        .and_then(|r| r.require_complete())
        .and_then(|r| {
            r.operation_id
                .map(|id| (r.object_count, r.attachment_count, id))
                .ok_or_else(|| "Missing import operation".into())
        })
    })
    .await
    .map_err(|e| format!("导入任务 join 失败: {e}"))?;

    let (objects, attachments, operation_id) = result.map_err(|e| format!("自动导入失败: {e}"))?;
    #[cfg(test)]
    pre.checkpoint(CloudTestStage::BeforeWaterline).await;
    tracing::info!(
        "[CloudSync] auto-imported snapshot from {}: {} objects, {} attachments",
        device_id,
        objects,
        attachments
    );

    let svc = pre
        .service
        .read()
        .map_err(|_| "Vault service lock poisoned")?;
    finalize_cloud_import(&svc, &pre.session, path, &operation_id)?;
    Ok(true)
}

/// Native 的固定来源定位，不接受前端 device/HLC 代替实际 incoming 路径。
fn controlled_cloud_source(
    svc: &VaultService,
    session: &VaultSession,
    path: &Path,
) -> Result<(std::path::PathBuf, String, String), String> {
    solosoul_core::export_import::import::source::controlled_cloud_source(svc, session, path)
}

/// 手动和自动共用：journal Complete + 当前会话 + 实际源密文 proof 全部成立才推进水线。
/// 大包 hash 在会话门闩外；提交时重载 journal，并检查 Native 源未被替换。
pub(crate) fn finalize_cloud_import(
    svc: &VaultService,
    session: &VaultSession,
    path: &Path,
    operation_id: &str,
) -> Result<(), String> {
    use solosoul_core::export_import::operation::{import_request_fingerprint, OwnedImportPackage};
    use solosoul_vault::{ImportOperationPhase, ImportSourceKind};
    let _activity = solosoul_core::import_activity::begin_import_activity(svc.base_path())?;
    let (source, device, hlc) = controlled_cloud_source(svc, session, path)?;
    let original = std::fs::metadata(&source).map_err(|_| "Cloud snapshot source unavailable")?;
    let owned = OwnedImportPackage::capture(&source, svc.base_path())
        .map_err(|_| "Cloud snapshot source verification failed")?;
    let options = serde_json::json!({"strategy":"skipExisting","selectedObjectIds":null,"selectedAttachmentIds":null,
        "objectStrategies":{},"locale":"en-US","deviceId":device,"hlc":hlc});
    let fingerprint = import_request_fingerprint(&options)
        .map_err(|_| "Cloud snapshot source verification failed")?;
    let key = format!("{APPLIED_KEY_PREFIX}{device}");
    svc.with_session(session, |vault| {
        let operation = vault
            .load_import_operation(session.account_id(), operation_id)?
            .ok_or("Cloud import operation missing")?;
        if operation.phase != ImportOperationPhase::Complete
            || operation.start.source_kind != ImportSourceKind::Cloud
            || operation.start.source != *owned.source_proof()
            || operation.start.request_fingerprint != fingerprint
            || operation.start.root_binding != vault.import_root_binding()?
        {
            return Err("Cloud import operation is incomplete or source changed".into());
        }
        let current =
            std::fs::metadata(&source).map_err(|_| "Cloud snapshot source unavailable")?;
        if original.len() != current.len() || original.modified().ok() != current.modified().ok() {
            return Err("Cloud snapshot source changed".into());
        }
        vault.set_sys_config(&key, &hlc)?;
        if let Err(error) = std::fs::remove_file(&source) {
            tracing::warn!("[CloudSync] imported source cleanup failed: {error}");
        }
        Ok(())
    })
}

// ── 工具函数 ────────────────────────────────────────────────

fn validate_cloud_path_component(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid cloud snapshot path component".into());
    }
    Ok(())
}

fn root_prefix(config: &solosoul_vault::CloudSyncConfig) -> String {
    config
        .config_json
        .get("rootPrefix")
        .and_then(|v| v.as_str())
        .unwrap_or("/SoloSoul/")
        .to_string()
}

fn to_core_config(
    config: &solosoul_vault::CloudSyncConfig,
) -> solosoul_core::cloud_sync::CloudSyncConfig {
    solosoul_core::cloud_sync::CloudSyncConfig {
        connector_type: config.connector_type.clone(),
        config_json: config.config_json.clone(),
        enabled: config.enabled,
        interval_secs: config.interval_secs,
        wifi_only: config.wifi_only,
        retention: solosoul_core::cloud_sync::RetentionPolicy {
            recent_full: config.retention.recent_full,
            daily: config.retention.daily,
            weekly: config.retention.weekly,
            monthly: config.retention.monthly,
        },
        last_sync_at: config.last_sync_at,
    }
}

fn remote_parent(remote_path: &str) -> Option<String> {
    let trimmed = remote_path.trim_end_matches('/');
    let pos = trimmed.rfind('/')?;
    if pos == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..pos].to_string())
    }
}

/// 下载小文件（索引 JSON）到内存。404 映射为 NotFound 错误供调用方分支。
async fn download_to_vec(
    connector: &dyn CloudConnector,
    remote_path: &str,
) -> Result<Vec<u8>, solosoul_core::cloud_sync::CloudSyncError> {
    // 先 HEAD 确认存在（download 对 404 会报 DownloadFailed，无法区分）
    connector.head(remote_path).await?;
    let mut buf = Vec::new();
    {
        let mut writer = std::io::Cursor::new(&mut buf);
        let pinned: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)> = Pin::new(&mut writer);
        connector.download(remote_path, pinned).await?;
    }
    Ok(buf)
}

/// N-002：清理 cloud_sync_tmp 中超过 24h 的陈旧临时快照（best-effort）。
async fn sweep_stale_temp_snapshots(temp_dir: &Path) {
    const STALE_SECS: i64 = 24 * 3600;
    let cutoff = chrono::Utc::now().timestamp() - STALE_SECS;
    if let Ok(mut entries) = tokio::fs::read_dir(temp_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let stale = tokio::fs::metadata(&path)
                .await
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .map(|secs| secs < cutoff)
                .unwrap_or(false);
            if stale {
                tracing::info!("[CloudSync] sweeping stale temp snapshot {:?}", path);
                tokio::fs::remove_file(&path).await.ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::export_import::collect_all_attachment_ids;
    use std::sync::atomic::AtomicUsize;

    mod rf003;
    mod rf014;

    #[tokio::test]
    async fn rf015_cloud_explicit_all_exports_plain_and_solc_without_manual_ids() {
        use crate::commands::export_import::tests::rf015::{
            assert_package, paired_fixture, EXPORT_PASSWORD, OBJECT_ID, PLAIN_BYTES, PLAIN_ID,
            SOLC_BYTES, SOLC_ID,
        };
        let f = paired_fixture();
        let pre = {
            let svc = f.service.read().unwrap();
            CloudPreContext {
                service: f.service.clone(),
                session: svc.capture_session(&f.account).unwrap(),
                account_id: f.account.clone(),
                config: solosoul_vault::CloudSyncConfig {
                    snapshot_password: EXPORT_PASSWORD.into(),
                    ..Default::default()
                },
                base_path: svc.base_path().to_path_buf(),
                device_id: "rf015-cloud-device".into(),
                emit_event: Arc::new(|_, _| {
                    panic!("local RF015 export must not contact cloud transport")
                }),
                barrier: None,
            }
        };
        let path = f.dir.path().join("cloud-all.solosoul");
        export_full_snapshot(&pre, &path).await.unwrap();
        assert_package(
            &path,
            &[OBJECT_ID],
            &[
                (OBJECT_ID, PLAIN_ID, PLAIN_BYTES),
                (OBJECT_ID, SOLC_ID, SOLC_BYTES),
            ],
        );
    }

    #[tokio::test]
    async fn rf020_cloud_partial_preserves_source_and_waterline() {
        use crate::commands::export_import::tests::rf020::{objects, package, Fixture};
        for partial in [true, false] {
            let f = Fixture::new();
            if partial {
                f.reject_nth_object_write(2);
            }
            let incoming = f
                .service
                .read()
                .unwrap()
                .base_path()
                .join("cloud_sync_incoming")
                .join(&f.account)
                .join("remote-device");
            std::fs::create_dir_all(&incoming).unwrap();
            let path = package(&incoming, objects(), false, false, false);
            let applied_key = format!("{APPLIED_KEY_PREFIX}remote-device");
            f.vault.set_sys_config(&applied_key, "previous").unwrap();
            let pre = CloudPreContext {
                service: f.service.clone(),
                session: f
                    .service
                    .read()
                    .unwrap()
                    .capture_session(&f.account)
                    .unwrap(),
                account_id: f.account.clone(),
                config: solosoul_vault::CloudSyncConfig {
                    snapshot_password: "export-password".into(),
                    ..Default::default()
                },
                base_path: f.dir.path().to_path_buf(),
                device_id: "local-device".into(),
                emit_event: Arc::new(|_, _| Ok(())),
                barrier: None,
            };
            let result = auto_import_one(&pre, path.to_str().unwrap()).await;
            assert_eq!(result.is_err(), partial);
            assert_eq!(path.exists(), partial);
            assert_eq!(
                f.vault.get_sys_config(&applied_key).unwrap().as_deref(),
                Some(if partial { "previous" } else { "incoming" })
            );
        }
    }

    struct MockAction {
        calls: Arc<AtomicUsize>,
    }

    impl CloudSyncActionTrait for MockAction {
        fn run(&self, _source: CloudSyncSource) -> BoxFuture<'static, Result<(), String>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
    }

    fn test_config() -> CloudAutoSyncConfig {
        CloudAutoSyncConfig {
            debounce_delay: Duration::from_millis(50),
            periodic_interval: Duration::from_secs(3600),
            max_retries: 0,
            retry_delay: Duration::from_millis(1),
        }
    }

    #[tokio::test]
    async fn test_manual_triggers_immediately() {
        let calls = Arc::new(AtomicUsize::new(0));
        let manager = CloudAutoSyncManager::new_with_action(
            Arc::new(MockAction {
                calls: calls.clone(),
            }),
            test_config(),
        );
        manager.trigger_manual();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_data_change_debounce() {
        let calls = Arc::new(AtomicUsize::new(0));
        let manager = CloudAutoSyncManager::new_with_action(
            Arc::new(MockAction {
                calls: calls.clone(),
            }),
            test_config(),
        );
        for _ in 0..5 {
            manager.trigger_data_change();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_remote_parent() {
        assert_eq!(
            remote_parent("/SoloSoul/acc/snapshots/dev/123.solosoul"),
            Some("/SoloSoul/acc/snapshots/dev".to_string())
        );
        assert_eq!(remote_parent("/file.solosoul"), Some("/".to_string()));
        assert_eq!(remote_parent("nofile"), None);
    }

    #[test]
    fn test_root_prefix_fallback() {
        let mut config = solosoul_vault::CloudSyncConfig::default();
        assert_eq!(root_prefix(&config), "/SoloSoul/");
        config.config_json = serde_json::json!({ "rootPrefix": "/MyVault/" });
        assert_eq!(root_prefix(&config), "/MyVault/");
    }

    #[tokio::test]
    async fn test_sweep_stale_temp_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        // 陈旧文件：mtime 设为 2 天前
        let stale_path = dir.path().join("snapshot_old.solosoul");
        std::fs::write(&stale_path, b"x").unwrap();
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(48 * 3600);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&stale_path)
            .unwrap()
            .set_modified(old_time)
            .unwrap();
        // 新鲜文件
        let fresh_path = dir.path().join("snapshot_new.solosoul");
        std::fs::write(&fresh_path, b"y").unwrap();

        sweep_stale_temp_snapshots(dir.path()).await;

        assert!(!stale_path.exists(), "陈旧残留应被清理");
        assert!(fresh_path.exists(), "新鲜文件不应被清理");
    }
}
