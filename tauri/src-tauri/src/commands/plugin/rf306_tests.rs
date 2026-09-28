//! RF306：使用共享插件真实 serde 类型与 Tauri 资源表，不安装插件或访问用户目录。

use super::{emit_install_progress, PluginInstallOperation};
use serde_json::{json, Value};
use solosoul_plugin::event::PluginEvent;
use solosoul_plugin::install_progress::{PluginInstallPhase, PluginInstallProgress};
use solosoul_plugin::manifest::{
    MarketPluginInfo, PluginAuditAction, PluginAuditEntry, PluginInstallResult, PluginLogLine,
    PluginManifest, PluginResult, PluginResultPayload, PluginTier, RegistryEntry,
};
use solosoul_plugin::session::PluginSession;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{ipc::Channel, Resource, ResourceTable};

const PLUGIN_ID: &str = "com.solosoul.synthetic.rf306";
const HASH: &str = "1111111111111111111111111111111111111111111111111111111111111111";

#[test]
fn rf306_minimal_legacy_manifest_preserves_defaults_null_and_omission() {
    let input = json!({
        "id": PLUGIN_ID,
        "name": "合成插件",
        "version": "1.0.0",
        "description": "Synthetic fixture only"
    });
    let manifest: PluginManifest = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&manifest).unwrap(),
        json!({
            "id": PLUGIN_ID,
            "name": "合成插件",
            "version": "1.0.0",
            "description": "Synthetic fixture only",
            "author": null,
            "homepage": null,
            "permissions": [],
            "requiredCoreVersion": null,
            "wasmHashSha256": null,
            "dataTtlSeconds": 300,
            "networkPolicy": {"blockAllOutbound": false, "allowedDomains": []},
            "requireUserConfirmation": false,
            "tier": "p3",
            "category": "",
            "params": [],
            "contracts": [],
            "fieldBindings": []
        })
    );

    // 整体字段缺失走派生 Default；显式空对象才执行字段的 serde default 函数。
    let mut explicit_policy = input;
    explicit_policy["networkPolicy"] = json!({});
    let manifest: PluginManifest = serde_json::from_value(explicit_policy).unwrap();
    assert_eq!(
        serde_json::to_value(manifest).unwrap()["networkPolicy"],
        json!({"blockAllOutbound": true, "allowedDomains": []})
    );
}

#[test]
fn rf306_full_manifest_keeps_fields_and_normalizes_legacy_localized_labels() {
    for (display_name, label, expected_display, expected_label) in [
        (
            json!("Display"),
            json!("Field"),
            Some("Display"),
            Some("Field"),
        ),
        (
            json!({"zh": "合约", "en": "Contract"}),
            json!({"en": "English field"}),
            Some("合约"),
            Some("English field"),
        ),
        (Value::Null, Value::Null, None, None),
    ] {
        let input = json!({
            "id": PLUGIN_ID,
            "name": "Synthetic full manifest",
            "version": "2.3.4",
            "description": "description",
            "author": "synthetic author",
            "homepage": "https://example.invalid/rf306",
            "permissions": ["identity.name"],
            "requiredCoreVersion": "2.0.0",
            "wasmHashSha256": HASH,
            "dataTtlSeconds": 90,
            "networkPolicy": {"blockAllOutbound": true, "allowedDomains": []},
            "requireUserConfirmation": true,
            "tier": "p1",
            "category": "synthetic",
            "params": [{
                "id": "choice", "label": "选择", "type": "select", "required": true,
                "description": "description", "defaultValue": "one",
                "options": [{"value": "one", "label": "一"}]
            }],
            "contracts": [{
                "typeId": "synthetic/v1", "version": 7, "displayName": display_name,
                "strictContractGate": true, "typeIdAliases": ["synthetic"],
                "roles": [{"roleId": "name", "label": label, "required": true,
                    "defaultPropertyId": "name"}]
            }],
            "fieldBindings": [{"contractTypeId": "synthetic/v1", "propertyId": "name",
                "abiName": "synthetic.name", "sensitivity": "sensitive"}],
            "i18n": {"zh-CN": {"name": "合成", "description": "说明"}},
            "customUi": "synthetic-panel"
        });
        let mut expected = input.clone();
        let contract = expected["contracts"][0].as_object_mut().unwrap();
        match expected_display {
            Some(value) => {
                contract.insert("displayName".into(), json!(value));
            }
            None => {
                contract.remove("displayName");
            }
        }
        let role = expected["contracts"][0]["roles"][0]
            .as_object_mut()
            .unwrap();
        match expected_label {
            Some(value) => {
                role.insert("label".into(), json!(value));
            }
            None => {
                role.remove("label");
            }
        }
        let manifest: PluginManifest = serde_json::from_value(input).unwrap();
        assert_eq!(serde_json::to_value(&manifest).unwrap(), expected);
        let roundtrip: PluginManifest = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(serde_json::to_value(roundtrip).unwrap(), expected);
    }

    let defaults: PluginManifest = serde_json::from_value(json!({
        "id": PLUGIN_ID, "name": "defaulted", "version": "1", "description": "",
        "params": [{"id": "text", "label": "Text", "type": "string"}],
        "contracts": [{"typeId": "synthetic/v1", "roles": [{"roleId": "name"}]}],
        "fieldBindings": [{"contractTypeId": "synthetic/v1", "propertyId": "name"}]
    }))
    .unwrap();
    let wire = serde_json::to_value(defaults).unwrap();
    assert_eq!(
        wire["params"][0],
        json!({"id": "text", "label": "Text", "type": "string",
        "required": false, "description": "", "defaultValue": null, "options": []})
    );
    assert_eq!(
        wire["contracts"][0],
        json!({"typeId": "synthetic/v1", "version": 1,
        "strictContractGate": false, "typeIdAliases": [],
        "roles": [{"roleId": "name", "required": false}]})
    );
    assert_eq!(
        wire["fieldBindings"][0],
        json!({"contractTypeId": "synthetic/v1", "propertyId": "name"})
    );
}

