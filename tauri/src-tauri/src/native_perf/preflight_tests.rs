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

// Runtime 组件回归只使用显式 TempDir 和合成 PE 头，不启动 Loader/GUI，
// 不读取已安装 Runtime、用户 Vault 或更改任何全局环境。
fn copied_runtime_config(parent: &Path) -> RuntimeConfig {
    let root = parent.join("run");
    fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    for child in ["temp", "profile", "webview", "runtime"] {
        fs::create_dir(root.join(child)).unwrap();
    }
    RuntimeConfig {
        vault: root.join("vault"),
        identifier: format!("{IDENTIFIER_PREFIX}{}", "a".repeat(32)),
        webview: root.join("webview"),
        root,
        port: 44123,
        run_id: "a".repeat(32),
        chromium_log: None,
        sdk_cdp: false,
        sdk_journey: false,
        media_journey: false,
        pdf_diagnostic: false,
        pdf_preview: false,
        ocr_journey: false,
        pdf_resource: None,
        pdf_ciphertext_sha256: None,
        startup: None,
        object_count: 100,
    }
}

fn copied_runtime_manifest(config: &RuntimeConfig) -> Value {
    let folder = config.root.join("runtime");
    let mut files = Vec::new();
    let mut cores = Vec::new();
    for relative in runtime::CORE_FILES {
        let target = folder.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let mut pe = vec![0_u8; 96];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64_u32.to_le_bytes());
        pe[64..70].copy_from_slice(b"PE\0\0\x64\x86");
        pe.extend_from_slice(relative.as_bytes());
        fs::write(&target, pe).unwrap();
        let sha = sha256_file(&target).unwrap();
        files.push(json!({
            "relative": relative, "bytes": fs::metadata(&target).unwrap().len(), "sha256": sha,
        }));
        cores.push(json!({
            "relative": relative, "sha256": sha, "architecture": "AMD64",
            "fileVersion": "153.0.4234.48", "productVersion": "153.0.4234.48",
            "signatureStatus": "Valid", "signerSubject": "CN=Microsoft Corporation",
            "signerThumbprint": "A".repeat(40),
        }));
    }
    let resource = folder.join("Locales/en-US.pak");
    fs::create_dir(resource.parent().unwrap()).unwrap();
    fs::write(&resource, b"public synthetic Runtime resource").unwrap();
    files.push(json!({
        "relative": "Locales/en-US.pak", "bytes": fs::metadata(&resource).unwrap().len(),
        "sha256": sha256_file(&resource).unwrap(),
    }));
    fs::create_dir(folder.join("EmptyPublicDirectory")).unwrap();
    files.sort_by(|left, right| {
        left["relative"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .cmp(right["relative"].as_str().unwrap().encode_utf16())
    });
    json!({
        "schemaVersion": 1, "scope": "windows-native-perf-copied-runtime",
        "root": config.root, "runId": config.run_id, "sourceKind": "copied-local-evergreen",
        "sourceFolder": r"C:\synthetic-installed-runtime\153.0.4234.48",
        "expectedVersion": "153.0.4234.48", "runtimeFolder": folder,
        "files": files, "directories": ["EBWebView", "EBWebView/x64", "EmptyPublicDirectory", "Locales"], "coreFiles": cores,
    })
}

