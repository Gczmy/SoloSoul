//! RF016：真实GUI删除worker、SQLite失败与原会话；不启动用户应用。
use super::super::crud::execute_attachment_deletion;
use super::*;
use solosoul_core::vault_service::{VaultService, VaultSession};
use std::sync::{Arc, RwLock};

fn fixture() -> (Arc<RwLock<VaultService>>, TempDir, VaultSession) {
    let (svc, dir, _) = setup_unlocked_attachment();
    let vault = svc.get_vault_store().unwrap();
    let mut record = vault.load_object("obj-1").unwrap().unwrap();
    let mut atts = load_attachments(&record.properties);
    for id in ["att-2", "att-3"] {
        let mut att = atts[0].clone();
        att.id = id.into();
        let path = svc
            .base_path()
            .join("attachments/obj-1")
            .join(id)
            .join("a.pdf");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"unselected synthetic bytes").unwrap();
        att.vault_path = Some(path.to_string_lossy().into());
        atts.push(att);
    }
    save_attachments(&mut record.properties, &atts);
    vault.save_object(&record).unwrap();
    let session = svc.capture_session("acc-1").unwrap();
    (Arc::new(RwLock::new(svc)), dir, session)
}

#[tokio::test]
async fn rf016_gui_single_worker_deletes_only_the_selected_attachment() {
    let (svc, _dir, session) = fixture();
    let base = svc.read().unwrap().base_path().clone();
    let report = execute_attachment_deletion(
        svc.clone(),
        session.clone(),
        "obj-1".into(),
        vec!["att-1".into()],
    )
    .await
    .unwrap();
    assert_eq!((report.completed, report.pending), (1, 0));
    assert!(!base.join("attachments/obj-1/att-1").exists());
    assert!(base.join("attachments/obj-1/att-2/a.pdf").exists());
    assert_eq!(
        load_attachments(
            &session
                .vault()
                .load_object("obj-1")
                .unwrap()
                .unwrap()
                .properties
        )
        .len(),
        2
    );
    assert!(session
        .vault()
        .list_attachment_cleanup_intents("acc-1")
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn rf016_gui_batch_worker_accepts_one_batch_and_preserves_unselected_files() {
    let (svc, _dir, session) = fixture();
    let base = svc.read().unwrap().base_path().clone();
    let before = session
        .vault()
        .load_object("obj-1")
        .unwrap()
        .unwrap()
        .version;
    let report = execute_attachment_deletion(
        svc.clone(),
        session.clone(),
        "obj-1".into(),
        vec!["att-1".into(), "att-2".into()],
    )
    .await
    .unwrap();
    assert_eq!((report.completed, report.pending), (2, 0));
    let record = session.vault().load_object("obj-1").unwrap().unwrap();
    assert_eq!(record.version, before + 1);
    assert_eq!(load_attachments(&record.properties)[0].id, "att-3");
    assert!(!base.join("attachments/obj-1/att-1").exists());
    assert!(!base.join("attachments/obj-1/att-2").exists());
    assert!(base.join("attachments/obj-1/att-3/a.pdf").exists());
}

#[tokio::test]
async fn rf016_gui_database_failure_keeps_records_and_files() {
    let (svc, _dir, session) = fixture();
    let before =
        serde_json::to_value(session.vault().load_object("obj-1").unwrap().unwrap()).unwrap();
    let db = rusqlite::Connection::open(session.vault().base_path().join("vault.db")).unwrap();
    db.execute_batch("CREATE TRIGGER rf016_reject_insert BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'synthetic object failure'); END; CREATE TRIGGER rf016_reject_update BEFORE UPDATE ON objects BEGIN SELECT RAISE(ABORT,'synthetic object failure'); END;").unwrap();
    assert!(execute_attachment_deletion(
        svc.clone(),
        session.clone(),
        "obj-1".into(),
        vec!["att-1".into(), "att-2".into()]
    )
    .await
    .is_err());
    assert_eq!(
        serde_json::to_value(session.vault().load_object("obj-1").unwrap().unwrap()).unwrap(),
        before
    );
    let base = svc.read().unwrap().base_path().clone();
    for id in ["att-1", "att-2", "att-3"] {
        assert!(base
            .join("attachments/obj-1")
            .join(id)
            .join("a.pdf")
            .exists());
    }
    assert!(session
        .vault()
        .list_attachment_cleanup_intents("acc-1")
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn rf016_gui_stale_worker_cannot_delete_after_lock_switch_or_reunlock() {
    for transition in ["lock", "switch", "reunlock"] {
        let (svc, _dir, session) = fixture();
        {
            let service = svc.read().unwrap();
            service.lock();
            if transition == "switch" {
                service
                    .create_account_with_id("acc-2", "acc-2", "pw1234567890", None)
                    .unwrap();
                service.unlock("acc-2", "pw1234567890").unwrap();
            } else if transition == "reunlock" {
                service.unlock("acc-1", "pw1234567890").unwrap();
            }
        }
        assert!(
            execute_attachment_deletion(svc.clone(), session, "obj-1".into(), vec!["att-1".into()])
                .await
                .is_err(),
            "{transition}"
        );
        let service = svc.read().unwrap();
        let base = service.base_path().clone();
        assert!(
            base.join("attachments/obj-1/att-1/a.pdf").exists(),
            "{transition}"
        );
        service.lock();
        service.unlock("acc-1", "pw1234567890").unwrap();
        let vault = service.get_vault_store().unwrap();
        assert_eq!(
            load_attachments(&vault.load_object("obj-1").unwrap().unwrap().properties).len(),
            3
        );
        assert!(vault
            .list_attachment_cleanup_intents("acc-1")
            .unwrap()
            .is_empty());
    }
}

#[cfg(windows)]
#[tokio::test]
async fn rf016_gui_windows_occupied_file_is_pending_then_retry_completes() {
    use std::os::windows::fs::OpenOptionsExt;
    let (svc, _dir, session) = fixture();
    let file = svc
        .read()
        .unwrap()
        .base_path()
        .join("attachments/obj-1/att-1/a.pdf");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&file)
        .unwrap();
    let report = execute_attachment_deletion(
        svc.clone(),
        session.clone(),
        "obj-1".into(),
        vec!["att-1".into()],
    )
    .await
    .unwrap();
    assert_eq!((report.completed, report.pending), (0, 1));
    assert!(file.exists());
    assert_eq!(
        load_attachments(
            &session
                .vault()
                .load_object("obj-1")
                .unwrap()
                .unwrap()
                .properties
        )
        .len(),
        2
    );
    assert_eq!(
        session
            .vault()
            .list_attachment_cleanup_intents("acc-1")
            .unwrap()
            .len(),
        1
    );
    drop(held);
    let report =
        execute_attachment_deletion(svc, session.clone(), "obj-1".into(), vec!["att-1".into()])
            .await
            .unwrap();
    assert_eq!((report.completed, report.pending), (1, 0));
    assert!(!file.exists());
    assert!(session
        .vault()
        .list_attachment_cleanup_intents("acc-1")
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn rf016_gui_fresh_maintenance_waits_for_old_slot_then_runs() {
    use crate::commands::object::trash::acquire_cleanup_slot;
    use std::sync::atomic::{AtomicBool, Ordering};
    let (svc, _dir, old) = fixture();
    let fresh = {
        let service = svc.read().unwrap();
        service.lock();
        service.unlock("acc-1", "pw1234567890").unwrap();
        service.capture_session("acc-1").unwrap()
    };
    let running = Arc::new(AtomicBool::new(true));
    assert!(acquire_cleanup_slot(&svc, &old, running.clone())
        .await
        .is_err());
    assert!(running.load(Ordering::Acquire));
    let waiting = acquire_cleanup_slot(&svc, &fresh, running.clone());
    tokio::pin!(waiting);
    tokio::select! { result=&mut waiting => panic!("busy slot must wait: {}",result.is_ok()), _=tokio::task::yield_now()=>{} }
    running.store(false, Ordering::Release);
    let guard = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    assert!(running.load(Ordering::Acquire));
    drop(guard);
    assert!(!running.load(Ordering::Acquire));
}
