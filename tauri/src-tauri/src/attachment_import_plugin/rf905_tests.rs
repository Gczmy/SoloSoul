//! 桌面可执行的 worker 边界回归：URI 传输闭包用真实文件复制，落盘加密走真实 Core。
//! Android Kotlin/JNI 传输本身仍需设备检查；这里不声称执行 JNI。
use super::{complete_content_uri_import, ImportContentUriResult};
use crate::commands::vault_handle_for_service;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rf905_uri_copy_encrypt_replace_remain_owned_after_waiter_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let service = RwLock::new(
        solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
    );
    service
        .read()
        .unwrap()
        .create_account_with_id("acc_rf905_uri", "Test", "password123", None)
        .unwrap();
    let owner = service.read().unwrap().root_owner();
    let handle = vault_handle_for_service(&service).unwrap();
    let source = dir.path().join("source.bin");
    let plain = b"private URI payload that must be encrypted after cancel";
    std::fs::write(&source, plain).unwrap();
    let dest_dir = owner.root().join("attachments/rf905-object/rf905-file");
    std::fs::create_dir_all(&dest_dir).unwrap();
    let dest = dest_dir.join("example.bin");
    let dest_for_copy = dest.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = ReleaseOnDrop(Some(release_tx));
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let worker = tokio::task::spawn_blocking(move || {
        let result =
            complete_content_uri_import(handle, [7; 32], dest_dir, "example.bin".into(), || {
                let size_bytes =
                    std::fs::copy(source, &dest_for_copy).map_err(|e| e.to_string())?;
                let _ = entered_tx.send(());
                release_rx.recv().unwrap();
                Ok(ImportContentUriResult {
                    vault_path: dest_for_copy.to_string_lossy().into_owned(),
                    size_bytes,
                    display_name: None,
                })
            });
        let _ = done_tx.send(result);
    });
    let waiter = tokio::spawn(worker);
    tokio::time::timeout(Duration::from_secs(10), entered_rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), plain);
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert_eq!(
        begin_owned_root_maintenance(Arc::clone(&owner))
            .err()
            .as_deref(),
        Some("IMPORT_OPERATIONS_ACTIVE")
    );
    drop(release);
    let result = tokio::time::timeout(Duration::from_secs(10), done_rx)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.size_bytes, plain.len() as u64);
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(&dest));
    assert_ne!(std::fs::read(&dest).unwrap(), plain);
    let restored = dir.path().join("restored.bin");
    solosoul_core::attachment_crypto::copy_decrypt_file(&[7; 32], &dest, &restored).unwrap();
    assert_eq!(std::fs::read(restored).unwrap(), plain);
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    drop(maintenance);
}

#[test]
fn rf905_uri_copy_failure_releases_permit_and_preserves_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let service = RwLock::new(
        solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
    );
    service
        .read()
        .unwrap()
        .create_account_with_id("acc_rf905_uri", "Test", "password123", None)
        .unwrap();
    let owner = service.read().unwrap().root_owner();
    let dest_dir = owner.root().join("attachments/rf905-object/rf905-file");
    std::fs::create_dir_all(&dest_dir).unwrap();
    let dest = dest_dir.join("example.bin");
    std::fs::write(&dest, b"original").unwrap();
    let result = complete_content_uri_import(
        vault_handle_for_service(&service).unwrap(),
        [7; 32],
        dest_dir,
        "example.bin".into(),
        || Err("URI_COPY_FAILED".into()),
    );
    assert_eq!(result.unwrap_err(), "URI_COPY_FAILED");
    assert_eq!(std::fs::read(dest).unwrap(), b"original");
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    drop(maintenance);
}
