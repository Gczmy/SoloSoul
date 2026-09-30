use super::*;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn folders(parent: &Path) -> KnownFolders {
    let roaming = parent.join("roaming");
    let local = parent.join("local");
    fs::create_dir(&roaming).unwrap();
    fs::create_dir(&local).unwrap();
    KnownFolders {
        roaming: roaming.canonicalize().unwrap(),
        local: local.canonicalize().unwrap(),
    }
}

fn source(parent: &Path) -> PathBuf {
    let path = parent.join("source");
    fs::create_dir(&path).unwrap();
    fixture::write_test_fixture(&path);
    path
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap().port()
}

fn junction(link: &Path, target: &Path) {
    let status = Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .creation_flags(0x08000000)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "could not create the isolated test junction"
    );
}

#[test]
fn native_perf_args_require_explicit_modes_and_reject_environment_overrides() {
    assert!(parse_args(&[]).is_err());
    for bad in [
        args(&["--native-perf-root", "C:/owned"]),
        args(&["--native-perf-root", "C:/owned", "--native-perf-port", "0"]),
        args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222",
            "--fixture",
            "C:/fixture",
        ]),
        args(&[
            "--native-perf-prepare",
            "C:/new",
            "--fixture",
            "C:/fixture",
            "--fixture",
            "C:/other",
        ]),
    ] {
        assert!(parse_args(&bad).is_err());
    }
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222"
        ])),
        Ok(Mode::Run { .. })
    ));
    assert!(is_webview_override(std::ffi::OsStr::new(
        "webview2_user_data_folder"
    )));
    assert!(is_webview_override(std::ffi::OsStr::new(
        "WebView2_Additional_Browser_Arguments"
    )));
    assert!(!is_webview_override(std::ffi::OsStr::new("USERPROFILE")));
    assert!(
        root().is_err(),
        "no default data path may be returned before configuration"
    );
}

#[test]
fn native_perf_prepares_real_encrypted_fixture_and_consumes_it_only_once() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let source_hash = sha256_file(&source.join("acc_rf312_100/vault.db")).unwrap();
    let folders = folders(work.path());
    let output = work.path().join("new-run");
    let result = prepare(&output, &source, &folders).unwrap();
    assert_eq!(result["objectCount"], 100);
    assert_eq!(result["accountId"], "acc_rf312_100");
    assert_eq!(result["success"], true);
    assert!(!output
        .join("vault/acc_rf312_100/vault.db.pre_enc.bak")
        .exists());
    assert_eq!(
        sha256_file(&source.join("acc_rf312_100/vault.db")).unwrap(),
        source_hash
    );
    let native_root = output.canonicalize().unwrap();
    let config = consume(&native_root, free_port(), &folders).unwrap();
    assert_eq!(config.vault, native_root.join("vault"));
    assert_eq!(config.run_id.len(), 32);
    assert_eq!(
        config.identifier,
        format!("{IDENTIFIER_PREFIX}{}", config.run_id)
    );
    assert!(native_root.join(CONSUMED_FILE).is_file());
    assert!(!native_root.join(READY_FILE).exists());
    assert!(native_root.join(OWNED_FILE).is_file());
    assert!(consume(&native_root, free_port(), &folders).is_err());
}

#[test]
fn native_perf_rejects_existing_relative_linked_and_source_nested_roots() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    assert!(prepare(Path::new("relative-root"), &source, &folders).is_err());
    let existing = work.path().join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("sentinel"), "keep").unwrap();
    assert!(prepare(&existing, &source, &folders).is_err());
    assert_eq!(
        fs::read_to_string(existing.join("sentinel")).unwrap(),
        "keep"
    );
    let linked = work.path().join("linked");
    junction(&linked, &existing);
    assert!(prepare(&linked, &source, &folders).is_err());
    assert!(prepare(&source.join("nested-run"), &source, &folders).is_err());
    assert!(!source.join("nested-run").exists());
    let linked_fixture = work.path().join("linked-fixture");
    junction(&linked_fixture, &source);
    assert!(prepare(
        &work.path().join("rejected-fixture"),
        &linked_fixture,
        &folders
    )
    .is_err());
    assert!(!work.path().join("rejected-fixture").exists());
    assert_eq!(fs::read_dir(&folders.roaming).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&folders.local).unwrap().count(), 0);
}