fn publish_test_runtime(config: &RuntimeConfig, manifest: &Value) {
    fs::write(
        config.root.join(runtime::MANIFEST_FILE),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn native_perf_copied_runtime_requires_explicit_logging_and_forbids_mode_conflicts() {
    let run = [
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
    ];
    for tail in [
        vec!["--native-perf-runtime", "C:/owned/runtime"],
        vec!["--native-perf-runtime"],
        vec![
            "--native-perf-diagnostics",
            "chromium-log-ordinary-tmp",
            "--native-perf-runtime",
            "C:/owned/runtime",
        ],
        vec![
            "--native-perf-diagnostics",
            "chromium-log",
            "--native-perf-runtime",
            "C:/owned/runtime",
            "--native-perf-runtime",
            "C:/owned/runtime",
        ],
    ] {
        let mut values = args(&run);
        values.extend(args(&tail));
        assert!(parse_args(&values).is_err());
    }
    assert!(parse_args(&args(&[
        "--native-perf-prepare",
        "C:/owned",
        "--fixture",
        "C:/fixture",
        "--native-perf-runtime",
        "C:/owned/runtime",
    ]))
    .is_err());
    assert!(parse_args(&args(&[
        "--native-perf-prepare",
        "C:/owned",
        "--fixture",
        "C:/fixture",
        "--native-perf-diagnostics",
        "chromium-log",
        "--native-perf-runtime",
        "C:/owned/runtime",
    ]))
    .is_err());
    let mut valid = args(&run);
    valid.extend(args(&[
        "--native-perf-runtime",
        "C:/owned/runtime",
        "--native-perf-diagnostics",
        "chromium-log",
    ]));
    assert!(matches!(
        parse_args(&valid),
        Ok(Mode::Run {
            chromium_log: true,
            ordinary_tmp: false,
            copied_runtime: Some(_),
            ..
        })
    ));
    for tail in [
        vec![],
        vec!["--native-perf-diagnostics", "chromium-log"],
        vec!["--native-perf-diagnostics", "chromium-log-ordinary-tmp"],
    ] {
        let mut values = args(&run);
        values.extend(args(&tail));
        assert!(matches!(
            parse_args(&values),
            Ok(Mode::Run {
                copied_runtime: None,
                ..
            })
        ));
    }
}

#[test]
fn native_perf_copied_runtime_preserves_flags_and_records_exact_owned_selection_once() {
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let manifest = copied_runtime_manifest(&config);
    publish_test_runtime(&config, &manifest);
    let requested = config.root.join("runtime");
    assert!(runtime::validate(&config, &requested).is_err());
    config.enable_chromium_diagnostics().unwrap();
    let browser_args = config.browser_arguments();
    let selection = runtime::validate(&config, &requested).unwrap();
    let ordinary = selection.browser_executable_folder.clone();
    assert!(!ordinary.to_string_lossy().starts_with(r"\\?\"));
    assert_eq!(ordinary.canonicalize().unwrap(), requested);
    // CLI 普通绝对路径可进入，但 manifest/selected 身份仍要求原 canonical 表示。
    runtime::validate(&config, &ordinary).unwrap();
    // 仅注入返回字符串验证拒绝分支；不调用真实 SDK 或宣称合成 PE 是 Runtime。
    for (actual, available) in [
        (&requested, "153.0.4234.48"),
        (&ordinary, "154.0.4258.37"),
        (&ordinary, ""),
        (&ordinary, "153.0.4234.48 beta"),
        (&ordinary, "0153.0.4234.48"),
    ] {
        assert!(selection
            .record_actual(&config, actual.as_os_str(), available)
            .is_err());
        assert!(!config.root.join(runtime::SELECTED_FILE).exists());
    }
    selection
        .record_actual(&config, ordinary.as_os_str(), "153.0.4234.48")
        .unwrap();
    assert_eq!(config.browser_arguments(), browser_args);
    let marker = read_json(&config.root.join(runtime::SELECTED_FILE)).unwrap();
    assert_eq!(marker["scope"], "windows-native-perf-selected-runtime");
    assert_eq!(marker["mode"], "copied-local-evergreen");
    assert_eq!(marker["performanceSample"], false);
    assert_eq!(marker["root"], json!(config.root));
    assert_eq!(marker["runId"], config.run_id);
    assert_eq!(marker["pid"], std::process::id());
    assert_eq!(marker["port"], config.port);
    assert_eq!(marker["runtimeFolder"], json!(requested));
    assert_eq!(marker["browserExecutableFolder"], json!(ordinary));
    assert_eq!(marker["expectedVersion"], "153.0.4234.48");
    assert_eq!(marker["availableVersion"], "153.0.4234.48");
    assert_eq!(
        marker["manifestSha256"],
        sha256_file(&config.root.join(runtime::MANIFEST_FILE)).unwrap()
    );
    assert_eq!(
        marker["executableSha256"],
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|proof| proof["relative"] == "msedgewebview2.exe")
            .unwrap()["sha256"]
    );
    assert_eq!(marker["sourceFolder"], manifest["sourceFolder"]);
    let original = fs::read(config.root.join(runtime::SELECTED_FILE)).unwrap();
    assert!(selection
        .record_actual(&config, ordinary.as_os_str(), "153.0.4234.48")
        .is_err());
    assert!(runtime::validate(&config, &requested).is_err());
    assert_eq!(
        fs::read(config.root.join(runtime::SELECTED_FILE)).unwrap(),
        original
    );
}

#[test]
fn native_perf_copied_runtime_rejects_identity_metadata_and_noncanonical_manifests() {
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let manifest = copied_runtime_manifest(&config);
    config.enable_chromium_diagnostics().unwrap();
    let requested = config.root.join("runtime");
    for (key, value) in [
        ("schemaVersion", json!(2)),
        ("scope", json!("windows-native-perf-owned")),
        (
            "root",
            json!(config.root.to_str().unwrap().strip_prefix(r"\\?\").unwrap()),
        ),
        ("runId", json!("b".repeat(32))),
        ("sourceKind", json!("fixed-version-package")),
        ("sourceFolder", json!("relative/runtime")),
        ("sourceFolder", json!(r"\\server\share\runtime")),
        ("expectedVersion", json!("153")),
        ("expectedVersion", json!("0153.0.4234.48")),
        ("runtimeFolder", json!(work.path())),
        (
            "runtimeFolder",
            json!(requested.to_str().unwrap().strip_prefix(r"\\?\").unwrap()),
        ),
        ("unknown", json!(true)),
    ] {
        let mut bad = manifest.clone();
        bad[key] = value;
        publish_test_runtime(&config, &bad);
        assert!(
            runtime::validate(&config, &requested).is_err(),
            "accepted field {key}"
        );
    }
    for (key, value) in [
        ("relative", json!("msedge-other.exe")),
        ("sha256", json!("0".repeat(64))),
        ("architecture", json!("ARM64")),
        ("fileVersion", json!("154.0.4258.37")),
        ("productVersion", json!("154.0.4258.37")),
        ("signatureStatus", json!("NotSigned")),
        ("signerSubject", json!("CN=Untrusted Publisher")),
        ("signerThumbprint", json!("bad")),
    ] {
        let mut bad = manifest.clone();
        bad["coreFiles"][0][key] = value;
        publish_test_runtime(&config, &bad);
        assert!(
            runtime::validate(&config, &requested).is_err(),
            "accepted core field {key}"
        );
    }
    let mut duplicate = manifest.clone();
    duplicate["coreFiles"][1] = duplicate["coreFiles"][0].clone();
    publish_test_runtime(&config, &duplicate);
    assert!(runtime::validate(&config, &requested).is_err());
    publish_test_runtime(&config, &manifest);
    assert!(runtime::validate(&config, work.path()).is_err());
    assert!(runtime::validate(&config, Path::new("runtime")).is_err());
    assert!(!config.root.join(runtime::SELECTED_FILE).exists());
}

#[test]
fn native_perf_copied_runtime_rejects_unsafe_relative_proofs_and_resource_limits() {
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let manifest = copied_runtime_manifest(&config);
    config.enable_chromium_diagnostics().unwrap();
    let requested = config.root.join("runtime");
    for relative in [
        "",
        ".",
        "..",
        "dir/../msedgewebview2.exe",
        "dir/./file",
        "/absolute",
        "C:/absolute",
        "dir\\file",
        "dir//file",
        "dir/file ",
        "dir/file.",
        "CON",
        "aux.dll",
        "COM1.bin",
        "LPT².bin",
    ] {
        let mut bad = manifest.clone();
        bad["files"][3]["relative"] = json!(relative);
        publish_test_runtime(&config, &bad);
        assert!(
            runtime::validate(&config, &requested).is_err(),
            "accepted relative {relative}"
        );
    }
    let mut bad = manifest.clone();
    bad["files"]
        .as_array_mut()
        .unwrap()
        .push(manifest["files"][0].clone());
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested).is_err());
    bad = manifest.clone();
    bad["files"][3]["sha256"] = json!("A".repeat(64));
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested).is_err());
    bad = manifest.clone();
    bad["files"][0]["bytes"] = json!(runtime::MAX_TOTAL_BYTES + 1);
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested)
        .unwrap_err()
        .contains("1 GiB"));
    bad["files"] = json!(vec![manifest["files"][0].clone(); runtime::MAX_FILES + 1]);
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested).is_err());
    fs::write(
        config.root.join(runtime::MANIFEST_FILE),
        vec![b' '; runtime::MAX_MANIFEST_BYTES as usize + 1],
    )
    .unwrap();
    assert!(runtime::validate(&config, &requested)
        .unwrap_err()
        .contains("4 MiB"));
    assert!(!config.root.join(runtime::SELECTED_FILE).exists());
}

