//! 字段解析器 — 插件读取 Vault 字段的真实实现
//!
//! 支持字段路径：
//! - `<typeId>.count`              返回该类型对象数量
//! - `<typeId>[<index>].<prop>`    返回指定对象的属性值
//! - `<typeId>.<prop>`             返回第一个对象的属性值（便捷写法）
//!
//! 属性支持嵌套路径，如 `primary_passport.number`。

use super::PluginError;
use solosoul_vault::VaultStore;
use std::sync::Arc;

/// 字段解析器
#[derive(Clone, Default)]
pub struct FieldResolver {
    vault: Option<Arc<VaultStore>>,
    account_id: Option<String>,
    allowed_patterns: Vec<String>,
}

impl FieldResolver {
    /// 创建空解析器（测试或 Vault 未解锁时使用）
    pub fn new() -> Self {
        Self::default()
    }

    /// 绑定 Vault 与会话信息
    pub fn with_vault(
        vault: Arc<VaultStore>,
        account_id: String,
        allowed_patterns: Vec<String>,
    ) -> Self {
        Self {
            vault: Some(vault),
            account_id: Some(account_id),
            allowed_patterns,
        }
    }

    /// 解析字段值
    pub fn resolve(&self, field_id: &str) -> Result<String, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;

        if field_id.is_empty() {
            return Err(PluginError::InvalidField("字段路径为空".to_string()));
        }

        // 验证并简化权限路径（去掉数组下标）
        let normalized = normalize_for_permission(field_id)
            .ok_or_else(|| PluginError::InvalidField(format!("非法字段路径: {}", field_id)))?;

        if !self.is_allowed(&normalized) {
            return Err(PluginError::InvalidField(format!(
                "字段未在 manifest 中声明: {}",
                field_id
            )));
        }

        // 解析具体请求
        if let Some(type_id) = field_id.strip_suffix(".count") {
            if type_id.is_empty() {
                return Err(PluginError::InvalidField("类型 ID 为空".to_string()));
            }
            let objects = vault
                .list_objects(account_id, Some(type_id), None, None, false, false)
                .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
            return Ok(objects.len().to_string());
        }

        // 尝试 <typeId>[<index>].<prop>
        if let Some((type_id, index, prop_path)) = parse_indexed_field(field_id) {
            let objects = vault
                .list_objects(account_id, Some(&type_id), None, None, false, false)
                .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;

            let mut objects = objects;
            objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));

            let record = objects
                .get(index)
                .ok_or_else(|| PluginError::InvalidField(format!("索引越界: {}", field_id)))?;

            return Ok(extract_property(&record.properties, &prop_path));
        }

        // 尝试 <typeId>.<prop>（默认取第一个对象）
        if let Some((type_id, prop_path)) = parse_type_property(field_id) {
            let objects = vault
                .list_objects(account_id, Some(&type_id), None, None, false, false)
                .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;

            if objects.is_empty() {
                return Ok(String::new());
            }
            let mut objects = objects;
            objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            return Ok(extract_property(&objects[0].properties, &prop_path));
        }

        Err(PluginError::InvalidField(format!(
            "不支持的字段路径: {}",
            field_id
        )))
    }

    fn is_allowed(&self, normalized: &str) -> bool {
        if self.allowed_patterns.is_empty() {
            // 没有声明权限时默认放行（兼容旧插件与测试）
            return true;
        }
        self.allowed_patterns
            .iter()
            .any(|p| pattern_matches(p, normalized))
    }

    /// 获取字段元数据（字段标签与敏感度等级）
    ///
    /// 支持路径：
    /// - `<typeId>[<index>].<prop>`
    /// - `<typeId>.<prop>`（默认取第一个对象）
    /// - 嵌套属性取第一级属性名匹配
    pub fn field_metadata(&self, field_id: &str) -> Result<(String, String), PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;

        if field_id.is_empty() {
            return Err(PluginError::InvalidField("字段路径为空".to_string()));
        }

        // 解析出类型 ID 与属性路径
        let (type_id, prop_path) =
            if let Some((type_id, _, prop_path)) = parse_indexed_field(field_id) {
                (type_id, prop_path)
            } else if let Some((type_id, prop_path)) = parse_type_property(field_id) {
                (type_id, prop_path)
            } else {
                return Err(PluginError::InvalidField(format!(
                    "无法解析字段元数据路径: {}",
                    field_id
                )));
            };

        let prop_first = prop_path.split('.').next().unwrap_or("").to_string();
        if prop_first.is_empty() {
            return Err(PluginError::InvalidField(format!(
                "字段路径缺少属性名: {}",
                field_id
            )));
        }

        let templates = vault
            .list_user_templates(account_id)
            .map_err(|e| PluginError::ExecutionFailed(format!("读取模板失败: {}", e)))?;

        let template = templates
            .into_iter()
            .find(|t| t.id == type_id)
            .ok_or_else(|| PluginError::InvalidField(format!("未找到类型: {}", type_id)))?;

        let property = template
            .properties
            .into_iter()
            .find(|p| p.id == prop_first)
            .ok_or_else(|| {
                PluginError::InvalidField(format!("类型 {} 中未找到属性: {}", type_id, prop_first))
            })?;

        let label = property.name;
        let sensitivity = property
            .sensitivity_level
            .unwrap_or_else(|| "internal".to_string());
        Ok((label, sensitivity))
    }

    /// 构建用户数据结构树（仅元数据，不含字段值）
    pub fn build_structure_tree(&self) -> Result<String, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;

        let templates = vault
            .list_user_templates(account_id)
            .map_err(|e| PluginError::ExecutionFailed(format!("读取模板失败: {}", e)))?;

        let types: Vec<serde_json::Value> = templates
            .into_iter()
            .map(|tpl| {
                let count = vault
                    .list_objects(account_id, Some(&tpl.id), None, None, false, false)
                    .map(|list| list.len())
                    .unwrap_or(0);

                let properties: Vec<serde_json::Value> = tpl
                    .properties
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "type": p.prop_type,
                            "sensitivity": p.sensitivity_level.unwrap_or_else(|| "internal".to_string())
                        })
                    })
                    .collect();

                serde_json::json!({
                    "id": tpl.id,
                    "name": tpl.name,
                    "category": tpl.category.unwrap_or_default(),
                    "count": count,
                    "properties": properties
                })
            })
            .collect();

        let tree = serde_json::json!({ "types": types });
        Ok(tree.to_string())
    }
}

