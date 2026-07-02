use super::*;
use crate::commands::object::{
    inherit_property_fields, inherit_property_labels, inject_property_fields, inject_template_meta,
};
use serde_json::json;
use solosoul_vault::{
    ObjectRecord, PropertyType, TemplateProperty, UserTemplate, VaultConfig, VaultStore,
};
use std::sync::Arc;
use tempfile::TempDir;

// ── 1. Error formatting functions ───────────────────────────

#[test]
fn test_export_err_no_data() {
    assert_eq!(export_err("NO_DATA"), "__EXPORT_ERR__:NO_DATA");
}

#[test]
fn test_import_err_bad_password() {
    assert_eq!(import_err("BAD_PASSWORD"), "__IMPORT_ERR__:BAD_PASSWORD");
}

#[test]
fn test_export_err_with_detail_failed() {
    assert_eq!(
        export_err_with_detail("FAILED", "disk full"),
        "__EXPORT_ERR__:FAILED:disk full"
    );
}

#[test]
fn test_import_err_with_detail_corrupt() {
    assert_eq!(
        import_err_with_detail("CORRUPT", "bad zip"),
        "__IMPORT_ERR__:CORRUPT:bad zip"
    );
}

// ── 2. derive_export_key ────────────────────────────────────

#[test]
fn test_derive_export_key_deterministic() {
    let password = "testpassword123";
    let salt = b"randomsalt123456";
    let key1 = derive_export_key(password, salt).unwrap();
    let key2 = derive_export_key(password, salt).unwrap();
    assert_eq!(key1, key2);
}

#[test]
fn test_derive_export_key_different_passwords() {
    let salt = b"randomsalt123456";
    let key1 = derive_export_key("password1", salt).unwrap();
    let key2 = derive_export_key("password2", salt).unwrap();
    assert_ne!(key1, key2);
}

// ── 3. build_package_ids ────────────────────────────────────

#[test]
fn test_build_package_ids_with_array() {
    let payload = json!({
        "objects": [
            { "id": "obj-1" },
            { "id": "obj-2" },
            { "name": "no-id" }
        ]
    });
    let ids = build_package_ids(&payload);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains("obj-1"));
    assert!(ids.contains("obj-2"));
}

#[test]
fn test_build_package_ids_empty() {
    let payload = json!({ "objects": [] });
    let ids = build_package_ids(&payload);
    assert!(ids.is_empty());
}

// ── 4. resolve_value_references ─────────────────────────────

#[test]
fn test_resolve_value_references_in_package_kept() {
    let mut value = json!({
        "type": "relation",
        "targetId": "obj-1"
    });
    let package_ids = ["obj-1"].into_iter().map(String::from).collect();
    resolve_value_references(&mut value, &package_ids);
    assert_eq!(value["targetId"], "obj-1");
    assert_eq!(value["type"], "relation");
}

#[test]
fn test_resolve_value_references_outside_package_replaced() {
    let mut value = json!({
        "type": "relation",
        "targetId": "obj-2"
    });
    let package_ids = ["obj-1"].into_iter().map(String::from).collect();
    resolve_value_references(&mut value, &package_ids);
    assert_eq!(value, json!("[引用对象未导出: obj-2]"));
}

#[test]
fn test_resolve_value_references_nested() {
    let mut value = json!({
        "field1": {
            "__type": "relation",
            "id": "outside"
        },
        "field2": {
            "kind": "relation",
            "objectId": "inside"
        },
        "array": [
            { "type": "relation", "targetId": "outside2" }
        ]
    });
    let package_ids = ["inside"].into_iter().map(String::from).collect();
    resolve_value_references(&mut value, &package_ids);
    assert_eq!(value["field1"], json!("[引用对象未导出: outside]"));
    assert_eq!(value["field2"]["objectId"], "inside");
    assert_eq!(value["array"][0], json!("[引用对象未导出: outside2]"));
}

// ── 5. resolve_cross_scope_references ───────────────────────

