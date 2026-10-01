//! RF905：CLI 全局插件根接线；只用明确指定的临时 Native/Plugin/market 根。
//! 这些是生产 capsule 的语义回归，不作为跨进程竞争证据；跨进程见 integration。
#![cfg(not(any(target_os = "android", target_os = "ios")))]

use super::*;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_core::VaultService;
use solosoul_vault::root_owner::VaultRootOwner;
use tempfile::TempDir;

fn app(directory: &TempDir, same_root: bool) -> (App, PathBuf) {
    let native = directory.path().join("vault");
    let service = Arc::new(VaultService::try_with_base_path(native).unwrap());
    let mut app = App::new(service).unwrap();
    let market = directory.path().join("market");
    std::fs::create_dir_all(&market).unwrap();
    let data = if same_root {
        app.vault_service.base_path().to_path_buf()
    } else {
        directory.path().join("plugin-data")
    };
    app.plugin_test_dirs = Some((market, data.clone()));
    (app, data)
}

#[test]
fn rf905_plugin_manager_foreign_global_owner_stops_before_constructor_write() {
    let directory = TempDir::new().unwrap();
    let (mut app, data) = app(&directory, false);
    let other = VaultRootOwner::acquire(&data).unwrap();
    let sentinel = data.join("sentinel.bin");
    std::fs::write(&sentinel, b"RF905 foreign global root").unwrap();
    assert!(create_manager(&mut app).is_none());
    assert!(app
        .error_message
        .as_deref()
        .unwrap()
        .contains("VAULT_DIRECTORY_BUSY"));
    assert!(
        !data.join("plugins").exists(),
        "constructing PluginStore would write this directory"
    );
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"RF905 foreign global root"
    );
    assert!(app.plugin_root_owner.lock().unwrap().is_none());
    drop(other);
    assert!(
        create_manager(&mut app).is_some(),
        "release must allow the unchanged actual global root"
    );
    assert!(data.join("plugins").is_dir());
}

#[test]
fn rf905_plugin_same_native_root_explicitly_reuses_owner_and_activity() {
    let directory = TempDir::new().unwrap();
    let (mut app, data) = app(&directory, true);
    let native_owner = app.vault_service.root_owner();
    let selected = plugin_data_owner(&app, &data).unwrap();
    assert_eq!(selected.id(), native_owner.id());
    drop(selected);
    let first = create_manager(&mut app).unwrap();
    let second = create_manager(&mut app).unwrap();
    assert!(
        app.plugin_root_owner.lock().unwrap().is_none(),
        "same root must not acquire a second owner"
    );
    assert!(begin_owned_root_maintenance(Arc::clone(&native_owner)).is_err());
    drop(first);
    assert!(begin_owned_root_maintenance(Arc::clone(&native_owner)).is_err());
    drop(second);
    let maintenance = begin_owned_root_maintenance(native_owner).unwrap();
    drop(maintenance);
}

#[test]
fn rf905_plugin_different_root_cached_owner_lives_through_actual_manager_drop() {
    let directory = TempDir::new().unwrap();
    let (mut app, data) = app(&directory, false);
    let first = create_manager(&mut app).unwrap();
    let owner_id = app.plugin_root_owner.lock().unwrap().as_ref().unwrap().id();
    let second = create_manager(&mut app).unwrap();
    assert_eq!(
        app.plugin_root_owner.lock().unwrap().as_ref().unwrap().id(),
        owner_id
    );
    assert_ne!(owner_id, app.vault_service.root_owner().id());
    drop(app);
    assert!(
        VaultRootOwner::acquire(&data).is_err(),
        "real global owner must outlive App through worker capsule"
    );
    drop(first);
    assert!(VaultRootOwner::acquire(&data).is_err());
    drop(second);
    let reopened = VaultRootOwner::acquire(&data).unwrap();
    assert_ne!(reopened.id(), owner_id);
}
