//! RF-302：真实 Vault 与 Host wire 序列化兼容，不复制创建/回滚算法。

use super::super::snapshot::{list_snapshots_in_vault, rollback_snapshot_in_vault, SnapshotEntry};
use super::super::{
    create_object_in_vault, record_to_data, CreateObjectInput, ObjectFilter, SyncFieldChangeItem,
    UpdateObjectInput,
};
use super::setup_vault;
use serde_json::{json, Value};
use solosoul_vault::ObjectRecord;

const ACCOUNT: &str = "test_account";
const NOW: &str = "2001-02-03T04:05:06Z";

fn record(id: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: ACCOUNT.into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        name: "Synthetic RF302 对象".into(),
        icon_name: "document".into(),
        properties: json!({}),
        sensitivity_level: "internal".into(),
        created_at: NOW.into(),
        updated_at: NOW.into(),
        version: 1,
        ..Default::default()
    }
}

#[test]
fn rf302_snapshot_wire_matches_real_vault_order_limit_and_preserves_payloads() {
    let (vault, _dir) = setup_vault();
    let object = record("rf302-snapshots");
    vault.save_object(&object).unwrap();
    assert!(list_snapshots_in_vault(&vault, &object.id)
        .unwrap()
        .is_empty());
    for index in 0..52 {
        let payload = json!({
            "name": format!("历史 {index}"),
            "properties": {"value": [null, false, 0, "Unicode 雪"]},
            "property_labels": {"value": "critical"},
        });
        vault
            .save_snapshot_at(
                &object.id,
                if index % 2 == 0 {
                    "user_edit"
                } else {
                    "rollback"
                },
                &serde_json::to_vec(&payload).unwrap(),
                &format!("synthetic-diff-{index}"),
                1_000 + index,
            )
            .unwrap();
    }
    vault
        .save_snapshot_at("rf302-other", "user_edit", b"null", "other", 9_999)
        .unwrap();

    let before_object = serde_json::to_value(vault.load_object(&object.id).unwrap()).unwrap();
    let raw = vault.list_snapshots(&object.id).unwrap();
    let data_before: Vec<_> = raw
        .iter()
        .map(|entry| {
            vault
                .get_snapshot(entry["id"].as_str().unwrap())
                .unwrap()
                .unwrap()
        })
        .collect();
    let result = list_snapshots_in_vault(&vault, &object.id).unwrap();
    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized, serde_json::to_value(&raw).unwrap());
    assert_eq!(result.len(), 50);
    assert_eq!(result.first().unwrap().timestamp, 1_051);
    assert_eq!(result.last().unwrap().timestamp, 1_002);
    assert_eq!(result.first().unwrap().diff_summary, "synthetic-diff-51");
    assert_eq!(result.first().unwrap().triggered_by, "rollback");
    for (index, entry) in result.iter().enumerate() {
        assert_eq!(
            serde_json::to_value(entry)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            4
        );
        let bytes = vault.get_snapshot(&entry.id).unwrap().unwrap();
        assert_eq!(bytes, data_before[index]);
        let payload: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["property_labels"], json!({"value": "critical"}));
        assert_eq!(
            payload["properties"]["value"],
            json!([null, false, 0, "Unicode 雪"])
        );
    }
    let count = vault
        .count_snapshots_batch(&[object.id.clone(), "rf302-other".into()])
        .unwrap();
    assert_eq!(count[&object.id], 52);
    assert_eq!(count["rf302-other"], 1);
    assert_eq!(
        serde_json::to_value(vault.load_object(&object.id).unwrap()).unwrap(),
        before_object
    );
    assert!(vault.list_audit_log(100).unwrap().is_empty());
}

#[test]
fn rf302_snapshot_wire_propagates_real_vault_read_error() {
    let (vault, _dir) = setup_vault();
    vault.lock();
    let expected = vault.list_snapshots("synthetic-locked").unwrap_err();
    assert!(!expected.is_empty());
    assert_eq!(
        list_snapshots_in_vault(&vault, "synthetic-locked").unwrap_err(),
        expected
    );
    assert!(serde_json::from_value::<SnapshotEntry>(json!({
        "id": "bad", "timestamp": "not-a-number", "triggeredBy": "user_edit", "diffSummary": ""
    }))
    .is_err());
    assert!(serde_json::from_value::<SnapshotEntry>(json!({
        "id": "missing", "timestamp": 0, "triggeredBy": "user_edit"
    }))
    .is_err());
}

