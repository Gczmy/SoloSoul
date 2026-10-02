use super::*;

fn rf214_manifest(id: &str, version: &str, bytes: &[u8]) -> PluginManifest {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "Synthetic plugin",
        "version": version,
        "description": "Synthetic local RF214 fixture",
        "wasmHashSha256": compute_sha256(bytes)
    }))
    .unwrap()
}

#[cfg(windows)]
#[test]
fn rf214_update_never_corrupts_legacy_pair_when_old_wasm_is_write_locked() {
    use std::os::windows::fs::OpenOptionsExt;

    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-update";
    let old_bytes = b"\0asm\x01\0\0\0old";
    let new_bytes = b"\0asm\x01\0\0\0new";
    let old_manifest = rf214_manifest(id, "1.0.0", old_bytes);
    let new_manifest = rf214_manifest(id, "2.0.0", new_bytes);
    let plugin_dir = root.path().join("plugins").join(id);
    fs::create_dir(&plugin_dir).unwrap();
    let manifest_path = plugin_dir.join("manifest.json");
    let wasm_path = plugin_dir.join("plugin.wasm");
    let old_manifest_bytes = serde_json::to_vec_pretty(&old_manifest).unwrap();
    fs::write(&manifest_path, &old_manifest_bytes).unwrap();
    fs::write(&wasm_path, old_bytes).unwrap();

    // 真实 Windows 共享模式允许读取旧包，但拒绝旧实现在第二文件上覆盖/截断。
    let old_wasm_handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&wasm_path)
        .unwrap();
    let result = store.save_plugin(&new_manifest, new_bytes);
    assert_eq!(fs::read(&manifest_path).unwrap(), old_manifest_bytes);
    assert_eq!(fs::read(&wasm_path).unwrap(), old_bytes);
    if result.is_ok() {
        // 不可变新布局允许更新成功，不要求锁住旧文件阻挡新版本发布。
        assert_eq!(store.load_manifest(id).unwrap().version, "2.0.0");
        assert_eq!(store.load_wasm(id).unwrap(), new_bytes);
    } else {
        assert_eq!(store.load_manifest(id).unwrap().version, "1.0.0");
        assert_eq!(store.load_wasm(id).unwrap(), old_bytes);
    }
    drop(old_wasm_handle);
}

#[test]
fn rf214_failed_new_install_never_lists_manifest_without_complete_package() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-incomplete";
    let bytes = b"\0asm\x01\0\0\0";
    let plugin_dir = root.path().join("plugins").join(id);
    fs::create_dir(&plugin_dir).unwrap();
    // 两种布局都遇到真实文件系统失败：旧布局第二个文件、新布局唯一发布指针。
    fs::create_dir(plugin_dir.join("plugin.wasm")).unwrap();
    fs::create_dir(plugin_dir.join("current.json")).unwrap();

    assert!(store
        .save_plugin(&rf214_manifest(id, "1.0.0", bytes), bytes)
        .is_err());
    assert!(store.installed_manifests().unwrap().is_empty());
    assert!(store.load_wasm(id).is_err());
}
fn rf214_legacy(root: &Path, id: &str, version: &str, bytes: &[u8]) -> PathBuf {
    let directory = root.join("plugins").join(id);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&rf214_manifest(id, version, bytes)).unwrap(),
    )
    .unwrap();
    fs::write(directory.join("plugin.wasm"), bytes).unwrap();
    directory
}

fn rf214_generation(root: &Path, id: &str) -> PathBuf {
    let directory = root.join("plugins").join(id);
    let pointer: CurrentPointer =
        serde_json::from_slice(&fs::read(directory.join("current.json")).unwrap()).unwrap();
    directory.join("versions").join(pointer.generation)
}

