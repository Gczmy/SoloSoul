use super::*;
use solosoul_core::export_import::{imported_template_id, user_template_content_hash};
use std::sync::RwLock;

fn incoming_template() -> UserTemplate {
    UserTemplate {
        id: "rf018-template".into(),
        account_id: "acc_source".into(),
        name: "Imported template".into(),
        icon_id: None,
        properties: vec![],
        category: None,
        contract_type_id: None,
        created_at: "2026-09-25T00:00:00Z".into(),
        updated_at: None,
    }
}

fn reject_template_writes(db: &rusqlite::Connection) {
    db.execute_batch(
        "CREATE TRIGGER rf018_reject_template BEFORE INSERT ON user_templates
         BEGIN SELECT RAISE(ABORT, 'rf018 injected template write failure'); END;",
    )
    .unwrap();
}

fn package_with_template(path: &std::path::Path, template: &UserTemplate) {
    let salt = solosoul_crypto::kdf::generate_salt();
    // 显式使用旧包约定的 balanced，不依赖进程环境中的 KDF 配置。
    let key = derive_export_key_cfg(
        "export-password",
        &salt,
        &solosoul_crypto::kdf::KdfConfig::balanced(),
    )
    .unwrap();
    let manifest = json!({
        "version": "2.0", "salt_hex": hex::encode(salt),
        "has_attachments": false, "extra_files": []
    });
    let payload = serde_json::to_vec(&json!({
        "templates": [template],
        "objects": [{
            "id": "rf018-object", "name": "Imported object", "type_id": "note",
            "section_type": "identity", "template_id": template.id, "properties": {}
        }]
    }))
    .unwrap();
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    zip.start_file("payload.enc", options).unwrap();
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        payload.len() as u64,
        &mut std::io::Cursor::new(payload),
        &mut zip,
    )
    .unwrap();
    zip.finish().unwrap();
}

#[test]
fn rf018_template_write_failures_abort_before_objects() {
    for conflict in [false, true] {
        let dir = TempDir::new().unwrap();
        let account = format!("acc_{}", Uuid::new_v4().simple());
        let service = RwLock::new(solosoul_core::VaultService::with_base_path(
            dir.path().join("vault"),
        ));
        service
            .read()
            .unwrap()
            .create_account_with_id(&account, "Test", "password123", None)
            .unwrap();
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let template = incoming_template();
        if conflict {
            let mut local = template.clone();
            local.account_id = account.clone();
            local.name = "Local template".into();
            vault.save_user_template(&local).unwrap();
        }
        let before = vault.list_user_templates(&account).unwrap();
        let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
        reject_template_writes(&db);

        // 映射构建必须整体返回错误，不能发布一个实际未保存的模板 ID。
        let error = rebuild_imported_templates(&vault, &account, &json!({"templates": [template]}))
            .unwrap_err();
        assert_eq!(error, "import_batch_templates_failed");
        assert!(!error.contains("rf018 injected template write failure"));

        // 走真实解密与导入入口，确认失败不会继续写引用该模板的对象。
        let package = dir.path().join("incoming.solosoul");
        package_with_template(&package, &template);
        let outcome = import_execute_internal(
            service.read().unwrap(),
            account.clone(),
            package.to_string_lossy().into_owned(),
            Zeroizing::new("export-password".into()),
            ImportStrategy::SkipExisting,
            None,
            None,
            HashMap::new(),
            "en-US",
            None,
        )
        .unwrap();
        assert_eq!(outcome.status, ImportStatus::NotCommitted);
        assert_eq!(outcome.failure_stage, Some(ImportStage::Templates));
        assert!(vault.load_object("rf018-object").unwrap().is_none());
        assert_eq!(
            serde_json::to_value(vault.list_user_templates(&account).unwrap()).unwrap(),
            serde_json::to_value(before).unwrap()
        );
    }
}

#[test]
fn rf018_template_read_failures_are_not_absence() {
    for (conflict, owner) in [
        (false, "acc_target"),
        (true, "acc_target"),
        (false, "acc_other"),
        (true, "acc_other"),
    ] {
        let (_dir, vault) = test_vault("acc_target");
        let template = incoming_template();
        let hash = user_template_content_hash(&template);
        let corrupt_id = if conflict {
            let mut local = template.clone();
            local.account_id = "acc_target".into();
            local.name = "Local template".into();
            vault.save_user_template(&local).unwrap();
            imported_template_id(&template.id, &hash)
        } else {
            template.id.clone()
        };
        // 同账户坏行必须返回读取错误；跨账户同主键必须拒绝覆盖。
        let mut corrupt = template.clone();
        corrupt.id = corrupt_id.clone();
        corrupt.account_id = owner.into();
        vault.save_user_template(&corrupt).unwrap();
        let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
        db.execute(
            "UPDATE user_templates SET properties_json = X'00' WHERE id = ?1",
            [&corrupt_id],
        )
        .unwrap();
        let error =
            rebuild_imported_templates(&vault, "acc_target", &json!({"templates": [template]}))
                .unwrap_err();
        if owner == "acc_target" {
            assert!(error.contains("list_user_templates row"), "{error}");
        } else {
            assert_eq!(error, "import_batch_account_mismatch");
        }
        let unchanged: Vec<u8> = db
            .query_row(
                "SELECT properties_json FROM user_templates WHERE id = ?1",
                [&corrupt_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unchanged, vec![0]);
    }
}

#[test]
fn rf018_content_hash_reuse_requires_no_write() {
    let (_dir, vault) = test_vault("acc_target");
    let template = incoming_template();
    let mut local = template.clone();
    local.id = "existing-template".into();
    local.account_id = "acc_target".into();
    vault.save_user_template(&local).unwrap();
    let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
    reject_template_writes(&db);
    let map = rebuild_imported_templates(&vault, "acc_target", &json!({"templates": [template]}))
        .unwrap();
    assert_eq!(map.get(&template.id), Some(&local.id));
    assert_eq!(vault.count_user_templates("acc_target").unwrap(), 1);
}
