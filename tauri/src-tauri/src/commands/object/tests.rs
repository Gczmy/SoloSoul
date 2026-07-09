use super::*;

use solosoul_vault::{ObjectRecord, Profile, TrashItem, UserTemplate, VaultConfig, VaultStore};
use tempfile::TempDir;

fn setup_vault() -> (VaultStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let config =
        VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
    let vault = VaultStore::open(config).unwrap();
    (vault, dir)
}

#[test]
fn test_inherit_contract_type_id() {
    let (vault, _dir) = setup_vault();

    // Missing template_id → None
    assert_eq!(inherit_contract_type_id(&vault, None), None);

    // Non-existent template → None (graceful fallback)
    assert_eq!(inherit_contract_type_id(&vault, Some("nonexistent")), None);

    // Template with contract_type_id → Some
    let tpl = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr-template".to_string(),
        account_id: "acc-1".to_string(),
        name: "Address".to_string(),
        icon_id: None,
        properties: vec![],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl).unwrap();
    assert_eq!(
        inherit_contract_type_id(&vault, Some("addr-template")),
        Some("com.solosoul.address/v1".to_string())
    );

    // Template without contract_type_id → None
    let tpl2 = UserTemplate {
        contract_type_id: None,
        id: "plain-template".to_string(),
        account_id: "acc-1".to_string(),
        name: "Plain".to_string(),
        icon_id: None,
        properties: vec![],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl2).unwrap();
    assert_eq!(
        inherit_contract_type_id(&vault, Some("plain-template")),
        None
    );
}

#[test]
fn test_record_to_data_conversion() {
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Test Object".to_string(),
        icon_name: "document".to_string(),
        parent_id: Some("parent-1".to_string()),
        children_ids: vec!["child-1".to_string()],
        properties: serde_json::json!({"key": "value"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec!["tag1".to_string()],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-02T00:00:00Z".to_string(),
        version: 1,
    };
    let data = record_to_data(&record);
    assert_eq!(data.id, "obj-1");
    assert_eq!(data.account_id, "acc-1");
    assert_eq!(data.collection_type, "note");
    assert_eq!(data.name, "Test Object");
    assert_eq!(data.sensitivity_level, "internal");
    assert_eq!(data.deleted_at, None);
}

#[test]
fn test_object_data_serde_roundtrip() {
    let original = ObjectData {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Test".to_string(),
        collection_type: "note".to_string(),
        properties: serde_json::json!({"foo": "bar"}),
        sensitivity_level: "public".to_string(),
        property_labels: None,
        contract_type_id: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
        template_id: None,
        template_type: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("accountId"));
    assert!(json.contains("collectionType"));
    let restored: ObjectData = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.name, original.name);
}

#[test]
fn test_restored_suffix_localization() {
    assert_eq!(restored_suffix("zh-CN"), "（已恢复）");
    assert_eq!(restored_suffix("en-US"), " (restored)");
    assert_eq!(restored_suffix("ja-JP"), " (restored)");
    assert_eq!(restored_suffix(""), " (restored)");
}

#[test]
fn test_retention_ms_parsing() {
    assert_eq!(retention_ms("30d"), 30 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("60d"), 60 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("half_year"), 180 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("one_year"), 365 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("never"), i64::MAX);
    assert_eq!(retention_ms("unknown"), 30 * 24 * 3600 * 1000i64);
}

#[test]
fn test_object_filter_deserialization() {
    let json = r#"{"collectionType":"note","keyword":"test"}"#;
    let filter: ObjectFilter = serde_json::from_str(json).unwrap();
    assert_eq!(filter.collection_type, Some("note".to_string()));
    assert_eq!(filter.keyword, Some("test".to_string()));
    assert_eq!(filter.sensitivity_level, None);
    assert_eq!(filter.parent_id, None);
}

#[test]
fn test_create_object_input_deserialization() {
    let json = r#"{"accountId":"acc-1","name":"My Note","collectionType":"note","properties":{},"parentId":"parent-1","iconName":"star"}"#;
    let input: CreateObjectInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.account_id, "acc-1");
    assert_eq!(input.icon_name, Some("star".to_string()));
    assert_eq!(input.parent_id, Some("parent-1".to_string()));
}

