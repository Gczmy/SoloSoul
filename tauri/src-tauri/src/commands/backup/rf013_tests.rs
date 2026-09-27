//! RF013：通过真实 GUI writer / restore worker 验证共享 Profile 备份契约。
use super::{create_profile_backup, restore_profile_backup};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use solosoul_vault::{Profile, VaultConfig, VaultStore};
use std::path::PathBuf;
use tempfile::TempDir;

const ACCOUNT: &str = "rf013-synthetic-account";
const CREATED: &str = "2025-01-02T03:04:05Z";
const UPDATED: &str = "2026-02-03T04:05:06Z";
const HEADER_TIME: &str = "2026-09-27T12:34:56Z";
const GUI_V2: &[u8] = include_bytes!(
    "../../../../crates/solosoul-core/tests/fixtures/profile_backup/gui-v2-base64.json"
);
const CLI_V2: &[u8] = include_bytes!(
    "../../../../crates/solosoul-core/tests/fixtures/profile_backup/cli-v2-array.json"
);
const LEGACY_ARRAY: &[u8] = include_bytes!(
    "../../../../crates/solosoul-core/tests/fixtures/profile_backup/legacy-v1-array.json"
);
const LEGACY_BASE64: &[u8] = include_bytes!(
    "../../../../crates/solosoul-core/tests/fixtures/profile_backup/legacy-v1-base64.json"
);

struct Fixture {
    // Windows 上先释放所有 SQLite 句柄，再清理本测试的合成目录。
    vault: VaultStore,
    db: Connection,
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let vault = VaultStore::open(
            VaultConfig::new(ACCOUNT, directory.path().to_path_buf()).with_data_key([0x13; 32]),
        )
        .unwrap();
        let db = Connection::open(directory.path().join("vault.db")).unwrap();
        Self {
            vault,
            db,
            directory,
        }
    }

    fn save(&self, profile: &Profile) {
        self.vault.save_profile(profile).unwrap();
    }

    fn seed_fixed_profiles(&self) -> Vec<Profile> {
        let profiles = fixed_profiles();
        for profile in &profiles {
            self.save(profile);
            // 为实际 writer 准备固定历史元数据；生产 save_profile 会刷新 updated_at。
            self.db
                .execute(
                    "UPDATE profiles SET updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![UPDATED, profile.id],
                )
                .unwrap();
            assert_eq!(
                profile_value(&self.vault.load_profile(&profile.id).unwrap().unwrap()),
                profile_value(profile),
            );
        }
        profiles
    }

    fn dump(&self, table: &str) -> Value {
        // 仅调用本文件固定的 profiles/sync_hlc 表；比较原密文可检测失败前的重写。
        let mut statement = self
            .db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|index| {
                        Ok(match row.get_ref(index)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(value) => json!(value),
                            ValueRef::Real(value) => json!(value),
                            ValueRef::Text(value) => json!(String::from_utf8_lossy(value)),
                            ValueRef::Blob(value) => json!(value),
                        })
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        json!(rows)
    }

    fn state(&self) -> Value {
        json!({"profiles": self.dump("profiles"), "hlc": self.dump("sync_hlc")})
    }

    fn assert_fixed_profiles(&self, started: DateTime<Utc>, finished: DateTime<Utc>) {
        for expected in fixed_profiles() {
            let actual = self.vault.load_profile(&expected.id).unwrap().unwrap();
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.data, expected.data);
            assert_eq!(actual.version, expected.version);
            assert_eq!(actual.created_at, expected.created_at);
            assert!(
                actual.updated_at >= started && actual.updated_at <= finished,
                "save_profile 应写恢复时间，而非清单中的旧 updated_at"
            );
        }
    }
}

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn fixed_profiles() -> Vec<Profile> {
    [
        (
            "synthetic-unicode",
            "合成档案🌟",
            "你好 / café / 🌍".as_bytes().to_vec(),
            3,
        ),
        (
            "synthetic-binary",
            "合成二进制",
            vec![0, 255, 128, 1, 10, 13, 0],
            4,
        ),
        ("synthetic-empty", "合成空内容", Vec::new(), 5),
    ]
    .into_iter()
    .map(|(id, name, data, version)| Profile {
        id: id.into(),
        name: name.into(),
        data,
        created_at: time(CREATED),
        updated_at: time(UPDATED),
        version,
    })
    .collect()
}