#[test]
fn native_perf_rejects_wrong_kdf_occupied_port_and_tampered_prepared_fixture() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    let config_path = source.join("acc_rf312_100/config.json");
    let original = fs::read(&config_path).unwrap();
    let mut wrong = read_json(&config_path).unwrap();
    wrong["kdf_memory_kb"] = json!(8192);
    fs::write(&config_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
    let rejected = work.path().join("wrong-fixture");
    assert!(prepare(&rejected, &source, &folders).is_err());
    assert!(!rejected.exists());
    fs::write(&config_path, original).unwrap();
    let output = work.path().join("new-run");
    prepare(&output, &source, &folders).unwrap();
    let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = occupied.local_addr().unwrap().port();
    assert!(consume(&output, port, &folders).is_err());
    assert!(output.join(READY_FILE).is_file());
    assert!(!output.join(CONSUMED_FILE).exists());
    drop(occupied);
    let db = output.join("vault/acc_rf312_100/vault.db");
    OpenOptions::new()
        .append(true)
        .open(db)
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    assert!(consume(&output, free_port(), &folders).is_err());
    assert!(output.join(READY_FILE).is_file());
    assert!(!output.join(CONSUMED_FILE).exists());
}

#[test]
fn native_perf_failed_preparation_preserves_owned_paths_without_ready_marker() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    fs::write(source.join("acc_rf312_100/vault.db"), b"not sqlite").unwrap();
    let output = work.path().join("failed-run");
    assert!(prepare(&output, &source, &folders).is_err());
    let owned = read_json(&output.join(OWNED_FILE)).unwrap();
    assert_eq!(owned["preparationStatus"], "preparing");
    assert_eq!(owned["ownedPaths"].as_array().unwrap().len(), 3);
    assert!(!output.join(READY_FILE).exists());
    assert!(consume(&output, free_port(), &folders).is_err());
}

#[test]
fn native_perf_records_canonical_claims_for_logical_known_folder_paths() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let canonical_folders = folders(work.path());
    // Windows 的逻辑 Known Folder 路径和实际句柄路径可不同；此处以普通
    // 路径/extended canonical prefix 差异复现，真实 MSIX 映射由 GUI 样本核验。
    let logical_folders = KnownFolders {
        roaming: PathBuf::from(
            canonical_folders
                .roaming
                .to_string_lossy()
                .strip_prefix(r"\\?\")
                .unwrap(),
        ),
        local: PathBuf::from(
            canonical_folders
                .local
                .to_string_lossy()
                .strip_prefix(r"\\?\")
                .unwrap(),
        ),
    };
    assert_ne!(logical_folders.roaming, canonical_folders.roaming);
    let output = work.path().join("logical-known-folders");
    prepare(&output, &source, &logical_folders).unwrap();
    let owned: OwnedManifest =
        serde_json::from_value(read_json(&output.join(OWNED_FILE)).unwrap()).unwrap();
    assert_eq!(
        owned.owned_paths,
        vec![
            output.canonicalize().unwrap(),
            checked_dir(&logical_folders.roaming.join(&owned.identifier)).unwrap(),
            checked_dir(&logical_folders.local.join(&owned.identifier)).unwrap(),
        ]
    );
    assert_eq!(owned.roaming, owned.owned_paths[1]);
    assert_eq!(owned.local, owned.owned_paths[2]);
    consume(
        &output.canonicalize().unwrap(),
        free_port(),
        &logical_folders,
    )
    .unwrap();
}

#[test]
fn native_perf_chromium_logging_requires_explicit_run_mode() {
    for bad in [
        args(&[
            "--native-perf-prepare",
            "C:/new",
            "--fixture",
            "C:/source",
            "--native-perf-diagnostics",
            "chromium-log",
        ]),
        args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "arbitrary-args",
        ]),
        args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "chromium-log",
            "--native-perf-diagnostics",
            "chromium-log",
        ]),
    ] {
        assert!(parse_args(&bad).is_err());
    }
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222"
        ])),
        Ok(Mode::Run {
            chromium_log: false,
            ..
        })
    ));
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "chromium-log"
        ])),
        Ok(Mode::Run {
            chromium_log: true,
            ..
        })
    ));
}