#[test]
fn native_perf_copied_runtime_requires_the_entire_unchanged_regular_tree() {
    for mutation in [
        "extra-file",
        "missing-file",
        "changed-bytes",
        "changed-size",
        "extra-dir",
        "missing-empty-dir",
        "wrong-pe",
    ] {
        let work = tempfile::tempdir().unwrap();
        let mut config = copied_runtime_config(work.path());
        let mut manifest = copied_runtime_manifest(&config);
        config.enable_chromium_diagnostics().unwrap();
        let requested = config.root.join("runtime");
        let resource = requested.join("Locales/en-US.pak");
        match mutation {
            "extra-file" => fs::write(requested.join("extra.bin"), b"unlisted").unwrap(),
            "missing-file" => fs::remove_file(&resource).unwrap(),
            "changed-bytes" => {
                let bytes = fs::metadata(&resource).unwrap().len() as usize;
                fs::write(&resource, vec![b'x'; bytes]).unwrap();
            }
            "changed-size" => fs::write(&resource, b"short").unwrap(),
            "extra-dir" => fs::create_dir(requested.join("extra-empty")).unwrap(),
            "missing-empty-dir" => fs::remove_dir(requested.join("EmptyPublicDirectory")).unwrap(),
            "wrong-pe" => {
                let target = requested.join("msedgewebview2.exe");
                let mut bytes = fs::read(&target).unwrap();
                bytes[68..70].copy_from_slice(&0x14c_u16.to_le_bytes());
                fs::write(&target, bytes).unwrap();
                let sha = sha256_file(&target).unwrap();
                let proof = manifest["files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|proof| proof["relative"] == "msedgewebview2.exe")
                    .unwrap();
                proof["sha256"] = json!(sha);
                manifest["coreFiles"][0]["sha256"] = json!(sha);
            }
            _ => unreachable!(),
        }
        publish_test_runtime(&config, &manifest);
        assert!(
            runtime::validate(&config, &requested).is_err(),
            "accepted mutation {mutation}"
        );
        assert!(!config.root.join(runtime::SELECTED_FILE).exists());
    }
}

#[test]
fn native_perf_copied_runtime_rejects_reparse_entries_before_external_reads() {
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let manifest = copied_runtime_manifest(&config);
    config.enable_chromium_diagnostics().unwrap();
    let requested = config.root.join("runtime");
    let outside = work.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("untouched");
    fs::write(&sentinel, b"owned test sentinel").unwrap();
    let core = requested.join("EBWebView/x64/EmbeddedBrowserWebView.dll");
    fs::remove_file(&core).unwrap(); // 只删除此用例新建的合成文件。
    fs::remove_dir(core.parent().unwrap()).unwrap(); // 只删除此用例新建的空目录。
    junction(core.parent().unwrap(), &outside);
    publish_test_runtime(&config, &manifest);
    assert!(runtime::validate(&config, &requested).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"owned test sentinel");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert!(!config.root.join(runtime::SELECTED_FILE).exists());
}

#[test]
fn native_perf_copied_runtime_requires_complete_ordered_directory_proofs() {
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let manifest = copied_runtime_manifest(&config);
    config.enable_chromium_diagnostics().unwrap();
    let requested = config.root.join("runtime");
    for directories in [
        json!(["EBWebView", "EBWebView/x64", "Locales"]),
        json!([
            "EBWebView",
            "EBWebView/x64",
            "EmptyPublicDirectory",
            "Locales",
            "MissingEmpty"
        ]),
        json!([
            "Locales",
            "EmptyPublicDirectory",
            "EBWebView/x64",
            "EBWebView"
        ]),
        json!([
            "EBWebView",
            "EBWebView",
            "EBWebView/x64",
            "EmptyPublicDirectory",
            "Locales"
        ]),
        json!([
            "EBWebView",
            "EBWebView/x64",
            "EmptyPublicDirectory",
            "Locales",
            "locales"
        ]),
        json!([
            "EBWebView",
            "EBWebView/x64",
            "EmptyPublicDirectory",
            "Locales",
            "msedge.dll"
        ]),
        json!([
            "EBWebView",
            "EBWebView/x64",
            "EmptyPublicDirectory",
            "Locales",
            "outside/../invalid"
        ]),
        json!(vec!["synthetic"; runtime::MAX_DIRECTORIES + 1]),
        json!([vec!["deep"; 65].join("/")]),
    ] {
        let mut bad = manifest.clone();
        bad["directories"] = directories;
        publish_test_runtime(&config, &bad);
        assert!(runtime::validate(&config, &requested).is_err());
    }
    let mut bad = manifest.clone();
    bad.as_object_mut().unwrap().remove("directories");
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested).is_err());
    bad = manifest.clone();
    bad["files"].as_array_mut().unwrap().reverse();
    publish_test_runtime(&config, &bad);
    assert!(runtime::validate(&config, &requested).is_err());
    publish_test_runtime(&config, &manifest);
    runtime::validate(&config, &requested).unwrap(); // 明确接纳声明且存在的空目录。
}

