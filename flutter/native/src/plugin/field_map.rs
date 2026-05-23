//! 字段路径到 Vault 查询的运行时映射表
//!
//! 新增字段时仅需修改 FIELD_MAP，无需重新编译核心逻辑。
//! 未来支持从 ~/.solosoul/field_mapping.json 热加载自定义映射。

use std::collections::HashMap;

/// Vault 查询描述
#[derive(Debug, Clone)]
pub struct VaultQuery {
    pub object_type: String,
    pub property_key: String,
    pub tag: Option<String>,
}

lazy_static::lazy_static! {
    /// 字段路径到 VaultQuery 的运行时映射表
    static ref FIELD_MAP: HashMap<&'static str, VaultQuery> = {
        let mut m = HashMap::new();
        m.insert("identity.full_name", VaultQuery {
            object_type: "identity".to_string(),
            property_key: "full_name".to_string(),
            tag: None,
        });
        m.insert("identity.id_card.number", VaultQuery {
            object_type: "id_card".to_string(),
            property_key: "number".to_string(),
            tag: Some("primary".to_string()),
        });
        m.insert("travel.primary_passport.number", VaultQuery {
            object_type: "passport".to_string(),
            property_key: "number".to_string(),
            tag: Some("primary".to_string()),
        });
        m.insert("identity.contact.emails", VaultQuery {
            object_type: "identity".to_string(),
            property_key: "emails".to_string(),
            tag: None,
        });
        m.insert("identity.contact.phones", VaultQuery {
            object_type: "identity".to_string(),
            property_key: "phones".to_string(),
            tag: None,
        });
        m.insert("financial.primary_bank_account.number", VaultQuery {
            object_type: "bank_account".to_string(),
            property_key: "number".to_string(),
            tag: Some("primary".to_string()),
        });
        // TODO: 从 ~/.solosoul/field_mapping.json 热加载自定义映射
        m
    };
}

/// 将字段路径解析为 VaultQuery
pub fn resolve_field_to_vault_query(field_id: &str) -> Result<VaultQuery, String> {
    FIELD_MAP
        .get(field_id)
        .cloned()
        .ok_or_else(|| format!("unknown field: {}", field_id))
}

/// 获取所有已注册的字段路径
pub fn list_registered_fields() -> Vec<&'static str> {
    FIELD_MAP.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_known_field() {
        let query = resolve_field_to_vault_query("identity.full_name").unwrap();
        assert_eq!(query.object_type, "identity");
        assert_eq!(query.property_key, "full_name");
        assert!(query.tag.is_none());
    }

    #[test]
    fn test_resolve_tagged_field() {
        let query = resolve_field_to_vault_query("travel.primary_passport.number").unwrap();
        assert_eq!(query.object_type, "passport");
        assert_eq!(query.property_key, "number");
        assert_eq!(query.tag, Some("primary".to_string()));
    }

    #[test]
    fn test_resolve_unknown_field() {
        assert!(resolve_field_to_vault_query("unknown.field").is_err());
    }
}