#[test]
fn rf302_created_object_and_real_summary_keep_distinct_null_and_omitted_fields() {
    let (vault, _dir) = setup_vault();
    let properties =
        json!({"plain": "雪", "zero": 0, "flag": false, "empty": "", "nested": [null, {}, []]});
    let input: CreateObjectInput = serde_json::from_value(json!({
        "id": "rf302-client-id", "accountId": ACCOUNT, "name": "Synthetic creation",
        "typeId": "note", "properties": properties,
    }))
    .unwrap();
    let response = create_object_in_vault(&vault, &input, ACCOUNT, NOW).unwrap();
    let output = serde_json::to_value(response).unwrap();
    assert_eq!(
        output,
        json!({
            "id": "rf302-client-id", "accountId": ACCOUNT, "name": "Synthetic creation",
            "typeId": "note", "properties": properties, "sensitivityLevel": "internal",
            "templateId": null, "templateType": null, "propertyLabels": null,
            "createdAt": NOW, "updatedAt": NOW, "deletedAt": null,
            "contractTypeId": null, "templateHash": null, "ignoredTemplateHash": null,
        })
    );
    let summaries = vault
        .list_objects(ACCOUNT, None, None, None, false, false)
        .unwrap();
    assert_eq!(summaries.len(), 1);
    let summary = serde_json::to_value(&summaries[0]).unwrap();
    assert_eq!(summary["templateId"], Value::Null);
    assert_eq!(summary["templateType"], Value::Null);
    assert!(summary.as_object().unwrap().contains_key("templateId"));
    assert!(summary.as_object().unwrap().contains_key("templateType"));
    assert_eq!(summary["tags"], json!([]));
    assert_eq!(summary["isDeleted"], false);
    assert_eq!(summary["hasAttachments"], false);
    assert!(summary["sectionType"].is_string());
    assert!(summary["iconName"].is_string());
    for omitted in [
        "parentId",
        "contractTypeId",
        "templateHash",
        "ignoredTemplateHash",
        "propertyLabels",
        "sensitivityLevels",
    ] {
        assert!(
            summary.get(omitted).is_none(),
            "unexpected wire field {omitted}"
        );
    }

    let mut stored = vault.load_object("rf302-client-id").unwrap().unwrap();
    let labels = json!({"plain": "public", "zero": "internal", "flag": "sensitive", "empty": "critical", "future": "unrecognized"});
    stored.property_labels = Some(labels.clone());
    stored.tags_json = vec!["标签".into(), "second".into()];
    vault.save_object(&stored).unwrap();
    let detailed = serde_json::to_value(record_to_data(
        &vault.load_object(&stored.id).unwrap().unwrap(),
    ))
    .unwrap();
    let summary = serde_json::to_value(
        &vault
            .list_objects(ACCOUNT, None, None, None, false, false)
            .unwrap()[0],
    )
    .unwrap();
    assert_eq!(detailed["properties"], properties);
    assert_eq!(detailed["propertyLabels"], labels);
    assert_eq!(summary["propertyLabels"], labels);
    assert_eq!(detailed["tags"], json!(["标签", "second"]));
    assert_eq!(summary["tags"], detailed["tags"]);
    assert_eq!(
        summary["sensitivityLevels"],
        json!(["public", "internal", "sensitive", "critical"])
    );
    assert_eq!(detailed["sensitivityLevel"], "internal");
}

