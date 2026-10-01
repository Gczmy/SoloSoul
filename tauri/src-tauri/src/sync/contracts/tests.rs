use super::*;
use serde_json::{json, Value};
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}
fn matches(key: &str, value: impl serde::Serialize) {
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        fixture()[key],
        "{key}"
    );
}
#[test]
fn rf305_status_and_conflict_wire_preserves_nullable_and_snake_case_fields() {
    let f = fixture();
    matches(
        "status",
        SyncStatus {
            is_discovering: false,
            sync_enabled: false,
            auto_sync_enabled: false,
            local_fingerprint: "fp-synthetic".into(),
            connected_peers: vec![SyncPeer {
                id: "node-synthetic".into(),
                name: "Device".into(),
                custom_name: None,
                addr: "127.0.0.1:42069".into(),
                fingerprint: "fp-synthetic".into(),
                trusted: true,
                last_seen: "now".into(),
                last_seen_ts: None,
                trusted_at: None,
                client_type: "windows".into(),
            }],
        },
    );
    let c: SyncConflictDto = serde_json::from_value(f["result"]["conflicts"][0].clone()).unwrap();
    matches(
        "result",
        SyncResult {
            summary: "synthetic".into(),
            examined: 2,
            applied: 1,
            skipped: 1,
            conflicts: vec![c],
            per_table: vec![TableResult {
                table: "objects".into(),
                examined: 2,
                applied: 1,
                skipped: 1,
            }],
        },
    );
    assert!(serde_json::from_value::<ConflictHlc>(
        json!({"wall_time_ms":1,"counter":0,"node_id":[0,1]})
    )
    .is_err());
}
#[test]
fn rf305_pairing_and_completion_preserve_identity_sas_and_both_directions() {
    matches(
        "pairing",
        SyncPairingRequest {
            node_id: "peer-synthetic".into(),
            fingerprint: "fp-synthetic".into(),
            addr: "127.0.0.1:42069".into(),
            device_name: "Device".into(),
            sas_code: "482913".into(),
        },
    );
    matches(
        "completed",
        SyncCompleted {
            peer_node_id: "peer-synthetic".into(),
            examined: 7,
            applied: 4,
            skipped: 3,
            conflicts: 2,
            outbound_records: 5,
        },
    );
    matches("conflictsUpdated", SyncConflictsUpdated { count: 2 });
    matches(
        "nsdFailed",
        SyncNsdFailed {
            error: "synthetic".into(),
        },
    );
}
#[test]
fn rf305_device_terminal_events_keep_required_null_message_and_start_peer_count() {
    matches(
        "deviceStart",
        DeviceSyncAutoStatus::Start {
            source: "foreground".into(),
            peer_count: 2,
        },
    );
    matches(
        "deviceComplete",
        DeviceSyncAutoStatus::Complete {
            source: "foreground".into(),
            message: None,
        },
    );
    matches(
        "deviceError",
        DeviceSyncAutoStatus::Error {
            source: "foreground".into(),
            message: Some("synthetic".into()),
        },
    );
}
#[test]
fn rf305_progress_cloud_and_recovery_events_preserve_real_omission_rules() {
    matches(
        "progressManual",
        SyncProgress::counters("sync_to_remote", 0, 1),
    );
    matches(
        "progressAutomatic",
        SyncProgress {
            source: Some("periodic".into()),
            silent: Some(true),
            ..SyncProgress::counters("sync_start", 0, 1)
        },
    );
    matches(
        "progressError",
        SyncProgress {
            phase: "error".into(),
            message: Some("synthetic".into()),
            source: Some("periodic".into()),
            silent: Some(true),
            ..Default::default()
        },
    );
    matches(
        "cloudStatus",
        CloudSyncStatus {
            account_id: "synthetic-account".into(),
            session_generation: 7,
            phase: "error".into(),
            source: "manual".into(),
            message: Some("synthetic".into()),
        },
    );
    matches(
        "cloudIncoming",
        CloudSyncIncoming {
            account_id: "synthetic-account".into(),
            session_generation: 7,
            files: vec!["synthetic.solosoul".into()],
            hint: "synthetic".into(),
        },
    );
    matches(
        "recoveryDownload",
        RecoveryProgress {
            phase: "download".into(),
            percent: 25,
            operation_id: None,
        },
    );
    matches(
        "recoveryImport",
        RecoveryProgress {
            phase: "import".into(),
            percent: 72,
            operation_id: Some("synthetic-op".into()),
        },
    );
    matches("safRevoked", ());
}
#[test]
fn rf305_recovery_flatten_keeps_all_import_terminal_fields_and_account_identity() {
    for key in [
        "recoveryComplete",
        "recoveryPartial",
        "recoveryNotCommitted",
    ] {
        let f = fixture();
        let outcome = serde_json::from_value(f[key].clone()).unwrap();
        matches(
            key,
            ImportResultSummary {
                outcome,
                account_id: "synthetic-account".into(),
                account_name: "Synthetic".into(),
            },
        );
        assert!(f[key].get("outcome").is_none());
        assert!(f[key].get("failureStage").is_some());
        assert!(f[key].get("errorCode").is_some());
    }
}
