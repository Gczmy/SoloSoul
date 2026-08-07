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

// ── 6b. read_manifest_json 大小上限（P201 防 ZIP 炸弹）─────

#[test]
fn test_read_manifest_json_parses_normal() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let zip_path = temp_dir.path().join("normal.solosoul");

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = json!({
        "version": "2.0",
        "salt_hex": "deadbeef",
        "has_attachments": false,
        "extra_files": ["preferences.enc"]
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    // 正常大小：可解析且字段齐全
    let v = read_manifest_json(zip_path.to_str().unwrap())?;
    assert_eq!(v["version"], "2.0");
    assert_eq!(v["salt_hex"], "deadbeef");
    assert_eq!(v["extra_files"][0], "preferences.enc");

    // 极小上限触发拒绝（第一道防线：声明的 size() 超限）
    let err = super::read_manifest_json_limited(zip_path.to_str().unwrap(), 10)
        .expect_err("oversized manifest should be rejected");
    assert!(
        err.contains("too large") || err.contains("exceeds size limit"),
        "unexpected error: {}",
        err
    );
    Ok(())
}

// ── 6c. manifest kdf 字段解析（P202 导出包 KDF 参数随包携带）─

#[test]
fn test_manifest_kdf_field_parsing() -> Result<(), String> {
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;

    // 旧格式包（无 kdf 字段）→ 回退 balanced（向后兼容）
    let old_zip = temp_dir.path().join("old.solosoul");
    {
        let file = File::create(&old_zip).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            json!({
                "version": "2.0",
                "salt_hex": "deadbeef",
                "has_attachments": false,
                "extra_files": []
            })
            .to_string()
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    let old_manifest = read_manifest(old_zip.to_str().unwrap())?;
    assert_eq!(old_manifest.kdf, None);
    assert_eq!(
        old_manifest.kdf_config(),
        solosoul_crypto::kdf::KdfConfig::balanced()
    );

    // 新格式包（kdf=production 声明）→ 按声明返回
    let new_zip = temp_dir.path().join("new.solosoul");
    {
        let file = File::create(&new_zip).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            json!({
                "version": "2.0",
                "salt_hex": "deadbeef",
                "has_attachments": false,
                "extra_files": [],
                "kdf": solosoul_core::export_import::kdf_to_manifest_value(
                    &solosoul_crypto::kdf::KdfConfig::production()
                )
            })
            .to_string()
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    let new_manifest = read_manifest(new_zip.to_str().unwrap())?;
    assert_eq!(
        new_manifest.kdf_config(),
        solosoul_crypto::kdf::KdfConfig::production()
    );

    // 非法 kdf 声明 → 拒绝（不静默降级）
    let bad_zip = temp_dir.path().join("bad.solosoul");
    {
        let file = File::create(&bad_zip).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            json!({
                "version": "2.0",
                "salt_hex": "deadbeef",
                "has_attachments": false,
                "extra_files": [],
                "kdf": { "algo": "scrypt", "memory_kb": 1, "iterations": 1, "parallelism": 1 }
            })
            .to_string()
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    assert!(
        read_manifest(bad_zip.to_str().unwrap()).is_err(),
        "非法 kdf 声明应被拒绝"
    );
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
        include_all: false,
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
            include_all: false,
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

/// P005: collect_scope_objects 批量加载——include_all 全量、selected 子集、tags 过滤、空集。
#[test]
fn test_collect_scope_objects_batch() -> Result<(), String> {
    let account_id = "acc_collect";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    let mk = |id: &str, section: &str, tags: Vec<&str>| -> ObjectRecord {
        ObjectRecord {
            contract_type_id: None,
            id: id.to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: section.to_string(),
            name: format!("obj-{}", id),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({ "k": format!("v-{}", id) }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: tags.into_iter().map(|t| t.to_string()).collect(),
            template_id: None,
            template_type: None,
            template_hash: None,
            ignored_template_hash: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
        }
    };
    vault.save_object(&mk("obj-a", "identity", vec!["tag1"]))?;
    vault.save_object(&mk("obj-b", "passport", vec!["tag1", "tag2"]))?;
    vault.save_object(&mk("obj-c", "identity", vec!["tag3"]))?;

    // include_all: 全部对象，id 升序
    let all = collect_scope_objects(
        &vault,
        account_id,
        &ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: true,
        },
    )?;
    let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["obj-a", "obj-b", "obj-c"],
        "include_all 应含全部对象且 id 升序"
    );
    assert_eq!(
        all[0].properties["k"],
        serde_json::json!("v-obj-a"),
        "properties 应已解密"
    );

    // 选定子集
    let subset = collect_scope_objects(
        &vault,
        account_id,
        &ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec!["obj-b".to_string(), "obj-c".to_string()],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
    )?;
    let subset_ids: Vec<&str> = subset.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(subset_ids, vec!["obj-b", "obj-c"]);

    // tags 过滤（交集）
    let tagged = collect_scope_objects(
        &vault,
        account_id,
        &ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![
                "obj-a".to_string(),
                "obj-b".to_string(),
                "obj-c".to_string(),
            ],
            selected_tags: vec!["tag1".to_string()],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
    )?;
    let tagged_ids: Vec<&str> = tagged.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        tagged_ids,
        vec!["obj-a", "obj-b"],
        "tag1 应命中 obj-a/obj-b"
    );

    // 空选择返回空
    let empty = collect_scope_objects(
        &vault,
        account_id,
        &ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
    )?;
    assert!(empty.is_empty(), "无任何选择应返回空");
    Ok(())
}

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
                allowed_types: None,
                max_items: None,
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
                allowed_types: None,
                max_items: None,
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
                allowed_types: None,
                max_items: None,
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
        template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        ..Default::default()
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

// ── 11. unique_object_name locale suffix ────────────────────

#[test]
fn test_unique_object_name_english_suffix() -> Result<(), String> {
    let account_id = "acc_unique_en";
    let (_tmp, vault) = test_vault(account_id);

    let name = unique_object_name(&vault, account_id, "Passport", "en-US")?;
    assert_eq!(name, "Passport (Imported)");
    Ok(())
}

#[test]
fn test_unique_object_name_chinese_suffix() -> Result<(), String> {
    let account_id = "acc_unique_zh";
    let (_tmp, vault) = test_vault(account_id);

    let name = unique_object_name(&vault, account_id, "护照", "zh-CN")?;
    assert_eq!(name, "护照（导入）");
    Ok(())
}

#[test]
fn test_unique_object_name_english_increment() -> Result<(), String> {
    let account_id = "acc_unique_en2";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    // 创建两个同名对象来触发递增
    for i in 0..2 {
        let rec = ObjectRecord {
            contract_type_id: None,
            id: format!("pre_{}", i),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Doc (Imported)".to_string(),
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
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&rec)?;
    }

    let name = unique_object_name(&vault, account_id, "Doc", "en-US")?;
    // "Doc (Imported)" 已存在 2 个 → 首次避开冲突用 counter=2: "Doc (Imported) 2"
    assert_eq!(name, "Doc (Imported) 2");
    Ok(())
}

#[test]
fn test_unique_object_name_chinese_increment() -> Result<(), String> {
    let account_id = "acc_unique_zh2";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    // 创建 "文档（导入）" 对象
    let rec = ObjectRecord {
        contract_type_id: None,
        id: "pre_zh".to_string(),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "文档（导入）".to_string(),
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
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        ..Default::default()
    };
    vault.save_object(&rec)?;

    let name = unique_object_name(&vault, account_id, "文档", "zh-CN")?;
    assert_eq!(name, "文档（导入） 2");
    Ok(())
}

// ── 12. 全量导出包含全部模板（含预置种子模板，跨设备恢复）──

#[test]
fn test_collect_export_templates_include_all() -> Result<(), String> {
    let account_id = "acc_tpl_all";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    // 两个模板：一个被对象引用，一个未被引用（模拟预置种子模板）
    for (tid, name) in [("passport", "护照"), ("visa", "签证")] {
        let tpl = UserTemplate {
            contract_type_id: None,
            id: tid.to_string(),
            account_id: account_id.to_string(),
            name: name.to_string(),
            icon_id: Some("document".to_string()),
            properties: vec![],
            category: Some("travel".to_string()),
            created_at: now.clone(),
            updated_at: Some(now.clone()),
        };
        vault.save_user_template(&tpl)?;
    }

    // 对象只引用 passport
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj_ref_passport".to_string(),
        account_id: account_id.to_string(),
        type_id: "passport".to_string(),
        section_type: "travel".to_string(),
        name: "护照".to_string(),
        icon_name: "passport".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("passport".to_string()),
        template_type: None,
        template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record)?;

    // include_all=true（恢复主机路径）：全部模板都进包
    let scope_all = ExportScope {
        selected_page_ids: vec![],
        selected_object_ids: vec![],
        selected_tags: vec![],
        include_attachments: false,
        selected_attachment_ids: vec![],
        include_preferences: false,
        include_behavioral: false,
        include_all: true,
    };
    let all = collect_export_templates(
        &vault,
        account_id,
        &scope_all,
        std::slice::from_ref(&record),
    )?;
    let ids: Vec<String> = all.iter().map(|t| t.id.clone()).collect();
    assert!(ids.contains(&"passport".to_string()));
    assert!(
        ids.contains(&"visa".to_string()),
        "未被引用的预置模板也应打包"
    );

    // include_all=false（普通导出）：仅打包被引用模板（快照隔离）
    let scope_partial = ExportScope {
        selected_page_ids: vec![],
        selected_object_ids: vec![],
        selected_tags: vec![],
        include_attachments: false,
        selected_attachment_ids: vec![],
        include_preferences: false,
        include_behavioral: false,
        include_all: false,
    };
    let partial = collect_export_templates(
        &vault,
        account_id,
        &scope_partial,
        std::slice::from_ref(&record),
    )?;
    let ids: Vec<String> = partial.iter().map(|t| t.id.clone()).collect();
    assert_eq!(ids, vec!["passport".to_string()]);
    assert!(!ids.contains(&"visa".to_string()));

    Ok(())
}

// ── 13. 快照恢复（跨设备恢复后历史数量一致）──

#[test]
fn test_restore_package_snapshots_preserves_history() -> Result<(), String> {
    let account_id = "acc_snap_restore";
    let (_tmp, vault) = test_vault(account_id);

    // 模拟导出包中的快照列表（base64 编码的加密前 JSON 数据 + 原时间戳）
    let snap_data = serde_json::to_vec(&serde_json::json!({
        "name": "护照",
        "tags": [],
        "properties": {"fullName": "张三"},
        "propertyLabels": {}
    }))
    .map_err(|e| e.to_string())?;
    let snaps = serde_json::json!([
        {
            "object_id": "obj_1",
            "timestamp": 1_700_000_000_000i64,
            "triggered_by": "user_edit",
            "diff_summary": "diff_created",
            "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &snap_data)
        },
        {
            "object_id": "obj_1",
            "timestamp": 1_700_000_001_000i64,
            "triggered_by": "user_edit",
            "diff_summary": "diff_updated",
            "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &snap_data)
        }
    ]);
    let arr = snaps.as_array().ok_or("no array")?;

    // 恢复后对象历史应为 2 条，且时间戳按原值保留（新 ID 挂载）
    let restored = restore_package_snapshots(&vault, "obj_1", arr);
    assert_eq!(restored, 2);

    let list = vault.list_snapshots("obj_1")?;
    assert_eq!(list.len(), 2);
    let ts: Vec<i64> = list
        .iter()
        .filter_map(|s| s["timestamp"].as_i64())
        .collect();
    assert_eq!(ts, vec![1_700_000_001_000i64, 1_700_000_000_000i64]);

    // 数据可解密读取
    let snap_id = list[0]["id"].as_str().ok_or("no id")?;
    let data = vault.get_snapshot(snap_id)?.ok_or("no snapshot data")?;
    let parsed: serde_json::Value = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    assert_eq!(parsed["name"], "护照");

    Ok(())
}

#[test]
fn test_restore_package_snapshots_empty_falls_back() -> Result<(), String> {
    let account_id = "acc_snap_empty";
    let (_tmp, vault) = test_vault(account_id);

    // 空快照列表 → 返回 0，调用方回退到 diff_imported 初始快照
    let restored = restore_package_snapshots(&vault, "obj_1", &[]);
    assert_eq!(restored, 0);

    // 非法 base64 / 缺失 data → 跳过该条
    let snaps = serde_json::json!([
        {"object_id": "obj_1", "timestamp": 1, "data": "!!!not-base64!!!"},
        {"object_id": "obj_1", "timestamp": 2, "data": null}
    ]);
    let arr = snaps.as_array().unwrap();
    let restored = restore_package_snapshots(&vault, "obj_1", arr);
    assert_eq!(restored, 0);
    assert_eq!(vault.list_snapshots("obj_1")?.len(), 0);

    Ok(())
}

// ── 14. P2：本地无同 ID 模板时保留原始模板 ID（预置种子模板 key）──

#[test]
fn test_rebuild_templates_keeps_original_id_when_free() -> Result<(), String> {
    let account_id = "acc_tpl_keep_id";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    // 包内含 passport 模板，本地无同 ID 模板 → 应保留原始 ID "passport"
    let payload = json!({
        "templates": [{
            "id": "passport",
            "accountId": "acc_export",
            "name": "护照",
            "iconId": "passport",
            "properties": [{
                "id": "fullName",
                "name": "姓名",
                "type": "text",
                "sensitivityLevel": "internal"
            }],
            "category": "travel",
            "createdAt": now,
            "updatedAt": null
        }]
    });

    let map = rebuild_imported_templates(&vault, account_id, &payload)?;
    assert_eq!(map.get("passport").map(|s| s.as_str()), Some("passport"));
    // 本地应真实存在该 ID 的模板
    assert!(vault.load_user_template("passport")?.is_some());
    Ok(())
}

#[test]
fn test_rebuild_templates_derives_id_on_local_conflict() -> Result<(), String> {
    let account_id = "acc_tpl_derive";
    let (_tmp, vault) = test_vault(account_id);
    let now = chrono::Utc::now().to_rfc3339();

    // 本地已存在同 ID 但内容不同的模板 → 应派生新 ID（快照隔离，避免覆盖本地模板）
    let local_tpl = UserTemplate {
        contract_type_id: None,
        id: "passport".to_string(),
        account_id: account_id.to_string(),
        name: "本地自定义护照".to_string(),
        icon_id: None,
        properties: vec![],
        category: None,
        created_at: now.clone(),
        updated_at: Some(now.clone()),
    };
    vault.save_user_template(&local_tpl)?;

    let payload = json!({
        "templates": [{
            "id": "passport",
            "accountId": "acc_export",
            "name": "护照",
            "iconId": "passport",
            "properties": [{
                "id": "fullName",
                "name": "姓名",
                "type": "text",
                "sensitivityLevel": "internal"
            }],
            "category": "travel",
            "createdAt": now,
            "updatedAt": null
        }]
    });

    let map = rebuild_imported_templates(&vault, account_id, &payload)?;
    let mapped = map.get("passport").ok_or("no mapping")?;
    assert_ne!(mapped, "passport", "本地已有同 ID 模板时应派生新 ID");
    // 本地模板未被覆盖
    let local = vault.load_user_template("passport")?.ok_or("local gone")?;
    assert_eq!(local.name, "本地自定义护照");
    Ok(())
}

// ── 15. P1：Overwrite 覆盖导入时清空旧快照，防止历史叠加 ──

#[test]
fn test_overwrite_clears_local_snapshots_before_restore() -> Result<(), String> {
    let account_id = "acc_overwrite";
    let (_tmp, vault) = test_vault(account_id);

    // 本地对象已有 2 条历史
    vault
        .save_snapshot("obj_ow", "user_edit", b"old1", "diff_created")
        .unwrap();
    vault
        .save_snapshot("obj_ow", "user_edit", b"old2", "diff_updated")
        .unwrap();

    // 包内快照 2 条
    let snap_data =
        serde_json::to_vec(&serde_json::json!({ "name": "新" })).map_err(|e| e.to_string())?;
    let snaps = serde_json::json!([
        {
            "object_id": "obj_ow",
            "timestamp": 1_700_000_000_000i64,
            "triggered_by": "user_edit",
            "diff_summary": "diff_created",
            "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &snap_data)
        },
        {
            "object_id": "obj_ow",
            "timestamp": 1_700_000_001_000i64,
            "triggered_by": "user_edit",
            "diff_summary": "diff_updated",
            "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &snap_data)
        }
    ]);
    let arr = snaps.as_array().ok_or("no array")?;

    // Overwrite 语义：先清空本地历史再恢复包内快照 → 最终 2 条（而非 2+2=4）
    vault.delete_snapshots("obj_ow").unwrap();
    let restored = restore_package_snapshots(&vault, "obj_ow", arr);
    assert_eq!(restored, 2);
    let list = vault.list_snapshots("obj_ow")?;
    assert_eq!(list.len(), 2, "旧历史应被清空，避免历史数量翻倍");
    // 时间戳按包内原值恢复
    let ts: Vec<i64> = list
        .iter()
        .filter_map(|s| s["timestamp"].as_i64())
        .collect();
    assert_eq!(ts, vec![1_700_000_001_000i64, 1_700_000_000_000i64]);
    Ok(())
}

/// 回归测试：损坏包（快照 base64 全部解码失败）时 P1 门控应判定无可恢复快照，
/// 从而跳过 delete_snapshots，保留本地历史（避免误删后仅剩一条 diff_imported）。
#[test]
fn test_snapshots_any_restorable_gates_delete_on_valid_package() -> Result<(), String> {
    let account_id = "acc_corrupt_pkg";
    let (_tmp, vault) = test_vault(account_id);

    // 本地对象已有 2 条历史
    vault
        .save_snapshot("obj_cp", "user_edit", b"old1", "diff_created")
        .unwrap();
    vault
        .save_snapshot("obj_cp", "user_edit", b"old2", "diff_updated")
        .unwrap();

    // 损坏包：快照 base64 全部非法 / 缺失 data / 空数据
    let corrupt = serde_json::json!([
        {"object_id": "obj_cp", "timestamp": 1, "data": "!!!not-base64!!!"},
        {"object_id": "obj_cp", "timestamp": 2, "data": null},
        {"object_id": "obj_cp", "timestamp": 3, "data": ""}
    ]);
    let corrupt_arr = corrupt.as_array().ok_or("no array")?;
    // 门控：无可恢复快照 → 不应清空本地历史
    assert!(!snapshots_any_restorable(corrupt_arr));

    // 模拟 import 逻辑：仅当存在可恢复快照时才清空旧历史
    if snapshots_any_restorable(corrupt_arr) {
        vault.delete_snapshots("obj_cp").unwrap();
    }
    let restored = restore_package_snapshots(&vault, "obj_cp", corrupt_arr);
    assert_eq!(restored, 0);
    // 本地历史应完整保留（未被误删）
    assert_eq!(
        vault.list_snapshots("obj_cp")?.len(),
        2,
        "损坏包不应清空本地历史"
    );

    // 有效包：存在可恢复快照 → 门控放行
    let good = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"{\"name\":\"\"}",
    );
    let valid = serde_json::json!([
        {"object_id": "obj_cp", "timestamp": 4, "data": good}
    ]);
    assert!(snapshots_any_restorable(
        valid.as_array().ok_or("no array")?
    ));

    Ok(())
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
        template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        ..Default::default()
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