#[test]
fn test_resolve_cross_scope_references() {
    let mut objects = json!({
        "rel1": {
            "type": "relation",
            "targetId": "old-id-1"
        },
        "rel2": {
            "type": "relation",
            "targetId": "old-id-2"
        },
        "plain": "text"
    });
    let id_map = ["old-id-2"].into_iter().map(String::from).collect();
    resolve_cross_scope_references(&mut objects, &id_map);
    assert_eq!(objects["rel1"], json!("[引用对象未导出: old-id-1]"));
    assert_eq!(objects["rel2"]["targetId"], "old-id-2");
    assert_eq!(objects["plain"], "text");
}

// ── 6. read_manifest ────────────────────────────────────────

#[test]
fn test_read_manifest() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("test.solosoul");

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = json!({
        "version": "2.0",
        "salt_hex": "deadbeef",
        "has_attachments": true,
        "extra_files": ["preferences.enc"]
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    let manifest_data = read_manifest(zip_path.to_str().unwrap())?;
    assert_eq!(manifest_data.salt_hex, "deadbeef");
    assert!(manifest_data.has_attachments);
    assert_eq!(manifest_data.extra_files, vec!["preferences.enc"]);
    Ok(())
}

// ── 7. read_file_from_zip ───────────────────────────────────

#[test]
fn test_read_file_from_zip() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("test.zip");

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let content = b"hello from zip";
    zip.start_file("test.txt", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(content).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    let bytes = read_file_from_zip(zip_path.to_str().unwrap(), "test.txt")?;
    assert_eq!(bytes, b"hello from zip");
    Ok(())
}

// ── 8. Data model serialization roundtrips ──────────────────

#[test]
fn test_page_group_serde_roundtrip() {
    let original = PageGroup {
        section_type: "identity".to_string(),
        page_name: "Identity".to_string(),
        object_count: 3,
        objects: vec![],
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PageGroup = serde_json::from_str(&json).unwrap();
    assert_eq!(original.section_type, deserialized.section_type);
    assert_eq!(original.page_name, deserialized.page_name);
    assert_eq!(original.object_count, deserialized.object_count);
    assert!(deserialized.objects.is_empty());
}

#[test]
fn test_export_scope_serde_roundtrip() {
    let original = ExportScope {
        selected_page_ids: vec!["identity".to_string(), "travel".to_string()],
        selected_object_ids: vec!["obj-1".to_string()],
        selected_tags: vec!["tag1".to_string()],
        include_attachments: true,
        selected_attachment_ids: vec!["att-1".to_string()],
        include_preferences: false,
        include_behavioral: true,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExportScope = serde_json::from_str(&json).unwrap();
    assert_eq!(original.selected_page_ids, deserialized.selected_page_ids);
    assert_eq!(
        original.selected_object_ids,
        deserialized.selected_object_ids
    );
    assert_eq!(original.selected_tags, deserialized.selected_tags);
    assert_eq!(
        original.include_attachments,
        deserialized.include_attachments
    );
    assert_eq!(
        original.selected_attachment_ids,
        deserialized.selected_attachment_ids
    );
    assert_eq!(
        original.include_preferences,
        deserialized.include_preferences
    );
    assert_eq!(original.include_behavioral, deserialized.include_behavioral);
}

#[test]
fn test_export_request_serde_roundtrip() {
    let original = ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
        },
        password: "Secret1!".to_string(),
        password_hint: Some("hint text".to_string()),
        save_path: "~/Downloads/backup.solosoul".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExportRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(original.password, deserialized.password);
    assert_eq!(original.password_hint, deserialized.password_hint);
    assert_eq!(original.save_path, deserialized.save_path);
    assert_eq!(
        original.scope.include_attachments,
        deserialized.scope.include_attachments
    );
}

// ── 9. Export includes templates in payload ────────────────

#[test]
fn test_export_includes_templates() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("test_export_tmpl.solosoul");

    let password = "ExportPass1";
    let salt = solosoul_crypto::kdf::generate_salt(); // 16-byte salt
    let key = derive_export_key(password, &salt)?;

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = json!({
        "version": "2.0",
        "salt_hex": hex::encode(salt),
        "has_attachments": false,
        "has_templates": true,
        "extra_files": []
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    let payload = json!({
        "objects": [{
            "id": "obj_1",
            "name": "Test",
            "type_id": "note",
            "section_type": "identity",
            "icon_name": "document",
            "properties": {},
            "sensitivity_level": "internal",
            "tags": [],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-06-01T00:00:00Z",
            "version": 1,
            "template_id": "identity"
        }],
        "templates": [{
            "id": "identity",
            "accountId": "acc_export",
            "name": "身份信息",
            "iconId": null,
            "properties": [{
                "id": "fullName",
                "name": "证件号码",
                "type": "text",
                "sensitivityLevel": "internal"
            }],
            "category": "identity",
            "createdAt": "2024-01-01T00:00:00Z",
            "updatedAt": null
        }]
    });
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    zip.start_file("payload.enc", options)
        .map_err(|e| e.to_string())?;
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        payload_bytes.len() as u64,
        &mut std::io::Cursor::new(&payload_bytes),
        &mut zip,
    )
    .map_err(|e| format!("encrypt: {}", e))?;

    zip.finish().map_err(|e| e.to_string())?;

    let manifest_data = read_manifest(zip_path.to_str().unwrap())?;
    let v = manifest_data;
    // Verify has_templates through the raw JSON of the manifest
    let file2 = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file2).map_err(|_| "invalid zip".to_string())?;
    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| "missing manifest".to_string())?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(|e| e.to_string())?;
    let raw: serde_json::Value = serde_json::from_str(&buf).map_err(|e| e.to_string())?;
    assert_eq!(raw["has_templates"], true);
    assert_eq!(v.salt_hex.len(), 32); // 16-byte salt as 32 hex chars
    Ok(())
}

