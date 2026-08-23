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
//!    `{data_dir}/cloud_sync_incoming/{device_id}/` 并 emit `cloud-sync-incoming`
//!    事件，由前端引导用户一键导入（复用既有 import 命令 + 冲突 UI）。
//!
//! 锁纪律：所有跨 await 的阶段均不持有 `vault_service` 读锁——导出走
//! `spawn_blocking`（锁在阻塞线程内获取/释放），其余仅短暂内联取数。

use futures::future::BoxFuture;
use solosoul_core::cloud_sync::{
    build_latest_index_path, build_snapshot_remote_path, CloudConnector,
};
use solosoul_core::VaultService;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::commands::export_import::{ExportRequest, ExportScope};

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
    pub fn new(
        vault_service: Arc<std::sync::RwLock<VaultService>>,
        app_handle: AppHandle,
    ) -> Self {
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
        let vault = match svc.get_vault_store() {
            Some(v) => v,
            None => {
                tracing::debug!("[CloudSync] vault locked, skipping ({})", source_str);
                return Ok(());
            }
        };
        let account_id = svc.get_current_account().unwrap_or_default();
        if account_id.is_empty() {
            return Ok(());
        }
        let cfg = match vault.get_cloud_sync_config(&account_id)? {
            Some(c) if c.enabled => c,
            _ => return Ok(()), // 未配置或未启用：静默跳过
        };
        if cfg.snapshot_password.is_empty() {
            tracing::warn!("[CloudSync] snapshot_password 未设置，跳过");
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
                vault.set_sync_node_id(&id)?;
                id
            }
        };
        CloudPreContext {
            account_id,
            config: cfg,
            base_path: svc.base_path().to_path_buf(),
            device_id,
        }
    };

    app_handle
        .emit("cloud-sync-status", serde_json::json!({ "phase": "sync_start", "source": source_str }))
        .ok();

    let connector = solosoul_core::cloud_sync::create_connector(&to_core_config(&pre.config))
        .map_err(|e| format!("创建连接器失败: {}", e))?;

    let result = run_sync_inner(&connector, &pre, vault_service, app_handle).await;

    match &result {
        Ok(()) => {
            // 更新 last_sync_at（原子读-改-写）
            if let Ok(svc) = vault_service.read() {
                if let Some(vault) = svc.get_vault_store() {
                    let mut cfg = pre.config.clone();
                    cfg.last_sync_at = Some(chrono::Utc::now());
                    let _ = vault.set_cloud_sync_config(&pre.account_id, cfg);
                }
            }
            app_handle
                .emit(
                    "cloud-sync-status",
                    serde_json::json!({ "phase": "sync_complete", "source": source_str }),
                )
                .ok();
        }
        Err(e) => {
            tracing::warn!("[CloudSync] round failed ({}): {}", source_str, e);
            app_handle
                .emit(
                    "cloud-sync-status",
                    serde_json::json!({ "phase": "error", "source": source_str, "message": e }),
                )
                .ok();
        }
    }
    result

}

/// 一轮同步的预取上下文（解锁态下一次性收集，避免跨 await 持锁）。
struct CloudPreContext {
    account_id: String,
    config: solosoul_vault::CloudSyncConfig,
    base_path: PathBuf,
    device_id: String,
}

async fn run_sync_inner(
    connector: &Arc<dyn CloudConnector>,
    pre: &CloudPreContext,
    vault_service: &Arc<std::sync::RwLock<VaultService>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let root = root_prefix(&pre.config);

    // ── 2. 上行：导出 → 上传 ──────────────────────────────────
    let hlc = format!("{}-0", chrono::Utc::now().timestamp_millis());
    let remote_path =
        build_snapshot_remote_path(&root, &pre.account_id, &pre.device_id, &hlc);

    let temp_dir = pre.base_path.join("cloud_sync_tmp");
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("创建临时目录失败: {e}"))?;
    let temp_path = temp_dir.join(format!("snapshot_{hlc}{}", SNAPSHOT_EXT));

    export_full_snapshot(vault_service, &pre.account_id, &pre.config.snapshot_password, &temp_path).await?;
    let file_size = tokio::fs::metadata(&temp_path).await.map_err(|e| e.to_string())?.len();

    let file = tokio::fs::File::open(&temp_path).await.map_err(|e| e.to_string())?;
    let mut reader = tokio::io::BufReader::new(file);
    let pinned: Pin<&mut (dyn tokio::io::AsyncRead + Send + Unpin)> = Pin::new(&mut reader);
    let (_, etag) = connector
        .upload(&remote_path, pinned, file_size)
        .await
        .map_err(|e| format!("上传快照失败: {}", e))?;

    tokio::fs::remove_file(&temp_path).await.ok();

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

    // ── 4. 保留策略清理（仅本设备目录）─────────────────────────
    apply_retention(connector.as_ref(), &pre.config, &root, &pre.account_id, &pre.device_id).await?;

    // ── 5. 下行检测：其他设备新水线 → 下载待导入 → 通知前端 ────
    detect_and_fetch_incoming(connector.as_ref(), pre, app_handle).await?;

    Ok(())
}

