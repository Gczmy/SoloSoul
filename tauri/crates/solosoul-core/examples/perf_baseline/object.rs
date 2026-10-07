//! RF-312 固定合成对象；示例生成与原生验收共用。
use solosoul_vault::ObjectRecord;

pub(super) fn make_object(account_id: &str, index: usize) -> ObjectRecord {
    let needle = if index.is_multiple_of(20) {
        "needle"
    } else {
        "haystack"
    };
    let now = "2026-09-28T00:00:00Z".to_string();
    ObjectRecord {
        id: format!("obj_perf_{index:08}"),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: format!("Record {index:08}"),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "title": format!("Synthetic {index:08}"),
            "body": format!("{needle} reproducible vault performance sample {index:08}"),
            "category": format!("group-{}", index % 10),
            "fields": ["alpha", "beta", "gamma", "delta"]
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        contract_type_id: None,
        template_hash: None,
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    }
}