#[test]
fn native_perf_chromium_log_is_opt_in_owned_new_and_never_reused() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    let output = work.path().join("diagnostic-run");
    prepare(&output, &source, &folders).unwrap();
    let mut config = consume(&output, free_port(), &folders).unwrap();
    let expected = format!("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port={} --remote-debugging-address=127.0.0.1", config.port);
    assert_eq!(config.browser_arguments(), expected);
    assert!(!config.root.join(CHROMIUM_LOG_MARKER).exists());
    config.enable_chromium_diagnostics().unwrap();
    let log = config.chromium_log.clone().unwrap();
    assert!(!log.to_string_lossy().starts_with(r"\\?\"));
    assert_eq!(
        log.canonicalize().unwrap(),
        config.root.join("temp").join(CHROMIUM_LOG_NAME)
    );
    assert_eq!(fs::metadata(&log).unwrap().len(), 0);
    assert_eq!(
        config.browser_arguments(),
        format!(
            "{expected} --enable-logging --v=1 --log-file=\"{}\"",
            log.display()
        )
    );
    let marker = read_json(&config.root.join(CHROMIUM_LOG_MARKER)).unwrap();
    assert_eq!(marker["scope"], "windows-native-perf-chromium-log");
    assert_eq!(marker["runId"], config.run_id);
    assert_eq!(marker["performanceSample"], false);
    assert_eq!(marker["nativeTempUnchanged"], true);
    fs::write(&log, b"preserve diagnostic bytes").unwrap();
    assert!(config.enable_chromium_diagnostics().is_err());
    assert_eq!(fs::read(&log).unwrap(), b"preserve diagnostic bytes");
}

#[test]
fn native_perf_chromium_log_rejects_preexisting_file_and_redirected_temp() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    for linked in [false, true] {
        let output = work.path().join(if linked {
            "linked-temp-run"
        } else {
            "existing-log-run"
        });
        prepare(&output, &source, &folders).unwrap();
        let mut config = consume(&output, free_port(), &folders).unwrap();
        if linked {
            let outside = work.path().join("outside-temp");
            fs::create_dir(&outside).unwrap();
            fs::remove_dir(config.root.join("temp")).unwrap(); // 仅空的自有测试目录。
            junction(&config.root.join("temp"), &outside);
            assert!(config.enable_chromium_diagnostics().is_err());
            assert!(!outside.join(CHROMIUM_LOG_NAME).exists());
        } else {
            let sentinel = config.root.join("temp").join(CHROMIUM_LOG_NAME);
            fs::write(&sentinel, b"preserve original log").unwrap();
            assert!(config.enable_chromium_diagnostics().is_err());
            assert_eq!(fs::read(sentinel).unwrap(), b"preserve original log");
        }
        assert!(config.chromium_log.is_none());
        assert!(!config.root.join(CHROMIUM_LOG_MARKER).exists());
    }
}

#[test]
fn native_perf_ordinary_tmp_is_explicit_logging_run_only() {
    for bad in [
        args(&[
            "--native-perf-prepare",
            "C:/new",
            "--fixture",
            "C:/fixture",
            "--native-perf-diagnostics",
            "chromium-log-ordinary-tmp",
        ]),
        args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "ordinary-tmp",
        ]),
        args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "chromium-log-ordinary-tmp",
            "--native-perf-diagnostics",
            "chromium-log",
        ]),
    ] {
        assert!(parse_args(&bad).is_err());
    }
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "chromium-log-ordinary-tmp"
        ])),
        Ok(Mode::Run {
            chromium_log: true,
            ordinary_tmp: true,
            ..
        })
    ));
    for values in [
        args(&["--native-perf-root", "C:/new", "--native-perf-port", "9222"]),
        args(&[
            "--native-perf-root",
            "C:/new",
            "--native-perf-port",
            "9222",
            "--native-perf-diagnostics",
            "chromium-log",
        ]),
    ] {
        assert!(matches!(
            parse_args(&values),
            Ok(Mode::Run {
                ordinary_tmp: false,
                ..
            })
        ));
    }
}