/// 将 `address[0].street` 简化为 `address.street` 用于权限匹配
fn normalize_for_permission(field_id: &str) -> Option<String> {
    let mut result = String::with_capacity(field_id.len());
    let mut chars = field_id.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            // 跳过 [...] 内容
            let mut closed = false;
            for c in chars.by_ref() {
                if c == ']' {
                    closed = true;
                    break;
                }
                if !c.is_ascii_digit() {
                    return None;
                }
            }
            if !closed {
                return None;
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            result.push(ch);
        } else {
            return None;
        }
    }
    Some(result)
}

/// 匹配权限模式：精确匹配、`*.prop` 后缀、`type.*` 前缀、`*` 通配
fn pattern_matches(pattern: &str, field: &str) -> bool {
    if pattern == "*" || pattern == field {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        if field == prefix || field.starts_with(&format!("{}.", prefix)) {
            return true;
        }
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        if field == suffix || field.ends_with(&format!(".{}", suffix)) {
            return true;
        }
    }
    false
}

/// 解析 `<typeId>[<index>].<prop>`
fn parse_indexed_field(field_id: &str) -> Option<(String, usize, String)> {
    let bracket_open = field_id.find('[')?;
    let bracket_close = field_id.find(']')?;
    if bracket_close < bracket_open || bracket_close + 1 >= field_id.len() {
        return None;
    }
    let type_id = &field_id[..bracket_open];
    let index_str = &field_id[bracket_open + 1..bracket_close];
    let index: usize = index_str.parse().ok()?;
    if !field_id[bracket_close + 1..].starts_with('.') {
        return None;
    }
    let prop_path = field_id[bracket_close + 2..].to_string();
    Some((type_id.to_string(), index, prop_path))
}

/// 解析 `<typeId>.<prop>`（不是 count 且不含下标）
fn parse_type_property(field_id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = field_id.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let type_id = parts[0].to_string();
    let prop_path = parts[1..].join(".");
    Some((type_id, prop_path))
}