#[test]
fn native_perf_sdk_cdp_is_exclusive_run_only_and_preserves_browser_flags() {
    let run = [
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
    ];
    let mut valid = args(&run);
    valid.extend(args(&["--native-perf-diagnostics", "sdk-cdp"]));
    assert!(matches!(
        parse_args(&valid),
        Ok(Mode::Run {
            sdk_cdp: true,
            sdk_journey: false,
            media_journey: false,
            pdf_diagnostic: false,
            pdf_preview: false,
            chromium_log: false,
            ordinary_tmp: false,
            copied_runtime: None,
            ..
        })
    ));
    for tail in [
        vec!["--native-perf-diagnostics", "chromium-log"],
        vec!["--native-perf-diagnostics", "chromium-log-ordinary-tmp"],
        vec!["--native-perf-diagnostics", "sdk-cdp"],
        vec!["--native-perf-runtime", "C:/owned/runtime"],
    ] {
        let mut bad = valid.clone();
        bad.extend(args(&tail));
        assert!(parse_args(&bad).is_err());
    }
    assert!(parse_args(&args(&[
        "--native-perf-prepare",
        "C:/new",
        "--fixture",
        "C:/fixture",
        "--native-perf-diagnostics",
        "sdk-cdp"
    ]))
    .is_err());
    let work = tempfile::tempdir().unwrap();
    let mut config = copied_runtime_config(work.path());
    let original = config.browser_arguments();
    config.sdk_cdp = true;
    sdk_cdp::claim_request(&config).unwrap();
    assert_eq!(config.browser_arguments(), original);
    let marker = read_json(&config.root.join(sdk_cdp::REQUESTED_FILE)).unwrap();
    assert_eq!(marker["scope"], "windows-native-sdk-cdp-requested");
    assert_eq!(marker["performanceSample"], false);
    assert_eq!(marker["root"], json!(config.root));
    assert_eq!(marker["runId"], config.run_id);
    assert_eq!(marker["pid"], std::process::id());
    assert_eq!(marker["port"], config.port);
    assert_eq!(marker["windowLabel"], "main");
    assert!(sdk_cdp::claim_request(&config).is_err());
}

