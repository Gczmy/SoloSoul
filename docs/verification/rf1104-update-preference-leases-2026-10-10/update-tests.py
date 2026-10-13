from pathlib import Path

source = Path('D:/SoloSoul/tauri/src-tauri/src/commands/update_preferences.rs')
text = source.read_text(encoding='utf-8')
assert 'struct AccountSnapshot' in text
text = text.replace('let service = std::sync::RwLock::new(\n            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),\n        );', 'let service = Arc::new(RwLock::new(\n            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),\n        ));')
text = text.replace('unlock_secure_with_maintenance(account, "password123", &maintenance)', 'unlock_secure_with_maintenance(\n                account,\n                &zeroize::Zeroizing::new("password123".to_owned()),\n                &maintenance,\n            )')
old = '''        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let entered = Arc::new(tokio::sync::Notify::new());'''
assert text.count(old) == 1
text = text.replace(old, '''        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        // 原 Store 仍存活时也必须拒绝同账户的新会话，不能只依赖弱引用过期。
        let original = service.read().unwrap().get_vault_store().unwrap();
        let entered = Arc::new(tokio::sync::Notify::new());''')
text = text.replace('''        writer.await.unwrap();
        assert_eq!(std::fs::read(cache).unwrap(), before);''', '''        writer.await.unwrap();
        assert!(!Arc::ptr_eq(&original, &service.read().unwrap().get_vault_store().unwrap()));
        assert_eq!(std::fs::read(cache).unwrap(), before);''')
start = text.index('        let mut prefs = SourcePreferences {')
end = text.index('        prefs.preferences.remember(', start)
text = text[:start] + '        let mut prefs = SourcePreferences::empty();\n' + text[end:]
start = text.index('    #[test]\n    fn preferences_survive_reopen')
end = text.index('    #[test]\n    fn rf905_locked_source_preferences', start)
text = text[:start] + '''    #[test]
    fn preferences_survive_reopen_and_preserve_other_settings_and_account_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, r#"{"theme":"dark"}"#).unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(root.clone()).unwrap(),
        ));
        let account = "acc_prefs_reopen";
        service.read().unwrap()
            .create_account_with_id(account, "Test", "password123", None).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        prefs.remember(Channel::Release, "https://other.example/release.json", true);
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let before = vault.load_profile(account).unwrap().unwrap();
        let unchanged = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        unchanged.remember(Channel::Release, "https://other.example/release.json", false);
        assert_eq!(vault.load_profile(account).unwrap().unwrap().version, before.version);
        drop(vault);
        service.read().unwrap().lock();
        drop(service);
        let reopened = solosoul_core::VaultService::try_with_base_path(root).unwrap();
        reopened.unlock(account, "password123").unwrap();
        let vault = reopened.get_vault_store().unwrap();
        let profile = vault.load_profile(account).unwrap().unwrap();
        let data: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["preferences"][PREF_KEY]["manifest"]["url"], "https://mirror.example/latest.json");
        assert_eq!(data["preferences"][PREF_KEY]["release"]["url"], "https://other.example/release.json");
        assert!(vault.load_profile("account-b").unwrap().is_none());
        let cached: serde_json::Value = serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert_eq!(cached["theme"], "dark");
        assert!(!cached.to_string().contains(account));
    }

''' + text[end:]
old = '''        assert_eq!(
            solosoul_core::import_activity::begin_owned_root_maintenance(Arc::clone(&owner))
                .err()
                .as_deref(),
            Some("IMPORT_OPERATIONS_ACTIVE")
        );'''
assert text.count(old) == 2
text = text.replace(old, '''        let during_network =
            solosoul_core::import_activity::begin_owned_root_maintenance(Arc::clone(&owner)).unwrap();
        drop(during_network);''')