#[test]
fn test_vault_object_save_and_load() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Test Note".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "hello"}),
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
    };
    vault.save_object(&record).unwrap();
    let loaded = vault.load_object("obj-1").unwrap().unwrap();
    assert_eq!(loaded.name, "Test Note");
    assert_eq!(loaded.properties, serde_json::json!({"content": "hello"}));
}

#[test]
fn test_vault_object_list_and_soft_delete() {
    let (vault, _dir) = setup_vault();
    for i in 0..3 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("obj-{}", i),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: format!("Note {}", i),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Object(serde_json::Map::new()),
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
        };
        vault.save_object(&record).unwrap();
    }
    let all = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(all.len(), 3);

    vault.delete_object("obj-1", true).unwrap();
    let remaining = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(remaining.len(), 2);

    let deleted = vault
        .list_objects("acc-1", None, None, None, false, true)
        .unwrap();
    assert_eq!(deleted.len(), 1);
}

#[test]
fn test_update_object_input_deserialization() {
    let json = r#"{"name":"Updated Name","properties":{"key":"val"},"sensitivityLevel":"private"}"#;
    let input: UpdateObjectInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.name, "Updated Name");
    assert_eq!(input.sensitivity_level, Some("private".to_string()));
}

#[test]
fn test_object_create_with_parent() {
    let (vault, _dir) = setup_vault();
    let parent = ObjectRecord {
        contract_type_id: None,
        id: "parent-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Parent".to_string(),
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
    };
    vault.save_object(&parent).unwrap();

    let child = ObjectRecord {
        contract_type_id: None,
        id: "child-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Child".to_string(),
        icon_name: "document".to_string(),
        parent_id: Some("parent-1".to_string()),
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
    };
    vault.save_object(&child).unwrap();

    // Simulate object_create parent update logic
    if let Ok(Some(mut p)) = vault.load_object("parent-1") {
        if !p.children_ids.contains(&"child-1".to_string()) {
            p.children_ids.push("child-1".to_string());
            p.updated_at = chrono::Utc::now().to_rfc3339();
            p.version += 1;
            vault.save_object(&p).unwrap();
        }
    }

    let updated_parent = vault.load_object("parent-1").unwrap().unwrap();
    assert!(updated_parent.children_ids.contains(&"child-1".to_string()));
}

#[test]
fn test_trash_item_lifecycle() {
    let (vault, _dir) = setup_vault();
    let trash = TrashItem {
        id: "trash_001".to_string(),
        item_type: "object".to_string(),
        original_id: "obj-1".to_string(),
        original_parent_id: Some("parent-1".to_string()),
        original_section_type: Some("identity".to_string()),
        original_sort_order: Some(1),
        data: serde_json::to_vec(&serde_json::json!({"name": "Test"})).unwrap_or_default(),
        deleted_at: 1234567890,
        expires_at: Some(1234567890 + 30 * 24 * 3600 * 1000),
        deleted_by: "user".to_string(),
        name_snapshot: "Test Object".to_string(),
        icon_snapshot: Some("document".to_string()),
    };
    vault.save_trash_item(&trash).unwrap();
    let loaded = vault.get_trash_item("trash_001").unwrap().unwrap();
    assert_eq!(loaded.original_id, "obj-1");
    assert_eq!(loaded.name_snapshot, "Test Object");
    assert_eq!(loaded.item_type, "object");
    assert_eq!(loaded.icon_snapshot, Some("document".to_string()));
    vault.delete_trash_item("trash_001").unwrap();
    assert!(vault.get_trash_item("trash_001").unwrap().is_none());
}