/// 全量导出加密快照到指定路径。
///
/// 导出为纯同步 CPU/IO 密集操作，放 `spawn_blocking` 执行并在闭包内部
/// 获取读锁（不跨 await 持锁）。口令校验（≠ 主密码）在核心函数内执行。
async fn export_full_snapshot(
    vault_service: &Arc<std::sync::RwLock<VaultService>>,
    account_id: &str,
    password: &str,
    dest: &Path,
) -> Result<(), String> {
    let vs = vault_service.clone();
    let account = account_id.to_string();
    let pw = password.to_string();
    let dest_str = dest.to_string_lossy().to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let svc = vs
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let req = ExportRequest {
            scope: ExportScope {
                selected_page_ids: vec![],
                selected_object_ids: vec![],
                selected_tags: vec![],
                include_attachments: true,
                selected_attachment_ids: vec![],
                include_preferences: true,
                include_behavioral: false,
                include_all: true,
            },
            password: pw,
            password_hint: None,
            save_path: dest_str.clone(),
        };
        crate::commands::export_import::execute_export_core(&svc, &account, &req, &dest_str)
    })
    .await
    .map_err(|e| format!("导出任务 join 失败: {e}"))?
}

/// 读取既有 latest.json（不存在视为空索引）→ 写入本设备条目 → 回传云端。
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
    use solosoul_core::cloud_sync::{LatestIndex, DeviceSnapshotMeta};

    let index_remote = build_latest_index_path(root, account_id);
    let mut index = match download_to_vec(connector, &index_remote).await {
        Ok(bytes) => serde_json::from_slice::<LatestIndex>(&bytes)
            .map_err(|e| format!("解析 latest.json 失败: {e}"))?,
        Err(solosoul_core::cloud_sync::CloudSyncError::NotFound(_)) => LatestIndex::default(),
        Err(e) => return Err(format!("拉取索引失败: {}", e)),
    };

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
    let mut cursor = std::io::Cursor::new(bytes);
    let pinned: Pin<&mut (dyn tokio::io::AsyncRead + Send + Unpin)> = Pin::new(&mut cursor);
    if let Some(parent) = remote_parent(&index_remote) {
        connector.ensure_dir(&parent).await.map_err(|e| format!("ensure_dir 索引目录失败: {}", e))?;
    }
    connector
        .upload(&index_remote, pinned, payload_len)
        .await
        .map_err(|e| format!("上传索引失败: {}", e))?;
    Ok(())
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
    let metas = connector.list(&dir).await.map_err(|e| format!("列取快照列表失败: {}", e))?;

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
                if !hit
                    && retention.monthly
                    && seen_monthly.insert(dt.format("%Y-%m").to_string())
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
        tracing::info!("[CloudSync] retention removed {} old snapshots of {}", removed, device_id);
    }
    Ok(())
}

/// 下行检测：其他设备水线新于本地已记录值 → 下载到 incoming 目录 → 通知前端。
async fn detect_and_fetch_incoming(
    connector: &dyn CloudConnector,
    pre: &CloudPreContext,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let root = root_prefix(&pre.config);
    let index_remote = build_latest_index_path(&root, &pre.account_id);
    let bytes = match download_to_vec(connector, &index_remote).await {
        Ok(b) => b,
        Err(solosoul_core::cloud_sync::CloudSyncError::NotFound(_)) => return Ok(()),
        Err(e) => return Err(format!("下行拉取索引失败: {}", e)),
    };
    let index: solosoul_core::cloud_sync::LatestIndex =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析 latest.json 失败: {e}"))?;

    let mut incoming_files: Vec<String> = Vec::new();
    for (device_id, meta) in &index.devices {
        if *device_id == pre.device_id {
            continue; // 跳过自己刚上传的
        }
        // 本地已记录的该设备水线（短暂持锁读 sys_config）
        let applied_key = format!("{}{}", APPLIED_KEY_PREFIX, device_id);
        let applied = {
            // 通过 AppHandle 取 AppState 再借 VaultStore（同步、无 await 点）
            let state = app_handle.try_state::<crate::state::AppState>();
            match state {
                Some(s) => match s.vault_service.read() {
                    Ok(svc) => match svc.get_vault_store() {
                        Some(vault) => vault.get_sys_config(&applied_key).unwrap_or(None),
                        None => None,
                    },
                    Err(_) => None,
                },
                None => None,
            }
        };
        if applied.as_deref() == Some(meta.hlc.as_str()) {
            continue; // 已应用
        }

        let incoming_dir = pre.base_path.join(INCOMING_DIR).join(device_id);
        tokio::fs::create_dir_all(&incoming_dir).await.ok();
        let dest = incoming_dir.join(format!("{}{}", meta.hlc, SNAPSHOT_EXT));
        if !dest.exists() {
            let file = tokio::fs::File::create(&dest).await.map_err(|e| e.to_string())?;
            let mut writer = tokio::io::BufWriter::new(file);
            let pinned: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)> = Pin::new(&mut writer);
            connector
                .download(&meta.remote_path, pinned)
                .await
                .map_err(|e| format!("下载快照 {} 失败: {}", meta.hlc, e))?;
        }
        incoming_files.push(dest.to_string_lossy().to_string());
    }

    if !incoming_files.is_empty() {
        tracing::info!("[CloudSync] {} incoming snapshot(s) ready for import", incoming_files.len());
        app_handle
            .emit(
                "cloud-sync-incoming",
                serde_json::json!({
                    "files": incoming_files,
                    "hint": "使用导入功能并输入云同步快照口令即可合并其他设备的数据",
                }),
            )
            .ok();
    }
    Ok(())
}

// ── 工具函数 ────────────────────────────────────────────────

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

// ── 单元测试 ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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
            Arc::new(MockAction { calls: calls.clone() }),
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
            Arc::new(MockAction { calls: calls.clone() }),
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
}
