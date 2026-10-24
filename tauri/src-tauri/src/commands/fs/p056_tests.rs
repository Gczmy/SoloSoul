use super::*;
use tempfile::TempDir;

struct OwnedDirectoryLink(PathBuf);
impl Drop for OwnedDirectoryLink {
    fn drop(&mut self) {
        // 仅移除本测试创建的链接，绝不递归操作链接目标。
        #[cfg(windows)]
        let _ = fs_std::remove_dir(&self.0);
        #[cfg(unix)]
        let _ = fs_std::remove_file(&self.0);
    }
}

fn directory_link(link: &Path, target: &Path) -> OwnedDirectoryLink {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    OwnedDirectoryLink(link.to_path_buf())
}

#[tokio::test]
async fn scan_rejects_external_child_link_instead_of_returning_partial_metadata() {
    let selected = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let sentinel = outside.path().join("external.txt");
    fs_std::write(&sentinel, b"outside must remain").unwrap();
    fs_std::write(selected.path().join("own.txt"), b"inside").unwrap();
    let _link = directory_link(&selected.path().join("external-link"), outside.path());
    let app = tauri::test::mock_app();
    let error = fs_scan_directory(
        app.handle().clone(),
        selected.path().to_string_lossy().into_owned(),
    )
    .await
    .unwrap_err();
    assert!(error.contains("outside the scan directory"));
    assert_eq!(fs_std::read(sentinel).unwrap(), b"outside must remain");
}

#[test]
fn scan_internal_alias_returns_each_canonical_file_once() {
    let selected = TempDir::new().unwrap();
    let target = selected.path().join("real");
    fs_std::create_dir(&target).unwrap();
    let file = target.join("public.txt");
    fs_std::write(&file, b"public").unwrap();
    let _link = directory_link(&selected.path().join("alias"), &target);
    let mut files = Vec::new();
    scan_dir_recursive(selected.path(), &mut files, MAX_SCAN_DEPTH).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].path,
        display_fs_path(&file.canonicalize().unwrap())
    );
    assert_eq!(files[0].name, "public.txt");
    assert_eq!(files[0].size, 6);
}

#[test]
fn scan_cycle_is_an_error_even_at_the_depth_boundary() {
    let selected = TempDir::new().unwrap();
    let _link = directory_link(&selected.path().join("cycle"), selected.path());
    for depth in [1, MAX_SCAN_DEPTH] {
        let error = scan_dir_recursive(selected.path(), &mut Vec::new(), depth).unwrap_err();
        assert!(error.contains("cycle"));
    }
}

#[test]
fn scan_broken_link_is_an_explicit_error() {
    let selected = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let target = outside.path().join("removed-target");
    fs_std::create_dir(&target).unwrap();
    let _link = directory_link(&selected.path().join("broken"), &target);
    fs_std::remove_dir(&target).unwrap();
    let error = scan_dir_recursive(selected.path(), &mut Vec::new(), MAX_SCAN_DEPTH).unwrap_err();
    assert!(error.contains("Resolve scanned entry"));
}

#[test]
fn scan_revisits_an_alias_target_when_a_later_path_has_more_depth_budget() {
    let selected = TempDir::new().unwrap();
    let target = selected.path().join("real");
    let deeper = target.join("deeper");
    fs_std::create_dir_all(&deeper).unwrap();
    fs_std::write(target.join("shallow.txt"), b"one").unwrap();
    fs_std::write(deeper.join("deep.txt"), b"two").unwrap();
    let root = selected.path().canonicalize().unwrap();
    let target = target.canonicalize().unwrap();
    let mut scan = DirectoryScan {
        root,
        active_directories: Default::default(),
        visited_directories: Default::default(),
        visited_files: Default::default(),
    };
    let mut files = Vec::new();
    scan.walk(&target, &mut files, 1).unwrap();
    assert_eq!(files.len(), 1);
    scan.walk(&target, &mut files, 3).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(
        files.iter().filter(|file| file.name == "deep.txt").count(),
        1
    );
    assert_eq!(
        files
            .iter()
            .filter(|file| file.name == "shallow.txt")
            .count(),
        1
    );
}

#[test]
fn scan_preserves_the_file_count_limit() {
    let selected = TempDir::new().unwrap();
    for index in 0..MAX_SCAN_FILES + 5 {
        fs_std::write(
            selected.path().join(format!("public-{index}.txt")),
            b"public",
        )
        .unwrap();
    }
    let mut files = Vec::new();
    scan_dir_recursive(selected.path(), &mut files, MAX_SCAN_DEPTH).unwrap();
    assert_eq!(files.len(), MAX_SCAN_FILES);
}

#[cfg(unix)]
#[test]
fn scan_file_symlinks_deduplicate_internal_targets_and_reject_external_targets() {
    let selected = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let file = selected.path().join("public.txt");
    let sentinel = outside.path().join("external.txt");
    fs_std::write(&file, b"inside").unwrap();
    fs_std::write(&sentinel, b"outside").unwrap();
    std::os::unix::fs::symlink(&file, selected.path().join("alias.txt")).unwrap();
    let mut files = Vec::new();
    scan_dir_recursive(selected.path(), &mut files, MAX_SCAN_DEPTH).unwrap();
    assert_eq!(files.len(), 1);
    std::os::unix::fs::symlink(&sentinel, selected.path().join("external.txt")).unwrap();
    let error = scan_dir_recursive(selected.path(), &mut Vec::new(), MAX_SCAN_DEPTH).unwrap_err();
    assert!(error.contains("outside the scan directory"));
    assert_eq!(fs_std::read(sentinel).unwrap(), b"outside");
}