#[test]
fn sdk_journey_args_are_exclusive_and_never_prepare() {
    let run = args(&[
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
        "--native-perf-journey",
        "sdk-input",
    ]);
    assert!(matches!(
        parse_args(&run),
        Ok(Mode::Run {
            sdk_journey: true,
            sdk_cdp: false,
            chromium_log: false,
            ..
        })
    ));
    for tail in [
        args(&["--native-perf-journey", "sdk-input"]),
        args(&["--native-perf-diagnostics", "sdk-cdp"]),
        args(&["--native-perf-diagnostics", "chromium-log"]),
        args(&["--native-perf-runtime", "C:/runtime"]),
    ] {
        let mut bad = run.clone();
        bad.extend(tail);
        assert!(parse_args(&bad).is_err());
    }
    assert!(parse_args(&args(&[
        "--native-perf-prepare",
        "C:/owned",
        "--fixture",
        "C:/fixture",
        "--native-perf-journey",
        "sdk-input"
    ]))
    .is_err());
}

#[test]
fn native_perf_warm_ticket_preserves_fixture_and_legacy_consume_and_is_one_use() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let work = tempfile::tempdir().unwrap();
    let source = source(work.path());
    let folders = folders(work.path());
    let output = work.path().join("warm-test");
    prepare(&output, &source, &folders).unwrap();
    let mut config = consume(&output, free_port(), &folders).unwrap();
    startup::configure_initial(&mut config).unwrap();
    let owner = checked_manifest(&config.root, &folders).unwrap();
    // These unit-test process identities are deliberately nonexistent; this is not GUI evidence.
    let pid = u32::MAX;
    let browser = u32::MAX - 1;
    let consumed = json!({"schemaVersion":1,"scope":"windows-native-perf-consumed","root":config.root,"runId":config.run_id,"pid":pid,"port":config.port});
    fs::write(
        config.root.join(CONSUMED_FILE),
        serde_json::to_vec(&consumed).unwrap(),
    )
    .unwrap();
    let observer = json!({"schemaVersion":1,"scope":"windows-native-tauri-invoke-observer","runId":config.run_id,"valid":true,"total":0,"observedCount":0,"invalidReasons":[],"installedAtMs":0,"timeOriginMs":1000,"maxEvents":16384,"commands":[]});
    let ipc = json!({"attempts":0,"commands":{},"reason":null});
    let trust = json!({"pointer":0,"text":0,"untrusted":0});
    let proof = json!({"schemaVersion":1,"scope":"windows-native-sdk-startup","root":config.root,"runId":config.run_id,"pid":pid,"port":config.port,"objectCount":100,"browserPid":browser,"binding":{"source":"http://tauri.localhost/login","mainFrameId":"unit-frame","loaderId":"unit-loader","timeOriginMs":null,"navigationEvents":0,"frameCreatedEvents":0},"timeOriginMs":1000,"inputMethod":"SDK-CDP-read-only","success":true,"reason":null,"failedStep":null,"calls":["Runtime.evaluate","Page.getFrameTree"],"phases":[{"name":"startup","success":true,"durationMs":10,"ipc":ipc,"inputTrust":trust,"fromAtMs":0,"toAtMs":5}],"lastProbe":{"schemaVersion":1,"scope":"windows-native-sdk-ui-probe","runId":config.run_id,"step":"startup","outcome":"ready","href":"http://tauri.localhost/login","origin":"http://tauri.localhost","timeOriginMs":1000,"atMs":5,"rootPresent":true,"frameCount":0,"target":null,"inputTrust":trust,"observer":observer},"ipcAll":ipc,"elapsedMs":1,"unmeasured":startup::unmeasured(1),"startup":config.startup});
    let proof_path = config.evidence_root().join("native-perf-sdk-journey.json");
    write_new_json(&proof_path, &proof).unwrap();
    let hashes = [
        sha256_file(&config.root.join(CONSUMED_FILE)).unwrap(),
        sha256_file(&proof_path).unwrap(),
    ];
    let preferences_path = config.vault.join("ui_preferences.json");
    let mut preferences = read_json(&preferences_path).unwrap();
    let sources = crate::commands::update::native_perf_cache_candidates().unwrap();
    preferences["updateSources"] = json!({"manifest":{"url":sources["manifest"][0],"probedAt":chrono::Utc::now().timestamp()},"release":null,"lastChannel":"manifest"});
    let preference_bytes = serde_json::to_vec(&preferences).unwrap();
    fs::write(&preferences_path, &preference_bytes).unwrap();
    assert!(checked_manifest(&config.root, &folders).is_err());
    assert!(checked_startup_manifest(&config.root, &folders).is_ok());
    // Startup permits the producer-owned cache, never changes to immutable encrypted data.
    let accounts_path = config.vault.join("accounts.json");
    let original_accounts = fs::read(&accounts_path).unwrap();
    let mut modified_accounts = original_accounts.clone();
    modified_accounts.push(b'\n');
    fs::write(&accounts_path, modified_accounts).unwrap();
    assert!(checked_startup_manifest(&config.root, &folders).is_err());
    fs::write(&accounts_path, original_accounts).unwrap();
    let exe = std::env::current_exe().unwrap();
    let stopped = json!({"schemaVersion":1,"scope":"windows-native-sdk-startup-stopped","root":config.root,"ownerRunId":config.run_id,"pid":pid,"browserPid":browser,"ownedSha256":sha256_file(&config.root.join(OWNED_FILE)).unwrap(),"consumedSha256":hashes[0],"proofSha256":hashes[1],"exeSha256":sha256_file(&exe).unwrap(),"uiPreferencesSha256":sha256_file(&config.vault.join("ui_preferences.json")).unwrap(),"identities":[{"pid":pid,"creationMs":1,"executableName":exe.file_name().unwrap().to_str().unwrap()},{"pid":browser,"creationMs":1,"executableName":"msedgewebview2.exe"}]});
    write_new_json(
        &config.root.join("native-perf-startup-stopped.json"),
        &stopped,
    )
    .unwrap();
    assert!(consume(&config.root, free_port(), &folders).is_err());
    let prepared = startup::prepare_restart(&config.root, &folders).unwrap();
    assert_ne!(prepared["runId"], config.run_id);
    assert!(startup::prepare_restart(&config.root, &folders).is_err());
    // Semantically identical cache with changed bytes must still fail the first-stop checkpoint.
    let mut changed_preferences = preference_bytes.clone();
    changed_preferences.push(b'\n');
    fs::write(&preferences_path, changed_preferences).unwrap();
    assert!(startup::consume_restart(&config.root, free_port(), &folders).is_err());
    fs::write(&preferences_path, &preference_bytes).unwrap();
    let warm = startup::consume_restart(&config.root, free_port(), &folders).unwrap();
    assert_eq!(warm.vault, config.vault);
    assert_eq!(warm.webview, config.webview);
    assert_eq!(warm.identifier, config.identifier);
    assert_ne!(warm.run_id, config.run_id);
    assert_eq!(warm.startup.unwrap().owner_run_id, config.run_id);
    assert!(startup::consume_restart(&config.root, free_port(), &folders).is_err());
    assert_eq!(
        sha256_file(&config.root.join(CONSUMED_FILE)).unwrap(),
        hashes[0]
    );
    assert_eq!(sha256_file(&proof_path).unwrap(), hashes[1]);
    assert_eq!(
        serde_json::to_value(
            checked_startup_manifest(&config.root, &folders)
                .unwrap()
                .fixture
        )
        .unwrap(),
        serde_json::to_value(owner.fixture).unwrap()
    );
}

