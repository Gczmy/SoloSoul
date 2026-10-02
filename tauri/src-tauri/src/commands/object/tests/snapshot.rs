//! object 命令测试 —— snapshot（P047 拆分）

use super::super::*;
use super::setup_vault;
use solosoul_core::objects::inherit_property_fields;
use solosoul_vault::{ObjectRecord, PropertyType, TemplateProperty, TrashItem, UserTemplate};

#[test]
fn test_snapshot_operations() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-snap-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Snapshot Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "v1"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    let snap1 = serde_json::to_vec(&serde_json::json!({
        "name": "Snapshot Test", "tags": [], "properties": {"content": "v1"}
    }))
    .unwrap();
    vault
        .save_snapshot(&record.id, "user_edit", &snap1, "Created")
        .unwrap();

    let snap2 = serde_json::to_vec(&serde_json::json!({
        "name": "Snapshot Test Updated", "tags": [], "properties": {"content": "v2"}
    }))
    .unwrap();
    vault
        .save_snapshot(&record.id, "user_edit", &snap2, "")
        .unwrap();

    let snapshots = vault.list_snapshots(&record.id).unwrap();
    assert_eq!(snapshots.len(), 2);

    let snap_id = snapshots[0]["id"].as_str().unwrap();
    let data = vault.get_snapshot(snap_id).unwrap().unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&data).unwrap();
    assert!(parsed.get("name").is_some());

    let counts = vault
        .count_snapshots_batch(std::slice::from_ref(&record.id))
        .unwrap();
    assert_eq!(counts.get(&record.id), Some(&2));
}

#[test]
fn test_copy_snapshots() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-copy-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Copy Snap Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    let snap = serde_json::to_vec(&serde_json::json!({"name": "v1"})).unwrap();
    vault
        .save_snapshot(&record.id, "user_edit", &snap, "")
        .unwrap();
    vault
        .save_snapshot(&record.id, "user_edit", &snap, "")
        .unwrap();

    let new_id = "obj-copy-2";
    vault.copy_snapshots(&record.id, new_id).unwrap();

    let original_snaps = vault.list_snapshots(&record.id).unwrap();
    let copied_snaps = vault.list_snapshots(new_id).unwrap();
    assert_eq!(original_snaps.len(), 2);
    assert_eq!(copied_snaps.len(), 2);
}

// RF-006 夹具只含 synthetic 数据；setup_vault 使用临时目录及固定测试密钥。
fn rf006_object(id: &str, name: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.to_string(),
        account_id: "test_account".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: name.to_string(),
        icon_name: "synthetic-icon".to_string(),
        parent_id: Some("synthetic-parent".to_string()),
        children_ids: vec!["synthetic-child".to_string()],
        properties: serde_json::json!({"content": format!("Synthetic current {id}")}),
        property_labels: Some(serde_json::json!({"content": "internal"})),
        sensitivity_level: "sensitive".to_string(),
        tags_json: vec!["synthetic-current-tag".to_string()],
        template_id: Some("synthetic-template".to_string()),
        template_type: Some("user".to_string()),
        template_hash: Some("synthetic-template-hash".to_string()),
        ignored_template_hash: Some("synthetic-ignored-hash".to_string()),
        contract_type_id: Some("synthetic-contract".to_string()),
        created_at: "2000-01-01T00:00:00Z".to_string(),
        updated_at: "2000-01-02T00:00:00Z".to_string(),
        version: 7,
        ..Default::default()
    }
}

fn rf006_vault() -> (solosoul_vault::VaultStore, tempfile::TempDir) {
    let (vault, dir) = setup_vault();
    for (id, name) in [
        ("rf006-a", "Synthetic object A"),
        ("rf006-b", "Synthetic object B"),
    ] {
        vault.save_object(&rf006_object(id, name)).unwrap();
    }
    vault
        .log_structured(
            "synthetic_seed",
            "object",
            Some("rf006-a"),
            Some("Synthetic object A"),
            "test",
            Some("Synthetic audit baseline"),
        )
        .unwrap();
    (vault, dir)
}