#[test]
fn test_object_soft_delete_with_trash_item() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-del-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Delete Me".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "hello"}),
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
    };
    vault.save_object(&record).unwrap();

    let now_ms = chrono::Utc::now().timestamp_millis();
    let full_record = serde_json::json!({
        "id": record.id, "account_id": record.account_id, "type_id": record.type_id,
        "section_type": record.section_type, "name": record.name, "icon_name": record.icon_name,
        "parent_id": record.parent_id, "children_ids": record.children_ids,
        "properties": record.properties, "property_labels": record.property_labels,
        "sensitivity_level": record.sensitivity_level, "tags": record.tags_json,
        "created_at": record.created_at, "updated_at": record.updated_at, "version": record.version,
        "contract_type_id": None::<String>,
    });
    let trash_id = format!("trash_{}", uuid::Uuid::new_v4());
    let trash = TrashItem {
        id: trash_id.clone(),
        item_type: "object".to_string(),
        original_id: record.id.clone(),
        original_parent_id: record.parent_id.clone(),
        original_section_type: Some(record.section_type.clone()),
        original_sort_order: None,
        data: serde_json::to_vec(&full_record).unwrap_or_default(),
        deleted_at: now_ms,
        expires_at: Some(now_ms + retention_ms("30d")),
        deleted_by: "user".to_string(),
        name_snapshot: record.name.clone(),
        icon_snapshot: Some(record.icon_name.clone()),
    };
    vault.save_trash_item(&trash).unwrap();
    vault.delete_object(&record.id, true).unwrap();

    let trash_list = vault.list_trash_items(None, None).unwrap();
    assert_eq!(trash_list.len(), 1);
    assert_eq!(trash_list[0].name, "Delete Me");

    let loaded_trash = vault.get_trash_item(&trash_id).unwrap().unwrap();
    assert_eq!(loaded_trash.original_id, record.id);

    let active = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(active.len(), 0);
}

#[test]
fn test_hard_delete_purges_object() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-purge-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Purge Me".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "bye"}),
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
    };
    vault.save_object(&record).unwrap();

    let trash = TrashItem {
        id: "trash_purge_1".to_string(),
        item_type: "object".to_string(),
        original_id: record.id.clone(),
        original_parent_id: None,
        original_section_type: Some(record.section_type.clone()),
        original_sort_order: None,
        data: serde_json::to_vec(&serde_json::json!({"name": "Purge Me"})).unwrap_or_default(),
        deleted_at: chrono::Utc::now().timestamp_millis(),
        expires_at: None,
        deleted_by: "user".to_string(),
        name_snapshot: record.name.clone(),
        icon_snapshot: None,
    };
    vault.save_trash_item(&trash).unwrap();

    // Hard delete object and trash item (object_purge equivalent)
    vault.delete_object(&record.id, false).unwrap();
    vault.delete_trash_item(&trash.id).unwrap();

    assert!(vault.load_object(&record.id).unwrap().is_none());
    assert!(vault.get_trash_item(&trash.id).unwrap().is_none());
}

