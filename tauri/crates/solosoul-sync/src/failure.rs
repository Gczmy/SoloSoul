//! RF319：实际连接错误保留 IO 类型；会话旧机器前缀集中适配，cause 不进 Debug。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncFailureKind {
    NotEnabled,
    NotRunning,
    InvalidAddress,
    PeerNotFound,
    ConnectFailed,
    ConnectTimeout,
    ConnectRefused,
    HandshakeFailed,
    PairingPending,
    PairingInvalid,
    SessionFailed,
    TaskUnconfirmed,
    VaultBusy,
    VaultLocked,
    SessionExpired,
}
pub struct SyncFailure {
    kind: SyncFailureKind,
    legacy: String,
    peer_id: Option<String>,
    sas: Option<String>,
}
impl SyncFailure {
    pub fn new(kind: SyncFailureKind, cause: impl ToString) -> Self {
        Self {
            kind,
            legacy: cause.to_string(),
            peer_id: None,
            sas: None,
        }
    }
    pub fn kind(&self) -> SyncFailureKind {
        self.kind
    }
    pub fn pairing(&self) -> Option<(&str, Option<&str>)> {
        self.peer_id
            .as_deref()
            .map(|peer| (peer, self.sas.as_deref()))
    }
    /// 原 CLI/调用者继续得到原 String；新 Host 只读取 kind/pairing。
    pub fn into_legacy(self) -> String {
        self.legacy
    }
    pub fn connection(error: std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::TimedOut => SyncFailureKind::ConnectTimeout,
            std::io::ErrorKind::ConnectionRefused => SyncFailureKind::ConnectRefused,
            _ => SyncFailureKind::ConnectFailed,
        };
        Self::new(kind, format!("__SYNC_ERR__:connect_failed:{error}"))
    }
    pub fn activity(cause: String) -> Self {
        let kind = match cause.as_str() {
            "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE" => SyncFailureKind::VaultBusy,
            _ => SyncFailureKind::SessionFailed,
        };
        Self::new(kind, cause)
    }
    /// 旧 session 唯一机器前缀兼容入口，禁止按自由英文正文推断新类别。
    pub fn from_legacy_session(cause: String) -> Self {
        let legacy = crate::session::wrap_session_error(cause);
        let rest = legacy.strip_prefix("__SYNC_ERR__:").unwrap_or("");
        let (token, detail) = rest.split_once(':').unwrap_or((rest, ""));
        let kind = match token {
            "not_enabled" => SyncFailureKind::NotEnabled,
            "not_running" => SyncFailureKind::NotRunning,
            "invalid_address" => SyncFailureKind::InvalidAddress,
            "peer_not_found" => SyncFailureKind::PeerNotFound,
            "connect_failed" => SyncFailureKind::ConnectFailed,
            "handshake_failed" => SyncFailureKind::HandshakeFailed,
            "session_failed" => SyncFailureKind::SessionFailed,
            "pairing_pending" => SyncFailureKind::PairingPending,
            _ => SyncFailureKind::SessionFailed,
        };
        if kind != SyncFailureKind::PairingPending {
            return Self::new(kind, legacy);
        }
        let pieces: Vec<_> = detail.split(':').collect();
        let valid_peer = pieces.first().is_some_and(|p| {
            !p.is_empty()
                && p.len() <= 128
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        });
        let valid_sas = pieces.len() == 1
            || (pieces.len() == 2
                && pieces[1].len() == 6
                && pieces[1].chars().all(|c| c.is_ascii_digit()));
        if !valid_peer || !valid_sas {
            return Self::new(SyncFailureKind::PairingInvalid, legacy);
        }
        let peer = pieces[0].to_string();
        let sas = pieces.get(1).map(|s| s.to_string());
        Self {
            kind,
            legacy,
            peer_id: Some(peer),
            sas,
        }
    }
}
impl std::fmt::Debug for SyncFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncFailure")
            .field("kind", &self.kind)
            .finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rf319_io_kind_classification_does_not_inspect_private_body() {
        for (io, kind) in [
            (
                std::io::ErrorKind::TimedOut,
                SyncFailureKind::ConnectTimeout,
            ),
            (
                std::io::ErrorKind::ConnectionRefused,
                SyncFailureKind::ConnectRefused,
            ),
            (
                std::io::ErrorKind::PermissionDenied,
                SyncFailureKind::ConnectFailed,
            ),
        ] {
            let f = SyncFailure::connection(std::io::Error::new(io, "RF319_PRIVATE_BODY"));
            assert_eq!(f.kind(), kind);
            assert!(!format!("{f:?}").contains("RF319_PRIVATE"));
            assert!(f.into_legacy().ends_with("RF319_PRIVATE_BODY"));
        }
    }
    #[test]
    fn rf319_local_pairing_payload_preserves_old_sas_and_validates_metadata() {
        for (raw, kind, peer, sas) in [
            (
                "__SYNC_ERR__:pairing_pending:node-B",
                SyncFailureKind::PairingPending,
                Some("node-B"),
                None,
            ),
            (
                "__SYNC_ERR__:pairing_pending:node-B:482913",
                SyncFailureKind::PairingPending,
                Some("node-B"),
                Some("482913"),
            ),
            (
                "__SYNC_ERR__:pairing_pending:node-B:RF319_PRIVATE",
                SyncFailureKind::PairingInvalid,
                None,
                None,
            ),
            (
                "__SYNC_ERR__:pairing_pending:../private:482913",
                SyncFailureKind::PairingInvalid,
                None,
                None,
            ),
        ] {
            let f = SyncFailure::from_legacy_session(raw.into());
            assert_eq!(f.kind(), kind);
            assert_eq!(f.pairing().map(|p| p.0), peer);
            assert_eq!(f.pairing().and_then(|p| p.1), sas);
            assert!(!format!("{f:?}").contains("482913"));
            assert_eq!(f.into_legacy(), raw);
        }
    }
}
