use super::*;
use tempfile::TempDir;

const BYTES: &[u8] = b"RF212 synthetic opaque model bytes";

fn prepared(root: &Path) -> PreparedDownload {
    let mut file = NamedTempFile::new_in(root).unwrap();
    file.write_all(BYTES).unwrap();
    file.flush().unwrap();
    file.as_file().sync_all().unwrap();
    PreparedDownload {
        path: file.into_temp_path(),
        bytes: BYTES.len() as u64,
    }
}

#[test]
fn rf212_model_ids_reject_paths_aliases_and_windows_device_names() {
    for id in [
        "small-v1",
        "MiniLM_L6.v2",
        "模型一",
        ".local-model",
        "COM10",
    ] {
        assert!(valid_model_id(id), "valid synthetic ID: {id}");
    }
    for id in [
        "",
        ".",
        "..",
        "../model",
        "model/child",
        "model\\child",
        "C:model",
        "/absolute",
        "model.",
        "model ",
        " model",
        "model:name",
        "CON",
        "con.bin",
        "PRN",
        "AUX.onnx",
        "NUL",
        "COM1",
        "com9.bin",
        "LPT1",
        "lpt9.data",
        "COM¹",
        "LPT².bin",
        "COM³",
        "name\nvalue",
    ] {
        assert!(!valid_model_id(id), "unsafe synthetic ID: {id:?}");
    }
}

#[test]
fn rf212_installed_size_only_accepts_regular_nonempty_model_files() {
    let root = TempDir::new().unwrap();
    let target = root.path().join("model");
    assert_eq!(installed_size(root.path(), "model"), None);
    fs::create_dir(&target).unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    let temp = NamedTempFile::new_in(root.path()).unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    fs::write(target.join("model.bin"), b"").unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    fs::write(target.join("model.bin"), BYTES).unwrap();
    assert_eq!(
        installed_size(root.path(), "model"),
        Some(BYTES.len() as u64)
    );
    assert_eq!(installed_size(root.path(), "../model"), None);
    assert_eq!(installed_size(temp.path(), "model"), None);
    fs::remove_file(target.join("model.bin")).unwrap();
    fs::create_dir(target.join("model.bin")).unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
}

#[test]
fn rf212_published_file_has_complete_bytes_and_no_temporary_entry() {
    let root = TempDir::new().unwrap();
    let download = prepared(root.path());
    let temporary = download.path.to_path_buf();
    assert!(temporary.is_file());
    assert_eq!(installed_size(root.path(), "model"), None);
    assert_eq!(
        publish_download(download, "model".to_string(), root.path()).unwrap(),
        TaskOutput::EmbedModelInstalled {
            model_id: "model".to_string(),
            bytes: BYTES.len() as u64,
        }
    );
    let target = root.path().join("model").join("model.bin");
    assert_eq!(fs::read(&target).unwrap(), BYTES);
    assert!(!temporary.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(
        installed_size(root.path(), "model"),
        Some(BYTES.len() as u64)
    );
    // Windows 禁止任何共享的重新打开必须成功，证明发布后不遗留本下载器句柄。
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let handle = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        drop(handle);
    }
}

#[test]
fn rf212_publish_reuses_empty_directory_after_previous_failure() {
    let root = TempDir::new().unwrap();
    let target = root.path().join("model");
    fs::create_dir(&target).unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    publish_download(prepared(root.path()), "model".to_string(), root.path()).unwrap();
    assert_eq!(fs::read(target.join("model.bin")).unwrap(), BYTES);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn rf212_target_appearing_after_preparation_is_not_overwritten() {
    let root = TempDir::new().unwrap();
    let download = prepared(root.path());
    let temporary = download.path.to_path_buf();
    let target = root.path().join("model");
    fs::create_dir(&target).unwrap();
    let original = b"existing independent model";
    fs::write(target.join("model.bin"), original).unwrap();
    assert!(publish_download(download, "model".to_string(), root.path()).is_err());
    assert_eq!(fs::read(target.join("model.bin")).unwrap(), original);
    assert!(!temporary.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn rf212_non_directory_target_preserves_bytes_and_cleans_prepared_file() {
    let root = TempDir::new().unwrap();
    let download = prepared(root.path());
    let temporary = download.path.to_path_buf();
    let target = root.path().join("model");
    fs::write(&target, b"unrelated existing file").unwrap();
    assert!(publish_download(download, "model".to_string(), root.path()).is_err());
    assert_eq!(fs::read(target).unwrap(), b"unrelated existing file");
    assert!(!temporary.exists());
}

#[cfg(unix)]
#[test]
fn rf212_symlink_root_directory_and_model_file_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("model.bin"), BYTES).unwrap();
    symlink(outside.path(), root.path().join("model")).unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    assert!(publish_download(prepared(root.path()), "model".to_string(), root.path()).is_err());
    assert_eq!(fs::read(outside.path().join("model.bin")).unwrap(), BYTES);

    fs::remove_file(root.path().join("model")).unwrap();
    fs::create_dir(root.path().join("model")).unwrap();
    symlink(
        outside.path().join("model.bin"),
        root.path().join("model/model.bin"),
    )
    .unwrap();
    assert_eq!(installed_size(root.path(), "model"), None);
    assert!(publish_download(prepared(root.path()), "model".to_string(), root.path()).is_err());
    symlink(root.path(), outside.path().join("root-link")).unwrap();
    assert!(ensure_models_root(&outside.path().join("root-link")).is_err());
    assert_eq!(
        installed_size(&outside.path().join("root-link"), "model"),
        None
    );
}