#[test]
fn test_trash_permanent_delete_flow() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-perm-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Perm Delete".to_string(),
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
    };
    vault.save_object(&record).unwrap();

    let trash = TrashItem {
        id: "trash_perm_1".to_string(),
        item_type: "object".to_string(),
        original_id: record.id.clone(),
        original_parent_id: None,
        original_section_type: Some(record.section_type.clone()),
        original_sort_order: None,
        data: serde_json::to_vec(&serde_json::json!({"name": "Perm Delete"})).unwrap_or_default(),
        deleted_at: chrono::Utc::now().timestamp_millis(),
        expires_at: None,
        deleted_by: "user".to_string(),
        name_snapshot: record.name.clone(),
        icon_snapshot: None,
    };
    vault.save_trash_item(&trash).unwrap();
    vault.delete_object(&record.id, true).unwrap();

    // Simulate trash_permanent_delete command logic
    if let Ok(Some(t)) = vault.get_trash_item("trash_perm_1") {
        vault.delete_object(&t.original_id, false).unwrap();
        vault.delete_trash_item("trash_perm_1").unwrap();
    }

    assert!(vault.load_object(&record.id).unwrap().is_none());
    assert!(vault.get_trash_item("trash_perm_1").unwrap().is_none());
}

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
fn test_page_restore_from_trash_reconstruction() {
    let (vault, _dir) = setup_vault();
    let section = "finance";
    for i in 0..2 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("obj-fin-{}", i),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: section.to_string(),
            name: format!("Finance {}", i),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"amount": i * 100}),
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
        };
        vault.save_object(&record).unwrap();
    }

    // Soft delete and trash
    let now_ms = chrono::Utc::now().timestamp_millis();
    for i in 0..2 {
        let id = format!("obj-fin-{}", i);
        let rec = vault.load_object(&id).unwrap().unwrap();
        let data = serde_json::json!({
            "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
            "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
            "properties": rec.properties, "parent_id": rec.parent_id,
            "children_ids": rec.children_ids, "property_labels": rec.property_labels,
            "sensitivity_level": rec.sensitivity_level, "tags": rec.tags_json,
            "created_at": rec.created_at, "updated_at": rec.updated_at, "version": rec.version,
            "contract_type_id": None::<String>,
        });
        let trash = TrashItem {
            id: format!("trash_fin_{}", i),
            item_type: "object".to_string(),
            original_id: id.clone(),
            original_parent_id: rec.parent_id.clone(),
            original_section_type: Some(section.to_string()),
            original_sort_order: None,
            data: serde_json::to_vec(&data).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms("30d")),
            deleted_by: "user".to_string(),
            name_snapshot: rec.name.clone(),
            icon_snapshot: Some(rec.icon_name.clone()),
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&id, true).unwrap();
    }

    // Replicate page_restore logic inline
    let all_trash = vault.list_trash_items(None, None).unwrap();
    let mut count = 0usize;
    for item in &all_trash {
        if item.original_section_type.as_deref() == Some(section) {
            if let Ok(Some(trash)) = vault.get_trash_item(&item.id) {
                let record_data: serde_json::Value =
                    serde_json::from_slice(&trash.data).unwrap_or_default();
                let account_id = record_data["account_id"].as_str().unwrap_or("");
                let active = vault
                    .list_objects(
                        account_id,
                        None,
                        None,
                        Some(&trash.name_snapshot),
                        false,
                        false,
                    )
                    .unwrap_or_default();
                let exists = active.iter().any(|o| o.name == trash.name_snapshot);
                let new_id = if exists {
                    format!(
                        "{}_{}",
                        trash.original_id,
                        uuid::Uuid::new_v4()
                            .to_string()
                            .split('-')
                            .next()
                            .unwrap_or("restored")
                    )
                } else {
                    trash.original_id.clone()
                };
                let new_name = if exists {
                    format!("{}{}", trash.name_snapshot, restored_suffix("en-US"))
                } else {
                    trash.name_snapshot.clone()
                };
                if let Ok(record_data) = serde_json::from_slice::<serde_json::Value>(&trash.data) {
                    // §13.10.3: 从模板继承 contract_type_id
                    let page_restore_ctid =
                        inherit_contract_type_id(&vault, record_data["template_id"].as_str());
                    let now = chrono::Utc::now().to_rfc3339();
                    let record = ObjectRecord {
                        contract_type_id: page_restore_ctid,
                        id: new_id.clone(),
                        account_id: record_data["account_id"]
                            .as_str()
                            .unwrap_or("imported")
                            .to_string(),
                        type_id: record_data["type_id"]
                            .as_str()
                            .unwrap_or("note")
                            .to_string(),
                        section_type: section.to_string(),
                        name: new_name,
                        icon_name: record_data["icon_name"]
                            .as_str()
                            .unwrap_or("document")
                            .to_string(),
                        parent_id: record_data["parent_id"].as_str().map(String::from),
                        children_ids: record_data["children_ids"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        properties: record_data["properties"].clone(),
                        property_labels: if record_data["property_labels"].is_null() {
                            None
                        } else {
                            Some(record_data["property_labels"].clone())
                        },
                        sensitivity_level: record_data["sensitivity_level"]
                            .as_str()
                            .unwrap_or("internal")
                            .to_string(),
                        is_deleted: false,
                        deleted_at: None,
                        tags_json: Vec::new(),
                        template_id: None,
                        template_type: None,
                        template_hash: None,
                        created_at: record_data["created_at"]
                            .as_str()
                            .unwrap_or(&now)
                            .to_string(),
                        updated_at: now,
                        version: record_data["version"].as_u64().unwrap_or(1) as u32,
                    };
                    if vault.save_object(&record).is_ok() {
                        if new_id != trash.original_id {
                            let _ = vault.copy_snapshots(&trash.original_id, &new_id);
                        }
                        vault.delete_trash_item(&item.id).ok();
                        count += 1;
                    }
                }
            }
        }
    }

    assert_eq!(count, 2);
    let restored = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(restored.len(), 2);
}

