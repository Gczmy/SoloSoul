//! RF010：真实 GUI 创建持久化入口与 Core 共享构造规则。
use super::super::*;
use super::setup_vault;
use serde_json::{json, Value};
use solosoul_core::objects::{self, CreateRecordInput};
use solosoul_vault::encryption::{encrypt_text_field, DataEncryptionKey};
use solosoul_vault::{ObjectRecord, UserTemplate, VaultStore};

const ACCOUNT: &str = "test_account";
const PARENT: &str = "rf010-parent";
const NOW: &str = "2026-01-02T03:04:05Z";

struct Fixture {
    vault: VaultStore,
    db: rusqlite::Connection,
    // Windows 上先释放两个真实数据库句柄，再移除合成目录。
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let (vault, dir) = setup_vault();
        let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
        vault
            .save_object(&ObjectRecord {
                id: PARENT.into(),
                account_id: ACCOUNT.into(),
                type_id: "page".into(),
                section_type: "page".into(),
                name: "Existing parent".into(),
                icon_name: "folder".into(),
                children_ids: vec!["existing-link".into()],
                properties: json!({"preserved": "parent value"}),
                sensitivity_level: "internal".into(),
                version: 9,
                created_at: "2025-01-01T00:00:00Z".into(),
                updated_at: "2025-01-01T00:00:00Z".into(),
                ..Default::default()
            })
            .unwrap();
        vault
            .save_snapshot(
                PARENT,
                "user_edit",
                br#"{"name":"Existing parent"}"#,
                "seed",
            )
            .unwrap();
        vault
            .log_structured(
                "seed",
                "page",
                Some(PARENT),
                Some("Existing parent"),
                "user",
                None,
            )
            .unwrap();
        Self {
            vault,
            db,
            _dir: dir,
        }
    }

    /// 比较全部对象（含父记录）、历史密文行和解密后的审计，防止失败路径提前写入。
    fn state(&self) -> Value {
        let mut records = self.vault.list_object_records(ACCOUNT).unwrap();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        let histories: Vec<Value> = self.db.prepare(
            "SELECT id, object_id, timestamp, triggered_by, data, diff_summary FROM object_snapshots ORDER BY id"
        ).unwrap().query_map([], |row| Ok(json!({
            "id": row.get::<_, String>(0)?, "object": row.get::<_, String>(1)?,
            "time": row.get::<_, i64>(2)?, "trigger": row.get::<_, String>(3)?,
            "data": row.get::<_, Vec<u8>>(4)?, "summary": row.get::<_, String>(5)?
        }))).unwrap().collect::<Result<_, _>>().unwrap();
        json!({"objects": records, "histories": histories, "audit": self.vault.list_audit_log(100).unwrap()})
    }

    fn assert_creation_side_effects(&self, record: &ObjectRecord, expected_action: &str) {
        let snapshots = self.vault.list_snapshots(&record.id).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0]["triggeredBy"], "user_edit");
        assert_eq!(snapshots[0]["diffSummary"], "diff_created");
        let bytes = self
            .vault
            .get_snapshot(snapshots[0]["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({
                "name": record.name, "tags": record.tags_json, "properties": record.properties,
                "propertyLabels": record.property_labels,
            })
        );
        let audits = self
            .vault
            .list_audit_log(100)
            .unwrap()
            .into_iter()
            .filter(|entry| entry.entity_id.as_deref() == Some(record.id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].action_type, expected_action);
        assert_eq!(audits[0].entity_name.as_deref(), Some(record.name.as_str()));
        assert_eq!(
            audits[0].entity_type,
            if record.type_id == "page" {
                "page"
            } else {
                "object"
            }
        );
        let details: Value = serde_json::from_str(audits[0].details.as_deref().unwrap()).unwrap();
        assert_eq!(details["section"], record.section_type);
    }
}

