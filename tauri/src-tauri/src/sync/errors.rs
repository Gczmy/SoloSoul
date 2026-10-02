//! RF319：Host 只暴露固定类别及明确配对字段。旧机器前缀在单一兼容入口读取。
use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use solosoul_sync::failure::{SyncFailure, SyncFailureKind as Kind};
pub(crate) fn typed(failure: SyncFailure) -> BackendError {
    let (code, stage) = match failure.kind() {
        Kind::NotEnabled => (Code::SyncNotEnabled, Stage::Task),
        Kind::NotRunning => (Code::SyncNotRunning, Stage::Task),
        Kind::InvalidAddress => (Code::SyncInvalidAddress, Stage::Validate),
        Kind::PeerNotFound => (Code::SyncPeerNotFound, Stage::Read),
        Kind::ConnectFailed => (Code::SyncConnectFailed, Stage::Connect),
        Kind::ConnectTimeout => (Code::SyncConnectTimeout, Stage::Connect),
        Kind::ConnectRefused => (Code::SyncConnectRefused, Stage::Connect),
        Kind::HandshakeFailed => (Code::SyncHandshakeFailed, Stage::Handshake),
        Kind::PairingPending => (Code::SyncPairingPending, Stage::Pairing),
        Kind::PairingInvalid => (Code::SyncPairingInvalid, Stage::Pairing),
        Kind::SessionFailed => (Code::SyncSessionFailed, Stage::Task),
        Kind::TaskUnconfirmed => (Code::SyncTaskUnconfirmed, Stage::Task),
        Kind::VaultBusy => (Code::VaultBusy, Stage::Task),
        Kind::VaultLocked => (Code::VaultLocked, Stage::Read),
        Kind::SessionExpired => (Code::SessionExpired, Stage::Read),
    };
    let pair = failure
        .pairing()
        .map(|(peer, sas)| (peer.to_owned(), sas.map(str::to_owned)));
    let error = BackendError::caused_by(code, stage, failure.into_legacy());
    match pair {
        Some((peer, sas)) => error.sync_pairing(&peer, sas.as_deref()),
        None => error,
    }
}
pub(crate) fn ensure_unlocked(state: &crate::state::AppState) -> Result<(), BackendError> {
    let service = state
        .vault_service
        .read()
        .map_err(|_| BackendError::new(Code::InternalError).at(Stage::Read))?;
    if service.get_current_account().is_none() || service.get_vault_store().is_none() {
        return Err(BackendError::new(Code::VaultLocked).at(Stage::Read));
    }
    Ok(())
}
/// 兼容旧事件的 string 字段，但只发送固定可本地化代码。
pub(crate) fn wire_code(error: &BackendError) -> String {
    serde_json::to_value(error.code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "INTERNAL_ERROR".into())
}
/// 内部 String API 的集中兼容桥：只读机器 token 和固定控制消息，不解析 IO/SQL 正文。
pub(crate) fn legacy_for(default: Code, stage: Stage, cause: String) -> BackendError {
    let fixed = match cause.as_str() {
        "SYNC_NOT_ENABLED" => Some(Code::SyncNotEnabled),
        "SYNC_NOT_RUNNING" => Some(Code::SyncNotRunning),
        "SYNC_INVALID_ADDRESS" => Some(Code::SyncInvalidAddress),
        "SYNC_PEER_NOT_FOUND" => Some(Code::SyncPeerNotFound),
        "SYNC_CONNECT_FAILED" => Some(Code::SyncConnectFailed),
        "SYNC_CONNECT_TIMEOUT" => Some(Code::SyncConnectTimeout),
        "SYNC_CONNECT_REFUSED" => Some(Code::SyncConnectRefused),
        "SYNC_HANDSHAKE_FAILED" => Some(Code::SyncHandshakeFailed),
        "SYNC_PAIRING_PENDING" => Some(Code::SyncPairingPending),
        "SYNC_PAIRING_INVALID" => Some(Code::SyncPairingInvalid),
        "SYNC_SESSION_FAILED" => Some(Code::SyncSessionFailed),
        "SYNC_TASK_UNCONFIRMED" => Some(Code::SyncTaskUnconfirmed),
        "SYNC_ENABLE_FAILED" => Some(Code::SyncEnableFailed),
        "SYNC_ENABLE_TIMEOUT" => Some(Code::SyncEnableTimeout),
        "SYNC_DISCOVERY_FAILED" => Some(Code::SyncDiscoveryFailed),
        "SYNC_DISCOVERY_TIMEOUT" => Some(Code::SyncDiscoveryTimeout),
        "SYNC_READ_FAILED" => Some(Code::SyncReadFailed),
        "SYNC_WRITE_FAILED" => Some(Code::SyncWriteFailed),
        "SYNC_CONFLICT_NOT_FOUND" => Some(Code::SyncConflictNotFound),
        "SYNC_CONFLICT_INVALID" => Some(Code::SyncConflictInvalid),
        "SYNC_CONFLICT_FAILED" => Some(Code::SyncConflictFailed),
        "SYNC_RECOVERY_INVALID" => Some(Code::SyncRecoveryInvalid),
        "SYNC_RECOVERY_FAILED" => Some(Code::SyncRecoveryFailed),
        "SYNC_PERMISSION_DENIED" => Some(Code::SyncPermissionDenied),
        "SYNC_UNSUPPORTED" => Some(Code::SyncUnsupported),

        "No account is currently unlocked"
        | "No account unlocked"
        | "No account is unlocked"
        | "Vault not unlocked"
        | "Vault is locked"
        | "Vault is not unlocked" => Some(Code::VaultLocked),
        "Request belongs to an expired session" | "Vault session is no longer current" => {
            Some(Code::SessionExpired)
        }
        "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE" => Some(Code::VaultBusy),
        "Vault service lock poisoned" | "UI preferences lock poisoned" => Some(Code::InternalError),
        "Conflict not found" => Some(Code::SyncConflictNotFound),
        _ => None,
    };
    if let Some(code) = fixed {
        let stage = match code {
            Code::SyncConnectFailed | Code::SyncConnectTimeout | Code::SyncConnectRefused => {
                Stage::Connect
            }
            Code::SyncHandshakeFailed => Stage::Handshake,
            Code::SyncPairingPending | Code::SyncPairingInvalid => Stage::Pairing,
            Code::SyncDiscoveryFailed | Code::SyncDiscoveryTimeout => Stage::Discovery,
            Code::SyncConflictNotFound | Code::SyncConflictInvalid | Code::SyncConflictFailed => {
                Stage::Conflict
            }
            Code::SyncInvalidAddress | Code::SyncRecoveryInvalid | Code::SyncPermissionDenied => {
                Stage::Validate
            }
            Code::SyncReadFailed | Code::SyncPeerNotFound => Stage::Read,
            Code::SyncWriteFailed => Stage::Write,
            _ => stage,
        };
        let mut error = BackendError::caused_by(code, stage, cause);
        // 旧事件仅带机器码时，没有身份/SAS，不能伪造确认字段。
        if code == Code::SyncPairingPending {
            error.safe_details = None;
        }
        return error;
    }
    if let Some(rest) = cause.strip_prefix("__SYNC_ERR__:") {
        let token = rest.split(':').next().unwrap_or("");
        let extra = match token {
            "enable_timeout" => Some((Code::SyncEnableTimeout, Stage::Task)),
            "discovery_timeout" => Some((Code::SyncDiscoveryTimeout, Stage::Discovery)),
            "nsd_failed" => Some((Code::SyncDiscoveryFailed, Stage::Discovery)),
            "recovery_invalid" => Some((Code::SyncRecoveryInvalid, Stage::Validate)),
            "permission_denied" => Some((Code::SyncPermissionDenied, Stage::Validate)),
            "unsupported" => Some((Code::SyncUnsupported, Stage::Task)),
            _ => None,
        };
        if let Some((code, stage)) = extra {
            return BackendError::caused_by(code, stage, cause);
        }
        return typed(SyncFailure::from_legacy_session(cause));
    }
    BackendError::caused_by(default, stage, cause)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn check(key: &str, error: BackendError) {
        let fixture: Value =
            serde_json::from_str(include_str!("contracts/rf319-fixtures.json")).unwrap();
        assert_eq!(serde_json::to_value(&error).unwrap(), fixture[key]);
        assert!(!format!("{error:?}").contains("RF319_PRIVATE"));
    }
    #[test]
    fn rf319_host_wire_fixture_tracks_actual_core_io_and_handshake_categories() {
        for (io, key) in [
            (std::io::ErrorKind::TimedOut, "timeout"),
            (std::io::ErrorKind::ConnectionRefused, "refused"),
        ] {
            check(
                key,
                typed(SyncFailure::connection(std::io::Error::new(
                    io,
                    "RF319_PRIVATE_KEY_PATH_BODY",
                ))),
            );
        }
        check(
            "handshake",
            typed(SyncFailure::from_legacy_session(
                "RF319_PRIVATE_KEY_PATH_BODY".into(),
            )),
        );
        check(
            "unconfirmed",
            typed(SyncFailure::new(
                Kind::TaskUnconfirmed,
                "RF319_PRIVATE_PANIC",
            )),
        );
    }
    #[test]
    fn rf319_host_pairing_preserves_validated_node_and_optional_sas() {
        check(
            "pairing",
            typed(SyncFailure::from_legacy_session(
                "__SYNC_ERR__:pairing_pending:node-B:482913".into(),
            )),
        );
        check(
            "legacyPairing",
            typed(SyncFailure::from_legacy_session(
                "__SYNC_ERR__:pairing_pending:node-B".into(),
            )),
        );
        check(
            "invalidPairing",
            typed(SyncFailure::from_legacy_session(
                "__SYNC_ERR__:pairing_pending:node-B:RF319_PRIVATE".into(),
            )),
        );
    }
    #[test]
    fn rf319_host_guards_and_recovery_validation_do_not_parse_private_body() {
        check(
            "locked",
            legacy_for(
                Code::SyncReadFailed,
                Stage::Read,
                "No account is currently unlocked".into(),
            ),
        );
        check(
            "conflictMissing",
            legacy_for(
                Code::SyncConflictInvalid,
                Stage::Conflict,
                "Conflict not found".into(),
            ),
        );
        check(
            "invalidRecovery",
            legacy_for(
                Code::SyncRecoveryFailed,
                Stage::Task,
                "__SYNC_ERR__:recovery_invalid:RF319_PRIVATE_PIN".into(),
            ),
        );
        let e = legacy_for(
            Code::SyncReadFailed,
            Stage::Read,
            "RF319_PRIVATE Permission denied connection refused".into(),
        );
        assert_eq!(e.code, Code::SyncReadFailed);
        let timeout = typed(SyncFailure::connection(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "RF319_PRIVATE",
        )));
        check(
            "timeout",
            legacy_for(Code::SyncTaskUnconfirmed, Stage::Task, wire_code(&timeout)),
        );
        let pair = legacy_for(
            Code::SyncTaskUnconfirmed,
            Stage::Task,
            "SYNC_PAIRING_PENDING".into(),
        );
        assert_eq!(pair.code, Code::SyncPairingPending);
        assert_eq!(pair.safe_details, None);
    }
}
