use super::*;
use rusqlite::Connection;
use serde_json::json;
use solosoul_vault::VaultConfig;
use tempfile::TempDir;

const ACCOUNT: &str = "rf010-synthetic-account";
const NOW: &str = "2026-09-27T12:34:56Z";

struct Fixture {
    vault: VaultStore,
    db: Connection,
    _dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let vault = VaultStore::open(
            VaultConfig::new(ACCOUNT, dir.path().to_path_buf()).with_data_key([0x10; 32]),
        )
        .unwrap();
        let db = Connection::open(dir.path().join("vault.db")).unwrap();
        Self {
            vault,
            db,
            _dir: dir,
        }
    }

    fn counts(&self) -> Vec<i64> {
        ["objects", "object_snapshots", "audit_log", "sync_hlc"]
            .iter()
            .map(|table| {
                self.db
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .unwrap()
            })
            .collect()
    }
}

fn input(properties: Value, template_id: Option<&str>) -> CreateRecordInput {
    CreateRecordInput {
        id: "client-explicit-id".into(),
        type_id: "explicit-type".into(),
        section_type: "explicit-section".into(),
        name: "  显式名称 🌙  ".into(),
        icon_name: "explicit-icon".into(),
        parent_id: Some("explicit-parent".into()),
        properties,
        template_id: template_id.map(str::to_string),
        template_type: Some("explicit-template-kind".into()),
    }
}

fn template() -> UserTemplate {
    serde_json::from_value(json!({
        "id": "rf010-template", "accountId": ACCOUNT, "name": "共享模板", "iconId": "template-icon",
        "createdAt": "2026-01-01T00:00:00Z", "contractTypeId": "shared.contract",
        "properties": [
            {"id": "status", "name": "状态", "type": "select", "sensitivityLevel": "public",
             "options": ["on", "off"], "deprecatedAt": "2026-02-01T00:00:00Z", "contractField": false,
             "allowedTypes": ["number"], "maxItems": 99},
            {"id": "group", "name": "组", "type": "dynamic_group", "sensitivityLevel": "internal",
             "allowedTypes": ["text"], "maxItems": 1}
        ]
    })).unwrap()
}