#[test]
fn test_trash_detail_serialization() {
    let detail = TrashDetail {
        id: "trash_001".to_string(),
        item_type: "object".to_string(),
        original_id: "obj-1".to_string(),
        name: "Test Object".to_string(),
        section_type: Some("identity".to_string()),
        deleted_at: 1234567890,
        expires_at: Some(1234567890000),
        deleted_by: "user".to_string(),
        remaining_days: Some(29),
        original_location: "From page: identity".to_string(),
        template_id: None,
        preview_properties: vec![serde_json::json!({"key": "title", "value": "Hello"})],
        attachments: vec![TrashAttachmentInfo {
            id: "att-1".to_string(),
            file_name: "file.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        }],
        deleted_attachments: vec![],
        snapshots: vec![serde_json::json!({"id": "snap-1", "timestamp": 0})],
        child_items: vec![],
    };
    let json = serde_json::to_string(&detail).unwrap();
    assert!(json.contains("\"id\":\"trash_001\""));
    assert!(json.contains("\"itemType\":\"object\""));
    assert!(json.contains("\"originalId\":\"obj-1\""));
    assert!(json.contains("\"name\":\"Test Object\""));
    assert!(json.contains("\"sectionType\":\"identity\""));
    assert!(json.contains("\"deletedAt\":1234567890"));
    assert!(json.contains("\"remainingDays\":29"));
    assert!(json.contains("\"originalLocation\":\"From page: identity\""));
    assert!(json.contains("\"previewProperties\""));
    assert!(json.contains("\"attachments\""));
    assert!(json.contains("\"deletedAttachments\""));
    assert!(json.contains("\"snapshots\""));
}

#[test]
fn test_trash_page_detail_includes_children() {
    let (vault, _dir) = setup_vault();
    let page_id = "custom-page-uuid-1234";
    let section = page_id;

    // Save child objects that belong to the page
    for i in 0..3 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("child-obj-{}", i),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: section.to_string(),
            name: format!("Child Object {}", i),
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
        };
        vault.save_object(&record).unwrap();
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    // Create page trash item (page type)
    let page_trash = TrashItem {
        id: "trash_page_001".to_string(),
        item_type: "page".to_string(),
        original_id: page_id.to_string(),
        original_parent_id: None,
        original_section_type: Some("page".to_string()),
        original_sort_order: None,
        data: serde_json::to_vec(&serde_json::json!({"name": "My Custom Page"}))
            .unwrap_or_default(),
        deleted_at: now_ms,
        expires_at: Some(now_ms + retention_ms("30d")),
        deleted_by: "user".to_string(),
        name_snapshot: "My Custom Page".to_string(),
        icon_snapshot: Some("folder".to_string()),
    };
    vault.save_trash_item(&page_trash).unwrap();

    // Create child object trash items (matching original_section_type == page_id)
    for i in 0..3 {
        let rec = vault
            .load_object(&format!("child-obj-{}", i))
            .unwrap()
            .unwrap();
        let full_record = serde_json::json!({
            "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
            "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
            "properties": rec.properties,
        });
        let trash = TrashItem {
            id: format!("trash_child_{}", i),
            item_type: "object".to_string(),
            original_id: rec.id.clone(),
            original_parent_id: None,
            original_section_type: Some(rec.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&full_record).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms("30d")),
            deleted_by: "user".to_string(),
            name_snapshot: rec.name.clone(),
            icon_snapshot: Some(rec.icon_name.clone()),
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&rec.id, true).unwrap();
    }

    // Also create an unrelated trash item that should NOT appear in children
    let unrelated_trash = TrashItem {
        id: "trash_unrelated".to_string(),
        item_type: "object".to_string(),
        original_id: "unrelated-obj".to_string(),
        original_parent_id: None,
        original_section_type: Some("other-page".to_string()),
        original_sort_order: None,
        data: vec![],
        deleted_at: now_ms,
        expires_at: Some(now_ms + retention_ms("30d")),
        deleted_by: "user".to_string(),
        name_snapshot: "Unrelated".to_string(),
        icon_snapshot: None,
    };
    vault.save_trash_item(&unrelated_trash).unwrap();

    // Fetch and verify: replicate trash_get_detail child logic
    let page_item = vault.get_trash_item("trash_page_001").unwrap().unwrap();
    assert_eq!(page_item.item_type, "page");

    let all = vault.list_trash_items(None, None).unwrap();
    let children: Vec<TrashChildSummary> = all
        .into_iter()
        .filter(|t| t.item_type == "object" && t.original_section_type.as_deref() == Some(page_id))
        .filter_map(|t| {
            let item_id = t.id.clone();
            match vault.get_trash_item(&item_id) {
                Ok(Some(full)) => Some(TrashChildSummary {
                    id: item_id,
                    original_id: full.original_id,
                    name: t.name,
                    item_type: t.item_type,
                }),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    let mut children = children;
    children.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(
        children.len(),
        3,
        "Should return 3 child objects for the page"
    );
    assert!(children.iter().any(|c| c.name == "Child Object 0"));
    assert!(children.iter().any(|c| c.name == "Child Object 1"));
    assert!(children.iter().any(|c| c.name == "Child Object 2"));
    // Verify names are sorted
    let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Child Object 0", "Child Object 1", "Child Object 2"]
    );
}

#[test]
fn test_load_trash_retention_default() {
    let (vault, _dir) = setup_vault();
    let period = load_trash_retention(&vault, "nonexistent");
    assert_eq!(period, "30d");
}

#[test]
fn test_load_trash_retention_from_profile() {
    let (vault, _dir) = setup_vault();
    let account_id = "acc-retention";
    let prefs = serde_json::json!({
        "preferences": {
            "trashRetention": "60d"
        }
    });
    let profile = Profile::new_with_id(account_id, "Test", serde_json::to_vec(&prefs).unwrap());
    vault.save_profile(&profile).unwrap();
    let period = load_trash_retention(&vault, account_id);
    assert_eq!(period, "60d");
}

#[test]
fn test_validate_dynamic_groups_ok() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "allowedTypes": ["phone", "email"]
            }
        },
        "contactMethods": [
            { "id": "1", "name": "手机", "type": "phone", "value": "123" },
            { "id": "2", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_ok());
}

