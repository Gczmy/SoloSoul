use super::*;
use serde_json::{json, Value};

const FIXTURE_TIME: &str = "2026-09-27T12:34:56Z";
const PROFILE_CREATED: &str = "2025-01-02T03:04:05Z";
const PROFILE_UPDATED: &str = "2026-02-03T04:05:06Z";

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn expected_profiles() -> Vec<Profile> {
    [
        (
            "synthetic-unicode",
            "合成档案🌟",
            3,
            "你好 / café / 🌍".as_bytes().to_vec(),
        ),
        (
            "synthetic-binary",
            "合成二进制",
            4,
            vec![0, 255, 128, 1, 10, 13, 0],
        ),
        ("synthetic-empty", "合成空内容", 5, vec![]),
    ]
    .into_iter()
    .map(|(id, name, version, data)| Profile {
        id: id.into(),
        name: name.into(),
        data,
        created_at: time(PROFILE_CREATED),
        updated_at: time(PROFILE_UPDATED),
        version,
    })
    .collect()
}

fn assert_profiles(actual: &[Profile], expected: &[Profile]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.name, expected.name);
        assert_eq!(actual.data, expected.data);
        assert_eq!(actual.created_at, expected.created_at);
        assert_eq!(actual.updated_at, expected.updated_at);
        assert_eq!(actual.version, expected.version);
    }
}