#[test]
fn rf214_prepare_is_invisible_and_drop_cleans_real_staged_files() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-cancel";
    let old = b"old installed bytes";
    let new = b"new prepared bytes";
    store
        .save_plugin(&rf214_manifest(id, "1.0.0", old), old)
        .unwrap();
    let before_pointer =
        fs::read(root.path().join("plugins").join(id).join("current.json")).unwrap();
    let prepared = store
        .prepare_plugin(&rf214_manifest(id, "2.0.0", new), new)
        .unwrap();
    assert_eq!(prepared.manifest().version, "2.0.0");
    let (staged_dir, staged_pointer) = match &prepared.kind {
        PreparedKind::Staged {
            directory, pointer, ..
        } => (directory.path().to_owned(), pointer.to_path_buf()),
        PreparedKind::Reuse(_) => panic!("new bytes must have their own staged generation"),
    };
    assert_eq!(fs::read(staged_dir.join("plugin.wasm")).unwrap(), new);
    assert!(staged_pointer.is_file());
    assert_eq!(store.load_plugin(id).unwrap().1, old);
    assert_eq!(store.installed_manifests().unwrap()[0].version, "1.0.0");
    drop(prepared);
    assert!(!staged_dir.exists());
    assert!(!staged_pointer.exists());
    assert_eq!(
        fs::read(root.path().join("plugins").join(id).join("current.json")).unwrap(),
        before_pointer
    );
    assert_eq!(store.load_plugin(id).unwrap().1, old);
}

#[test]
fn rf214_new_preparation_and_unreferenced_crash_generation_are_not_installed() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-unpublished";
    let bytes = b"complete but not published";
    let prepared = store
        .prepare_plugin(&rf214_manifest(id, "1.0.0", bytes), bytes)
        .unwrap();
    assert!(store.installed_manifests().unwrap().is_empty());
    assert!(store.load_plugin(id).is_err());
    drop(prepared);
    let orphan = root
        .path()
        .join("plugins")
        .join(id)
        .join("versions")
        .join("v-crashorphan1234");
    fs::create_dir(&orphan).unwrap();
    fs::write(
        orphan.join("manifest.json"),
        serde_json::to_vec(&rf214_manifest(id, "9.0.0", bytes)).unwrap(),
    )
    .unwrap();
    fs::write(orphan.join("plugin.wasm"), bytes).unwrap();
    assert!(store.installed_manifests().unwrap().is_empty());
    assert!(store.load_plugin(id).is_err());
    assert!(
        orphan.is_dir(),
        "reads must not guess or garbage-collect unreferenced generations"
    );
    rf214_legacy(root.path(), id, "1.0.0", bytes);
    assert_eq!(store.load_plugin(id).unwrap().0.version, "1.0.0");
}

