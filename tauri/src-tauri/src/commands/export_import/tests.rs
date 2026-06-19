use super::*;
use serde_json::json;
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