#[test]
fn rf306_registry_aliases_keep_version_map_and_market_nullable_fields() {
    let entry: RegistryEntry = serde_json::from_value(json!({
        "name": "Synthetic registry", "publisher": "publisher", "latest_version": "1.2.3",
        "versions": {
            "1.2.3": {"sha256": HASH, "plugin_api_version": "1", "min_app_version": "2.0.0",
                "max_app_version": "9.0.0", "download_url": "https://example.invalid/file",
                "raw_url": "https://example.invalid/raw", "released_at": "2001-02-03",
                "changelog": "synthetic"},
            "1.0.0": {"sha256": HASH, "min_app_version": "1.0.0", "max_app_version": "9.0.0"}
        },
        "field_bindings": [{"contractTypeId": "synthetic/v1", "propertyId": "name"}],
        "custom_ui": "synthetic-panel"
    }))
    .unwrap();
    let expected_entry = json!({
        "name": "Synthetic registry", "author": "publisher", "latestVersion": "1.2.3",
        "versions": {
            "1.2.3": {"sha256": HASH, "pluginApiVersion": "1", "minAppVersion": "2.0.0",
                "maxAppVersion": "9.0.0", "downloadUrl": "https://example.invalid/file",
                "rawUrl": "https://example.invalid/raw", "releasedAt": "2001-02-03",
                "changelog": "synthetic"},
            "1.0.0": {"sha256": HASH, "pluginApiVersion": null, "minAppVersion": "1.0.0",
                "maxAppVersion": "9.0.0", "downloadUrl": null, "rawUrl": null,
                "releasedAt": null, "changelog": null}
        },
        "description": "", "homepage": null, "i18n": null, "tier": "p3", "category": "",
        "params": [], "contracts": [],
        "fieldBindings": [{"contractTypeId": "synthetic/v1", "propertyId": "name"}],
        "customUi": "synthetic-panel"
    });
    assert_eq!(serde_json::to_value(&entry).unwrap(), expected_entry);
    let info = MarketPluginInfo {
        plugin_id: PLUGIN_ID.into(),
        installed_version: None,
        has_update: false,
        is_compatible: true,
        tier: PluginTier::P3,
        category: "synthetic".into(),
        registry_entry: entry,
    };
    assert_eq!(
        serde_json::to_value(info).unwrap(),
        json!({
            "pluginId": PLUGIN_ID, "installedVersion": null, "hasUpdate": false,
            "isCompatible": true, "tier": "p3", "category": "synthetic", "registryEntry": expected_entry
        })
    );
    let minimal: RegistryEntry =
        serde_json::from_value(json!({"name": "minimal", "versions": {}})).unwrap();
    let wire = serde_json::to_value(minimal).unwrap();
    assert_eq!(wire.get("latestVersion"), Some(&Value::Null));
    assert_eq!(wire.get("author"), Some(&Value::Null));
    assert!(!wire.as_object().unwrap().contains_key("customUi"));
}