#[test]
fn native_perf_media_mode_is_explicit_and_exclusive() {
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-media-prepare",
            "C:/new",
            "--fixture",
            "C:/media"
        ])),
        Ok(Mode::MediaPrepare { .. })
    ));
    assert!(matches!(
        parse_args(&args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222",
            "--native-perf-journey",
            "sdk-media"
        ])),
        Ok(Mode::Run {
            media_journey: true,
            pdf_diagnostic: false,
            pdf_preview: false,
            sdk_journey: true,
            startup_only: false,
            ..
        })
    ));
    for extra in [
        args(&["--native-perf-restart", "startup"]),
        args(&["--native-perf-diagnostics", "sdk-cdp"]),
        args(&["--native-perf-journey", "sdk-input"]),
    ] {
        let mut run = args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222",
            "--native-perf-journey",
            "sdk-media",
        ]);
        run.extend(extra);
        assert!(parse_args(&run).is_err());
    }
    assert!(parse_args(&args(&[
        "--native-perf-media-prepare",
        "C:/new",
        "--native-perf-prepare",
        "C:/other",
        "--fixture",
        "C:/media"
    ]))
    .is_err());
}
#[test]
fn native_perf_media_preparation_binds_raw_bytes_and_preserves_v1_rejection() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let old = std::env::var_os("SOLOSOUL_SECURE");
    std::env::set_var("SOLOSOUL_SECURE", "1");
    let work = tempfile::tempdir().unwrap();
    let source = work.path().join("source");
    media_fixture::write_test_fixture(&source);
    match old {
        Some(v) => std::env::set_var("SOLOSOUL_SECURE", v),
        None => std::env::remove_var("SOLOSOUL_SECURE"),
    };
    let folders = folders(work.path());
    let marker_hash = sha256_file(&source.join("rf312-media-fixture.json")).unwrap();
    let db_hash = sha256_file(&source.join("acc_rf312_100/vault.db")).unwrap();
    let legacy = work.path().join("legacy-rejected");
    assert!(prepare(&legacy, &source, &folders).is_err());
    assert!(!legacy.exists());
    let output = work.path().join("owned");
    let value = prepare_mode(&output, &source, &folders, true).unwrap();
    assert_eq!(value["success"], true);
    let root = output.canonicalize().unwrap();
    assert!(checked_manifest(&root, &folders).is_err());
    let proof = checked_manifest_mode(&root, &folders, false, true).unwrap();
    assert_eq!(proof.fixture.object_count, 100);
    let file = root.join("vault/acc_rf312_100/vault.db");
    let original = fs::read(&file).unwrap();
    let mut tampered = original.clone();
    tampered[100] ^= 1;
    fs::write(&file, &tampered).unwrap();
    assert!(consume_mode(&root, free_port(), &folders, true).is_err());
    assert!(!root.join(CONSUMED_FILE).exists());
    fs::write(&file, original).unwrap();
    let config = consume_mode(&root, free_port(), &folders, true).unwrap();
    assert!(config.media_journey);
    assert!(consume_mode(&root, free_port(), &folders, true).is_err());
    assert_eq!(
        sha256_file(&source.join("rf312-media-fixture.json")).unwrap(),
        marker_hash
    );
    assert_eq!(
        sha256_file(&source.join("acc_rf312_100/vault.db")).unwrap(),
        db_hash
    );
}

