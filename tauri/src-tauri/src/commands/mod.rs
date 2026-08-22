pub mod attachment;
pub mod auth;
pub mod backup;
pub mod biometric;
pub mod cloud_targets;
pub mod discovery;
pub mod embed_model;
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
pub mod vault;
pub mod vault_directory;
pub mod window;

use crate::state::AppState;
use std::sync::Arc;

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

/// 获取当前已解锁 Vault 的句柄，避免在每个命令中重复加锁/解包样板。
pub fn vault_handle(state: &AppState) -> Result<Arc<solosoul_vault::VaultStore>, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())
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
