use super::tests::{confirm_prompt, unlocked_app};
use super::*;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use solosoul_core::Profile;
use std::collections::{BTreeMap, BTreeSet};

const GUI_V2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tauri/crates/solosoul-core/tests/fixtures/profile_backup/gui-v2-base64.json"
));
const CLI_V2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tauri/crates/solosoul-core/tests/fixtures/profile_backup/cli-v2-array.json"
));
const LEGACY_ARRAY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tauri/crates/solosoul-core/tests/fixtures/profile_backup/legacy-v1-array.json"
));
const LEGACY_BASE64: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tauri/crates/solosoul-core/tests/fixtures/profile_backup/legacy-v1-base64.json"
));

fn parse_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn profile_state(app: &App) -> BTreeMap<String, Value> {
    let vault = app.vault_service.get_vault_store().unwrap();
    vault
        .list_profiles()
        .unwrap()
        .into_iter()
        .map(|summary| {
            let profile = vault.load_profile(&summary.id).unwrap().unwrap();
            (summary.id, serde_json::to_value(profile).unwrap())
        })
        .collect()
}

fn write_restore_input(app: &App, bytes: &[u8]) {
    let dir = backups_dir(app);
    fs::create_dir_all(&dir).unwrap();
    crate::util::write_private_file(&dir.join("rf013.solosoul_backup"), bytes).unwrap();
}

fn restore_after_confirmation(app: &mut App, bytes: &[u8]) -> (DateTime<Utc>, DateTime<Utc>) {
    write_restore_input(app, bytes);
    app.error_message = None;
    app.success_message = None;
    let before = profile_state(app);
    handle(app, &["restore", "rf013"]).unwrap();
    assert_eq!(profile_state(app), before, "确认前不能写入");
    let started = Utc::now();
    confirm_prompt(app);
    let finished = Utc::now();
    assert!(app.error_message.is_none(), "{:?}", app.error_message);
    assert!(app.success_message.is_some());
    (started, finished)
}

fn assert_fixed_profiles(app: &App, started: DateTime<Utc>, finished: DateTime<Utc>) {
    let vault = app.vault_service.get_vault_store().unwrap();
    // 独立固定期望值，不用再次调用共享 decoder 充当恢复正确性的 oracle。
    for (id, name, version, bytes) in [
        (
            "synthetic-unicode",
            "合成档案🌟",
            3,
            "你好 / café / 🌍".as_bytes(),
        ),
        (
            "synthetic-binary",
            "合成二进制",
            4,
            &[0, 255, 128, 1, 10, 13, 0][..],
        ),
        ("synthetic-empty", "合成空内容", 5, &[][..]),
    ] {
        let actual = vault.load_profile(id).unwrap().unwrap();
        assert_eq!(actual.id, id);
        assert_eq!(actual.name, name);
        assert_eq!(actual.version, version);
        assert_eq!(actual.data, bytes);
        assert_eq!(actual.created_at, parse_time("2025-01-02T03:04:05Z"));
        assert!(actual.updated_at >= started);
        assert!(actual.updated_at <= finished);
    }
}

fn exchange_dir() -> Option<PathBuf> {
    std::env::var_os("SOLOSOUL_RF013_EXCHANGE_DIR").map(|path| {
        let path = PathBuf::from(path).canonicalize().unwrap();
        let temp = std::env::temp_dir().canonicalize().unwrap();
        assert!(path.is_dir(), "交换目录必须预先创建");
        assert!(
            path != temp && path.starts_with(&temp),
            "交换仅允许使用系统临时目录下的独立合成夹具目录"
        );
        path
    })
}

#[test]
fn rf013_cli_restores_all_shared_fixtures_after_confirmation() {
    for bytes in [GUI_V2, CLI_V2, LEGACY_ARRAY, LEGACY_BASE64] {
        let (mut app, _, _dir) = unlocked_app();
        let (started, finished) = restore_after_confirmation(&mut app, bytes);
        assert_fixed_profiles(&app, started, finished);
        assert_eq!(profile_state(&app).len(), 3);
    }
}