#[test]
fn native_perf_pdf_diagnostic_is_explicit_media_only_and_exclusive() {
    let base = args(&[
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
        "--native-perf-journey",
        "sdk-pdf-diagnostic",
    ]);
    assert!(matches!(
        parse_args(&base),
        Ok(Mode::Run {
            media_journey: true,
            pdf_diagnostic: true,
            pdf_preview: false,
            sdk_journey: true,
            startup_only: false,
            ..
        })
    ));
    for extra in [
        args(&["--native-perf-restart", "startup"]),
        args(&["--native-perf-diagnostics", "sdk-cdp"]),
        args(&["--native-perf-journey", "sdk-media"]),
    ] {
        let mut input = base.clone();
        input.extend(extra);
        assert!(parse_args(&input).is_err());
    }
    for mode in ["sdk-media", "sdk-input", "sdk-startup"] {
        let ordinary = args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222",
            "--native-perf-journey",
            mode,
        ]);
        assert!(matches!(
            parse_args(&ordinary),
            Ok(Mode::Run {
                pdf_diagnostic: false,
                pdf_preview: false,
                ..
            })
        ));
    }
}

#[test]
fn native_perf_pdf_first_page_mode_is_explicit_and_keeps_diagnostic_exclusive() {
    let base = args(&[
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
        "--native-perf-journey",
        "sdk-pdf-preview",
    ]);
    assert!(matches!(
        parse_args(&base),
        Ok(Mode::Run {
            sdk_journey: true,
            media_journey: true,
            pdf_preview: true,
            pdf_diagnostic: false,
            startup_only: false,
            ..
        })
    ));
    for extra in [
        args(&["--native-perf-restart", "startup"]),
        args(&["--native-perf-diagnostics", "sdk-cdp"]),
        args(&["--native-perf-journey", "sdk-pdf-diagnostic"]),
    ] {
        let mut bad = base.clone();
        bad.extend(extra);
        assert!(parse_args(&bad).is_err());
    }
}

