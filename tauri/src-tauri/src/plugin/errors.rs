//! RF320：typed Core 错误直接分类；旧消息仅在固定控制 token 的兼容桥读取。
use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use solosoul_plugin::PluginError;
pub(crate) fn typed(error: PluginError) -> BackendError {
    let (code, stage) = match &error {
        PluginError::NotFound(..) => (Code::PluginNotFound, Stage::Read),
        PluginError::InvalidManifest(..) => (Code::PluginManifestInvalid, Stage::Validate),
        PluginError::WasmTooLarge(..) => (Code::PluginWasmTooLarge, Stage::Validate),
        PluginError::ChecksumMismatch => (Code::PluginChecksumMismatch, Stage::Validate),
        PluginError::IncompatibleVersion(..) => (Code::PluginVersionIncompatible, Stage::Validate),
        PluginError::ExecutionFailed(..) => (Code::PluginExecutionFailed, Stage::Execute),
        PluginError::ConsentDenied => (Code::PluginConsentDenied, Stage::Validate),
        PluginError::InvalidField(..) => (Code::PluginInvalidField, Stage::Validate),
        PluginError::InvalidArgument(..) => (Code::PluginInvalidArgument, Stage::Validate),
        PluginError::RateLimited => (Code::PluginRateLimited, Stage::Execute),
        PluginError::StoreError(cause)
            if matches!(
                cause.as_str(),
                "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE"
            ) =>
        {
            (Code::VaultBusy, Stage::Task)
        }
        PluginError::StoreError(..) => (Code::PluginStoreFailed, Stage::Write),
        PluginError::RegistryError(..) => (Code::PluginRegistryFailed, Stage::Read),
        PluginError::NetworkError(..) => (Code::PluginNetworkFailed, Stage::Read),
        PluginError::SessionExpired(..) => (Code::PluginSessionExpired, Stage::Execute),
        PluginError::TaskUnconfirmed(..) => (Code::PluginTaskUnconfirmed, Stage::Task),
        PluginError::VaultLocked(..) => (Code::VaultLocked, Stage::Read),
    };
    BackendError::caused_by(code, stage, error)
}
fn known_code(token: &str) -> Option<Code> {
    match token {
        "PLUGIN_CHECKSUM_MISMATCH" => Some(Code::PluginChecksumMismatch),
        "PLUGIN_CONSENT_DENIED" => Some(Code::PluginConsentDenied),
        "PLUGIN_EXECUTION_FAILED" => Some(Code::PluginExecutionFailed),
        "PLUGIN_INVALID_ARGUMENT" => Some(Code::PluginInvalidArgument),
        "PLUGIN_INVALID_FIELD" => Some(Code::PluginInvalidField),
        "PLUGIN_MANIFEST_INVALID" => Some(Code::PluginManifestInvalid),
        "PLUGIN_NETWORK_FAILED" => Some(Code::PluginNetworkFailed),
        "PLUGIN_NOT_FOUND" => Some(Code::PluginNotFound),
        "PLUGIN_RATE_LIMITED" => Some(Code::PluginRateLimited),
        "PLUGIN_REGISTRY_FAILED" => Some(Code::PluginRegistryFailed),
        "PLUGIN_SESSION_EXPIRED" => Some(Code::PluginSessionExpired),
        "PLUGIN_STORE_FAILED" => Some(Code::PluginStoreFailed),
        "PLUGIN_TASK_UNCONFIRMED" => Some(Code::PluginTaskUnconfirmed),
        "PLUGIN_VERSION_INCOMPATIBLE" => Some(Code::PluginVersionIncompatible),
        "PLUGIN_WASM_TOO_LARGE" => Some(Code::PluginWasmTooLarge),
        "PLUGIN_INSTALL_CANCELLED" => Some(Code::PluginInstallCancelled),
        "PLUGIN_INSTALL_ALREADY_STARTED" => Some(Code::PluginInstallAlreadyStarted),
        "PLUGIN_INVALID_OPERATION" => Some(Code::PluginInvalidOperation),
        "PLUGIN_OUTPUT_INVALID" => Some(Code::PluginOutputInvalid),
        "PLUGIN_OUTPUT_DENIED" => Some(Code::PluginOutputDenied),
        "PLUGIN_OUTPUT_READ_FAILED" => Some(Code::PluginOutputReadFailed),
        "PLUGIN_OUTPUT_WRITE_FAILED" => Some(Code::PluginOutputWriteFailed),
        "PLUGIN_OUTPUT_OPEN_FAILED" => Some(Code::PluginOutputOpenFailed),
        "PLUGIN_UNSUPPORTED" => Some(Code::PluginUnsupported),
        "PLUGIN_READ_FAILED" => Some(Code::PluginReadFailed),
        "PLUGIN_INSTALL_FAILED" => Some(Code::PluginInstallFailed),
        "VAULT_LOCKED" => Some(Code::VaultLocked),
        "VAULT_BUSY" => Some(Code::VaultBusy),
        "SESSION_EXPIRED" => Some(Code::SessionExpired),
        _ => None,
    }
}
pub(crate) fn legacy(default: Code, stage: Stage, cause: String) -> BackendError {
    let code = known_code(&cause)
        .or(match cause.as_str() {
            "PLUGIN_INSTALL_CANCELLED" => Some(Code::PluginInstallCancelled),
            "安装任务已启动" => Some(Code::PluginInstallAlreadyStarted),
            "Vault not unlocked"
            | "Vault 未解锁"
            | "未选择账户"
            | "No account is currently unlocked" => Some(Code::VaultLocked),
            "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE" => Some(Code::VaultBusy),
            "Vault session is no longer current" | "Request belongs to an expired session" => {
                Some(Code::SessionExpired)
            }
            _ => None,
        })
        .unwrap_or(default);
    BackendError::caused_by(code, stage, cause)
}
/// GUI 的错误通道丢弃正文和任意元数据；协议外层/正常日志结果保持。
pub(crate) fn project_event(
    mut event: solosoul_plugin::PluginEvent,
) -> solosoul_plugin::PluginEvent {
    if event.event_type != "error" {
        return event;
    }
    let parsed: serde_json::Value = serde_json::from_str(&event.json_data).unwrap_or_default();
    let code = parsed
        .get("code")
        .and_then(serde_json::Value::as_str)
        .and_then(known_code)
        .unwrap_or(Code::PluginExecutionFailed);
    let error = BackendError::new(code).at(Stage::Execute);
    let code = serde_json::to_value(error.code).unwrap_or(serde_json::Value::Null);
    event.json_data = serde_json::json!({"message":code,"code":code}).to_string();
    event.custom_type = None;
    event.request_id = None;
    event.plugin_name = None;
    event.field_id = None;
    event.field_label = None;
    event.sensitivity_level = None;
    event
}

/// 历史审计失败正文也必须在 GUI 返回边界投影；正常动作与字段授权审计保持。
pub(crate) fn project_audit(
    mut entry: solosoul_plugin::manifest::PluginAuditEntry,
) -> solosoul_plugin::manifest::PluginAuditEntry {
    if let solosoul_plugin::manifest::PluginAuditAction::PluginRunFailed { reason } =
        &mut entry.action
    {
        let code = known_code(reason).unwrap_or(Code::PluginExecutionFailed);
        *reason = serde_json::to_value(code)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "PLUGIN_EXECUTION_FAILED".into());
    }
    entry
}
