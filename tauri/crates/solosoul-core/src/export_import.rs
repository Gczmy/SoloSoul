//! Export/Import 辅助函数
//!
//! 提供模板内容哈希（用于快照去重）、导入模板 ID 生成等功能。
//! 供 Tauri 后端和 CLI 共用。

/// 计算 UserTemplate 的内容哈希，用于判断是否是同一份"快照模板"。
/// 忽略 account_id、id、created_at、updated_at 等随账户/时间变化的字段。
fn template_properties_sorted(tpl: &solosoul_vault::UserTemplate) -> Vec<serde_json::Value> {
    let mut sorted: Vec<&solosoul_vault::TemplateProperty> = tpl.properties.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    sorted
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "prop_type": p.prop_type,
                "sensitivity_level": p.sensitivity_level,
                "options": p.options,
                "contract_field": p.contract_field,
            })
        })
        .collect()
}

/// 计算 UserTemplate 的内容哈希，用于判断是否是同一份"快照模板"。
/// 忽略 account_id、id、created_at、updated_at 等随账户/时间变化的字段。
pub fn user_template_content_hash(tpl: &solosoul_vault::UserTemplate) -> String {
    use sha2::Digest;

    let sorted_props = template_properties_sorted(tpl);
    let canonical = serde_json::json!({
        "name": tpl.name,
        "icon_id": tpl.icon_id,
        "category": tpl.category,
        "contract_type_id": tpl.contract_type_id,
        "properties": sorted_props,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    hex::encode(sha2::Sha256::digest(&bytes))
}

/// 生成导入模板的本地 ID，格式：`imported:<content_hash_prefix>:<original_id>`。
pub fn imported_template_id(original_id: &str, content_hash: &str) -> String {
    // 取哈希前 12 字符作为可读前缀，保证唯一性同时不过长
    let prefix = &content_hash[..content_hash.len().min(12)];
    format!("imported:{}:{}", prefix, original_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{PropertyType, TemplateProperty, UserTemplate};

    fn make_template(id: &str, name: &str, field_name: &str) -> UserTemplate {
        UserTemplate {
            contract_type_id: None,
            id: id.to_string(),
            account_id: "acc_1".to_string(),
            name: name.to_string(),
            icon_id: Some("identity".to_string()),
            category: Some("identity".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: Some("2024-06-01T00:00:00Z".to_string()),
            properties: vec![TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: field_name.to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
            }],
        }
    }

    #[test]
    fn test_content_hash_ignores_id_and_timestamps() {
        let tpl_a = make_template("identity", "身份信息", "证件号码");
        let tpl_b = make_template("utpl_xxx", "身份信息", "证件号码");

        let hash_a = user_template_content_hash(&tpl_a);
        let hash_b = user_template_content_hash(&tpl_b);

        // 不同 id 但相同内容 → 相同 hash
        assert_eq!(hash_a, hash_b);
        assert_eq!(hash_a.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_content_hash_differs_when_content_differs() {
        let tpl_cn = make_template("identity", "身份信息", "证件号码");
        let tpl_en = make_template("identity", "Identity", "ID Number");

        let hash_cn = user_template_content_hash(&tpl_cn);
        let hash_en = user_template_content_hash(&tpl_en);

        assert_ne!(hash_cn, hash_en);
    }

    #[test]
    fn test_imported_template_id_format() {
        let tpl = make_template("identity", "身份信息", "证件号码");
        let hash = user_template_content_hash(&tpl);
        let imported = imported_template_id("identity", &hash);

        // 格式: imported:<12-char-prefix>:identity
        assert!(imported.starts_with("imported:"));
        assert!(imported.ends_with(":identity"));
        // 总长度前缀(9) + 12 + 分隔符(1) + "identity"(8) = 30
        assert_eq!(imported.len(), 30);
    }

    #[test]
    fn test_imported_template_id_deterministic() {
        let tpl = make_template("identity", "身份信息", "证件号码");
        let hash = user_template_content_hash(&tpl);

        let id1 = imported_template_id("identity", &hash);
        let id2 = imported_template_id("identity", &hash);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let tpl = make_template("identity", "身份信息", "证件号码");
        let h1 = user_template_content_hash(&tpl);
        let h2 = user_template_content_hash(&tpl);
        assert_eq!(h1, h2);
    }
}