text = text.replace('rf905_network_delayed_preferences_keep_original_store_owned_until_real_write', 'rf905_network_delayed_preferences_reacquire_original_store_for_real_write')
new_tests = '''
    fn locked_service(root: PathBuf) -> Arc<RwLock<solosoul_core::VaultService>> {
        Arc::new(RwLock::new(solosoul_core::VaultService::try_with_base_path(root).unwrap()))
    }

    #[test]
    fn rf1104_pending_cache_does_not_keep_service_or_directory_owned() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let cache = dir.path().join("ui_preferences.json");
        let service = locked_service(root.clone());
        let owner = Arc::downgrade(&service.read().unwrap().root_owner());
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        drop(service);
        assert!(owner.upgrade().is_none());
        let reopened = solosoul_vault::root_owner::VaultRootOwner::acquire(&root).unwrap();
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        assert!(!cache.exists());
        drop(reopened);
    }

    #[test]
    fn rf1104_late_cache_write_rejects_maintenance_busy_and_replaced_root() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).unwrap();
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        assert_eq!(std::fs::read(&cache).unwrap(), before);
        drop(maintenance);
        *service.write().unwrap() = solosoul_core::VaultService::try_with_base_path(dir.path().join("other")).unwrap();
        // 保留原 owner，确保拒绝来自身份比较而非弱引用提前过期。
        assert!(prefs.owner.upgrade().is_some());
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        assert_eq!(std::fs::read(cache).unwrap(), before);
        drop(owner);
    }

    #[test]
    fn rf1104_same_path_replacement_cannot_revive_old_cache_request() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let service = locked_service(root.clone());
        let cache = dir.path().join("ui_preferences.json");
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        *service.write().unwrap() = solosoul_core::VaultService::try_with_base_path(dir.path().join("other")).unwrap();
        *service.write().unwrap() = solosoul_core::VaultService::try_with_base_path(root).unwrap();
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        assert!(!cache.exists());
    }

    #[test]
    fn rf1104_account_switch_rejects_late_account_and_cache_write() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        service.read().unwrap().create_account_with_id("acc_prefs_a", "A", "password123", None).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let before_a = service.read().unwrap().get_vault_store().unwrap()
            .load_profile("acc_prefs_a").unwrap().map(|profile| (profile.version, profile.data));
        service.read().unwrap().create_account_with_id("acc_prefs_b", "B", "password123", None).unwrap();
        let before_b = service.read().unwrap().get_vault_store().unwrap()
            .load_profile("acc_prefs_b").unwrap().map(|profile| (profile.version, profile.data));
        prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        assert_eq!(std::fs::read(cache).unwrap(), before);
        for (account, expected) in [("acc_prefs_b", before_b), ("acc_prefs_a", before_a)] {
            service.read().unwrap().lock();
            service.read().unwrap().unlock(account, "password123").unwrap();
            let vault = service.read().unwrap().get_vault_store().unwrap();
            assert_eq!(vault.load_profile(account).unwrap().map(|profile| (profile.version, profile.data)), expected);
        }
    }

    #[test]
    fn rf1104_concurrent_source_channels_preserve_account_and_cache_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, br#"{"theme":"dark"}"#).unwrap();
        let account = "acc_prefs_concurrent";
        service.read().unwrap().create_account_with_id(account, "Test", "password123", None).unwrap();
        let manifest = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let release = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let ready = Arc::new(std::sync::Barrier::new(3));
        let writers = [(manifest, Channel::Manifest, "https://mirror.example/latest.json"),
            (release, Channel::Release, "https://mirror.example/release.json")]
            .into_iter().map(|(prefs, channel, url)| {
                let ready = ready.clone();
                std::thread::spawn(move || {
                    ready.wait();
                    prefs.remember(channel, url, true);
                })
            }).collect::<Vec<_>>();
        ready.wait();
        for writer in writers {
            writer.join().unwrap();
        }
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let profile = vault.load_profile(account).unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        let cached: serde_json::Value = serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        for value in [&stored["preferences"], &cached] {
            assert_eq!(value[PREF_KEY]["manifest"]["url"], "https://mirror.example/latest.json");
            assert_eq!(value[PREF_KEY]["release"]["url"], "https://mirror.example/release.json");
        }
        assert_eq!(cached["theme"], "dark");
        assert!(!cached.to_string().contains(account));
    }

    #[test]
    fn rf1104_actual_cache_write_retains_activity_until_file_work_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        drop(solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).unwrap());
        let file_lock = UI_PREFS_LOCK.lock().unwrap();
        let writer = std::thread::spawn(move || {
            prefs.remember(Channel::Manifest, "https://mirror.example/latest.json", true);
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        // 只读观察实际 worker 保活，避免轮询维护许可抢在 worker 前占用准入。
        while Arc::strong_count(&owner) == 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(10));
        let blocked = solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone())
            .err().as_deref() == Some("IMPORT_OPERATIONS_ACTIVE");
        let while_blocked = std::fs::read(&cache).unwrap();
        drop(file_lock);
        writer.join().unwrap();
        assert!(blocked, "actual file work must retain its own root activity permit");
        assert_eq!(while_blocked, before);
        let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert_eq!(saved["theme"], "dark");
        assert_eq!(saved[PREF_KEY]["manifest"]["url"], "https://mirror.example/latest.json");
        drop(solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap());
    }
'''
end = text.rfind('\n}')
text = text[:end] + new_tests + text[end:]
source.write_bytes(text.replace('\n', '\r\n').encode('utf-8'))
print('Updated existing protection tests and added RF-1104 lifecycle regressions')