fn input() -> CreateObjectInput {
    CreateObjectInput {
        account_id: ACCOUNT.into(),
        name: "Optimistic object".into(),
        collection_type: "note".into(),
        properties: json!({"value": "kept", "false": false, "zero": 0, "empty": ""}),
        parent_id: Some(PARENT.into()),
        icon_name: Some("star".into()),
        template_id: None,
        template_type: None,
        id: Some("client-optimistic-rf010".into()),
    }
}

fn template() -> UserTemplate {
    serde_json::from_value(json!({
        "id": "rf010-template", "accountId": ACCOUNT, "name": "Synthetic template",
        "iconId": "template-icon", "category": "identity", "createdAt": NOW,
        "contractTypeId": "com.synthetic.rf010/v1",
        "properties": [
            {"id": "choice", "name": "历史选项", "type": "select", "options": ["one", "two"], "sensitivityLevel": "critical", "deprecatedAt": "2025-01-01T00:00:00Z", "contractField": true},
            {"id": "group", "name": "动态组", "type": "dynamic_group", "allowedTypes": ["text"], "maxItems": 2, "sensitivityLevel": "public"}
        ]
    })).unwrap()
}

#[test]
fn rf010_gui_persists_optimistic_id_parent_and_single_history_audit() {
    let fixture = Fixture::new();
    let input = input();
    let result = create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap();
    assert_eq!(result.id, "client-optimistic-rf010");
    let record = fixture.vault.load_object(&result.id).unwrap().unwrap();
    assert_eq!(record.id, result.id);
    assert_eq!(record.parent_id.as_deref(), Some(PARENT));
    assert_eq!(record.icon_name, "star");
    assert_eq!(record.properties, input.properties);
    assert_eq!(record.created_at, NOW);
    assert_eq!(record.updated_at, NOW);
    assert_eq!(record.version, 1);
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(record_to_data(&record)).unwrap()
    );
    let parent = fixture.vault.load_object(PARENT).unwrap().unwrap();
    assert_eq!(parent.children_ids, ["existing-link", record.id.as_str()]);
    assert_eq!(parent.version, 10);
    assert_eq!(parent.properties, json!({"preserved": "parent value"}));
    assert_eq!(fixture.vault.list_snapshots(PARENT).unwrap().len(), 1);
    fixture.assert_creation_side_effects(&record, "object_create");
}

#[test]
fn rf010_gui_generated_ids_and_page_type_keep_gui_defaults() {
    for collection_type in ["note", "page"] {
        let fixture = Fixture::new();
        let mut input = input();
        input.id = None;
        input.icon_name = None;
        input.parent_id = None;
        input.collection_type = collection_type.into();
        let parent_before =
            serde_json::to_value(fixture.vault.load_object(PARENT).unwrap()).unwrap();
        let result = create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap();
        assert!(uuid::Uuid::parse_str(&result.id).is_ok(), "{}", result.id);
        let record = fixture.vault.load_object(&result.id).unwrap().unwrap();
        assert_eq!(result.collection_type, collection_type);
        assert_eq!(record.type_id, collection_type);
        assert_eq!(record.section_type, collection_type);
        assert_eq!(record.icon_name, "document");
        assert!(record.parent_id.is_none());
        assert_eq!(
            serde_json::to_value(fixture.vault.load_object(PARENT).unwrap()).unwrap(),
            parent_before
        );
        fixture.assert_creation_side_effects(
            &record,
            if collection_type == "page" {
                "page_create"
            } else {
                "object_create"
            },
        );
    }
}

#[test]
fn rf010_active_client_id_conflict_performs_no_writes() {
    let fixture = Fixture::new();
    let mut input = input();
    create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap();
    let before = fixture.state();
    input.name = "Must not overwrite".into();
    input.properties = json!({"replacement": true});
    let error = create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap_err();
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(fixture.state(), before);
}

