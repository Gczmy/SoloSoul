//! object 命令测试 —— snapshot（P047 拆分）

use super::super::*;
use super::setup_vault;
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

#[test]
fn test_snapshot_rollback_via_vault() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-roll-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Original".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "v1"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec!["tag1".to_string()],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    // Save snapshot
    let snap = serde_json::to_vec(&serde_json::json!({
        "name": "Original", "tags": ["tag1"], "properties": {"content": "v1"}
    }))
    .unwrap();
    vault
        .save_snapshot(&record.id, "user_edit", &snap, "")
        .unwrap();

    // Update object
    let mut updated = vault.load_object(&record.id).unwrap().unwrap();
    updated.name = "Updated".to_string();
    updated.properties = serde_json::json!({"content": "v2"});
    updated.tags_json = vec!["tag2".to_string()];
    updated.version += 1;
    vault.save_object(&updated).unwrap();

    // Rollback: load snapshot and restore (snapshot_rollback logic)
    let snapshots = vault.list_snapshots(&record.id).unwrap();
    let snap_id = snapshots[0]["id"].as_str().unwrap();
    let data = vault.get_snapshot(snap_id).unwrap().unwrap();
    let snapshot: serde_json::Value = serde_json::from_slice(&data).unwrap();

    let mut rec = vault.load_object(&record.id).unwrap().unwrap();
    if let Some(name) = snapshot["name"].as_str() {
        rec.name = name.to_string();
    }
    if let Some(tags) = snapshot["tags"].as_array() {
        rec.tags_json = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if !snapshot["properties"].is_null() {
        rec.properties = snapshot["properties"].clone();
    }
    rec.updated_at = chrono::Utc::now().to_rfc3339();
    rec.version += 1;
    vault.save_object(&rec).unwrap();

    // Save rollback snapshot
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": rec.name, "tags": rec.tags_json, "properties": rec.properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(
        &record.id,
        "rollback",
        &rollback_data,
        "Rolled back to previous version",
    );

    // Verify rollback
    let rolled = vault.load_object(&record.id).unwrap().unwrap();
    assert_eq!(rolled.name, "Original");
    assert_eq!(rolled.properties, serde_json::json!({"content": "v1"}));
    assert_eq!(rolled.tags_json, vec!["tag1"]);

    let final_snaps = vault.list_snapshots(&record.id).unwrap();
    assert_eq!(final_snaps.len(), 2);
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
        .save_snapshot("obj-dg", "user_edit", &snap1_data, "diff_created")
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
        .save_snapshot("obj-dg", "template_sync", &snap2_data, "diff_template_sync")
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
