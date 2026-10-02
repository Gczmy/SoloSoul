//! RF320：真实类型/资源/路径、GUI错误通道与旧插件结果的兼容验证。
use super::*;
use serde_json::{json, Value};
use solosoul_plugin::PluginError;
const PRIVATE: &str = "RF320_PRIVATE_FIELD_KEY_PATH";
fn fixture() -> Value {
    serde_json::from_str(include_str!("contracts/rf320-fixtures.json")).unwrap()
}
#[test]
fn rf320_actual_core_errors_serialize_to_shared_safe_fixture() {
    let cases = [
        ("consent", PluginError::ConsentDenied),
        ("session", PluginError::SessionExpired(PRIVATE.into())),
        ("validation", PluginError::InvalidField(PRIVATE.into())),
        ("execution", PluginError::ExecutionFailed(PRIVATE.into())),
        ("unconfirmed", PluginError::TaskUnconfirmed(PRIVATE.into())),
        (
            "busy",
            PluginError::StoreError("IMPORT_OPERATIONS_ACTIVE".into()),
        ),
        ("network", PluginError::NetworkError(PRIVATE.into())),
    ];
    for (name, error) in cases {
        let wire = serde_json::to_value(errors::typed(error)).unwrap();
        assert_eq!(wire, fixture()[name]);
        assert!(!wire.to_string().contains(PRIVATE));
    }
    for cause in [
        format!("{PRIVATE} IMPORT_OPERATIONS_ACTIVE"),
        format!("{PRIVATE} PLUGIN_INSTALL_CANCELLED"),
    ] {
        assert_eq!(
            errors::typed(PluginError::StoreError(cause)).code,
            Code::PluginStoreFailed
        );
    }
    assert_eq!(
        errors::typed(PluginError::VaultLocked(PRIVATE.into())).code,
        Code::VaultLocked
    );
    // 旧CLI/SDK的Display兼容；新Host直接匹配变体。
    for error in [
        PluginError::SessionExpired(PRIVATE.into()),
        PluginError::TaskUnconfirmed(PRIVATE.into()),
        PluginError::VaultLocked(PRIVATE.into()),
    ] {
        assert_eq!(
            error.to_string(),
            PluginError::ExecutionFailed(PRIVATE.into()).to_string()
        );
    }
}
#[test]
fn rf320_legacy_control_tokens_are_exact_and_unknown_causes_never_escape() {
    for (cause, code) in [
        ("PLUGIN_INSTALL_CANCELLED", Code::PluginInstallCancelled),
        ("安装任务已启动", Code::PluginInstallAlreadyStarted),
        ("IMPORT_DIRECTORY_BUSY", Code::VaultBusy),
        ("Vault not unlocked", Code::VaultLocked),
    ] {
        assert_eq!(
            errors::legacy(Code::PluginInstallFailed, Stage::Install, cause.into()).code,
            code
        );
    }
    let error = errors::legacy(
        Code::PluginInstallFailed,
        Stage::Install,
        format!("{PRIVATE} PLUGIN_INSTALL_CANCELLED"),
    );
    assert_eq!(error.code, Code::PluginInstallFailed);
    assert!(!serde_json::to_string(&error).unwrap().contains(PRIVATE));
}
#[test]
fn rf320_gui_error_channel_drops_private_metadata_and_preserves_normal_events() {
    let mut event = PluginEvent::error_classified("synthetic", PRIVATE, "PLUGIN_CONSENT_DENIED");
    event.plugin_name = Some(PRIVATE.into());
    event.request_id = Some(PRIVATE.into());
    event.field_id = Some(PRIVATE.into());
    event.field_label = Some(PRIVATE.into());
    event.custom_type = Some(PRIVATE.into());
    event.sensitivity_level = Some(PRIVATE.into());
    let value = serde_json::to_value(errors::project_event(event)).unwrap();
    assert_eq!(value["pluginId"], "synthetic");
    assert_eq!(
        serde_json::from_str::<Value>(value["jsonData"].as_str().unwrap()).unwrap(),
        json!({"code":"PLUGIN_CONSENT_DENIED","message":"PLUGIN_CONSENT_DENIED"})
    );
    assert!(!value.to_string().contains(PRIVATE));
    for raw in [
        PRIVATE.to_string(),
        json!({"code":PRIVATE,"message":PRIVATE}).to_string(),
    ] {
        let mut event = PluginEvent::error("synthetic", PRIVATE);
        event.json_data = raw;
        let value = errors::project_event(event);
        assert_eq!(
            serde_json::from_str::<Value>(&value.json_data).unwrap()["code"],
            "PLUGIN_EXECUTION_FAILED"
        );
        assert!(!serde_json::to_string(&value).unwrap().contains(PRIVATE));
    }
    for event in [
        PluginEvent::log("info", "explicit log"),
        PluginEvent::result("{\"type\":\"text\",\"content\":\"explicit result\"}"),
        PluginEvent::completed("synthetic", 0, 12),
    ] {
        let original = serde_json::to_value(&event).unwrap();
        assert_eq!(
            serde_json::to_value(errors::project_event(event)).unwrap(),
            original
        );
    }
}
#[tokio::test]
async fn rf320_typed_resource_cancel_before_start_and_duplicate_handle() {
    let operation = Arc::new(PluginInstallOperation::new());
    Resource::close(operation.clone());
    let polled = AtomicBool::new(false);
    let error = operation
        .run_typed(async {
            polled.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap_err();
    assert!(!polled.load(Ordering::SeqCst));
    assert_eq!(serde_json::to_value(error).unwrap(), fixture()["cancelled"]);
    assert_eq!(
        operation
            .run_typed(async { Ok(()) })
            .await
            .unwrap_err()
            .code,
        Code::PluginInstallAlreadyStarted
    );
}
#[tokio::test]
async fn rf320_typed_resource_close_drops_pending_future_but_committed_install_succeeds() {
    struct Dropped<'a>(&'a AtomicBool);
    impl Drop for Dropped<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = AtomicBool::new(false);
    let operation = Arc::new(PluginInstallOperation::new());
    let (started, ready) = tokio::sync::oneshot::channel();
    let (result, ()) = tokio::join!(
        operation.run_typed(async {
            let _drop = Dropped(&dropped);
            let _ = started.send(());
            std::future::pending::<Result<(), BackendError>>().await
        }),
        async {
            ready.await.unwrap();
            Resource::close(operation.clone());
        }
    );
    assert_eq!(result.unwrap_err().code, Code::PluginInstallCancelled);
    assert!(dropped.load(Ordering::SeqCst));
    let operation = Arc::new(PluginInstallOperation::new());
    let result = operation
        .run_typed(async {
            Resource::close(operation.clone());
            Ok("installed")
        })
        .await;
    assert_eq!(result.unwrap(), "installed");
}
#[test]
fn rf320_actual_output_copy_validation_containment_and_io_errors_are_safe() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join(PRIVATE);
    std::fs::create_dir(&output).unwrap();
    let source = output.join("result.txt");
    std::fs::write(&source, "public synthetic result").unwrap();
    let dest = temp.path().join("dest");
    std::fs::create_dir(&dest).unwrap();
    let run = |path: &std::path::Path, name: &str| {
        plugin_copy_output_file(
            output.to_string_lossy().into(),
            path.to_string_lossy().into(),
            dest.to_string_lossy().into(),
            name.into(),
        )
    };
    run(&source, "copy.txt").unwrap();
    assert_eq!(
        std::fs::read(dest.join("copy.txt")).unwrap(),
        b"public synthetic result"
    );
    for name in ["", "..", "../outside", "dir\\outside"] {
        assert_eq!(
            run(&source, name).unwrap_err().code,
            Code::PluginOutputInvalid
        );
    }
    let outside = temp.path().join("outside.txt");
    std::fs::write(&outside, "synthetic").unwrap();
    let error = run(&outside, "copy.txt").unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        fixture()["outputDenied"]
    );
    let error = run(&output.join("missing.txt"), "copy.txt").unwrap_err();
    assert_eq!(error.code, Code::PluginOutputReadFailed);
    assert!(!serde_json::to_string(&error).unwrap().contains(PRIVATE));
    // 真实文件系统写拒绝：目标是已存在的目录，无需调整系统权限。
    std::fs::create_dir(dest.join("blocked")).unwrap();
    let error = run(&source, "blocked").unwrap_err();
    assert_eq!(error.code, Code::PluginOutputWriteFailed);
    assert!(!serde_json::to_string(&error).unwrap().contains(PRIVATE));
}