fn profile_value(profile: &Profile) -> Value {
    json!({
        "id": profile.id, "name": profile.name, "data": profile.data,
        "created_at": profile.created_at.to_rfc3339(),
        "updated_at": profile.updated_at.to_rfc3339(), "version": profile.version,
    })
}

fn entry(id: &str, name: &str, data: &[u8], version: u32) -> Value {
    json!({
        "id": id, "name": name, "data": data, "version": version,
        "created_at": CREATED, "updated_at": UPDATED,
    })
}

fn manifest(entries: Vec<Value>) -> Value {
    json!({
        "version": "2.0", "created_at": HEADER_TIME,
        "profile_count": entries.len(), "profiles": entries,
    })
}

fn exchange_dir() -> Option<PathBuf> {
    std::env::var_os("SOLOSOUL_RF013_EXCHANGE_DIR").map(|path| {
        let directory = PathBuf::from(path)
            .canonicalize()
            .expect("显式 RF013 交换目录必须已存在");
        let temporary_root = std::env::temp_dir().canonicalize().unwrap();
        assert!(directory.is_dir(), "RF013 交换路径必须是目录");
        assert!(
            directory != temporary_root && directory.starts_with(&temporary_root),
            "RF013 交换目录必须是系统 temp 内的独立合成目录"
        );
        directory
    })
}

#[test]
fn rf013_gui_restores_all_four_shared_formats_with_profile_metadata() {
    for (name, bytes) in [
        ("gui-v2-base64", GUI_V2),
        ("cli-v2-array", CLI_V2),
        ("legacy-v1-array", LEGACY_ARRAY),
        ("legacy-v1-base64", LEGACY_BASE64),
    ] {
        let fixture = Fixture::new();
        let started = Utc::now();
        assert_eq!(
            restore_profile_backup(&fixture.vault, bytes, time(HEADER_TIME)).unwrap(),
            3,
            "{name}"
        );
        let finished = Utc::now();
        fixture.assert_fixed_profiles(started, finished);
        assert_eq!(fixture.vault.list_profiles().unwrap().len(), 3, "{name}");
        assert!(
            fixture.vault.load_profile(ACCOUNT).unwrap().is_none(),
            "不得改写条目 ID 为当前账户"
        );
    }
}

#[test]
fn rf013_gui_rejects_invalid_manifest_before_overwriting_any_profile() {
    let base = manifest(vec![
        entry("existing", "Would overwrite", &[1, 2, 3], 9),
        entry("new-entry", "Must not appear", &[4, 5], 10),
    ]);
    let mut cases = Vec::new();
    let mut bad = base.clone();
    bad["profiles"][1]["data"] = json!([256]);
    cases.push(("bad-later-entry", bad));
    let mut bad = base.clone();
    bad["version"] = json!("99.0");
    cases.push(("unsupported-version", bad));
    let mut bad = base.clone();
    bad["profile_count"] = json!(1);
    cases.push(("count-mismatch", bad));
    for key in ["version", "created_at", "profile_count"] {
        let mut bad = base.clone();
        bad.as_object_mut().unwrap().remove(key);
        cases.push((key, bad));
    }
    let mut bad = base.clone();
    bad["profiles"][1].as_object_mut().unwrap().remove("data");
    cases.push(("missing-data", bad));
    let mut bad = base.clone();
    bad["profiles"][1]["data_b64"] = json!("invalid!");
    cases.push(("invalid-base64-with-valid-array", bad));
    let mut bad = base.clone();
    bad["profiles"][1]["data"] = Value::Null;
    bad["profiles"][1]["data_b64"] = Value::Null;
    cases.push(("null-data", bad));

    for (label, bad) in cases {
        let fixture = Fixture::new();
        for id in ["existing", "untouched"] {
            fixture.save(&Profile::new_with_id(id, "Original", vec![0, 255, 13]));
        }
        let before = fixture.state();
        let error = restore_profile_backup(
            &fixture.vault,
            &serde_json::to_vec(&bad).unwrap(),
            time(HEADER_TIME),
        )
        .unwrap_err();
        if label == "invalid-base64-with-valid-array" {
            assert!(error.contains("Base64 decode profile data"), "{error}");
        }
        assert_eq!(fixture.state(), before, "{label}");
        assert!(
            fixture.vault.load_profile("new-entry").unwrap().is_none(),
            "{label}"
        );
    }
}