#[test]
fn test_validate_dynamic_groups_invalid_type() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "allowedTypes": ["phone"]
            }
        },
        "contactMethods": [
            { "id": "1", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_err());
}

#[test]
fn test_validate_dynamic_groups_exceeds_max_items() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "maxItems": 1
            }
        },
        "contactMethods": [
            { "id": "1", "name": "手机", "type": "phone", "value": "123" },
            { "id": "2", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_err());
}

#[test]
fn test_template_fingerprint_stable_and_sensitive() {
    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "name".to_string(),
            name: "Name".to_string(),
            prop_type: solosoul_vault::PropertyType::Text,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        contract_type_id: None,
    };

    let hash1 = template_fingerprint(&tpl);
    let hash2 = template_fingerprint(&tpl);
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 16);

    let mut tpl_modified = tpl.clone();
    tpl_modified.name = "Contact Updated".to_string();
    let hash3 = template_fingerprint(&tpl_modified);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_compute_sync_changes_categorizes_fields() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "oldField": { "name": "Old Field", "type": "text" },
                "textField": { "name": "Text Field", "type": "text" },
                "numberField": { "name": "Number Field", "type": "number" }
            },
            "oldField": "old value",
            "textField": "hello",
            "numberField": 42
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "textField".to_string(),
                name: "Text Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("sensitive".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "numberField".to_string(),
                name: "Number Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "newField".to_string(),
                name: "New Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(result.has_changes);

    // oldField removed
    assert_eq!(result.fields_deprecated.len(), 1);
    assert_eq!(result.fields_deprecated[0].id, "oldField");

    // newField added
    assert_eq!(result.fields_added.len(), 1);
    assert_eq!(result.fields_added[0].id, "newField");

    // textField sensitivity updated; numberField number->text safe conversion
    assert_eq!(result.fields_updated.len(), 2);
    let updated_ids: std::collections::HashSet<_> = result
        .fields_updated
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    assert!(updated_ids.contains("textField"));
    assert!(updated_ids.contains("numberField"));

    // no incompatible fields in this scenario
    assert!(result.fields_incompatible.is_empty());
}