#[test]
fn rf306_session_and_install_use_real_identifiers_and_integer_timestamps() {
    let session = PluginSession {
        id: "synthetic-session".into(),
        plugin_id: PLUGIN_ID.into(),
        created_at: 1_700_000_000_123,
        expires_at: 1_700_000_300_123,
    };
    let expected = json!({"sessionId": "synthetic-session", "pluginId": PLUGIN_ID,
        "createdAt": 1_700_000_000_123_i64, "expiresAt": 1_700_000_300_123_i64});
    assert_eq!(serde_json::to_value(&session).unwrap(), expected);
    let decoded: PluginSession = serde_json::from_value(expected).unwrap();
    assert_eq!(decoded.id, session.id);
    assert_eq!(decoded.created_at, session.created_at);
    assert_eq!(decoded.expires_at, session.expires_at);
    let installed = PluginInstallResult {
        plugin_id: PLUGIN_ID.into(),
        version: "1.2.3".into(),
        installed_at: 1_700_000_000_456,
    };
    assert_eq!(
        serde_json::to_value(installed).unwrap(),
        json!({
            "pluginId": PLUGIN_ID, "version": "1.2.3", "installedAt": 1_700_000_000_456_i64
        })
    );
}

#[test]
fn rf306_audit_variants_keep_snake_case_payload_inside_camel_case_entry() {
    for (action, expected) in [
        (
            PluginAuditAction::PluginInstalled {
                version: "1.2.3".into(),
            },
            json!({"action": "plugin_installed", "version": "1.2.3"}),
        ),
        (
            PluginAuditAction::PluginUninstalled,
            json!({"action": "plugin_uninstalled"}),
        ),
        (
            PluginAuditAction::PluginRunStarted,
            json!({"action": "plugin_run_started"}),
        ),
        (
            PluginAuditAction::PluginRunCompleted { exit_code: -7 },
            json!({"action": "plugin_run_completed", "exit_code": -7}),
        ),
        (
            PluginAuditAction::PluginRunFailed {
                reason: "synthetic denied".into(),
            },
            json!({"action": "plugin_run_failed", "reason": "synthetic denied"}),
        ),
        (
            PluginAuditAction::ConsentApproved {
                field_id: "identity.name".into(),
            },
            json!({"action": "consent_approved", "field_id": "identity.name"}),
        ),
        (
            PluginAuditAction::ConsentDenied {
                field_id: "identity.name".into(),
            },
            json!({"action": "consent_denied", "field_id": "identity.name"}),
        ),
    ] {
        let entry = PluginAuditEntry {
            timestamp: "2001-02-03T04:05:06Z".into(),
            plugin_id: PLUGIN_ID.into(),
            session_id: None,
            action,
        };
        let wire = serde_json::to_value(entry).unwrap();
        assert_eq!(
            wire,
            json!({"timestamp": "2001-02-03T04:05:06Z", "pluginId": PLUGIN_ID,
            "sessionId": null, "action": expected})
        );
        let decoded: PluginAuditEntry = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
    }
}