#[test]
fn rf320_actual_tauri_sink_serializes_only_projected_error_code() {
    use crate::plugin::PluginEventSink;
    let received = Arc::new(std::sync::Mutex::new(Vec::new()));
    let messages = received.clone();
    let channel = Channel::new(move |body| {
        messages.lock().unwrap().push(body);
        Ok(())
    });
    let sink = crate::plugin::TauriChannelSink::new(channel);
    sink.send(PluginEvent::error_classified(
        "synthetic",
        PRIVATE,
        "PLUGIN_CONSENT_DENIED",
    ))
    .unwrap();
    sink.send(PluginEvent::completed("synthetic", 0, 12))
        .unwrap();
    let wire: Vec<Value> = received
        .lock()
        .unwrap()
        .drain(..)
        .map(|body| body.deserialize().unwrap())
        .collect();
    assert_eq!(
        serde_json::from_str::<Value>(wire[0]["jsonData"].as_str().unwrap()).unwrap(),
        json!({"code":"PLUGIN_CONSENT_DENIED","message":"PLUGIN_CONSENT_DENIED"})
    );
    assert!(!wire[0].to_string().contains(PRIVATE));
    assert_eq!(wire[1]["eventType"], "completed");
}

#[test]
fn rf320_historical_audit_failures_are_sanitized_without_changing_action_schema() {
    use solosoul_plugin::manifest::{PluginAuditAction, PluginAuditEntry};
    for reason in [PRIVATE, "PLUGIN_CONSENT_DENIED", "PLUGIN_SESSION_EXPIRED"] {
        let original = PluginAuditEntry {
            timestamp: "2001-02-03T00:00:00Z".into(),
            plugin_id: "synthetic".into(),
            session_id: None,
            action: PluginAuditAction::PluginRunFailed {
                reason: reason.into(),
            },
        };
        let wire = serde_json::to_value(errors::project_audit(original)).unwrap();
        assert!(!wire.to_string().contains(PRIVATE));
        assert_eq!(wire["action"]["action"], "plugin_run_failed");
        assert_eq!(
            wire["action"]["reason"],
            if reason == PRIVATE {
                "PLUGIN_EXECUTION_FAILED"
            } else {
                reason
            }
        );
        assert!(wire["sessionId"].is_null());
    }
    let normal = PluginAuditEntry {
        timestamp: "2001-02-03T00:00:00Z".into(),
        plugin_id: "synthetic".into(),
        session_id: Some("session".into()),
        action: PluginAuditAction::ConsentDenied {
            field_id: "identity.name".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(errors::project_audit(normal.clone())).unwrap(),
        serde_json::to_value(normal).unwrap()
    );
}