#[test]
fn rf214_prepared_package_cannot_be_published_by_another_store() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first_store = PluginStore::new_with_data_dir(first.path().to_owned()).unwrap();
    let second_store = PluginStore::new_with_data_dir(second.path().to_owned()).unwrap();
    let bytes = b"bound store bytes";
    let prepared = first_store
        .prepare_plugin(&rf214_manifest("bound", "1.0.0", bytes), bytes)
        .unwrap();
    let path = match &prepared.kind {
        PreparedKind::Staged { directory, .. } => directory.path().to_owned(),
        PreparedKind::Reuse(_) => unreachable!(),
    };
    assert!(second_store.publish_prepared(prepared).is_err());
    assert!(!path.exists());
    assert!(first_store.installed_manifests().unwrap().is_empty());
    assert!(second_store.installed_manifests().unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn rf214_windows_pointer_replace_failure_preserves_old_package_and_cleans_new_generation() {
    use std::os::windows::fs::OpenOptionsExt;

    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-pointer-lock";
    let old = b"old current bytes";
    let new = b"new candidate bytes";
    store
        .save_plugin(&rf214_manifest(id, "1.0.0", old), old)
        .unwrap();
    let directory = root.path().join("plugins").join(id);
    let current_path = directory.join("current.json");
    let before_pointer = fs::read(&current_path).unwrap();
    let before_generation = rf214_generation(root.path(), id);
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&current_path)
        .unwrap();
    let prepared = store
        .prepare_plugin(&rf214_manifest(id, "2.0.0", new), new)
        .unwrap();
    assert!(store.publish_prepared(prepared).is_err());
    assert_eq!(fs::read(&current_path).unwrap(), before_pointer);
    let (manifest, bytes) = store.load_plugin(id).unwrap();
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(bytes, old);
    assert_eq!(
        fs::read(before_generation.join("plugin.wasm")).unwrap(),
        old
    );
    assert_eq!(fs::read_dir(directory.join("versions")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    drop(held);
    store
        .save_plugin(&rf214_manifest(id, "2.0.0", new), new)
        .unwrap();
    assert_eq!(store.load_plugin(id).unwrap().1, new);
    assert_eq!(fs::read_dir(directory.join("versions")).unwrap().count(), 2);
}

#[test]
fn rf214_damaged_or_traversing_pointer_never_falls_back_to_valid_legacy_package() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-pointer";
    let bytes = b"valid legacy bytes";
    let directory = rf214_legacy(root.path(), id, "1.0.0", bytes);
    assert_eq!(store.load_plugin(id).unwrap().1, bytes);
    let bad_pointers = [
        b"not json".to_vec(),
        serde_json::to_vec(&CurrentPointer {
            schema_version: 1,
            generation: "../outside".into(),
            manifest_sha256: compute_sha256(bytes),
            wasm_sha256: compute_sha256(bytes),
            wasm_size: bytes.len() as u64,
        })
        .unwrap(),
        serde_json::to_vec(&CurrentPointer {
            schema_version: 99,
            generation: "v-synthetic1234".into(),
            manifest_sha256: compute_sha256(bytes),
            wasm_sha256: compute_sha256(bytes),
            wasm_size: bytes.len() as u64,
        })
        .unwrap(),
    ];
    for bad in bad_pointers {
        fs::write(directory.join("current.json"), bad).unwrap();
        assert!(store.load_plugin(id).is_err());
        assert!(store.load_manifest(id).is_err());
        assert!(store.load_wasm(id).is_err());
        assert!(store.installed_manifests().unwrap().is_empty());
        assert_eq!(fs::read(directory.join("plugin.wasm")).unwrap(), bytes);
    }
}

#[test]
fn rf214_reuse_is_bound_to_current_generation_and_legacy_file_metadata() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-reuse";
    let old = b"old reuse bytes";
    let new = b"new reuse bytes";
    let directory = rf214_legacy(root.path(), id, "1.0.0", old);
    let mut unhashed_legacy = rf214_manifest(id, "1.0.0", old);
    unhashed_legacy.wasm_hash_sha256 = None;
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&unhashed_legacy).unwrap(),
    )
    .unwrap();
    let reuse = store
        .prepare_reuse(id, "1.0.0", &compute_sha256(old))
        .unwrap()
        .unwrap();
    assert_eq!(reuse.manifest().version, "1.0.0");
    store.publish_prepared(reuse).unwrap();
    assert!(
        !directory.join("current.json").exists(),
        "valid reuse preserves legacy layout"
    );
    assert!(store
        .prepare_reuse(id, "2.0.0", &compute_sha256(old))
        .unwrap()
        .is_none());
    assert!(store
        .prepare_reuse(id, "1.0.0", &compute_sha256(new))
        .unwrap()
        .is_none());
    let legacy_changed = store
        .prepare_reuse(id, "1.0.0", &compute_sha256(old))
        .unwrap()
        .unwrap();
    fs::write(
        directory.join("plugin.wasm"),
        b"different length invalid legacy content",
    )
    .unwrap();
    assert!(store.publish_prepared(legacy_changed).is_err());
    fs::write(directory.join("plugin.wasm"), old).unwrap();
    store
        .save_plugin(&rf214_manifest(id, "1.0.0", old), old)
        .unwrap();
    let superseded = store
        .prepare_reuse(id, "1.0.0", &compute_sha256(old))
        .unwrap()
        .unwrap();
    store
        .save_plugin(&rf214_manifest(id, "2.0.0", new), new)
        .unwrap();
    assert!(store.publish_prepared(superseded).is_err());
    assert_eq!(store.load_plugin(id).unwrap().1, new);
    let deleted = store
        .prepare_reuse(id, "2.0.0", &compute_sha256(new))
        .unwrap()
        .unwrap();
    store.delete_plugin(id).unwrap();
    assert!(store.publish_prepared(deleted).is_err());
    assert!(store.installed_manifests().unwrap().is_empty());
}