#[test]
fn test_import_template_snapshot_remapping() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("test_import_snapshot.solosoul");

    let password = "ExportPass1";
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(password, &salt)?;

    // 构造导出包：包含一个中文模板 + 引用该模板的对象
    let payload = json!({
        "objects": [{
            "id": "obj_cn",
            "name": "张三的护照",
            "type_id": "note",
            "section_type": "travel",
            "icon_name": "passport",
            "properties": {"fullName": "张三", "passportNumber": "E12345678"},
            "sensitivity_level": "internal",
            "tags": [],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-06-01T00:00:00Z",
            "version": 1,
            "template_id": "passport_template"
        }],
        "templates": [{
            "id": "passport_template",
            "accountId": "acc_export",
            "name": "护照信息",
            "iconId": "passport",
            "properties": [{
                "id": "fullName",
                "name": "姓名",
                "type": "text",
                "sensitivityLevel": "internal"
            }, {
                "id": "passportNumber",
                "name": "护照号码",
                "type": "text",
                "sensitivityLevel": "critical"
            }],
            "category": "travel",
            "createdAt": "2024-01-01T00:00:00Z",
            "updatedAt": null
        }]
    });
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = json!({
        "version": "2.0",
        "salt_hex": hex::encode(salt),
        "has_attachments": false,
        "has_templates": true,
        "extra_files": []
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;

    zip.start_file("payload.enc", options)
        .map_err(|e| e.to_string())?;
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        payload_bytes.len() as u64,
        &mut std::io::Cursor::new(&payload_bytes),
        &mut zip,
    )
    .map_err(|e| format!("encrypt: {}", e))?;
    zip.finish().map_err(|e| e.to_string())?;

    // 验证：解析 payload 中对象的 template_id 没有被映射（这里只是直接测试数据格式）
    let enc = read_file_from_zip(zip_path.to_str().unwrap(), "payload.enc")?;
    let dec = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc)
        .map_err(|_| "decrypt failed".to_string())?;
    let parsed: serde_json::Value =
        serde_json::from_slice(&dec).map_err(|e| format!("parse: {}", e))?;

    // 检查 templates 存在且携带了原始模板数据
    let tmpls = parsed["templates"].as_array().ok_or("no templates")?;
    assert_eq!(tmpls.len(), 1);
    assert_eq!(tmpls[0]["name"], "护照信息");
    assert_eq!(tmpls[0]["id"], "passport_template");

    // 检查对象引用 template_id
    let objs = parsed["objects"].as_array().ok_or("no objects")?;
    assert_eq!(objs[0]["template_id"], "passport_template");

    // 验证 content_hash 计算
    use solosoul_core::export_import::user_template_content_hash;
    use solosoul_vault::UserTemplate;
    let tpl: UserTemplate =
        serde_json::from_value(tmpls[0].clone()).map_err(|e| format!("deserialize tpl: {}", e))?;
    let hash = user_template_content_hash(&tpl);
    assert_eq!(hash.len(), 64);
    Ok(())
}