#[test]
fn rf010_explicit_input_builds_complete_record_and_only_replaces_reserved_properties() {
    let fixture = Fixture::new();
    let template = template();
    fixture.vault.save_user_template(&template).unwrap();
    let properties = json!({
        "status": "off", "group": [{"id": "child", "name": "子项", "type": "text", "value": ""}],
        "false": false, "zero": 0, "empty": "", "null": null, "array": [null, false, 0],
        "nested": {"arbitrary": ["保留", 3]}, "contract_type_id": "ordinary user value",
        "__fields": {"spoof": {"type": "text"}}, "__templateName": "old name", "__templateHash": "old hash"
    });
    let before = fixture.counts();
    let built = build_create_record(
        &fixture.vault,
        ACCOUNT,
        input(properties.clone(), Some(&template.id)),
        NOW,
    )
    .unwrap();
    let fields = json!({
        "status": {"name": "状态", "type": "select", "options": ["on", "off"],
                   "deprecatedAt": "2026-02-01T00:00:00Z", "contractField": false},
        "group": {"name": "组", "type": "dynamic_group", "allowedTypes": ["text"], "maxItems": 1}
    });
    let hash = template_fingerprint(&template);
    assert_eq!(built.properties["__fields"], fields);
    assert_eq!(built.properties["__templateName"], "共享模板");
    assert_eq!(built.properties["__templateHash"], hash);
    let mut expected_properties = properties.clone();
    expected_properties["__fields"] = fields;
    expected_properties["__templateName"] = json!("共享模板");
    expected_properties["__templateHash"] = json!(hash);
    assert_eq!(built.properties, expected_properties);
    let expected = ObjectRecord {
        id: "client-explicit-id".into(),
        account_id: ACCOUNT.into(),
        type_id: "explicit-type".into(),
        section_type: "explicit-section".into(),
        name: "  显式名称 🌙  ".into(),
        icon_name: "explicit-icon".into(),
        parent_id: Some("explicit-parent".into()),
        properties: expected_properties,
        property_labels: Some(json!({"status": "public", "group": "internal"})),
        sensitivity_level: "internal".into(),
        template_id: Some(template.id),
        template_type: Some("explicit-template-kind".into()),
        contract_type_id: Some("shared.contract".into()),
        template_hash: Some(hash),
        created_at: NOW.into(),
        updated_at: NOW.into(),
        version: 1,
        ..ObjectRecord::default()
    };
    assert_eq!(
        serde_json::to_value(built).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(
        fixture.counts(),
        before,
        "building must not save an object, history or audit"
    );
}

#[test]
fn rf010_empty_missing_and_non_object_properties_preserve_compatibility() {
    let fixture = Fixture::new();
    let mut template = template();
    fixture.vault.save_user_template(&template).unwrap();
    for properties in [
        Value::Null,
        json!(false),
        json!(12),
        json!("legacy"),
        json!(["legacy"]),
    ] {
        let built = build_create_record(
            &fixture.vault,
            ACCOUNT,
            input(properties.clone(), Some(&template.id)),
            NOW,
        )
        .unwrap();
        assert_eq!(built.properties, properties);
        assert!(built.property_labels.is_some());
        assert_eq!(built.contract_type_id.as_deref(), Some("shared.contract"));
        assert_eq!(built.template_hash, Some(template_fingerprint(&template)));
    }
    let legacy = json!({"group": "legacy invalid group", "__fields": {"group": {"type": "dynamic_group", "maxItems": 0}},
        "__templateName": "local", "__templateHash": "local hash"});
    for template_id in [None, Some("missing-template")] {
        let built = build_create_record(
            &fixture.vault,
            ACCOUNT,
            input(legacy.clone(), template_id),
            NOW,
        )
        .unwrap();
        assert_eq!(built.properties, legacy);
        assert!(
            built.property_labels.is_none()
                && built.contract_type_id.is_none()
                && built.template_hash.is_none()
        );
        assert_eq!(built.template_id.as_deref(), template_id);
        assert_eq!(
            built.template_type.as_deref(),
            Some("explicit-template-kind")
        );
    }
    template.properties.clear();
    fixture.vault.save_user_template(&template).unwrap();
    let properties = json!({"__fields": {"local": {"type": "text"}}, "value": false});
    let built = build_create_record(
        &fixture.vault,
        ACCOUNT,
        input(properties.clone(), Some(&template.id)),
        NOW,
    )
    .unwrap();
    assert_eq!(built.properties["__fields"], properties["__fields"]);
    assert!(built.property_labels.is_none());
    assert_eq!(built.properties["__templateName"], template.name);
    assert_eq!(built.template_hash, Some(template_fingerprint(&template)));
    let empty = build_create_record(&fixture.vault, ACCOUNT, input(json!({}), None), NOW).unwrap();
    assert_eq!(empty.properties, json!({}));
}

#[test]
fn rf010_template_read_failure_is_strict_before_any_creation_write() {
    let fixture = Fixture::new();
    let page = crate::objects::create_page(&fixture.vault, ACCOUNT, "Existing parent").unwrap();
    fixture.vault.save_user_template(&template()).unwrap();
    // 使用本测试自己的 SQLite 库制造真实读取失败，无外部工具或模拟读取结果。
    fixture
        .db
        .execute_batch("DROP TABLE user_templates")
        .unwrap();
    let expected = fixture
        .vault
        .load_user_template("rf010-template")
        .unwrap_err();
    let counts = fixture.counts();
    let parent = serde_json::to_value(fixture.vault.load_object(&page.id).unwrap()).unwrap();
    let hlc = fixture.vault.get_record_hlc("objects", &page.id).unwrap();
    let error = build_create_record(
        &fixture.vault,
        ACCOUNT,
        input(json!({}), Some("rf010-template")),
        NOW,
    )
    .unwrap_err();
    assert_eq!(error, expected);
    assert_eq!(
        crate::objects::create_object(
            &fixture.vault,
            ACCOUNT,
            &page.id,
            "must not save",
            json!({}),
            Some("rf010-template"),
            None
        )
        .unwrap_err(),
        expected
    );
    assert_eq!(fixture.counts(), counts);
    assert_eq!(
        serde_json::to_value(fixture.vault.load_object(&page.id).unwrap()).unwrap(),
        parent
    );
    assert_eq!(
        fixture.vault.get_record_hlc("objects", &page.id).unwrap(),
        hlc
    );
    // 无模板构建不读取已损坏的模板表，兼容辅助仍保留各自 best-effort 返回值。
    assert!(build_create_record(&fixture.vault, ACCOUNT, input(json!({}), None), NOW).is_ok());
    assert_eq!(
        inherit_contract_type_id(&fixture.vault, Some("rf010-template")),
        None
    );
    assert_eq!(
        inherit_property_labels(&fixture.vault, Some("rf010-template")),
        None
    );
    assert_eq!(
        inherit_property_fields(&fixture.vault, Some("rf010-template")),
        Value::Null
    );
    let mut properties = json!({"__templateName": "keep"});
    inject_template_meta(&fixture.vault, Some("rf010-template"), &mut properties);
    assert_eq!(properties, json!({"__templateName": "keep"}));
}

#[test]
fn rf010_template_dynamic_constraints_replace_spoofed_fields_before_validation() {
    let fixture = Fixture::new();
    let template = template();
    fixture.vault.save_user_template(&template).unwrap();
    let counts = fixture.counts();
    let text = json!({"id": "child", "name": "Text", "type": "text", "value": ""});
    for group in [
        json!([text.clone(), text.clone()]),
        json!([{"id": "child", "name": "Number", "type": "number", "value": 0}]),
    ] {
        let properties =
            json!({"group": group, "__fields": {"group": {"type": "text", "maxItems": 99}}});
        let error = build_create_record(
            &fixture.vault,
            ACCOUNT,
            input(properties, Some(&template.id)),
            NOW,
        )
        .unwrap_err();
        assert!(error.contains("group"));
        assert_eq!(fixture.counts(), counts);
    }
    let properties = json!({"group": [text]});
    let built = build_create_record(
        &fixture.vault,
        ACCOUNT,
        input(properties.clone(), Some(&template.id)),
        NOW,
    )
    .unwrap();
    assert_eq!(built.properties["group"], properties["group"]);
    assert_eq!(built.properties["__fields"]["group"]["maxItems"], 1);
    assert_eq!(fixture.counts(), counts);
}

#[test]
fn rf010_compatibility_helpers_share_projection_without_expanding_injection() {
    let fixture = Fixture::new();
    let template = template();
    fixture.vault.save_user_template(&template).unwrap();
    let built = build_create_record(
        &fixture.vault,
        ACCOUNT,
        input(json!({}), Some(&template.id)),
        NOW,
    )
    .unwrap();
    assert_eq!(
        inherit_contract_type_id(&fixture.vault, Some(&template.id)),
        built.contract_type_id
    );
    assert_eq!(
        inherit_property_labels(&fixture.vault, Some(&template.id)),
        built.property_labels
    );
    assert_eq!(
        inherit_property_fields(&fixture.vault, Some(&template.id)),
        built.properties["__fields"]
    );
    let mut properties =
        json!({"__fields": "keep fields", "__templateHash": "keep hash", "user": false});
    inject_template_meta(&fixture.vault, Some(&template.id), &mut properties);
    assert_eq!(
        properties,
        json!({"__fields": "keep fields", "__templateHash": "keep hash", "user": false, "__templateName": template.name})
    );
    for fields in [Value::Null, json!({}), json!([]), json!(false)] {
        let mut properties = json!({"__fields": "original", "user": 0});
        inject_property_fields(&mut properties, &fields);
        assert_eq!(
            properties["__fields"],
            if fields.is_null() {
                json!("original")
            } else {
                fields
            }
        );
        assert_eq!(properties["user"], 0);
    }
    let mut scalar = json!("legacy scalar");
    inject_property_fields(&mut scalar, &json!({"x": {"type": "text"}}));
    inject_template_meta(&fixture.vault, Some(&template.id), &mut scalar);
    assert_eq!(scalar, "legacy scalar");
}