#[test]
fn rf302_inputs_preserve_camel_case_optional_values_and_arbitrary_json() {
    for properties in [
        Value::Null,
        json!([]),
        json!(false),
        json!(0),
        json!("scalar"),
        json!({"a": [null, false]}),
    ] {
        for explicit_nulls in [false, true] {
            let mut payload = json!({"accountId": ACCOUNT, "name": "Synthetic", "typeId": "note", "properties": properties});
            if explicit_nulls {
                for key in ["parentId", "iconName", "templateId", "templateType", "id"] {
                    payload[key] = Value::Null;
                }
            }
            let input: CreateObjectInput = serde_json::from_value(payload).unwrap();
            assert_eq!(input.collection_type, "note");
            assert_eq!(input.properties, properties);
            assert!(
                input.parent_id.is_none()
                    && input.icon_name.is_none()
                    && input.template_id.is_none()
                    && input.template_type.is_none()
                    && input.id.is_none()
            );
        }
    }
    let update: UpdateObjectInput = serde_json::from_value(json!({"name": "Synthetic", "properties": [0, false], "sensitivityLevel": null, "iconName": ""})).unwrap();
    assert_eq!(update.properties, json!([0, false]));
    assert!(update.sensitivity_level.is_none());
    assert_eq!(update.icon_name.as_deref(), Some(""));
    let filter: ObjectFilter = serde_json::from_value(
        json!({"typeId": "note", "sensitivityLevel": null, "includeDeleted": false}),
    )
    .unwrap();
    assert_eq!(filter.collection_type.as_deref(), Some("note"));
    assert_eq!(filter.include_deleted, Some(false));
    assert!(
        filter.keyword.is_none()
            && filter.parent_id.is_none()
            && filter.sensitivity_level.is_none()
    );
    assert!(serde_json::from_value::<ObjectFilter>(json!({"includeDeleted": "false"})).is_err());
    assert!(serde_json::from_value::<CreateObjectInput>(json!({"accountId": ACCOUNT, "name": "Synthetic", "collection_type": "note", "properties": {}})).is_err());
}

#[test]
fn rf302_template_changes_keep_adjacent_payload_and_unit_variant_wire_shape() {
    let changes = vec![
        SyncFieldChangeItem::Type {
            old_type: "text".into(),
            new_type: "number".into(),
        },
        SyncFieldChangeItem::Name {
            old_name: "旧名".into(),
            new_name: "新名".into(),
        },
        SyncFieldChangeItem::Sensitivity {
            old_level: "public".into(),
            new_level: "critical".into(),
        },
        SyncFieldChangeItem::Options,
        SyncFieldChangeItem::Metadata {
            metadata_keys: vec!["allowedTypes".into(), "maxItems".into()],
        },
    ];
    let expected = json!([
        {"kind": "type", "payload": {"oldType": "text", "newType": "number"}},
        {"kind": "name", "payload": {"oldName": "旧名", "newName": "新名"}},
        {"kind": "sensitivity", "payload": {"oldLevel": "public", "newLevel": "critical"}},
        {"kind": "options"},
        {"kind": "metadata", "payload": {"metadataKeys": ["allowedTypes", "maxItems"]}},
    ]);
    assert_eq!(serde_json::to_value(&changes).unwrap(), expected);
    let decoded: Vec<SyncFieldChangeItem> = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn rf302_real_rollback_success_remains_null_with_legacy_labels_and_one_new_history() {
    for key in ["propertyLabels", "property_labels"] {
        let (vault, _dir) = setup_vault();
        let original = record("rf302-rollback");
        vault.save_object(&original).unwrap();
        let mut payload =
            json!({"name": "恢复 Unicode", "tags": ["tag"], "properties": {"secret": "synthetic"}});
        payload[key] = json!({"secret": "critical"});
        vault
            .save_snapshot_at(
                &original.id,
                "import",
                &serde_json::to_vec(&payload).unwrap(),
                "source",
                1_000,
            )
            .unwrap();
        let source = list_snapshots_in_vault(&vault, &original.id)
            .unwrap()
            .remove(0);
        let response = rollback_snapshot_in_vault(&vault, &source.id, &original.id)
            .map(serde_json::to_value)
            .unwrap()
            .unwrap();
        assert_eq!(response, Value::Null);
        let stored = vault.load_object(&original.id).unwrap().unwrap();
        assert_eq!(stored.property_labels, Some(json!({"secret": "critical"})));
        assert_eq!(stored.icon_name, original.icon_name);
        assert_eq!(stored.version, original.version + 1);
        let entries = list_snapshots_in_vault(&vault, &original.id).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].triggered_by, "rollback");
        assert_eq!(entries[0].diff_summary, "diff_rollback");
        assert_eq!(vault.list_audit_log(10).unwrap().len(), 1);
    }
}