#[test]
fn rf013_cli_invalid_manifests_preserve_every_existing_profile() {
    let (mut app, account_id, _dir) = unlocked_app();
    let vault = app.vault_service.get_vault_store().unwrap();
    for (id, name) in [
        (&account_id[..], "当前主 Profile"),
        ("untouched", "保持不变"),
    ] {
        let mut profile = Profile::new_with_id(id, name, vec![0, 255, 42]);
        profile.version = 17;
        profile.created_at = parse_time("2020-01-02T03:04:05Z");
        vault.save_profile(&profile).unwrap();
    }
    let before = profile_state(&app);
    let mut base: Value = serde_json::from_slice(GUI_V2).unwrap();
    // 第一条若提前保存会覆盖主 Profile，后面的损坏必须阻止这一写入。
    base["profiles"][0]["id"] = json!(account_id);
    let mut cases = Vec::new();
    let mut unsupported = base.clone();
    unsupported["version"] = json!("3.0");
    cases.push(("unsupported-version", unsupported));
    let mut wrong_count = base.clone();
    wrong_count["profile_count"] = json!(4);
    cases.push(("count-mismatch", wrong_count));
    for key in ["version", "created_at", "profile_count"] {
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(key);
        cases.push((key, missing));
    }
    let mut damaged = base.clone();
    damaged["profiles"][1]["data_b64"] = json!("invalid!");
    damaged["profiles"][1]["data"] = json!([42]);
    cases.push(("Base64", damaged));
    let mut missing_data = base;
    missing_data["profiles"][1]
        .as_object_mut()
        .unwrap()
        .remove("data_b64");
    cases.push(("缺少备份数据", missing_data));

    for (case, manifest) in cases {
        write_restore_input(&app, &serde_json::to_vec(&manifest).unwrap());
        app.error_message = None;
        app.success_message = None;
        handle(&mut app, &["restore", "rf013"]).unwrap();
        assert_eq!(profile_state(&app), before, "{case}: 确认前");
        confirm_prompt(&mut app);
        let error = app.error_message.as_deref().expect("应明确报告失败");
        if matches!(case, "Base64" | "缺少备份数据") {
            assert!(error.contains(case), "{case}: {error}");
        }
        assert!(app.success_message.is_none(), "{case}");
        assert_eq!(profile_state(&app), before, "{case}: 不能有任何前缀写入");
    }
}

#[test]
fn rf013_cli_preserves_main_profile_identity_duplicate_order_and_empty_restore() {
    let (mut app, account_id, _dir) = unlocked_app();
    let vault = app.vault_service.get_vault_store().unwrap();
    let mut main = Profile::new_with_id(&account_id, "当前主 Profile", b"old".to_vec());
    main.created_at = parse_time("2020-01-02T03:04:05Z");
    vault.save_profile(&main).unwrap();
    vault
        .save_profile(&Profile::new_with_id("untouched", "保留行", vec![128, 255]))
        .unwrap();
    let untouched = profile_state(&app)["untouched"].clone();
    let base: Value = serde_json::from_slice(GUI_V2).unwrap();
    let mut first = base["profiles"][0].clone();
    first["id"] = json!(account_id);
    first["name"] = json!("第一次覆盖");
    let mut last = first.clone();
    last["name"] = json!("最终主 Profile");
    last["data_b64"] = json!("ZmluYWw=");
    last["version"] = json!(42);
    last["created_at"] = json!("2001-01-01T00:00:00Z");
    let manifest = json!({
        "version": "2.0", "created_at": "2026-09-27T12:34:56Z", "profile_count": 3,
        "profiles": [first, base["profiles"][1], last]
    });
    let (started, finished) =
        restore_after_confirmation(&mut app, &serde_json::to_vec(&manifest).unwrap());
    let restored = vault.load_profile(&account_id).unwrap().unwrap();
    assert_eq!(restored.id, account_id);
    assert_eq!(restored.name, "最终主 Profile");
    assert_eq!(restored.data, b"final");
    assert_eq!(restored.version, 42);
    assert_eq!(restored.created_at, main.created_at, "已有行创建时间不变");
    assert!(restored.updated_at >= started && restored.updated_at <= finished);
    assert_eq!(profile_state(&app)["untouched"], untouched);
    assert_eq!(
        vault
            .load_profile("synthetic-binary")
            .unwrap()
            .unwrap()
            .data,
        [0, 255, 128, 1, 10, 13, 0]
    );
    assert!(vault.load_profile("synthetic-unicode").unwrap().is_none());
    assert_eq!(profile_state(&app).len(), 3);

    let before_empty = profile_state(&app);
    restore_after_confirmation(
        &mut app,
        br#"{"version":"2.0","created_at":"2026-09-27T12:34:56Z","profile_count":0,"profiles":[]}"#,
    );
    assert_eq!(
        profile_state(&app),
        before_empty,
        "空清单不能清库或重写主 Profile"
    );
}

