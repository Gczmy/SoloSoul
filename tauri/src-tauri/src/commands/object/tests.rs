use super::*;

use solosoul_vault::{
    ObjectRecord, Profile, PropertyType, TemplateProperty, TrashItem, UserTemplate, VaultConfig,
    VaultStore,
};
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
        ..Default::default()
    };
    let data = record_to_data(&record);
    assert_eq!(data.id, "obj-1");
    assert_eq!(data.account_id, "acc-1");
    assert_eq!(data.collection_type, "note");
    assert_eq!(data.name, "Test Object");
    assert_eq!(data.sensitivity_level, "internal");
    assert_eq!(data.deleted_at, None);
    assert_eq!(data.tags, vec!["tag1"]);
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
        tags: vec!["tag-a".to_string()],
        contract_type_id: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
        template_id: None,
        template_type: None,
        template_hash: None,
        ignored_template_hash: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("accountId"));
    assert!(json.contains("typeId"));
    let restored: ObjectData = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.name, original.name);
    assert_eq!(restored.tags, original.tags);
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
    let json = r#"{"typeId":"note","keyword":"test"}"#;
    let filter: ObjectFilter = serde_json::from_str(json).unwrap();
    assert_eq!(filter.collection_type, Some("note".to_string()));
    assert_eq!(filter.keyword, Some("test".to_string()));
    assert_eq!(filter.sensitivity_level, None);
    assert_eq!(filter.parent_id, None);
}