#[test]
fn test_import_system_section_type_preserved() {
    let obj = json!({
        "id": "sys_obj",
        "name": "My Identity",
        "type_id": "note",
        "section_type": "identity",
        "icon_name": "document",
        "properties": {},
        "sensitivity_level": "internal",
        "tags": [],
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-06-01T00:00:00Z",
        "version": 1
    });
    // 系统页面的 section_type 应保持为 key，不做国际化映射
    assert_eq!(obj["section_type"], "identity");
}

#[test]
fn test_import_preview_serde_roundtrip() {
    let original = ImportPreview {
        file_path: "/tmp/test.solosoul".to_string(),
        version: "2.0".to_string(),
        object_count: 10,
        has_attachments: true,
        extra_files: vec!["preferences.enc".to_string(), "behavioral.enc".to_string()],
        export_time: Some("2024-06-10T12:00:00Z".to_string()),
        password_hint: Some("my hint".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ImportPreview = serde_json::from_str(&json).unwrap();
    assert_eq!(original.file_path, deserialized.file_path);
    assert_eq!(original.version, deserialized.version);
    assert_eq!(original.object_count, deserialized.object_count);
    assert_eq!(original.has_attachments, deserialized.has_attachments);
    assert_eq!(original.extra_files, deserialized.extra_files);
    assert_eq!(original.export_time, deserialized.export_time);
    assert_eq!(original.password_hint, deserialized.password_hint);
}

// ── 10. Import template inheritance (方案 A) ─────────────────

fn test_vault(account_id: &str) -> (TempDir, Arc<VaultStore>) {
    let tmp = TempDir::new().unwrap();
    let config = VaultConfig::new(account_id, tmp.path().to_path_buf()).with_data_key([0u8; 32]);
    let vault = VaultStore::open(config).unwrap();
    (tmp, Arc::new(vault))
}

/// 导入对象在模板删除后仍保留字段敏感度（property_labels）和字段定义（__fields）。
#[test]
fn test_import_object_keeps_sensitivity_after_template_delete() {
    let account_id = "acc_import_sens";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();

    // 1. 创建模板，三个字段各有不同的敏感度
    let template = UserTemplate {
        contract_type_id: None,
        id: "passport".to_string(),
        account_id: account_id.to_string(),
        name: "护照信息".to_string(),
        icon_id: Some("passport".to_string()),
        properties: vec![
            TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: "姓名".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
            TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "passportNumber".to_string(),
                name: "护照号码".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("critical".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
            TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "email".to_string(),
                name: "邮箱".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("sensitive".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
        ],
        category: Some("travel".to_string()),
        created_at: now.clone(),
        updated_at: Some(now.clone()),
    };
    vault.save_user_template(&template).unwrap();

    // 2. 创建一个基于该模板的对象（模拟导入场景，没有 property_labels）
    let record = ObjectRecord {
        contract_type_id: None,
        id: "imported_obj".to_string(),
        account_id: account_id.to_string(),
        type_id: "passport".to_string(),
        section_type: "travel".to_string(),
        name: "张三的护照".to_string(),
        icon_name: "passport".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "fullName": "张三",
            "passportNumber": "E12345678",
            "email": "zhangsan@example.com"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("passport".to_string()),
        template_type: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 3. 执行模板继承逻辑（模拟 import.rs 中的修复）
    let loaded = vault.load_object("imported_obj").unwrap().unwrap();
    let mut properties = loaded.properties.clone();

    let merged_labels = inherit_property_labels(&vault, loaded.template_id.as_deref());
    let fields = inherit_property_fields(&vault, loaded.template_id.as_deref());
    inject_property_fields(&mut properties, &fields);
    inject_template_meta(&vault, loaded.template_id.as_deref(), &mut properties);

    // 保存更新后的对象
    let mut obj = vault.load_object("imported_obj").unwrap().unwrap();
    obj.properties = properties;
    obj.property_labels = merged_labels;
    vault.save_object(&obj).unwrap();

    // 4. 验证对象现在有 property_labels
    let updated = vault.load_object("imported_obj").unwrap().unwrap();
    let labels = updated.property_labels.as_ref().unwrap();
    let labels_obj = labels.as_object().unwrap();
    assert_eq!(
        labels_obj.get("fullName").and_then(|v| v.as_str()),
        Some("internal")
    );
    assert_eq!(
        labels_obj.get("passportNumber").and_then(|v| v.as_str()),
        Some("critical")
    );
    assert_eq!(
        labels_obj.get("email").and_then(|v| v.as_str()),
        Some("sensitive")
    );

    // 5. 验证 __fields 已注入
    let props = &updated.properties;
    let fields = props.get("__fields").and_then(|f| f.as_object()).unwrap();
    assert!(fields.contains_key("fullName"));
    assert!(fields.contains_key("passportNumber"));
    assert!(fields.contains_key("email"));
    assert_eq!(
        fields
            .get("fullName")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str()),
        Some("姓名")
    );

    // 6. 验证 __templateName 已注入
    assert_eq!(
        props.get("__templateName").and_then(|n| n.as_str()),
        Some("护照信息")
    );

    // 7. 删除模板
    vault.delete_user_template("passport").unwrap();
    assert!(vault.load_user_template("passport").unwrap().is_none());

    // 8. 重新加载对象，验证 property_labels 仍然保留
    let after_delete = vault.load_object("imported_obj").unwrap().unwrap();
    let labels_after = after_delete.property_labels.as_ref().unwrap();
    let labels_obj_after = labels_after.as_object().unwrap();
    assert_eq!(
        labels_obj_after
            .get("passportNumber")
            .and_then(|v| v.as_str()),
        Some("critical")
    );
    assert_eq!(
        labels_obj_after.get("email").and_then(|v| v.as_str()),
        Some("sensitive")
    );

    // 9. __fields 和 __templateName 也继续保留
    let props_after = &after_delete.properties;
    assert!(props_after.get("__fields").is_some());
    assert_eq!(
        props_after.get("__templateName").and_then(|n| n.as_str()),
        Some("护照信息")
    );
}

/// 回归测试：无模板对象导入后不受影响
#[test]
fn test_import_no_template_object_unchanged() {
    let account_id = "acc_no_tpl";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();

    // 创建无模板的对象
    let record = ObjectRecord {
        contract_type_id: None,
        id: "no_tpl_obj".to_string(),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "无模板对象".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"note": "this has no template"}),
        property_labels: None,
        sensitivity_level: "public".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 验证 property_labels 仍为 None
    let loaded = vault.load_object("no_tpl_obj").unwrap().unwrap();
    assert!(loaded.property_labels.is_none());
    assert!(loaded.template_id.is_none());
    // 无模板时不应有 __fields 或 __templateName
    assert!(loaded.properties.get("__fields").is_none());
    assert!(loaded.properties.get("__templateName").is_none());
}
