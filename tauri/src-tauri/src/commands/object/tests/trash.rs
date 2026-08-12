//! object 命令测试 —— trash（P047 拆分）

use super::super::*;
use super::setup_vault;
use solosoul_vault::{ObjectRecord, Profile, TrashItem};

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
            description: Some("护照扫描件".to_string()),
            tags: vec!["旅行".to_string(), "证件".to_string()],
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
    assert!(json.contains("\"description\":\"护照扫描件\""));
    assert!(json.contains("\"tags\":[\"旅行\",\"证件\"]"));
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
fn test_parse_trash_attachments_carries_description_and_tags() {
    // 真实调用 snapshot.rs 的 parse_trash_attachments（pub(crate)）：
    // __attachments 中携带的 description/tags 必须完整透传到 TrashAttachmentInfo。
    let data = serde_json::json!({
        "properties": {
            "__attachments": [
                {
                    "id": "att-1",
                    "fileName": "photo.png",
                    "mimeType": "image/png",
                    "sizeBytes": 1024,
                    "createdAt": "2024-01-01T00:00:00Z",
                    "deletedAt": null,
                    "description": "护照扫描件",
                    "tags": ["旅行", "证件"]
                },
                {
                    "id": "att-2",
                    "fileName": "legacy.pdf",
                    "mimeType": "application/pdf",
                    "sizeBytes": 2048,
                    "createdAt": "2024-01-02T00:00:00Z",
                    "deletedAt": "2024-02-01T00:00:00Z"
                }
            ]
        }
    });
    let trash = TrashItem {
        id: "trash_parse_1".to_string(),
        item_type: "object".to_string(),
        original_id: "obj-1".to_string(),
        original_parent_id: None,
        original_section_type: Some("identity".to_string()),
        original_sort_order: None,
        data: serde_json::to_vec(&data).unwrap_or_default(),
        deleted_at: 1234567890,
        expires_at: None,
        deleted_by: "user".to_string(),
        name_snapshot: "Parse Test".to_string(),
        icon_snapshot: None,
    };

    let (active, deleted) = super::super::snapshot::parse_trash_attachments(&trash);
    // 活跃附件：description/tags 透传
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].description.as_deref(), Some("护照扫描件"));
    assert_eq!(active[0].tags, vec!["旅行".to_string(), "证件".to_string()]);
    // 旧数据（无 description/tags 键）：安全回退
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].description, None);
    assert!(deleted[0].tags.is_empty());
    // 软删除附件仍按 deletedAt 归入 deleted 桶
    assert_eq!(deleted[0].file_name, "legacy.pdf");
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