#[test]
fn test_create_object_input_deserialization() {
    let json = r#"{"accountId":"acc-1","name":"My Note","typeId":"note","properties":{},"parentId":"parent-1","iconName":"star"}"#;
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
        ..Default::default()
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
            ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
fn test_trash_permanent_delete_batch_helper() {
    // P024：批量端点依赖的共享 helper——循环删除多条后对象与回收站条目均清空。
    let (vault, _dir) = setup_vault();
    for i in 0..3 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("obj-batch-{i}"),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: format!("Batch {i}"),
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
        let trash = TrashItem {
            id: format!("trash_batch_{i}"),
            item_type: "object".to_string(),
            original_id: record.id.clone(),
            original_parent_id: None,
            original_section_type: Some(record.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&serde_json::json!({ "name": record.name }))
                .unwrap_or_default(),
            deleted_at: chrono::Utc::now().timestamp_millis(),
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: record.name.clone(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&record.id, true).unwrap();
    }

    // 模拟 trash_permanent_delete_batch 的服务端循环
    for i in 0..3 {
        super::trash::permanent_delete_one(&vault, &format!("trash_batch_{i}")).unwrap();
    }

    for i in 0..3 {
        assert!(vault
            .load_object(&format!("obj-batch-{i}"))
            .unwrap()
            .is_none());
        assert!(vault
            .get_trash_item(&format!("trash_batch_{i}"))
            .unwrap()
            .is_none());
    }
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
        property_labels: None,
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
            ..Default::default()
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

    let mut tpl_modified = tpl;
    // 指纹排除模板名称/图标/分类等元数据，只反映字段定义
    tpl_modified.name = "Contact Updated".to_string();
    let hash3 = template_fingerprint(&tpl_modified);
    assert_eq!(hash1, hash3);

    // 修改字段敏感度应改变指纹
    tpl_modified.properties[0].sensitivity_level = Some("sensitive".to_string());
    let hash4 = template_fingerprint(&tpl_modified);
    assert_ne!(hash1, hash4);
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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

#[test]
fn test_apply_sync_changes_preserves_multiline_value_on_rename() {
    let (vault, _dir) = setup_vault();

    // 创建模板：字段 ID 为 f1，名称 "1"，类型 multiline
    let tpl = UserTemplate {
        contract_type_id: None,
        id: "tpl-rename".to_string(),
        account_id: "acc-1".to_string(),
        name: "Rename Test".to_string(),
        icon_id: None,
        properties: vec![TemplateProperty {
            contract_field: None,
            contract_bindings: None,
            id: "f1".to_string(),
            name: "1".to_string(),
            prop_type: PropertyType::MultilineText,
            sensitivity_level: None,
            sensitive: None,
            options: None,
            deprecated_at: None,
            allowed_types: None,
            max_items: None,
        }],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl).unwrap();

    // 创建对象：字段 f1 的值为 "a"，__fields 中记录旧名称 "1"
    let mut record = ObjectRecord {
        id: "obj-rename".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "note".to_string(),
        name: "Test Object".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "f1": { "name": "1", "type": "multiline" }
            },
            "f1": "a"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-rename".to_string()),
        template_type: Some("user".to_string()),
        template_hash: Some(template_fingerprint(&tpl)),
        ignored_template_hash: None,
        contract_type_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 修改模板字段名 "1" -> "2"，类型不变
    let mut modified_tpl = tpl;
    modified_tpl.properties[0].name = "2".to_string();
    modified_tpl.updated_at = Some(chrono::Utc::now().to_rfc3339());
    vault.save_user_template(&modified_tpl).unwrap();

    // 计算并应用同步
    let result = compute_sync_changes(&record, &modified_tpl);
    assert!(result.has_changes, "should detect name change");
    assert!(
        result.fields_updated.iter().any(|f| f.id == "f1"),
        "f1 should be in updated fields"
    );
    assert!(
        result.fields_added.is_empty(),
        "rename should not be treated as added field"
    );
    assert!(
        result.fields_deprecated.is_empty(),
        "rename should not be treated as deprecated field"
    );

    apply_sync_changes(&mut record, &modified_tpl, &result, false);

    // 关键断言：字段值必须保留，__fields 中的字段名应更新为 "2"
    assert_eq!(
        record.properties["f1"], "a",
        "multiline value must be preserved"
    );
    let fields = record.properties["__fields"].as_object().unwrap();
    assert_eq!(
        fields["f1"]["name"], "2",
        "__fields name should be updated to new template name"
    );
    assert_eq!(
        fields["f1"]["type"], "multiline",
        "__fields type should remain multiline"
    );
}

#[test]
fn test_apply_sync_changes_preserves_existing_values_when_fields_missing() {
    // 旧对象可能缺少 __fields（功能上线前创建），同步时不应覆盖已有字段值。
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        // 没有 __fields，但实际有字段值
        properties: serde_json::json!({
            "f1": "a"
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
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "f1".to_string(),
            name: "2".to_string(),
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
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 关键断言：即使缺少 __fields，已有字段值 "a" 也必须保留
    assert_eq!(
        record.properties["f1"], "a",
        "existing value must not be overwritten"
    );
    let fields = record.properties["__fields"].as_object().unwrap();
    assert_eq!(fields["f1"]["name"], "2");
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

#[test]
fn test_compute_sync_changes_detects_dynamic_group_metadata() {
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
                "contacts": {
                    "name": "联系方式",
                    "type": "dynamic_group",
                    "allowedTypes": ["text"],
                    "maxItems": 5
                }
            },
            "contacts": []
        }),
        property_labels: Some(serde_json::json!({ "contacts": "internal" })),
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
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "contacts".to_string(),
            name: "联系方式".to_string(),
            prop_type: solosoul_vault::PropertyType::DynamicGroup,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: Some(vec![
                solosoul_vault::PropertyType::Text,
                solosoul_vault::PropertyType::Phone,
            ]),
            max_items: Some(10),
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(
        result.has_changes,
        "should detect dynamic group metadata changes"
    );
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "contacts");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Metadata { metadata_keys } if metadata_keys.contains(&"allowedTypes".to_string()) && metadata_keys.contains(&"maxItems".to_string()))),
        "expected Metadata change for allowedTypes and maxItems, got {:?}",
        result.fields_updated[0].changes
    );
}

#[test]
fn test_compute_sync_changes_detects_field_metadata() {
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
                "nameField": {
                    "name": "姓名",
                    "type": "text",
                    "deprecatedAt": "",
                    "contractField": false
                }
            },
            "nameField": "Alice"
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
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "nameField".to_string(),
            name: "姓名".to_string(),
            prop_type: solosoul_vault::PropertyType::Text,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: Some("2024-01-01T00:00:00Z".to_string()),
            contract_field: Some(true),
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
    assert!(result.has_changes, "should detect field metadata changes");
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "nameField");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Metadata { metadata_keys } if metadata_keys.contains(&"deprecatedAt".to_string()) && metadata_keys.contains(&"contractField".to_string()))),
        "expected Metadata change for deprecatedAt and contractField, got {:?}",
        result.fields_updated[0].changes
    );
}

