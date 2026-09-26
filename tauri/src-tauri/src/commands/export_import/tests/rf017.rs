use super::rf020::Fixture;
use super::*;
use crate::commands::export_import::export::finalize_export_for_session;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const OBJECT_ID: &str = "rf017-object";
const EXPORT_PASSWORD: &str = "rf017-export-password";

fn request(path: &Path) -> ExportRequest {
    ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![OBJECT_ID.into()],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
        password: EXPORT_PASSWORD.into(),
        password_hint: None,
        save_path: path.to_string_lossy().into_owned(),
    }
}

fn run_export(f: &Fixture, req: &ExportRequest, target: &Path) -> Result<(), String> {
    execute_export_core(
        &f.service.read().unwrap(),
        &f.account,
        req,
        target.to_str().unwrap(),
    )
}

/// 旧目标也由生产入口生成，避免用任意哨兵字节冒充可恢复的备份包。
fn existing_package(f: &Fixture) -> (PathBuf, ExportRequest) {
    f.vault
        .save_object(&ObjectRecord {
            id: OBJECT_ID.into(),
            account_id: f.account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "原始对象".into(),
            properties: json!({"value": "original backup content"}),
            created_at: "2026-09-26T00:00:00Z".into(),
            updated_at: "2026-09-26T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
    let directory = f.dir.path().join("exports");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("keep.txt"), b"unrelated file").unwrap();
    let target = directory.join("既有备份.solosoul");
    let req = request(&target);
    run_export(f, &req, &target).unwrap();
    assert_valid_zip(&target);
    (target, req)
}

fn assert_valid_zip(path: &Path) {
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    assert!(manifest["salt_hex"].is_string());
    assert!(archive.by_name("payload.enc").unwrap().size() > 0);
}

/// 输出目录只有本测试创建的普通文件；多出目录或文件同样视为临时输出残留。
fn directory_bytes(target: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    std::fs::read_dir(target.parent().unwrap())
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

fn successful_exports(f: &Fixture) -> usize {
    f.service
        .read()
        .unwrap()
        .get_vault_store()
        .unwrap()
        .list_audit_log(100)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.action_type == "export_execute")
        .count()
}

fn assert_preserved(
    f: &Fixture,
    target: &Path,
    previous: &BTreeMap<PathBuf, Vec<u8>>,
    previous_audits: usize,
) {
    assert_eq!(&directory_bytes(target), previous);
    assert_valid_zip(target);
    assert_eq!(successful_exports(f), previous_audits);
}

/// 附件内容始终很小；sizeBytes 独立设置以触发生产大小校验。
fn attachment(f: &Fixture, id: &str, size_bytes: u64, contents: &[u8]) -> serde_json::Value {
    let base = f.service.read().unwrap().base_path().to_path_buf();
    let directory = base.join("attachments").join(OBJECT_ID).join(id);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("synthetic.bin");
    std::fs::write(&path, contents).unwrap();
    json!({
        "id": id, "objectId": OBJECT_ID, "fileName": "synthetic.bin",
        "mimeType": "application/octet-stream", "sizeBytes": size_bytes,
        "createdAt": "2026-09-26T00:00:00Z", "vaultPath": path
    })
}

fn select_attachments(f: &Fixture, req: &mut ExportRequest, attachments: Vec<serde_json::Value>) {
    req.scope.include_attachments = true;
    req.scope.selected_attachment_ids = attachments
        .iter()
        .map(|value| value["id"].as_str().unwrap().to_string())
        .collect();
    let mut object = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    object.properties["__attachments"] = serde_json::Value::Array(attachments);
    f.vault.save_object(&object).unwrap();
}

#[test]
fn rf017_success_replaces_valid_package_and_imports_new_content() {
    let f = Fixture::new();
    let (target, req) = existing_package(&f);
    let before = directory_bytes(&target);
    let audit_count = successful_exports(&f);
    let mut object = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    object.name = "替换后的对象".into();
    object.properties = json!({"value": "new encrypted content", "empty": "", "flag": false});
    f.vault.save_object(&object).unwrap();

    run_export(&f, &req, &target).unwrap();
    let after = directory_bytes(&target);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    assert_ne!(after[&target], before[&target]);
    assert_eq!(
        after[&target.parent().unwrap().join("keep.txt")],
        before[&target.parent().unwrap().join("keep.txt")]
    );
    assert_valid_zip(&target);
    assert_eq!(successful_exports(&f), audit_count + 1);

    // 用独立空 Vault 的真实导入入口验证新包可以解密，而非仅 ZIP 头有效。
    let restored = Fixture::new();
    assert!(restored
        .vault
        .list_object_metadata(&restored.account, None, None, false, false)
        .unwrap()
        .is_empty());
    let svc = restored.service.read().unwrap();
    let session = svc.capture_session(&restored.account).unwrap();
    let result = import_execute_for_session(
        &svc,
        &session,
        target.to_string_lossy().into_owned(),
        Zeroizing::new(EXPORT_PASSWORD.into()),
        ImportStrategy::Overwrite,
        None,
        None,
        HashMap::new(),
        "en-US",
        None,
    )
    .unwrap();
    assert_eq!(result.status, ImportStatus::Complete, "{result:?}");
    assert_eq!(result.object_count, 1);
    let imported = restored.vault.load_object(OBJECT_ID).unwrap().unwrap();
    assert_eq!(imported.account_id, restored.account);
    assert_eq!(imported.name, object.name);
    assert_eq!(imported.properties, object.properties);
}

#[test]
fn rf017_size_limits_preserve_existing_package_and_directory() {
    const HUNDRED_MIB: u64 = 100 * 1024 * 1024;
    for (count, declared_size, expected_error) in [
        (1, HUNDRED_MIB + 1, "ATTACHMENT_TOO_LARGE"),
        (11, HUNDRED_MIB, "TOTAL_SIZE_EXCEEDED"),
    ] {
        let f = Fixture::new();
        let (target, mut req) = existing_package(&f);
        let attachments = (0..count)
            .map(|index| attachment(&f, &format!("size-{index}"), declared_size, b"tiny"))
            .collect();
        select_attachments(&f, &mut req, attachments);
        let before = directory_bytes(&target);
        let audit_count = successful_exports(&f);

        let error = run_export(&f, &req, &target).unwrap_err();
        assert!(error.contains(expected_error), "{error}");
        assert_preserved(&f, &target, &before, audit_count);
    }
}

#[test]
fn rf017_corrupt_solc_attachment_preserves_existing_package() {
    let f = Fixture::new();
    let (target, mut req) = existing_package(&f);
    let corrupted = b"SOLC-incomplete-authenticated-ciphertext";
    let meta = attachment(&f, "corrupt", corrupted.len() as u64, corrupted);
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(
        Path::new(meta["vaultPath"].as_str().unwrap())
    ));
    select_attachments(&f, &mut req, vec![meta]);
    let before = directory_bytes(&target);
    let audit_count = successful_exports(&f);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();

    let error =
        execute_export_for_session(&svc, &session, &req, target.to_str().unwrap()).unwrap_err();
    drop(svc);
    assert!(error.contains("解密附件失败"), "{error}");
    assert_preserved(&f, &target, &before, audit_count);
}