#[test]
fn rf306_event_constructors_preserve_string_payload_and_explicit_null_keys() {
    let log = PluginEvent::log("notice", "Synthetic 日志");
    let log_data: Value = serde_json::from_str(&log.json_data).unwrap();
    assert_eq!(log_data["level"], "notice");
    assert_eq!(log_data["message"], "Synthetic 日志");
    assert!(!log_data["id"].as_str().unwrap().is_empty());
    assert!(log_data["timestamp"].as_i64().is_some());
    assert_eq!(log_data.as_object().unwrap().len(), 4);
    let cases = [
        (log, "log", json!({})),
        (
            PluginEvent::result("[null, false, \"结果\"]"),
            "result",
            json!({}),
        ),
        (
            PluginEvent::dialog_request(
                "request-dialog",
                PLUGIN_ID,
                "合成",
                "{\"type\":\"confirm\"}",
            ),
            "dialog_request",
            json!({"requestId": "request-dialog", "pluginId": PLUGIN_ID, "pluginName": "合成"}),
        ),
        (
            PluginEvent::consent_request(
                "request-consent",
                PLUGIN_ID,
                "合成",
                "identity.name",
                "名称",
                "sensitive",
            ),
            "consent_request",
            json!({"requestId": "request-consent", "pluginId": PLUGIN_ID,
                "pluginName": "合成", "fieldId": "identity.name", "fieldLabel": "名称", "sensitivityLevel": "sensitive"}),
        ),
        (
            PluginEvent::completed(PLUGIN_ID, -4, 456),
            "completed",
            json!({"pluginId": PLUGIN_ID}),
        ),
        (
            PluginEvent::error(PLUGIN_ID, "synthetic denial"),
            "error",
            json!({"pluginId": PLUGIN_ID}),
        ),
        (
            PluginEvent::custom(PLUGIN_ID, "合成", "future-widget", "{\"future\":true}"),
            "custom_event",
            json!({"pluginId": PLUGIN_ID, "pluginName": "合成", "customType": "future-widget"}),
        ),
    ];
    for (event, event_type, populated) in cases {
        let mut expected = json!({
            "eventType": event_type, "jsonData": event.json_data,
            "customType": null, "requestId": null, "pluginId": null, "pluginName": null,
            "fieldId": null, "fieldLabel": null, "sensitivityLevel": null
        });
        expected
            .as_object_mut()
            .unwrap()
            .extend(populated.as_object().unwrap().clone());
        let wire = serde_json::to_value(&event).unwrap();
        assert_eq!(wire, expected);
        let payload: Value = serde_json::from_str(wire["jsonData"].as_str().unwrap()).unwrap();
        match event_type {
            "result" => assert_eq!(payload, json!([null, false, "结果"])),
            "dialog_request" => assert_eq!(payload, json!({"type": "confirm"})),
            "consent_request" => assert_eq!(payload, json!({})),
            "completed" => assert_eq!(payload, json!({"exitCode": -4, "fuelConsumed": 456})),
            "error" => assert_eq!(payload, json!({"message": "synthetic denial"})),
            "custom_event" => assert_eq!(payload, json!({"future": true})),
            _ => assert_eq!(payload, log_data),
        }
        let decoded: PluginEvent = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
    }
    let unknown_wire = json!({"eventType": "future-event", "jsonData": "not parsed by the wire layer",
        "customType": null, "requestId": null, "pluginId": null, "pluginName": null,
        "fieldId": null, "fieldLabel": null, "sensitivityLevel": null});
    let unknown: PluginEvent = serde_json::from_value(unknown_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(unknown).unwrap(), unknown_wire);
}