#[test]
fn rf013_gui_preserves_main_id_existing_creation_time_and_duplicate_order() {
    let fixture = Fixture::new();
    let mut main = Profile::new_with_id(ACCOUNT, "Original main", b"before".to_vec());
    main.created_at = time("2001-02-03T04:05:06Z");
    fixture.save(&main);
    fixture.save(&Profile::new_with_id(
        "untouched",
        "Keep this row",
        vec![128, 255],
    ));
    let untouched = profile_value(&fixture.vault.load_profile("untouched").unwrap().unwrap());
    let mut last = entry(ACCOUNT, "Last duplicate wins", &[9, 0, 255], 42);
    last["created_at"] = json!("2010-01-01T00:00:00Z");
    let bytes = serde_json::to_vec(&manifest(vec![
        entry(ACCOUNT, "First duplicate", &[1], 20),
        entry("external-profile-id", "Keep original ID", &[2, 128], 8),
        last,
    ]))
    .unwrap();
    let started = Utc::now();
    assert_eq!(
        restore_profile_backup(&fixture.vault, &bytes, time(HEADER_TIME)).unwrap(),
        3
    );
    let finished = Utc::now();
    let actual = fixture.vault.load_profile(ACCOUNT).unwrap().unwrap();
    assert_eq!(actual.id, ACCOUNT);
    assert_eq!(actual.name, "Last duplicate wins");
    assert_eq!(actual.data, [9, 0, 255]);
    assert_eq!(actual.version, 42);
    assert_eq!(
        actual.created_at, main.created_at,
        "UPSERT 保留已有 created_at"
    );
    assert!(actual.updated_at >= started && actual.updated_at <= finished);
    let external = fixture
        .vault
        .load_profile("external-profile-id")
        .unwrap()
        .unwrap();
    assert_eq!(external.id, "external-profile-id");
    assert_eq!(external.name, "Keep original ID");
    assert_eq!(external.data, [2, 128]);
    assert_eq!(external.version, 8);
    assert_eq!(external.created_at, time(CREATED));
    assert_eq!(
        profile_value(&fixture.vault.load_profile("untouched").unwrap().unwrap()),
        untouched,
    );
    assert_eq!(fixture.vault.list_profiles().unwrap().len(), 3);
    let before_empty = fixture.state();
    assert_eq!(
        restore_profile_backup(
            &fixture.vault,
            &serde_json::to_vec(&manifest(Vec::new())).unwrap(),
            time(HEADER_TIME),
        )
        .unwrap(),
        0
    );
    assert_eq!(fixture.state(), before_empty, "空清单不能清库或刷新既有行");
}

#[test]
fn rf013_gui_invalid_profile_dates_use_supplied_fallback() {
    let fixture = Fixture::new();
    let mut invalid_dates = entry("invalid-dates", "Synthetic date fallback", &[0, 255], 7);
    invalid_dates["created_at"] = json!("not-a-date");
    invalid_dates["updated_at"] = json!("also-not-a-date");
    let started = Utc::now();
    assert_eq!(
        restore_profile_backup(
            &fixture.vault,
            &serde_json::to_vec(&manifest(vec![invalid_dates])).unwrap(),
            time(HEADER_TIME),
        )
        .unwrap(),
        1
    );
    let finished = Utc::now();
    let actual = fixture
        .vault
        .load_profile("invalid-dates")
        .unwrap()
        .unwrap();
    assert_eq!(actual.created_at, time(HEADER_TIME));
    assert!(actual.updated_at >= started && actual.updated_at <= finished);
    assert_eq!(actual.data, [0, 255]);
    assert_eq!(actual.version, 7);
}