#[test]
fn test_apply_sync_changes_archives_incompatible_field() {
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "numberField": { "name": "Number Field", "type": "number" }
            },
            "numberField": 42
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "numberField".to_string(),
            name: "Number Field".to_string(),
            prop_type: solosoul_vault::PropertyType::Date,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 原字段应被重置为空字符串（date 默认值）
    assert_eq!(record.properties.get("numberField").unwrap(), "");

    // 旧字段应被归档到 __deprecatedFields
    let deprecated = record
        .properties
        .get("__deprecatedFields")
        .and_then(|v| v.as_object())
        .unwrap();
    assert!(deprecated.contains_key("numberField"));
    let archived = deprecated.get("numberField").unwrap().as_object().unwrap();
    assert_eq!(archived.get("value").unwrap(), 42);
    assert_eq!(
        archived.get("reason").and_then(|v| v.as_str()).unwrap(),
        "type_incompatible"
    );

    // template_hash 应已更新
    assert_eq!(record.template_hash, Some(result.template_hash));
}

#[test]
fn test_apply_sync_changes_preserves_safe_type_conversion() {
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "textField": { "name": "Text Field", "type": "text" }
            },
            "textField": "hello"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "textField".to_string(),
            name: "Text Field".to_string(),
            prop_type: solosoul_vault::PropertyType::MultilineText,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(result.fields_incompatible.is_empty());
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 安全转换：text -> multiline 应保留原值
    assert_eq!(record.properties.get("textField").unwrap(), "hello");
    assert!(
        record.properties.get("__deprecatedFields").is_none()
            || record
                .properties
                .get("__deprecatedFields")
                .and_then(|v| v.as_object())
                .map(|m| m.is_empty())
                .unwrap_or(true)
    );
}

#[test]
fn test_compute_sync_changes_uses_property_labels_as_sensitivity_baseline() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        // __fields 中动态字段组敏感度为旧值 internal，但 property_labels 已为 public
        properties: serde_json::json!({
            "__fields": {
                "textField": { "name": "Text Field", "type": "text", "sensitivityLevel": "internal" },
                "dynamicGroup": { "name": "Dynamic Group", "type": "dynamic_group", "sensitivityLevel": "internal" }
            },
            "textField": "hello",
            "dynamicGroup": []
        }),
        property_labels: Some(serde_json::json!({
            "textField": "internal",
            "dynamicGroup": "public"
        })),
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "textField".to_string(),
                name: "Text Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("public".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "dynamicGroup".to_string(),
                name: "Dynamic Group".to_string(),
                prop_type: solosoul_vault::PropertyType::DynamicGroup,
                sensitive: None,
                sensitivity_level: Some("public".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);

    // 只有 textField 的敏感度真正从 internal 变到 public
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "textField");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Sensitivity { old_level, new_level } if old_level == "internal" && new_level == "public"))
    );

    // dynamicGroup 在 property_labels 中已经是 public，不应被误报
    assert!(!result.fields_updated.iter().any(|f| f.id == "dynamicGroup"));
}