#[test]
fn test_trash_detail_object_data_camel_case_and_snake_case_fallback() {
    // 模拟 object_delete 现在使用的 camelCase 序列化
    let camel_case_data = serde_json::json!({
        "id": "obj-1",
        "accountId": "acc-1",
        "typeId": "note",
        "sectionType": "identity",
        "name": "Test",
        "iconName": "document",
        "parentId": null,
        "childrenIds": [],
        "properties": {
            "__fields": {
                "f1": { "name": "字段1", "type": "text" }
            },
            "f1": "value1"
        },
        "propertyLabels": { "f1": "sensitive" },
        "sensitivityLevel": "internal",
        "tags": [],
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-01T00:00:00Z",
        "version": 1,
        "templateId": null,
        "templateType": null,
        "contractTypeId": null,
        "templateHash": null,
    });

    // 验证 trash_get_detail 内部的解析逻辑：优先读取 camelCase
    let sensitivity_map = camel_case_data
        .get("propertyLabels")
        .or_else(|| camel_case_data.get("property_labels"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        sensitivity_map.get("f1").and_then(|v| v.as_str()),
        Some("sensitive")
    );

    let template_id = camel_case_data
        .get("templateId")
        .or_else(|| camel_case_data.get("template_id"))
        .and_then(|v| v.as_str());
    assert_eq!(template_id, None);

    // 验证旧数据 snake_case 也能通过 fallback 读取
    let snake_case_data = serde_json::json!({
        "id": "obj-1",
        "account_id": "acc-1",
        "property_labels": { "f1": "public" },
        "template_id": "tpl-old",
    });
    let old_sensitivity = snake_case_data
        .get("propertyLabels")
        .or_else(|| snake_case_data.get("property_labels"))
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("f1"))
        .and_then(|v| v.as_str());
    assert_eq!(old_sensitivity, Some("public"));

    let old_template_id = snake_case_data
        .get("templateId")
        .or_else(|| snake_case_data.get("template_id"))
        .and_then(|v| v.as_str());
    assert_eq!(old_template_id, Some("tpl-old"));
}

#[test]
fn test_repair_restored_objects_fixes_legacy_fields() {
    let (vault, _dir) = setup_vault();

    // 创建一个自定义页面对象，作为后续子对象的 parent 目标
    let page = ObjectRecord {
        id: "custom-page".to_string(),
        account_id: "test_account".to_string(),
        type_id: "custom-page".to_string(),
        section_type: "custom-page".to_string(),
        name: "Custom Page".to_string(),
        icon_name: "folder".to_string(),
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
    vault.save_object(&page).unwrap();

    // 模拟旧版 object_restore 错误写入的对象：
    // account_id = 'imported'、type_id = 'note'、parent_id 丢失
    let corrupted_identity = ObjectRecord {
        id: "obj-repair-identity".to_string(),
        account_id: "imported".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Corrupted Identity".to_string(),
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
        ..Default::default()
    };
    vault.save_object(&corrupted_identity).unwrap();

    let corrupted_custom = ObjectRecord {
        id: "obj-repair-custom".to_string(),
        account_id: "imported".to_string(),
        type_id: "note".to_string(),
        section_type: "custom-page".to_string(),
        name: "Corrupted Custom".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "world"}),
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
    vault.save_object(&corrupted_custom).unwrap();

    // 重置标记，使修复逻辑可以再次执行
    vault
        .set_sys_config("restored_objects_repair_v1", "0")
        .unwrap();
    let fixed = vault.repair_restored_objects().unwrap();
    assert_eq!(fixed, 2, "should repair both corrupted objects");

    let repaired_identity = vault.load_object("obj-repair-identity").unwrap().unwrap();
    assert_eq!(repaired_identity.account_id, "test_account");
    assert_eq!(repaired_identity.type_id, "identity");
    assert_eq!(repaired_identity.parent_id, None);

    let repaired_custom = vault.load_object("obj-repair-custom").unwrap().unwrap();
    assert_eq!(repaired_custom.account_id, "test_account");
    assert_eq!(repaired_custom.type_id, "custom-page");
    assert_eq!(repaired_custom.parent_id, Some("custom-page".to_string()));
}