/// 从 JSON 属性中提取标量值（嵌套路径用 '.' 分隔）
fn extract_property(props: &serde_json::Value, prop_path: &str) -> String {
    let mut value = props;
    for key in prop_path.split('.') {
        if key.is_empty() {
            return String::new();
        }
        value = match value.get(key) {
            Some(v) => v,
            None => return String::new(),
        };
    }
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{ObjectRecord, PropertyType, TemplateProperty, UserTemplate, VaultConfig};
    use tempfile::TempDir;

    fn test_vault(account_id: &str) -> (TempDir, Arc<VaultStore>) {
        let tmp = TempDir::new().unwrap();
        let config =
            VaultConfig::new(account_id, tmp.path().to_path_buf()).with_data_key([0u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (tmp, Arc::new(vault))
    }

    #[test]
    fn test_field_metadata() {
        let account_id = "acc_test_meta";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            id: "address".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![
                TemplateProperty {
                    id: "street".to_string(),
                    name: "街道".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("private".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
                TemplateProperty {
                    id: "country".to_string(),
                    name: "国家".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("internal".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
            ],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now),
        };
        vault.save_user_template(&template).unwrap();

        let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);

        let (label, sensitivity) = resolver.field_metadata("address.street").unwrap();
        assert_eq!(label, "街道");
        assert_eq!(sensitivity, "private");

        let (label2, sensitivity2) = resolver.field_metadata("address[0].country").unwrap();
        assert_eq!(label2, "国家");
        assert_eq!(sensitivity2, "internal");

        // 嵌套路径取第一级属性
        let (label3, sensitivity3) = resolver.field_metadata("address.street.extra").unwrap();
        assert_eq!(label3, "街道");
        assert_eq!(sensitivity3, "private");

        assert!(resolver.field_metadata("unknown.street").is_err());
    }

    #[test]
    fn test_build_structure_tree() {
        let account_id = "acc_test_tree";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            id: "address".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![
                TemplateProperty {
                    id: "street".to_string(),
                    name: "街道".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("internal".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
                TemplateProperty {
                    id: "country".to_string(),
                    name: "国家".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("internal".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
            ],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now),
        };
        vault.save_user_template(&template).unwrap();

        // 写入一条地址对象，验证 count 统计
        let record = ObjectRecord {
            id: "addr_0".to_string(),
            account_id: account_id.to_string(),
            type_id: "address".to_string(),
            section_type: "identity".to_string(),
            name: "家".to_string(),
            icon_name: "map-pin".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"street": "长安街1号", "country": "CN"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);
        let json = resolver.build_structure_tree().unwrap();
        let tree: serde_json::Value = serde_json::from_str(&json).unwrap();

        let types = tree["types"].as_array().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0]["id"], "address");
        assert_eq!(types[0]["name"], "地址");
        assert_eq!(types[0]["category"], "identity");
        assert_eq!(types[0]["count"], 1);
        let props = types[0]["properties"].as_array().unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props[0]["id"], "street");
        assert_eq!(props[0]["type"], "text");
    }

    #[test]
    fn test_normalize_for_permission() {
        assert_eq!(
            normalize_for_permission("address[0].street"),
            Some("address.street".to_string())
        );
        assert_eq!(
            normalize_for_permission("travel.primary_passport.number"),
            Some("travel.primary_passport.number".to_string())
        );
        assert_eq!(
            normalize_for_permission("address.count"),
            Some("address.count".to_string())
        );
        assert!(normalize_for_permission("address[a].street").is_none());
        assert!(normalize_for_permission("").is_some());
    }

    #[test]
    fn test_pattern_matches() {
        assert!(pattern_matches("address.street", "address.street"));
        assert!(pattern_matches("address.*", "address.street"));
        assert!(pattern_matches("*.street", "address.street"));
        assert!(pattern_matches("*", "address.street"));
        assert!(!pattern_matches("address.city", "address.street"));
        assert!(!pattern_matches("identity.*", "address.street"));
    }

    #[test]
    fn test_parse_indexed_field() {
        assert_eq!(
            parse_indexed_field("address[0].street"),
            Some(("address".to_string(), 0, "street".to_string()))
        );
        assert_eq!(
            parse_indexed_field("travel[3].primary_passport.number"),
            Some((
                "travel".to_string(),
                3,
                "primary_passport.number".to_string()
            ))
        );
        assert!(parse_indexed_field("address.count").is_none());
    }

    #[test]
    fn test_extract_property() {
        let props = serde_json::json!({
            "street": "长安街1号",
            "postalCode": "100000",
            "primary_passport": { "number": "E12345678" }
        });
        assert_eq!(extract_property(&props, "street"), "长安街1号");
        assert_eq!(extract_property(&props, "postalCode"), "100000");
        assert_eq!(
            extract_property(&props, "primary_passport.number"),
            "E12345678"
        );
        assert_eq!(extract_property(&props, "missing"), "");
    }
}