fn rf006_save_snapshot(
    vault: &solosoul_vault::VaultStore,
    owner: &str,
    data: &serde_json::Value,
) -> String {
    let old_ids: Vec<_> = vault
        .list_snapshots(owner)
        .unwrap()
        .into_iter()
        .map(|snapshot| snapshot["id"].as_str().unwrap().to_string())
        .collect();
    vault
        .save_snapshot(
            owner,
            "user_edit",
            &serde_json::to_vec(data).unwrap(),
            "Synthetic source",
        )
        .unwrap();
    vault
        .list_snapshots(owner)
        .unwrap()
        .into_iter()
        .find_map(|snapshot| {
            let id = snapshot["id"].as_str().unwrap();
            (!old_ids.iter().any(|old| old == id)).then(|| id.to_string())
        })
        .unwrap()
}

/// 比较完整记录（含版本/时间）、历史内容与元数据、全量历史计数和审计内容。
fn rf006_vault_state(vault: &solosoul_vault::VaultStore) -> serde_json::Value {
    let mut objects: Vec<_> = vault
        .list_objects("test_account", None, None, None, true, false)
        .unwrap()
        .into_iter()
        .map(|summary| vault.load_object(&summary.id).unwrap().unwrap())
        .collect();
    objects.sort_by(|left, right| left.id.cmp(&right.id));
    let owners: Vec<_> = ["rf006-a", "rf006-b", "", "rf006-missing-target"]
        .into_iter()
        .map(String::from)
        .collect();
    let histories: Vec<_> = owners
        .iter()
        .map(|owner| {
            let entries: Vec<_> = vault
                .list_snapshots(owner)
                .unwrap()
                .into_iter()
                .map(|metadata| {
                    let id = metadata["id"].as_str().unwrap();
                    serde_json::json!({
                        "owner": vault.get_snapshot_owner(id).unwrap(),
                        "data": vault.get_snapshot(id).unwrap().unwrap(),
                        "metadata": metadata,
                    })
                })
                .collect();
            serde_json::json!({"objectId": owner, "entries": entries})
        })
        .collect();
    serde_json::json!({
        "objects": objects,
        "history": histories,
        "historyCounts": vault.count_snapshots_batch(&owners).unwrap(),
        "audit": vault.list_audit_log(100).unwrap(),
    })
}

#[test]
fn rf006_rejects_cross_object_snapshot_even_with_forged_json_owner() {
    let (vault, _dir) = rf006_vault();
    let snapshot = rf006_save_snapshot(
        &vault,
        "rf006-a",
        &serde_json::json!({
            "objectId": "rf006-b", "object_id": "rf006-b", "id": "rf006-b",
            "name": "Synthetic forged B", "tags": ["forged"],
            "properties": {"content": "Synthetic A snapshot content"},
            "propertyLabels": {"content": "critical"},
        }),
    );
    assert_eq!(
        vault.get_snapshot_owner(&snapshot).unwrap().as_deref(),
        Some("rf006-a")
    );
    let before = rf006_vault_state(&vault);
    let error = super::super::snapshot::rollback_snapshot_in_vault(&vault, &snapshot, "rf006-b")
        .unwrap_err();
    assert_eq!(
        error.code,
        crate::commands::error::BackendErrorCode::SnapshotOwnershipMismatch
    );
    assert_eq!(rf006_vault_state(&vault), before);
}

#[test]
fn rf006_rejects_missing_snapshot_without_mutation() {
    let (vault, _dir) = rf006_vault();
    rf006_save_snapshot(
        &vault,
        "rf006-a",
        &serde_json::json!({"name": "Synthetic baseline"}),
    );
    let before = rf006_vault_state(&vault);
    let error = super::super::snapshot::rollback_snapshot_in_vault(
        &vault,
        "rf006-missing-snapshot",
        "rf006-a",
    )
    .unwrap_err();
    assert_eq!(
        error.code,
        crate::commands::error::BackendErrorCode::SnapshotNotFound
    );
    assert_eq!(rf006_vault_state(&vault), before);
}