#[test]
fn rf214_digest_validation_rejects_invalid_preparation_and_tampered_published_files() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let bytes = b"verified complete bytes";
    let manifest = rf214_manifest("rf214-digest", "1.0.0", bytes);
    assert!(matches!(
        store.prepare_plugin(&manifest, b"wrong"),
        Err(PluginError::ChecksumMismatch)
    ));
    assert_eq!(
        fs::read_dir(root.path().join("plugins")).unwrap().count(),
        0
    );
    store.save_plugin(&manifest, bytes).unwrap();
    let generation = rf214_generation(root.path(), &manifest.id);
    // 同时伪造 manifest 自带 hash 仍不能越过发布指针对两个文件的绑定。
    let changed = b"changed complete bytes";
    fs::write(generation.join("plugin.wasm"), changed).unwrap();
    fs::write(
        generation.join("manifest.json"),
        serde_json::to_vec_pretty(&rf214_manifest(&manifest.id, "1.0.0", changed)).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        store.load_plugin(&manifest.id),
        Err(PluginError::ChecksumMismatch)
    ));
    assert!(store.installed_manifests().unwrap().is_empty());
}

#[test]
fn rf214_invalid_ids_and_staged_tampering_cannot_publish() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let bytes = b"synthetic";
    for id in [
        "CON",
        "aux.txt",
        "LPT1",
        "com9.log",
        "trailing.",
        "../escape",
        "a/b",
        "a\\b",
    ] {
        assert!(store
            .prepare_plugin(&rf214_manifest(id, "1.0.0", bytes), bytes)
            .is_err());
    }
    assert_eq!(
        fs::read_dir(root.path().join("plugins")).unwrap().count(),
        0
    );
    let prepared = store
        .prepare_plugin(&rf214_manifest("staged", "1.0.0", bytes), bytes)
        .unwrap();
    let staged = match &prepared.kind {
        PreparedKind::Staged { directory, .. } => directory.path().to_owned(),
        PreparedKind::Reuse(_) => unreachable!(),
    };
    fs::write(
        staged.join("plugin.wasm"),
        b"different size after verification",
    )
    .unwrap();
    assert!(store.publish_prepared(prepared).is_err());
    assert!(!staged.exists());
    assert!(store.installed_manifests().unwrap().is_empty());
}

#[test]
fn rf214_two_store_instances_read_manifest_and_bytes_from_one_published_generation() {
    use std::sync::{Arc, Barrier};

    let root = tempfile::tempdir().unwrap();
    let writer = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let reader = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-paired";
    let a = b"generation A body";
    let b = b"generation B body";
    writer
        .save_plugin(&rf214_manifest(id, "1.0.0", a), a)
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let writer_barrier = Arc::clone(&barrier);
    let publishing = std::thread::spawn(move || {
        writer_barrier.wait();
        for index in 0..12 {
            let (version, bytes) = if index % 2 == 0 {
                ("2.0.0", b)
            } else {
                ("1.0.0", a)
            };
            writer
                .save_plugin(&rf214_manifest(id, version, bytes), bytes)
                .unwrap();
        }
    });
    let reader_barrier = Arc::clone(&barrier);
    let reading = std::thread::spawn(move || {
        reader_barrier.wait();
        for _ in 0..48 {
            let (manifest, bytes) = reader.load_plugin(id).unwrap();
            match manifest.version.as_str() {
                "1.0.0" => assert_eq!(bytes, a),
                "2.0.0" => assert_eq!(bytes, b),
                other => panic!("unexpected synthetic version: {other}"),
            }
        }
    });
    barrier.wait();
    let writer_result = publishing.join();
    let reader_result = reading.join();
    writer_result.unwrap();
    reader_result.unwrap();
}