#[test]
fn rf017_session_expiry_before_publication_preserves_existing_package_and_audit() {
    let f = Fixture::new();
    let (target, _) = existing_package(&f);
    let before = directory_bytes(&target);
    let audit_count = successful_exports(&f);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    let (file, output) = create_export_output(&target).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file("prepared.txt", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"new ZIP content awaiting publication")
        .unwrap();

    // 已准备好 ZIP 正文后同账户重新解锁；账户 ID 相同也不能复用旧发布令牌。
    svc.lock();
    svc.unlock(&f.account, "password123").unwrap();
    let error =
        finalize_export_for_session(&svc, &session, zip, output, target.to_str().unwrap(), 1)
            .unwrap_err();
    drop(svc);
    assert!(error.contains("session"), "{error}");
    assert_preserved(&f, &target, &before, audit_count);
}

#[cfg(windows)]
#[test]
fn rf017_windows_locked_target_preserves_package_then_replaces_after_release() {
    use std::os::windows::fs::OpenOptionsExt;

    let f = Fixture::new();
    let (target, req) = existing_package(&f);
    let before = directory_bytes(&target);
    let audit_count = successful_exports(&f);
    // 允许普通读取/写入但拒绝 FILE_SHARE_DELETE：准确阻断替换，不能靠先删旧包兜底。
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001 | 0x0000_0002)
        .open(&target)
        .unwrap();

    assert!(run_export(&f, &req, &target).is_err());
    assert_preserved(&f, &target, &before, audit_count);
    drop(held);

    run_export(&f, &req, &target).unwrap();
    assert_valid_zip(&target);
    let after = directory_bytes(&target);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    assert_ne!(after[&target], before[&target]);
    assert_eq!(successful_exports(&f), audit_count + 1);
}