#[test]
fn rf006_rejects_empty_owner_even_when_target_id_is_empty() {
    let (vault, _dir) = rf006_vault();
    // 历史损坏记录可同时为空；仅做相等比较不能授权这一组合。
    vault
        .save_object(&rf006_object("", "Synthetic empty ID record"))
        .unwrap();
    let snapshot = rf006_save_snapshot(
        &vault,
        "",
        &serde_json::json!({
            "objectId": "rf006-a", "name": "Synthetic invalid owner", "properties": {"changed": true},
        }),
    );
    assert_eq!(
        vault.get_snapshot_owner(&snapshot).unwrap().as_deref(),
        Some("")
    );
    let before = rf006_vault_state(&vault);
    for target in ["rf006-a", ""] {
        let error = super::super::snapshot::rollback_snapshot_in_vault(&vault, &snapshot, target)
            .unwrap_err();
        assert_eq!(
            error.code,
            crate::commands::error::BackendErrorCode::SnapshotOwnershipMismatch
        );
        assert_eq!(rf006_vault_state(&vault), before);
    }
}

#[test]
fn rf006_rejects_missing_target_without_mutation() {
    let (vault, _dir) = rf006_vault();
    let target = "rf006-missing-target";
    let snapshot = rf006_save_snapshot(
        &vault,
        target,
        &serde_json::json!({
            "name": "Synthetic orphan snapshot", "properties": {"content": "Synthetic orphan"},
        }),
    );
    assert!(vault.load_object(target).unwrap().is_none());
    let before = rf006_vault_state(&vault);
    let error =
        super::super::snapshot::rollback_snapshot_in_vault(&vault, &snapshot, target).unwrap_err();
    assert_eq!(
        error.code,
        crate::commands::error::BackendErrorCode::ObjectNotFound
    );
    assert_eq!(rf006_vault_state(&vault), before);
    assert!(vault.load_object(target).unwrap().is_none());
}