#[test]
fn rf013_cli_writer_produces_compatible_array_backup() {
    let (mut app, _, _dir) = unlocked_app();
    let vault = app.vault_service.get_vault_store().unwrap();
    let started = Utc::now();
    // 用共享合成数据初始化，不删除账户可能已有的主 Profile。
    for profile in decode_profile_backup(GUI_V2, started).unwrap().profiles {
        vault.save_profile(&profile).unwrap();
    }
    assert_fixed_profiles(&app, started, Utc::now());
    let expected = profile_state(&app);
    handle(&mut app, &["create", "rf013-cli"]).unwrap();
    assert!(app.error_message.is_none(), "{:?}", app.error_message);
    assert!(app.success_message.is_some());
    let backups = list_backup_infos(&app).unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].object_count, expected.len());
    let bytes = fs::read(find_backup_path(&backups_dir(&app), &backups[0].id).unwrap()).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["version"], "2.0");
    assert_eq!(manifest["profile_count"], expected.len());
    assert!(parse_time(manifest["created_at"].as_str().unwrap()) >= started);
    let entries = manifest["profiles"].as_array().unwrap();
    assert_eq!(entries.len(), expected.len());
    let actual_ids: BTreeSet<_> = entries
        .iter()
        .map(|entry| entry["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        actual_ids,
        expected.keys().cloned().collect::<BTreeSet<_>>()
    );
    for entry in entries {
        assert!(entry.get("data_b64").is_none(), "CLI 输出保持仅 data 数组");
        assert!(entry["data"].is_array());
        let profile = &expected[entry["id"].as_str().unwrap()];
        for key in ["id", "name", "data", "version"] {
            assert_eq!(entry[key], profile[key], "{key}");
        }
        for key in ["created_at", "updated_at"] {
            assert_eq!(
                parse_time(entry[key].as_str().unwrap()),
                parse_time(profile[key].as_str().unwrap()),
                "{key}"
            );
        }
    }
    if let Some(dir) = exchange_dir() {
        crate::util::write_private_file(&dir.join("cli-created.solosoul_backup"), &bytes).unwrap();
    }
}

#[test]
fn rf013_cli_restores_gui_writer_backup_after_confirmation() {
    let bytes = match exchange_dir() {
        Some(dir) => fs::read(dir.join("gui-created.solosoul_backup"))
            .expect("交换模式必须先运行真实 GUI producer，不能静默回退固定夹具"),
        None => GUI_V2.to_vec(),
    };
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["version"], "2.0");
    let entries = manifest["profiles"].as_array().unwrap();
    for entry in entries {
        assert!(entry["data_b64"].is_string());
        assert!(entry.get("data").is_none(), "应消费 GUI 的真实 Base64 输出");
    }
    let (mut app, _, _dir) = unlocked_app();
    let before = profile_state(&app);
    let (started, finished) = restore_after_confirmation(&mut app, &bytes);
    assert_fixed_profiles(&app, started, finished);
    let expected_ids: BTreeSet<_> = before
        .keys()
        .cloned()
        .chain(
            entries
                .iter()
                .map(|entry| entry["id"].as_str().unwrap().to_string()),
        )
        .collect();
    assert_eq!(
        profile_state(&app).keys().cloned().collect::<BTreeSet<_>>(),
        expected_ids
    );
    let vault = app.vault_service.get_vault_store().unwrap();
    for entry in entries {
        let id = entry["id"].as_str().unwrap();
        let actual = vault.load_profile(id).unwrap().unwrap();
        assert_eq!(actual.name, entry["name"].as_str().unwrap());
        assert_eq!(
            u64::from(actual.version),
            entry["version"].as_u64().unwrap()
        );
        let created_at = before
            .get(id)
            .map(|profile| profile["created_at"].as_str().unwrap())
            .unwrap_or_else(|| entry["created_at"].as_str().unwrap());
        assert_eq!(actual.created_at, parse_time(created_at));
        assert!(actual.updated_at >= started && actual.updated_at <= finished);
    }
}