#[test]
fn test_backfill_missing_property_labels_from_template() {
    let (vault, _dir) = setup_vault();

    // 创建一个带字段敏感度的模板
    let tpl = UserTemplate {
        id: "tpl-sensitivity".to_string(),
        account_id: "test_account".to_string(),
        name: "ID Card".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "name".to_string(),
                name: "姓名".to_string(),
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
                id: "id_number".to_string(),
                name: "身份证号".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("critical".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "__dynamic_group__".to_string(),
                name: "联系方式".to_string(),
                prop_type: solosoul_vault::PropertyType::DynamicGroup,
                sensitive: None,
                sensitivity_level: Some("sensitive".to_string()),
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
    vault.save_user_template(&tpl).unwrap();

    // 模拟旧 bug 恢复出的对象：有模板，但 property_labels 为空
    let record = ObjectRecord {
        id: "obj-no-labels".to_string(),
        account_id: "test_account".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Old ID".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "name": "Alice",
            "id_number": "123456",
            "__dynamic_group__": []
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-sensitivity".to_string()),
        template_type: Some("user".to_string()),
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    // 重置标记并手动触发补齐
    vault
        .set_sys_config("property_labels_backfill_v1", "0")
        .unwrap();
    let filled = vault.backfill_missing_property_labels().unwrap();
    assert_eq!(filled, 1, "should backfill one object");

    let updated = vault.load_object("obj-no-labels").unwrap().unwrap();
    let labels = updated
        .property_labels
        .expect("property_labels should be populated")
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(labels.get("name").and_then(|v| v.as_str()), Some("public"));
    assert_eq!(
        labels.get("id_number").and_then(|v| v.as_str()),
        Some("critical")
    );
    assert_eq!(
        labels.get("__dynamic_group__").and_then(|v| v.as_str()),
        Some("sensitive")
    );
}

#[test]
fn test_truncate_preview_properties() {
    // P020：非 `__` 字段截断到前 N 个、`__*` 元数据完整保留、字符串值限长。
    let long = "x".repeat(500);
    let props = serde_json::json!({
        "field1": "v1",
        "field2": "v2",
        "field3": "v3",
        "field4": "v4",
        "field5": "v5",
        "field6": "v6",
        "field7": "v7",
        "field8": "v8",
        "field9": "v9", // 超出 PREVIEW_FIELD_LIMIT，应被截掉
        "__fields": { "field1": { "name": "字段1", "type": "text" } },
        "__templateName": "身份信息",
        "__deprecatedFields": [],
        "huge": long.clone(),
    });
    let out = truncate_preview_properties(&props, None);
    let obj = out.as_object().unwrap();
    // 9 个非 `__` 字段 → 恰好保留 8 个（Map 无序，按计数断言）
    let non_meta: Vec<&String> = obj.keys().filter(|k| !k.starts_with("__")).collect();
    assert_eq!(
        non_meta.len(),
        8,
        "非 __ 字段应截断到 8 个，实际: {non_meta:?}"
    );
    // `__*` 元数据完整保留
    for meta in ["__fields", "__templateName", "__deprecatedFields"] {
        assert!(obj.contains_key(meta), "{meta} 应保留");
    }
    // 所有字符串值限长 200
    for v in obj.values() {
        if let Some(s) = v.as_str() {
            assert!(s.len() <= 200, "字符串值应限长 200");
        }
    }
}

#[test]
fn test_truncate_preview_properties_field_order_priority() {
    // P020 二次复核：提供模板 fieldOrder 时，截断优先按模板顺序选取字段——
    // 模板首位重要字段（字母序靠后）不再被截掉；不足 8 个时再按 Map 序补足。
    let props = serde_json::json!({
        "a_field": "a",
        "b_field": "b",
        "zz_top": "important", // 模板首位，字母序最后
        "c_field": "c",
        "__fields": {},
    });
    let order = vec![
        "zz_top".to_string(),
        "a_field".to_string(),
        "b_field".to_string(),
        "c_field".to_string(),
    ];
    let out = truncate_preview_properties(&props, Some(&order));
    let obj = out.as_object().unwrap();
    // 模板首位字段必须保留（即使字母序靠后）
    assert!(obj.contains_key("zz_top"), "模板首位字段应优先保留");
    assert_eq!(obj.get("zz_top").unwrap(), "important");
    // 全部非 __ 字段都应保留（4 个 < 8 上限）
    for k in ["a_field", "b_field", "c_field"] {
        assert!(obj.contains_key(k), "{k} 应保留");
    }
    assert!(obj.contains_key("__fields"));

    // 超限场景：order 优先填满 8 个，Map 序字段被挤掉
    let props2 = serde_json::json!({
        "f01": "1", "f02": "2", "f03": "3", "f04": "4", "f05": "5",
        "f06": "6", "f07": "7", "f08": "8", "f09": "9", "f10": "10",
    });
    let order2: Vec<String> = (1..=10).map(|i| format!("f{i:02}")).collect();
    let out2 = truncate_preview_properties(&props2, Some(&order2));
    let obj2 = out2.as_object().unwrap();
    let non_meta2: Vec<&String> = obj2.keys().filter(|k| !k.starts_with("__")).collect();
    assert_eq!(non_meta2.len(), 8);
    assert!(obj2.contains_key("f01"), "order 首位应保留");
    assert!(obj2.contains_key("f08"));
    assert!(!obj2.contains_key("f09") && !obj2.contains_key("f10"));
}