#[test]
fn rf010_gui_and_core_builder_produce_identical_complete_records() {
    let fixture = Fixture::new();
    let template = template();
    fixture.vault.save_user_template(&template).unwrap();
    let mut input = input();
    input.template_id = Some(template.id.clone());
    input.template_type = Some("user".into());
    input.properties = json!({
        "choice": "two", "group": [{"id": "entry", "name": "子项", "type": "text", "value": "user value"}],
        "__fields": {"stale": {"type": "number"}}, "__templateName": "stale", "__templateHash": "stale",
        "extra": {"flag": false}
    });
    let before = fixture.state();
    let core = objects::build_create_record(
        &fixture.vault,
        ACCOUNT,
        CreateRecordInput {
            id: input.id.clone().unwrap(),
            type_id: input.collection_type.clone(),
            section_type: input.collection_type.clone(),
            name: input.name.clone(),
            icon_name: input.icon_name.clone().unwrap(),
            parent_id: input.parent_id.clone(),
            template_id: input.template_id.clone(),
            template_type: input.template_type.clone(),
            properties: input.properties.clone(),
        },
        NOW,
    )
    .unwrap();
    assert_eq!(fixture.state(), before, "共享构造不得持久化");
    let result = create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap();
    let stored = fixture.vault.load_object(&result.id).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&stored).unwrap(),
        serde_json::to_value(&core).unwrap()
    );
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(record_to_data(&core)).unwrap()
    );
    assert_eq!(
        stored.property_labels,
        Some(json!({"choice": "critical", "group": "public"}))
    );
    assert_eq!(stored.properties["choice"], "two");
    assert_eq!(stored.properties["extra"], json!({"flag": false}));
    assert_eq!(stored.properties["__templateName"], template.name);
    assert_eq!(
        stored.properties["__templateHash"],
        stored.template_hash.clone().unwrap()
    );
    fixture.assert_creation_side_effects(&stored, "object_create");
}

#[test]
fn rf010_corrupt_template_ciphertext_or_json_rejects_before_all_writes() {
    for corruption in ["ciphertext", "json"] {
        let fixture = Fixture::new();
        let template = template();
        fixture.vault.save_user_template(&template).unwrap();
        let bad = if corruption == "ciphertext" {
            // 可解析的真实 SOLO 密文，使用错误密钥，必须经过认证解密失败路径。
            encrypt_text_field(&DataEncryptionKey::new([0x43; 32]), "[]").unwrap()
        } else {
            // 正确密钥可解密，但内容不是合法 JSON，不能被当成空模板。
            encrypt_text_field(&DataEncryptionKey::new([0x42; 32]), "invalid-json").unwrap()
        };
        fixture
            .db
            .execute(
                "UPDATE user_templates SET properties_json=?1 WHERE id=?2",
                rusqlite::params![bad, template.id],
            )
            .unwrap();
        assert!(fixture.vault.load_user_template(&template.id).is_err());
        let mut input = input();
        input.template_id = Some(template.id.clone());
        input.template_type = Some("user".into());
        let before = fixture.state();
        assert!(
            create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).is_err(),
            "{corruption}"
        );
        assert_eq!(fixture.state(), before, "{corruption}");
        assert!(fixture
            .vault
            .load_object(input.id.as_deref().unwrap())
            .unwrap()
            .is_none());
    }
}

#[test]
fn rf010_gui_without_template_still_validates_dynamic_groups_while_core_keeps_legacy_input() {
    for template_id in [None, Some("missing-template")] {
        let fixture = Fixture::new();
        let mut input = input();
        input.template_id = template_id.map(str::to_string);
        input.template_type = template_id.map(|_| "user".into());
        input.properties = json!({"__fields": {"group": {"type": "dynamic_group", "maxItems": 1}}, "group": "legacy non-array"});
        let before = fixture.state();
        let error = create_object_in_vault(&fixture.vault, &input, ACCOUNT, NOW).unwrap_err();
        assert!(error.contains("必须是数组"), "{error}");
        assert_eq!(fixture.state(), before);
        let core = objects::create_object(
            &fixture.vault,
            ACCOUNT,
            PARENT,
            &input.name,
            input.properties.clone(),
            template_id,
            None,
        )
        .unwrap();
        assert_eq!(core.properties, input.properties);
        assert_eq!(
            fixture
                .vault
                .load_object(&core.id)
                .unwrap()
                .unwrap()
                .properties,
            input.properties
        );
    }
}