#[test]
fn native_perf_ordinary_tmp_records_exact_paths_and_refuses_reuse_or_aliases() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    let output = work.path().join("ordinary-tmp-run");
    prepare(&output, &source, &folders).unwrap();
    let mut config = consume(&output, free_port(), &folders).unwrap();
    let temp = config.root.join("temp");
    let tmp = config.ordinary_owned_temp().unwrap();
    let profile = config.root.join("profile");
    assert_eq!(tmp.canonicalize().unwrap(), temp);
    assert!(!tmp.to_string_lossy().starts_with(r"\\?\"));
    assert!(config
        .record_ordinary_tmp(
            temp.as_os_str(),
            tmp.as_os_str(),
            profile.as_os_str(),
            config.webview.as_os_str()
        )
        .is_err());
    config.enable_chromium_diagnostics().unwrap();
    // 等价但不同表示的 TEMP 或 extended TMP 均拒绝，避免同时改变第二个变量。
    for (actual_temp, actual_tmp, actual_profile, actual_webview) in [
        (&tmp, &tmp, &profile, &config.webview),
        (&temp, &temp, &profile, &config.webview),
        (
            &temp,
            &work.path().join("outside"),
            &profile,
            &config.webview,
        ),
        (&temp, &tmp, &config.root, &config.webview),
        (&temp, &tmp, &profile, &config.root),
    ] {
        assert!(config
            .record_ordinary_tmp(
                actual_temp.as_os_str(),
                actual_tmp.as_os_str(),
                actual_profile.as_os_str(),
                actual_webview.as_os_str()
            )
            .is_err());
        assert!(!config.root.join(ORDINARY_TMP_MARKER).exists());
    }
    config
        .record_ordinary_tmp(
            temp.as_os_str(),
            tmp.as_os_str(),
            profile.as_os_str(),
            config.webview.as_os_str(),
        )
        .unwrap();
    let marker = read_json(&config.root.join(ORDINARY_TMP_MARKER)).unwrap();
    assert_eq!(marker["scope"], "windows-native-perf-ordinary-tmp");
    assert_eq!(marker["runId"], config.run_id);
    assert_eq!(marker["pid"], std::process::id());
    assert_eq!(marker["port"], config.port);
    assert_eq!(marker["performanceSample"], false);
    assert_eq!(marker["temp"], json!(temp));
    assert_eq!(marker["tmp"], json!(tmp));
    assert_eq!(marker["userProfile"], json!(profile));
    assert_eq!(marker["webview"], json!(config.webview));
    let before = fs::read(config.root.join(ORDINARY_TMP_MARKER)).unwrap();
    assert!(config
        .record_ordinary_tmp(
            temp.as_os_str(),
            tmp.as_os_str(),
            profile.as_os_str(),
            config.webview.as_os_str()
        )
        .is_err());
    assert_eq!(
        fs::read(config.root.join(ORDINARY_TMP_MARKER)).unwrap(),
        before
    );
}

#[test]
fn native_perf_ordinary_tmp_rejects_redirected_directory() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    let output = work.path().join("ordinary-tmp-junction");
    prepare(&output, &source, &folders).unwrap();
    let config = consume(&output, free_port(), &folders).unwrap();
    let outside = work.path().join("outside-temp");
    fs::create_dir(&outside).unwrap();
    fs::remove_dir(config.root.join("temp")).unwrap(); // 只删除本用例新建的空目录。
    junction(&config.root.join("temp"), &outside);
    assert!(config.ordinary_owned_temp().is_err());
    assert!(!config.root.join(ORDINARY_TMP_MARKER).exists());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}