#[test]
fn rf006_rollback_restores_fields_labels_and_records_history_audit() {
    for label_key in ["propertyLabels", "property_labels"] {
        let (vault, _dir) = rf006_vault();
        let original = vault.load_object("rf006-a").unwrap().unwrap();
        let untouched = vault.load_object("rf006-b").unwrap().unwrap();
        let labels = serde_json::json!({"content": "critical", "group": "public"});
        let mut payload = serde_json::json!({
            "name": "Synthetic recovered name", "tags": ["synthetic-restored-tag"],
            "properties": {"content": "Synthetic historical value", "group": [{"name": "Synthetic child", "value": "historical"}]},
        });
        payload[label_key] = labels.clone();
        let snapshot = rf006_save_snapshot(&vault, &original.id, &payload);
        let before_counts = vault
            .count_snapshots_batch(&[original.id.clone(), untouched.id.clone()])
            .unwrap();
        let before_audit = vault.list_audit_log(100).unwrap();

        super::super::snapshot::rollback_snapshot_in_vault(&vault, &snapshot, &original.id)
            .unwrap();

        let restored = vault.load_object(&original.id).unwrap().unwrap();
        let mut expected = original.clone();
        expected.name = "Synthetic recovered name".to_string();
        expected.tags_json = vec!["synthetic-restored-tag".to_string()];
        expected.properties = payload["properties"].clone();
        expected.property_labels = Some(labels);
        expected.version += 1;
        assert_ne!(restored.updated_at, original.updated_at);
        expected.updated_at = restored.updated_at.clone();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&expected).unwrap(),
            "{label_key}: non-rollback fields must be preserved"
        );
        assert_eq!(
            serde_json::to_value(vault.load_object(&untouched.id).unwrap().unwrap()).unwrap(),
            serde_json::to_value(&untouched).unwrap()
        );
        let counts = vault
            .count_snapshots_batch(&[original.id.clone(), untouched.id.clone()])
            .unwrap();
        assert_eq!(
            counts.get(&original.id).copied().unwrap_or(0),
            before_counts.get(&original.id).copied().unwrap_or(0) + 1
        );
        assert_eq!(counts.get(&untouched.id), before_counts.get(&untouched.id));
        let histories = vault.list_snapshots(&original.id).unwrap();
        let rollback = histories
            .iter()
            .find(|entry| entry["id"].as_str() != Some(snapshot.as_str()))
            .unwrap();
        assert_eq!(rollback["triggeredBy"], "rollback");
        assert_eq!(rollback["diffSummary"], "diff_rollback");
        let rollback_id = rollback["id"].as_str().unwrap();
        assert_eq!(
            vault.get_snapshot_owner(rollback_id).unwrap().as_deref(),
            Some(original.id.as_str())
        );
        let data: serde_json::Value =
            serde_json::from_slice(&vault.get_snapshot(rollback_id).unwrap().unwrap()).unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "name": restored.name, "tags": restored.tags_json,
                "properties": restored.properties, "propertyLabels": restored.property_labels,
            })
        );
        let audit = vault.list_audit_log(100).unwrap();
        assert_eq!(audit.len(), before_audit.len() + 1);
        assert_eq!(audit[0].action_type, "object_rollback");
        assert_eq!(audit[0].entity_type, "object");
        assert_eq!(audit[0].entity_id.as_deref(), Some(original.id.as_str()));
        assert_eq!(
            audit[0].entity_name.as_deref(),
            Some("Synthetic recovered name")
        );
        assert_eq!(audit[0].performed_by, "user");
        let details: serde_json::Value =
            serde_json::from_str(audit[0].details.as_deref().unwrap()).unwrap();
        assert_eq!(
            details,
            serde_json::json!({"section": "identity", "snapshot": snapshot})
        );
        assert_eq!(
            serde_json::to_value(&audit[1..]).unwrap(),
            serde_json::to_value(&before_audit).unwrap()
        );
    }
}

#[test]
fn test_page_section_delete_and_restore() {
    let (vault, _dir) = setup_vault();
    let section = "work";
    for i in 0..3 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("obj-page-{}", i),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: section.to_string(),
            name: format!("Work Note {}", i),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"idx": i}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record).unwrap();
    }

    // Simulate page_delete: create trash items and soft delete all in section
    let now_ms = chrono::Utc::now().timestamp_millis();
    for i in 0..3 {
        let id = format!("obj-page-{}", i);
        let rec = vault.load_object(&id).unwrap().unwrap();
        let full_record = serde_json::json!({
            "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
            "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
            "properties": rec.properties,
        });
        let trash = TrashItem {
            id: format!("trash_page_{}", i),
            item_type: "object".to_string(),
            original_id: id.clone(),
            original_parent_id: None,
            original_section_type: Some(section.to_string()),
            original_sort_order: None,
            data: serde_json::to_vec(&full_record).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms("30d")),
            deleted_by: "user".to_string(),
            name_snapshot: rec.name.clone(),
            icon_snapshot: Some(rec.icon_name.clone()),
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&id, true).unwrap();
    }

    // Verify active list is empty
    let active = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(active.len(), 0);

    // Verify trash items exist
    let trash_items = vault.list_trash_items(None, None).unwrap();
    assert_eq!(trash_items.len(), 3);

    // Restore via VaultStore restore_object and delete trash items
    for item in &trash_items {
        let full = vault.get_trash_item(&item.id).unwrap().unwrap();
        vault.restore_object(&full.original_id).unwrap();
        vault.delete_trash_item(&item.id).unwrap();
    }

    // Verify restored
    let restored_active = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(restored_active.len(), 3);
}