#[cfg(unix)]
#[test]
fn rf214_symlink_paths_are_rejected_and_new_files_keep_private_permissions() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let bytes = b"private bytes";
    symlink(outside.path(), root.path().join("plugins").join("linked")).unwrap();
    assert!(store
        .save_plugin(&rf214_manifest("linked", "1.0.0", bytes), bytes)
        .is_err());
    assert!(store.load_plugin("linked").is_err());
    assert!(store.delete_plugin("linked").is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    store
        .save_plugin(&rf214_manifest("private", "1.0.0", bytes), bytes)
        .unwrap();
    let directory = root.path().join("plugins/private");
    let generation = rf214_generation(root.path(), "private");
    for path in [
        root.path().join("plugins"),
        directory.clone(),
        directory.join("versions"),
        generation.clone(),
    ] {
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700,
            "directory must stay private: {}",
            path.display()
        );
    }
    for path in [
        directory.join("current.json"),
        generation.join("manifest.json"),
        generation.join("plugin.wasm"),
    ] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_file(generation.join("plugin.wasm")).unwrap();
    symlink(
        outside.path().join("missing.wasm"),
        generation.join("plugin.wasm"),
    )
    .unwrap();
    assert!(store.load_plugin("private").is_err());
}

#[cfg(unix)]
#[test]
fn rf1066_prepared_generation_is_private_and_cancellation_removes_it() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let bytes = b"private prepared bytes";
    let prepared = store
        .prepare_plugin(&rf214_manifest("prepared-private", "1.0.0", bytes), bytes)
        .unwrap();
    let generation = match &prepared.kind {
        PreparedKind::Staged { directory, .. } => directory.path().to_owned(),
        PreparedKind::Reuse(_) => panic!("new install must stage a generation"),
    };
    // 发布前已经写有正文：权限不能等到指针发布后才收紧。
    assert_eq!(
        fs::metadata(&generation).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(fs::read(generation.join("plugin.wasm")).unwrap(), bytes);
    assert!(store.load_plugin("prepared-private").is_err());
    drop(prepared);
    assert!(!generation.exists());
    assert!(!root
        .path()
        .join("plugins/prepared-private/current.json")
        .exists());
}
#[cfg(windows)]
#[test]
fn rf214_listing_does_not_read_large_wasm_but_runtime_load_still_does() {
    use std::os::windows::fs::OpenOptionsExt;

    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-listing";
    let bytes = vec![0x42; MAX_WASM_SIZE];
    store
        .save_plugin(&rf214_manifest(id, "1.0.0", &bytes), &bytes)
        .unwrap();
    let wasm_path = rf214_generation(root.path(), id).join("plugin.wasm");
    // 拒绝真实正文读取但允许文件属性访问，无计时断言或算法替身。
    let held = fs::OpenOptions::new()
        .write(true)
        .share_mode(2 | 4)
        .open(&wasm_path)
        .unwrap();
    assert_eq!(store.load_manifest(id).unwrap().version, "1.0.0");
    assert_eq!(store.installed_manifests().unwrap().len(), 1);
    assert!(store.load_plugin(id).is_err());
    drop(held);
    assert_eq!(store.load_plugin(id).unwrap().1, bytes);
    fs::write(&wasm_path, b"truncated").unwrap();
    assert!(store.load_manifest(id).is_err());
    assert!(store.installed_manifests().unwrap().is_empty());
}

#[test]
fn rf214_manifest_limit_applies_to_preparation_and_legacy_reads() {
    let root = tempfile::tempdir().unwrap();
    let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
    let id = "rf214-manifest-limit";
    let bytes = b"complete wasm";
    let mut oversized = rf214_manifest(id, "1.0.0", bytes);
    oversized.description = "x".repeat(MAX_MANIFEST_SIZE);
    assert!(store.prepare_plugin(&oversized, bytes).is_err());
    assert_eq!(
        fs::read_dir(root.path().join("plugins")).unwrap().count(),
        0
    );
    let directory = rf214_legacy(root.path(), id, "1.0.0", bytes);
    let oversized_bytes = serde_json::to_vec(&oversized).unwrap();
    fs::write(directory.join("manifest.json"), &oversized_bytes).unwrap();
    assert!(store.load_manifest(id).is_err());
    assert!(store.load_plugin(id).is_err());
    assert!(store.installed_manifests().unwrap().is_empty());
    assert_eq!(
        fs::read(directory.join("manifest.json")).unwrap(),
        oversized_bytes
    );
}