#[test]
fn native_perf_first_ocr_is_exclusive_and_never_aliases_preview_or_startup() {
    let base = args(&[
        "--native-perf-root",
        "C:/owned",
        "--native-perf-port",
        "9222",
        "--native-perf-journey",
        "sdk-ocr",
    ]);
    assert!(matches!(
        parse_args(&base),
        Ok(Mode::Run {
            sdk_journey: true,
            media_journey: true,
            ocr_journey: true,
            pdf_preview: false,
            pdf_diagnostic: false,
            startup_only: false,
            ..
        })
    ));
    for extra in [
        args(&["--native-perf-restart", "startup"]),
        args(&["--native-perf-diagnostics", "sdk-cdp"]),
        args(&["--native-perf-journey", "sdk-media"]),
        args(&["--native-perf-journey", "sdk-pdf-preview"]),
    ] {
        let mut bad = base.clone();
        bad.extend(extra);
        assert!(parse_args(&bad).is_err());
    }
    for mode in [
        "sdk-input",
        "sdk-media",
        "sdk-pdf-preview",
        "sdk-pdf-diagnostic",
        "sdk-startup",
    ] {
        let input = args(&[
            "--native-perf-root",
            "C:/owned",
            "--native-perf-port",
            "9222",
            "--native-perf-journey",
            mode,
        ]);
        assert!(matches!(
            parse_args(&input),
            Ok(Mode::Run {
                ocr_journey: false,
                ..
            })
        ));
    }
}