#[test]
fn test_dynamic_group_sensitivity_preserved_in_snapshots_after_template_sync() {
    let (vault, _dir) = setup_vault();

    // 1. 创建模板：动态字段组敏感度为 critical
    let tpl = UserTemplate {
        contract_type_id: None,
        id: "tpl-dg".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![TemplateProperty {
            contract_field: None,
            contract_bindings: None,
            id: "contacts".to_string(),
            name: "联系方式".to_string(),
            prop_type: PropertyType::DynamicGroup,
            sensitivity_level: Some("critical".to_string()),
            sensitive: None,
            options: None,
            deprecated_at: None,
            allowed_types: Some(vec![PropertyType::Text, PropertyType::Phone]),
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl).unwrap();

    // 2. 模拟 object_create：继承 property_labels 与 __fields，并保存初始快照
    let property_labels = inherit_property_labels(&vault, Some("tpl-dg"));
    let property_fields = inherit_property_fields(&vault, Some("tpl-dg"));
    let mut properties = serde_json::json!({
        "contacts": [
            { "id": "c1", "name": "手机", "type": "phone", "value": "123" }
        ]
    });
    inject_property_fields(&mut properties, &property_fields);
    let template_hash = Some(template_fingerprint(&tpl));
    if let Some(obj) = properties.as_object_mut() {
        obj.insert(
            "__templateHash".to_string(),
            serde_json::Value::String(template_hash.clone().unwrap()),
        );
    }

    let record = ObjectRecord {
        id: "obj-dg".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test Contact".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: properties.clone(),
        property_labels,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-dg".to_string()),
        template_type: Some("user".to_string()),
        template_hash,
        ignored_template_hash: None,
        contract_type_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    let snap1_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
        "propertyLabels": record.property_labels,
    }))
    .unwrap();
    vault
        .save_snapshot_at(
            "obj-dg",
            "user_edit",
            &snap1_data,
            "diff_created",
            2_000_000_000_000,
        )
        .unwrap();

    // 3. 修改模板动态字段组敏感度为 sensitive
    let mut modified_tpl = tpl;
    modified_tpl.properties[0].sensitivity_level = Some("sensitive".to_string());
    modified_tpl.updated_at = Some(chrono::Utc::now().to_rfc3339());
    vault.save_user_template(&modified_tpl).unwrap();

    // 4. 加载对象并应用同步
    let mut record = vault.load_object("obj-dg").unwrap().unwrap();
    let result = compute_sync_changes(&record, &modified_tpl);
    assert!(result.has_changes, "should detect sensitivity change");
    apply_sync_changes(&mut record, &modified_tpl, &result, false);
    vault.save_object(&record).unwrap();

    let snap2_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
        "propertyLabels": record.property_labels,
    }))
    .unwrap();
    vault
        .save_snapshot_at(
            "obj-dg",
            "template_sync",
            &snap2_data,
            "diff_template_sync",
            2_000_000_000_000,
        )
        .unwrap();

    // 5. 加载两个快照并验证敏感度
    let snapshots = vault.list_snapshots("obj-dg").unwrap();
    assert_eq!(snapshots.len(), 2);

    let latest_snap_id = snapshots[0]["id"].as_str().unwrap();
    let old_snap_id = snapshots[1]["id"].as_str().unwrap();

    let latest_data = vault.get_snapshot(latest_snap_id).unwrap().unwrap();
    let latest: serde_json::Value = serde_json::from_slice(&latest_data).unwrap();
    let old_data = vault.get_snapshot(old_snap_id).unwrap().unwrap();
    let old: serde_json::Value = serde_json::from_slice(&old_data).unwrap();

    // 旧快照应保持 critical
    let old_labels = old["propertyLabels"]["contacts"].as_str();
    assert_eq!(
        old_labels,
        Some("critical"),
        "old snapshot should keep critical sensitivity, got {:?}",
        old["propertyLabels"]
    );

    // 新快照应为 sensitive
    let new_labels = latest["propertyLabels"]["contacts"].as_str();
    assert_eq!(
        new_labels,
        Some("sensitive"),
        "latest snapshot should have sensitive sensitivity, got {:?}",
        latest["propertyLabels"]
    );
}

