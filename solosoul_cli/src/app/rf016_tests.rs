//! RF016：真实CLI进入解锁首页时恢复持久意图。
use super::*;
use crate::commands::attachment::rf016_tests::Fixture;

#[test]
fn rf016_cli_enter_home_recovers_intents_after_a_new_unlock() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let vault = f.app.vault_service.get_vault_store().unwrap();
    vault
        .queue_attachment_deletions(&f.account, "rf016-cli-object", &["rf016-cli-att".into()])
        .unwrap();
    assert!(f.file.exists());
    assert_eq!(
        vault
            .list_attachment_cleanup_intents(&f.account)
            .unwrap()
            .len(),
        1
    );
    drop(vault);
    f.app.vault_service.lock();
    f.app
        .vault_service
        .unlock(&f.account, crate::TEST_PASSWORD)
        .unwrap();
    f.app.enter_home(&f.account);
    assert!(matches!(f.app.phase, AppPhase::Home { .. }));
    assert!(!f.file.exists());
    assert!(f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .list_attachment_cleanup_intents(&f.account)
        .unwrap()
        .is_empty());
}