#[test]
fn rf013_gui_writer_produces_compatible_base64_backup() {
    let fixture = Fixture::new();
    let expected = fixture.seed_fixed_profiles();
    let summaries = fixture.vault.list_profiles().unwrap();
    let info = create_profile_backup(
        &fixture.vault,
        fixture.directory.path(),
        "rf013-gui",
        &summaries,
        time(HEADER_TIME),
    )
    .unwrap();
    assert_eq!(info.id, "rf013-gui_20260927_123456");
    assert_eq!(info.object_count, 3);
    assert_eq!(info.created_at, time(HEADER_TIME).to_rfc3339());
    let path = fixture
        .directory
        .path()
        .join("backups")
        .join(format!("{}.solosoul_backup", info.id));
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(info.size_bytes, bytes.len() as u64);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["version"], "2.0");
    assert_eq!(manifest["profile_count"], 3);
    assert_eq!(
        time(manifest["created_at"].as_str().unwrap()),
        time(HEADER_TIME)
    );
    let entries = manifest["profiles"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    for profile in &expected {
        let matches = entries
            .iter()
            .filter(|entry| entry["id"] == profile.id)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        let entry = matches[0];
        assert_eq!(entry["name"], profile.name);
        assert_eq!(entry["version"], profile.version);
        assert_eq!(
            time(entry["created_at"].as_str().unwrap()),
            profile.created_at
        );
        assert_eq!(
            time(entry["updated_at"].as_str().unwrap()),
            profile.updated_at
        );
        assert_eq!(entry["data_b64"], BASE64.encode(&profile.data));
        assert!(
            entry.get("data").is_none(),
            "GUI 输出不可新增数组字段或 null 占位"
        );
    }
    // 无交换环境时也真正恢复当前生产 writer 的文件。
    let target = Fixture::new();
    let started = Utc::now();
    assert_eq!(
        restore_profile_backup(&target.vault, &bytes, time(HEADER_TIME)).unwrap(),
        3
    );
    target.assert_fixed_profiles(started, Utc::now());
    if let Some(directory) = exchange_dir() {
        std::fs::write(directory.join("gui-created.solosoul_backup"), &bytes).unwrap();
    }
}

#[test]
fn rf013_gui_restores_cli_writer_backup() {
    let bytes = match exchange_dir() {
        Some(directory) => std::fs::read(directory.join("cli-created.solosoul_backup"))
            .expect("显式交换模式必须先运行真实 CLI producer，不能回退夹具"),
        None => CLI_V2.to_vec(),
    };
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["version"], "2.0");
    let entries = manifest["profiles"].as_array().unwrap();
    assert!(entries.len() >= 3);
    assert_eq!(manifest["profile_count"], entries.len());
    for entry in entries {
        assert!(
            entry.get("data_b64").is_none(),
            "真实 CLI writer 应保持数组输出"
        );
        assert!(entry["data"].is_array());
    }
    let fixture = Fixture::new();
    let started = Utc::now();
    assert_eq!(
        restore_profile_backup(&fixture.vault, &bytes, time(HEADER_TIME)).unwrap(),
        entries.len(),
    );
    let finished = Utc::now();
    fixture.assert_fixed_profiles(started, finished);
    assert_eq!(fixture.vault.list_profiles().unwrap().len(), entries.len());
    // CLI 创建账户可能已有主 Profile：完整恢复所有真实条目，不能只检查三个夹具 ID。
    for entry in entries {
        let id = entry["id"].as_str().unwrap();
        let actual = fixture.vault.load_profile(id).unwrap().unwrap();
        assert_eq!(actual.id, id);
        assert_eq!(actual.name, entry["name"].as_str().unwrap());
        assert_eq!(actual.version as u64, entry["version"].as_u64().unwrap());
        let expected_data: Vec<u8> = serde_json::from_value(entry["data"].clone()).unwrap();
        assert_eq!(actual.data, expected_data);
        assert_eq!(
            actual.created_at,
            time(entry["created_at"].as_str().unwrap())
        );
        assert!(actual.updated_at >= started && actual.updated_at <= finished);
    }
}