#[test]
fn rf008_gui_uses_core_label_rules_before_any_write() {
    for payload in [
        serde_json::json!({"propertyLabels": null, "property_labels": {"content": "public"}}),
        serde_json::json!({"property_labels": null}),
        serde_json::json!({"propertyLabels": {}}),
        serde_json::json!({"name": "legacy"}),
    ] {
        let (vault, _dir) = rf006_vault();
        let (core_vault, _core_dir) = rf006_vault();
        let source = rf006_save_snapshot(&vault, "rf006-a", &payload);
        let core_source = rf006_save_snapshot(&core_vault, "rf006-a", &payload);
        let expected =
            solosoul_core::objects::rollback_object(&core_vault, "rf006-a", &core_source).unwrap();
        assert!(expected.snapshot_error.is_none() && expected.audit_error.is_none());
        super::super::snapshot::rollback_snapshot_in_vault(&vault, &source, "rf006-a").unwrap();
        let mut actual = vault.load_object("rf006-a").unwrap().unwrap();
        actual.updated_at = expected.record.updated_at.clone();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected.record).unwrap()
        );
        let snapshot_data = |store: &solosoul_vault::VaultStore| {
            let history = store.list_snapshots("rf006-a").unwrap();
            let item = history
                .iter()
                .find(|item| item["triggeredBy"] == "rollback")
                .unwrap();
            assert_eq!(item["diffSummary"], "diff_rollback");
            store
                .get_snapshot(item["id"].as_str().unwrap())
                .unwrap()
                .unwrap()
        };
        assert_eq!(snapshot_data(&vault), snapshot_data(&core_vault));
    }
    for bad in [
        serde_json::json!([]),
        serde_json::json!("invalid"),
        serde_json::json!(false),
    ] {
        let (vault, _dir) = rf006_vault();
        let source = rf006_save_snapshot(
            &vault,
            "rf006-a",
            &serde_json::json!({
                "name": "must not write", "propertyLabels": bad,
                "property_labels": {"content": "public"}
            }),
        );
        let before = rf006_vault_state(&vault);
        assert!(
            super::super::snapshot::rollback_snapshot_in_vault(&vault, &source, "rf006-a").is_err()
        );
        assert_eq!(rf006_vault_state(&vault), before);
    }
}

#[derive(Clone)]
struct Rf008LogWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for Rf008LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Rf008LogWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn rf008_gui_partial_writes_remain_successful_and_observable() {
    for (reject_snapshot, reject_audit) in [(true, false), (false, true), (true, true)] {
        let (vault, dir) = rf006_vault();
        let source =
            rf006_save_snapshot(&vault, "rf006-a", &serde_json::json!({"name": "restored"}));
        let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
        for (table, reject) in [
            ("object_snapshots", reject_snapshot),
            ("audit_log", reject_audit),
        ] {
            if reject {
                db.execute_batch(&format!("CREATE TRIGGER rf008_{table} BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'rf008 synthetic write failure'); END;")).unwrap();
            }
        }
        let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_max_level(tracing::Level::WARN)
            .with_writer(Rf008LogWriter(logs.clone()))
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            super::super::snapshot::rollback_snapshot_in_vault(&vault, &source, "rf006-a").unwrap();
        });
        let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
        assert_eq!(
            logs.contains("Snapshot save failed"),
            reject_snapshot,
            "{logs}"
        );
        assert_eq!(
            logs.contains("Audit log write failed"),
            reject_audit,
            "{logs}"
        );
        let restored = vault.load_object("rf006-a").unwrap().unwrap();
        assert_eq!(restored.name, "restored");
        assert_eq!(restored.version, 8);
        assert_eq!(
            vault.list_snapshots("rf006-a").unwrap().len(),
            if reject_snapshot { 1 } else { 2 }
        );
        assert_eq!(
            vault.list_audit_log(100).unwrap().len(),
            if reject_audit { 1 } else { 2 }
        );
    }
}