#[test]
fn rf306_transparent_result_keeps_arbitrary_json_and_open_log_levels() {
    let values = vec![
        Value::Null,
        json!(false),
        json!(42),
        json!(1.25),
        json!("任意结果"),
        json!([null, {"future": true}]),
        json!({"type": "future-plugin-output", "nested": {"empty": []}}),
    ];
    for value in &values {
        let decoded: PluginResultPayload = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(decoded.0, *value);
        assert_eq!(serde_json::to_value(decoded).unwrap(), *value);
    }
    let result = PluginResult {
        exit_code: -2,
        logs: vec![PluginLogLine {
            id: "synthetic-log".into(),
            level: "notice".into(),
            message: "message".into(),
            timestamp: 123,
        }],
        results: values.iter().cloned().map(PluginResultPayload).collect(),
        fuel_consumed: 987,
    };
    let expected = json!({"exitCode": -2, "logs": [{"id": "synthetic-log", "level": "notice",
        "message": "message", "timestamp": 123}], "results": values, "fuelConsumed": 987});
    assert_eq!(serde_json::to_value(result).unwrap(), expected);
    let decoded: PluginResult = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn rf306_progress_wire_and_real_channel_reserve_host_completion() {
    for (phase, name) in [
        (PluginInstallPhase::Preparing, "preparing"),
        (PluginInstallPhase::Downloading, "downloading"),
        (PluginInstallPhase::Verifying, "verifying"),
        (PluginInstallPhase::Installing, "installing"),
        (PluginInstallPhase::Finalizing, "finalizing"),
        (PluginInstallPhase::Completed, "completed"),
    ] {
        for total_bytes in [None, Some(4096)] {
            let progress = PluginInstallProgress {
                phase,
                percent: 12,
                downloaded_bytes: 1024,
                total_bytes,
            };
            assert_eq!(
                serde_json::to_value(progress).unwrap(),
                json!({
                    "phase": name, "percent": 12, "downloadedBytes": 1024, "totalBytes": total_bytes
                })
            );
        }
    }
    let responses = Arc::new(Mutex::new(Vec::new()));
    let received = responses.clone();
    let channel = Channel::new(move |body| {
        received.lock().unwrap().push(body);
        Ok(())
    });
    emit_install_progress(&channel, PluginInstallProgress::completed());
    channel.send(PluginInstallProgress::completed()).unwrap();
    let wire: Vec<Value> = responses
        .lock()
        .unwrap()
        .drain(..)
        .map(|body| body.deserialize().unwrap())
        .collect();
    assert_eq!(
        wire,
        vec![
            json!({"phase": "finalizing", "percent": 98, "downloadedBytes": 0, "totalBytes": null}),
            json!({"phase": "completed", "percent": 100, "downloadedBytes": 0, "totalBytes": null}),
        ]
    );
}

struct ForeignResource;
impl Resource for ForeignResource {}

#[tokio::test]
async fn rf306_resource_table_rejects_unknown_wrong_table_and_wrong_type() {
    let mut table = ResourceTable::default();
    let rid = table.add(PluginInstallOperation::new());
    let operation = table.get::<PluginInstallOperation>(rid).unwrap();
    let same = table.get::<PluginInstallOperation>(rid).unwrap();
    assert!(Arc::ptr_eq(&operation, &same));
    let weak = Arc::downgrade(&operation);
    let mut other_table = ResourceTable::default();
    assert!(
        matches!(other_table.get::<PluginInstallOperation>(rid), Err(tauri::Error::BadResourceId(id)) if id == rid)
    );
    assert!(matches!(other_table.close(rid), Err(tauri::Error::BadResourceId(id)) if id == rid));
    assert!(
        matches!(table.get::<ForeignResource>(rid), Err(tauri::Error::BadResourceId(id)) if id == rid)
    );
    let foreign_rid = table.add(ForeignResource);
    assert!(
        matches!(table.get::<PluginInstallOperation>(foreign_rid), Err(tauri::Error::BadResourceId(id)) if id == foreign_rid)
    );
    assert!(!*operation.cancelled.borrow());

    table.close(rid).unwrap();
    assert!(
        matches!(table.get::<PluginInstallOperation>(rid), Err(tauri::Error::BadResourceId(id)) if id == rid)
    );
    assert!(matches!(table.close(rid), Err(tauri::Error::BadResourceId(id)) if id == rid));
    let polled = AtomicBool::new(false);
    let result = operation
        .run(async {
            polled.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await;
    assert_eq!(result.unwrap_err(), "PLUGIN_INSTALL_CANCELLED");
    assert!(!polled.load(Ordering::SeqCst));
    drop(operation);
    drop(same);
    assert!(weak.upgrade().is_none());
    table.close(foreign_rid).unwrap();
}

struct PendingFutureGuard(Arc<AtomicBool>);
impl Drop for PendingFutureGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn rf306_table_close_cancels_running_operation_and_drops_pending_future() {
    let mut table = ResourceTable::default();
    let rid = table.add(PluginInstallOperation::new());
    let operation = table.get::<PluginInstallOperation>(rid).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let future_dropped = dropped.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            operation.run(async move {
                let _guard = PendingFutureGuard(future_dropped);
                started.send(()).unwrap();
                std::future::pending::<Result<(), String>>().await
            }),
            async {
                ready.await.unwrap();
                assert!(!dropped.load(Ordering::SeqCst));
                table.close(rid).unwrap();
                assert!(!table.has(rid));
            }
        )
    })
    .await
    .expect("resource close must release the pending operation");
    assert_eq!(result.unwrap_err(), "PLUGIN_INSTALL_CANCELLED");
    assert!(dropped.load(Ordering::SeqCst));
    assert!(operation
        .run(async { Ok(()) })
        .await
        .unwrap_err()
        .contains("已启动"));
}