fn entry(payload: Value) -> Value {
    let mut value = json!({
        "id": "synthetic-entry",
        "name": "合成档案",
        "created_at": PROFILE_CREATED,
        "updated_at": PROFILE_UPDATED,
        "version": 7
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(payload.as_object().unwrap().clone());
    value
}

fn manifest(entries: Vec<Value>) -> Value {
    json!({
        "version": "2.0",
        "created_at": FIXTURE_TIME,
        "profile_count": entries.len(),
        "profiles": entries
    })
}

fn decode(value: &Value) -> Result<DecodedProfileBackup, ProfileBackupError> {
    decode_profile_backup(&serde_json::to_vec(value).unwrap(), time(FIXTURE_TIME))
}

#[test]
fn rf013_shared_gui_cli_and_legacy_fixtures_preserve_exact_profiles() {
    let fixtures: [(&[u8], &str); 4] = [
        (
            include_bytes!("../../tests/fixtures/profile_backup/gui-v2-base64.json"),
            "2.0",
        ),
        (
            include_bytes!("../../tests/fixtures/profile_backup/cli-v2-array.json"),
            "2.0",
        ),
        (
            include_bytes!("../../tests/fixtures/profile_backup/legacy-v1-array.json"),
            "1.0",
        ),
        (
            include_bytes!("../../tests/fixtures/profile_backup/legacy-v1-base64.json"),
            "1.0",
        ),
    ];
    let expected = expected_profiles();
    for (bytes, version) in fixtures {
        let decoded = decode_profile_backup(bytes, time("2000-01-01T00:00:00Z")).unwrap();
        assert_eq!(decoded.header.version, version);
        assert_eq!(decoded.header.created_at, FIXTURE_TIME);
        assert_eq!(decoded.header.profile_count, 3);
        assert_profiles(&decoded.profiles, &expected);
    }
}

#[test]
fn rf013_encoder_emits_only_selected_payload_key_and_roundtrips_all_bytes() {
    let mut profiles = expected_profiles();
    let mut all_bytes = profiles[1].clone();
    all_bytes.id = "synthetic-all-bytes".into();
    all_bytes.data = (0..=255).collect();
    profiles.push(all_bytes);
    for encoding in [
        ProfilePayloadEncoding::Base64,
        ProfilePayloadEncoding::ByteArray,
    ] {
        let bytes = encode_profile_backup(&profiles, time(FIXTURE_TIME), encoding).unwrap();
        let wire: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(wire["version"], "2.0");
        assert_eq!(wire["profile_count"], 4);
        assert_eq!(wire["created_at"], "2026-09-27T12:34:56+00:00");
        for profile in wire["profiles"].as_array().unwrap() {
            let profile = profile.as_object().unwrap();
            assert_eq!(profile.len(), 6);
            match encoding {
                ProfilePayloadEncoding::Base64 => {
                    assert!(profile["data_b64"].is_string());
                    assert!(!profile.contains_key("data"));
                }
                ProfilePayloadEncoding::ByteArray => {
                    assert!(profile["data"].is_array());
                    assert!(!profile.contains_key("data_b64"));
                }
            }
        }
        match encoding {
            ProfilePayloadEncoding::Base64 => {
                assert_eq!(wire["profiles"][1]["data_b64"], "AP+AAQoNAA==");
                assert_eq!(wire["profiles"][2]["data_b64"], "");
            }
            ProfilePayloadEncoding::ByteArray => {
                assert_eq!(
                    wire["profiles"][1]["data"],
                    json!([0, 255, 128, 1, 10, 13, 0])
                );
                assert_eq!(wire["profiles"][2]["data"], json!([]));
            }
        }
        let decoded = decode_profile_backup(&bytes, time("2000-01-01T00:00:00Z")).unwrap();
        assert_profiles(&decoded.profiles, &profiles);
    }
}

#[test]
fn rf013_empty_backup_is_valid_in_both_encodings_and_supported_versions() {
    for encoding in [
        ProfilePayloadEncoding::Base64,
        ProfilePayloadEncoding::ByteArray,
    ] {
        let bytes = encode_profile_backup(&[], time(FIXTURE_TIME), encoding).unwrap();
        let decoded = decode_profile_backup(&bytes, time(FIXTURE_TIME)).unwrap();
        assert!(decoded.profiles.is_empty());
        assert_eq!(decoded.header.profile_count, 0);
        assert_eq!(decoded.header.version, "2.0");
    }
    let mut legacy = manifest(vec![]);
    legacy["version"] = json!("1.0");
    assert!(decode(&legacy).unwrap().profiles.is_empty());
}

#[test]
fn rf013_payload_priority_preserves_explicit_empty_and_fallback_array() {
    let cases = [
        (json!({"data_b64":"AQI=", "data":[9]}), vec![1, 2]),
        (json!({"data_b64":"AQI=", "data":null}), vec![1, 2]),
        (json!({"data_b64":"", "data":[255, 0]}), vec![255, 0]),
        (json!({"data_b64":null, "data":[128]}), vec![128]),
        (json!({"data":[7]}), vec![7]),
        (json!({"data_b64":""}), vec![]),
        (json!({"data_b64":"", "data":null}), vec![]),
        (json!({"data":[]}), vec![]),
        (json!({"data_b64":null, "data":[]}), vec![]),
        (json!({"data_b64":"", "data":[]}), vec![]),
    ];
    for (payload, expected) in cases {
        let decoded = decode(&manifest(vec![entry(payload)])).unwrap();
        assert_eq!(decoded.profiles[0].data, expected);
    }
}

#[test]
fn rf013_missing_payload_and_invalid_preferred_base64_reject_whole_backup() {
    for payload in [
        json!({}),
        json!({"data":null}),
        json!({"data_b64":null}),
        json!({"data_b64":null, "data":null}),
    ] {
        let value = manifest(vec![entry(json!({"data":[1]})), entry(payload)]);
        assert_eq!(
            decode(&value).unwrap_err(),
            ProfileBackupError::MissingData { index: 1 }
        );
    }
    for encoded in ["%%%", "A", "AQ", "AR==", "AQ== "] {
        let value = manifest(vec![
            entry(json!({"data":[1]})),
            entry(json!({"data_b64":encoded,"data":[9]})),
        ]);
        assert!(matches!(
            decode(&value),
            Err(ProfileBackupError::InvalidBase64 { index: 1, .. })
        ));
    }
}

#[test]
fn rf013_strict_payload_types_apply_even_when_other_encoding_would_win() {
    for payload in [
        json!({"data_b64":"AQ==", "data":"hidden invalid array"}),
        json!({"data_b64":"AQ==", "data":[256]}),
        json!({"data_b64":"AQ==", "data":[-1]}),
        json!({"data_b64":"AQ==", "data":[1.5]}),
        json!({"data_b64":"AQ==", "data":[null]}),
        json!({"data_b64":[], "data":[1]}),
        json!({"data_b64":false, "data":[1]}),
        json!({"data":{}}),
    ] {
        assert!(matches!(
            decode(&manifest(vec![entry(payload)])),
            Err(ProfileBackupError::Json { .. })
        ));
    }
}

#[test]
fn rf013_required_header_and_entry_metadata_remain_strict() {
    let valid = manifest(vec![entry(json!({"data":[]}))]);
    for field in ["version", "created_at", "profile_count", "profiles"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(matches!(
            decode(&missing),
            Err(ProfileBackupError::Json { .. })
        ));
        let mut null = valid.clone();
        null[field] = Value::Null;
        assert!(matches!(
            decode(&null),
            Err(ProfileBackupError::Json { .. })
        ));
    }
    for field in ["id", "name", "created_at", "updated_at", "version"] {
        let mut missing = valid.clone();
        missing["profiles"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(matches!(
            decode(&missing),
            Err(ProfileBackupError::Json { .. })
        ));
        let mut null = valid.clone();
        null["profiles"][0][field] = Value::Null;
        assert!(matches!(
            decode(&null),
            Err(ProfileBackupError::Json { .. })
        ));
    }
    for (field, value) in [
        ("version", json!(2)),
        ("created_at", json!(123)),
        ("profile_count", json!(-1)),
        ("profile_count", json!(1.0)),
        ("profiles", json!({})),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert!(matches!(
            decode(&invalid),
            Err(ProfileBackupError::Json { .. })
        ));
    }
    for (field, value) in [
        ("id", json!(1)),
        ("name", json!([])),
        ("created_at", json!(123)),
        ("updated_at", json!({})),
        ("version", json!(-1)),
        ("version", json!(4_294_967_296_u64)),
    ] {
        let mut invalid = valid.clone();
        invalid["profiles"][0][field] = value;
        assert!(matches!(
            decode(&invalid),
            Err(ProfileBackupError::Json { .. })
        ));
    }
    let duplicate_header =
        br#"{"version":"1.0","version":"2.0","created_at":"x","profile_count":0,"profiles":[]}"#;
    assert!(matches!(
        decode_profile_backup(duplicate_header, time(FIXTURE_TIME)),
        Err(ProfileBackupError::Json { .. })
    ));
}

#[test]
fn rf013_version_and_count_are_checked_before_payload_decoding() {
    for version in ["", "0.0", "3.0", "2", "2.0 "] {
        let mut value = manifest(vec![entry(json!({}))]);
        value["version"] = json!(version);
        assert_eq!(
            decode(&value).unwrap_err(),
            ProfileBackupError::UnsupportedVersion
        );
    }
    for declared in [0, 2] {
        let mut value = manifest(vec![entry(json!({}))]);
        value["profile_count"] = json!(declared);
        assert_eq!(
            decode(&value).unwrap_err(),
            ProfileBackupError::CountMismatch {
                declared,
                actual: 1
            }
        );
    }
}

#[test]
fn rf013_dates_use_one_fallback_without_normalizing_header_or_deduplicating_ids() {
    let mut first = entry(json!({"data":[1]}));
    first["created_at"] = json!("invalid created date");
    first["updated_at"] = json!("invalid updated date");
    let mut second = entry(json!({"data":[2]}));
    second["name"] = json!("同 ID 后一条");
    second["created_at"] = json!("2025-01-02T04:04:05+01:00");
    second["updated_at"] = json!("invalid updated date");
    second["version"] = json!(8);
    let mut value = manifest(vec![first, second]);
    value["created_at"] = json!("preserve invalid header time verbatim");
    value["extension"] = json!({"ignored":true});
    value["profiles"][0]["extension"] = json!("ignored");
    let fallback = time("2031-04-05T06:07:08.123456789Z");
    let decoded = decode_profile_backup(&serde_json::to_vec(&value).unwrap(), fallback).unwrap();
    assert_eq!(
        decoded.header.created_at,
        "preserve invalid header time verbatim"
    );
    assert_eq!(decoded.header.profile_count, 2);
    assert_eq!(decoded.profiles.len(), 2);
    assert_eq!(decoded.profiles[0].id, decoded.profiles[1].id);
    assert_eq!(decoded.profiles[0].data, vec![1]);
    assert_eq!(decoded.profiles[1].data, vec![2]);
    assert_eq!(decoded.profiles[1].name, "同 ID 后一条");
    assert_eq!(decoded.profiles[1].version, 8);
    assert_eq!(decoded.profiles[0].created_at, fallback);
    assert_eq!(decoded.profiles[0].updated_at, fallback);
    assert_eq!(decoded.profiles[1].created_at, time(PROFILE_CREATED));
    assert_eq!(decoded.profiles[1].updated_at, fallback);
}

#[test]
fn rf013_errors_do_not_expose_untrusted_field_values() {
    const SECRET: &str = "SYNTHETIC_PRIVATE_MARKER!";
    let mut invalid_type = manifest(vec![entry(json!({"data":[]}))]);
    invalid_type["profiles"][0]["version"] = json!(SECRET);
    let mut unsupported_version = invalid_type.clone();
    unsupported_version["profiles"][0]["version"] = json!(1);
    unsupported_version["version"] = json!(SECRET);
    let invalid_payload = manifest(vec![entry(json!({"data_b64":SECRET}))]);
    let mut missing_payload = manifest(vec![entry(json!({}))]);
    missing_payload["profiles"][0]["id"] = json!(SECRET);
    missing_payload["profiles"][0]["name"] = json!(SECRET);
    for value in [
        invalid_type,
        unsupported_version,
        invalid_payload,
        missing_payload,
    ] {
        let error = decode(&value).unwrap_err();
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
    }
    for bytes in [b"{".as_slice(), b"not JSON".as_slice(), &[0xff]] {
        assert!(matches!(
            decode_profile_backup(bytes, time(FIXTURE_TIME)),
            Err(ProfileBackupError::Json { .. })
        ));
    }
}
