pub mod attachment;
pub mod auth;
pub mod backup;
pub mod biometric;
pub mod cloud_targets;
pub mod discovery;
pub mod embed_model;
pub mod error;
pub mod export_import;
pub mod fs;
pub mod llm;
pub mod log;
pub mod object;
pub mod ocr;
pub mod pin;
pub mod plugin;
pub mod profile;
pub mod recovery;
pub mod search;
pub mod settings;
pub mod sync;
pub mod system;
pub mod template;
pub mod update;
mod update_download;
mod update_preferences;
mod update_sources;
pub mod vault;
pub mod vault_directory;
pub mod window;

use crate::state::AppState;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use solosoul_core::import_activity::{begin_owned_root_activity, RootActivityGuard};
use solosoul_core::VaultService;
use solosoul_vault::VaultStore;

/// P003: 审计日志 best-effort 封装——替代裸 `let _ = vault.log_structured(...)`
/// 吞错。审计轨迹是零知识应用的核心承诺，写入失败时 `tracing::warn!`（脱敏：
/// 仅记录动作/实体标识，不记录 details 内容）落日志，保留可观测信号。
pub fn log_audit_best_effort(
    vault: &solosoul_vault::VaultStore,
    action_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    entity_name: Option<&str>,
    performed_by: &str,
    details: Option<&str>,
) {
    if let Err(e) = vault.log_structured(
        action_type,
        entity_type,
        entity_id,
        entity_name,
        performed_by,
        details,
    ) {
        tracing::warn!(
            "Audit log write failed (action={}, entity_type={}, entity_id={:?}): {}",
            action_type,
            entity_type,
            entity_id,
            e
        );
    }
}

/// P003: 编辑快照 best-effort 封装——替代裸 `let _ = vault.save_snapshot(...)`
/// 吞错。回滚快照缺失会让历史视图静默缺漏，失败时 warn 落日志。
pub fn save_snapshot_best_effort(
    vault: &solosoul_vault::VaultStore,
    object_id: &str,
    triggered_by: &str,
    data: &[u8],
    diff_summary: &str,
) {
    if let Err(e) = vault.save_snapshot(object_id, triggered_by, data, diff_summary) {
        tracing::warn!(
            "Snapshot save failed (object_id={}, triggered_by={}): {}",
            object_id,
            triggered_by,
            e
        );
    }
}

/// 普通 Host 数据任务的句柄；Clone 同时保留 Store、真实 root owner 与维护准入。
/// 字段按声明顺序析构：最后一个句柄先释放 Store，再释放 activity。
#[derive(Clone)]
pub struct ActivityVaultHandle {
    store: Arc<VaultStore>,
    _activity: Arc<RootActivityGuard>,
}

impl Deref for ActivityVaultHandle {
    type Target = VaultStore;

    fn deref(&self) -> &Self::Target {
        self.store.as_ref()
    }
}

impl AsRef<VaultStore> for ActivityVaultHandle {
    fn as_ref(&self) -> &VaultStore {
        self.store.as_ref()
    }
}

impl ActivityVaultHandle {
    /// 仅供仍接收 Arc 的同步 resolver / 自行登记真实 worker 的插件边界。
    /// 调用方必须保留 capsule 到接收方完成，或到接收方获得自己的 worker activity。
    pub(crate) fn store_arc(&self) -> Arc<VaultStore> {
        Arc::clone(&self.store)
    }
}

/// 在读取 Store 前登记许可，关闭“先捕获旧密钥、维护完成后才登记”的窗口。
/// 不给普通句柄提供从原始 Arc 包装的构造入口。
pub(crate) fn vault_handle_for_service(
    service: &RwLock<VaultService>,
) -> Result<ActivityVaultHandle, String> {
    let svc = service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let activity = begin_owned_root_activity(svc.root_owner())?;
    let store = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;
    Ok(ActivityVaultHandle {
        store,
        _activity: Arc::new(activity),
    })
}

/// 获取当前已解锁 Vault 的任务句柄，避免每个命令重复准入/加锁样板。
pub fn vault_handle(state: &AppState) -> Result<ActivityVaultHandle, String> {
    vault_handle_for_service(state.vault_service.as_ref())
}

/// 获取当前已解锁账户 ID。
pub fn current_account(state: &AppState) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.get_current_account()
        .ok_or_else(|| "No account unlocked".to_string())
}

/// 可选地获取当前已解锁账户 ID（不返回错误）。
pub fn current_account_optional(state: &AppState) -> Option<String> {
    let svc = state.vault_service.read().ok()?;
    svc.get_current_account()
}

/// 移动端未支持功能的统一错误提示。
#[cfg(mobile)]
pub fn mobile_not_supported() -> Result<(), String> {
    Err("当前平台暂不支持该功能".to_string())
}

#[cfg(test)]
mod rf905_capsule_tests;
