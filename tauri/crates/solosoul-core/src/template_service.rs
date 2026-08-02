//! Default template seed registry — loads built-in object templates from embedded
//! JSON resource files (`resources/system_templates_*.json`).
//!
//! These templates are **not** runtime system templates. They are seed data that
//! gets imported once into the user's vault as regular `UserTemplate`s during
//! account creation. After import, users can freely edit or delete them.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data types (seed format — mirrors the JSON structure)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTemplateProperty {
    pub id: String,
    pub name_i18n_key: String,
    pub name_fallback: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    /// 4-tier sensitivity level: "public" | "internal" | "sensitive" | "critical".
    /// Replaces the legacy `sensitive` boolean.
    pub sensitivity_level: Option<String>,
    /// Legacy boolean — kept for backward-compat during deserialization only.
    #[serde(default, skip_serializing)]
    pub sensitive: Option<bool>,
    pub required: Option<bool>,
    pub options: Option<Vec<String>>,
    /// 插件合约字段映射 — 当此属性映射到插件合约中的字段时为 true（旧版）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_field: Option<bool>,
    /// 新版插件契约角色绑定。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "contractBindings"
    )]
    pub contract_bindings: Option<Vec<solosoul_vault::ContractRoleBinding>>,
    /// 动态字段组允许创建的子字段类型；空/缺失表示不限制。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "allowedTypes"
    )]
    pub allowed_types: Option<Vec<String>>,
    /// 动态字段组允许的最大子字段数量；缺失表示无限制。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "maxItems")]
    pub max_items: Option<u32>,
}

impl SystemTemplateProperty {
    /// Return the effective sensitivity level, migrating legacy `sensitive` boolean.
    pub fn effective_sensitivity_level(&self) -> String {
        self.sensitivity_level.clone().unwrap_or_else(|| {
            if self.sensitive.unwrap_or(false) {
                "sensitive".to_string()
            } else {
                "internal".to_string()
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTemplate {
    pub key: String,
    pub category: String,
    pub icon: String,
    pub name_i18n_key: String,
    pub name_fallback: String,
    pub properties: Vec<SystemTemplateProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_type_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemTemplateResource {
    version: u32,
    templates: Vec<SystemTemplate>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct SystemTemplateRegistry {
    templates: HashMap<String, SystemTemplate>,
    version: u32,
}

impl SystemTemplateRegistry {
    /// Load templates from an embedded JSON resource for the given locale.
    /// `locale` should be a language code like "zh-CN" or "en-US".
    pub fn load_for_locale(locale: &str) -> Result<Self, String> {
        let json_str = if locale.starts_with("zh") {
            include_str!("../resources/system_templates_zh.json")
        } else {
            include_str!("../resources/system_templates_en.json")
        };
        let data: SystemTemplateResource =
            serde_json::from_str(json_str).map_err(|e| format!("Parse system_templates: {}", e))?;

        let mut templates = HashMap::new();
        for tpl in data.templates {
            templates.insert(tpl.key.clone(), tpl);
        }

        Ok(Self {
            templates,
            version: data.version,
        })
    }

    // -- Public query API ---------------------------------------------------

    pub fn get(&self, key: &str) -> Option<SystemTemplate> {
        self.templates.get(key).cloned()
    }

    pub fn list_all(&self) -> Vec<SystemTemplate> {
        self.templates.values().cloned().collect()
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

// ---------------------------------------------------------------------------
// Seed import — convert SystemTemplates into UserTemplates and persist
// ---------------------------------------------------------------------------

/// Import default templates from the seed registry into the vault as regular
/// user templates. Called once during account creation.
pub fn seed_default_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    locale: &str,
) -> Result<(), String> {
    let registry = SystemTemplateRegistry::load_for_locale(locale)?;

    for st in registry.list_all() {
        let properties: Vec<solosoul_vault::TemplateProperty> = st
            .properties
            .iter()
            .map(|p| {
                let prop_type = solosoul_vault::PropertyType::parse(&p.prop_type)
                    .unwrap_or(solosoul_vault::PropertyType::Text);
                let allowed_types = p.allowed_types.as_ref().map(|list| {
                    list.iter()
                        .filter_map(|s| solosoul_vault::PropertyType::parse(s))
                        .collect()
                });
                solosoul_vault::TemplateProperty {
                    contract_field: p.contract_field,
                    contract_bindings: p.contract_bindings.clone(),
                    id: p.id.clone(),
                    name: p.name_fallback.clone(),
                    prop_type,
                    sensitivity_level: Some(p.effective_sensitivity_level()),
                    options: p.options.clone(),
                    sensitive: None,
                    deprecated_at: None,
                    allowed_types,
                    max_items: p.max_items,
                }
            })
            .collect();

        let now = chrono::Utc::now().to_rfc3339();
        let user_template = solosoul_vault::UserTemplate {
            contract_type_id: st.contract_type_id.clone(),
            id: st.key.clone(),
            account_id: account_id.to_string(),
            name: st.name_fallback.clone(),
            icon_id: Some(st.icon.clone()),
            properties,
            category: Some(st.category.clone()),
            created_at: now.clone(),
            updated_at: Some(now),
        };

        vault.save_user_template(&user_template)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Plugin install migration — seed templates contract_bindings 补齐
// ---------------------------------------------------------------------------

/// 计算模板指纹，用于判断对象是否需要同步模板更新。
/// 按字段 id 稳定排序后序列化再取 SHA-256 前 16 位。
fn template_fingerprint(tpl: &solosoul_vault::UserTemplate) -> String {
    let mut props: Vec<&solosoul_vault::TemplateProperty> = tpl.properties.iter().collect();
    props.sort_by(|a, b| a.id.cmp(&b.id));
    let canonical = serde_json::json!({
        "properties": props,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hex::encode(&hash[..8])
}

/// 插件安装后迁移种子模板的 contract_bindings。
/// 对模板中 contractField: true 但 contract_bindings 为空的字段，
/// 根据已安装插件的合同合约 roles[].defaultPropertyId 自动推导绑定并持久化。
///
/// # 参数
/// - `contracts`: 插件声明的合同列表，每项为 `(type_id, role_id, default_property_id)` 元组。
pub fn migrate_contract_bindings(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    contracts: &[(String, String, String)],
) -> Result<usize, String> {
    // 构建 contract_type_id → [(role_id, default_property_id)] 映射
    let mut contract_map: std::collections::HashMap<&str, Vec<(&str, &str)>> =
        std::collections::HashMap::new();
    for (ctid, role_id, default_pid) in contracts {
        contract_map
            .entry(ctid.as_str())
            .or_default()
            .push((role_id.as_str(), default_pid.as_str()));
    }

    if contract_map.is_empty() {
        return Ok(0);
    }

    let templates = vault.list_user_templates(account_id)?;
    let mut migrated_count = 0usize;

    for mut tpl in templates {
        // 跳过无 contract_type_id 的模板
        let ctid = match &tpl.contract_type_id {
            Some(id) => id.clone(),
            None => continue,
        };

        // 检查插件是否声明了此 type_id
        let Some(roles) = contract_map.get(ctid.as_str()) else {
            continue;
        };

        let mut changed = false;
        let mut new_properties = tpl.properties.clone();

        for prop in &mut new_properties {
            // 跳过无 contractField 或已有 bindings 的字段
            if !prop.contract_field.unwrap_or(false) {
                continue;
            }
            if prop
                .contract_bindings
                .as_ref()
                .is_some_and(|b| !b.is_empty())
            {
                continue;
            }

            // 在插件的 roles 中查找 defaultPropertyId 匹配
            for (role_id, default_pid) in roles {
                if *default_pid == prop.id {
                    let binding = solosoul_vault::ContractRoleBinding {
                        contract_type_id: ctid.clone(),
                        role_id: (*role_id).to_string(),
                    };
                    prop.contract_bindings = Some(vec![binding]);
                    changed = true;
                    migrated_count += 1;
                    break; // 一个字段只绑定一个 role
                }
            }
        }

        if changed {
            // 在修改前计算旧指纹，修改后计算新指纹
            let old_hash = template_fingerprint(&tpl);
            tpl.properties = new_properties;
            tpl.updated_at = Some(chrono::Utc::now().to_rfc3339());
            let new_hash = template_fingerprint(&tpl);
            vault.save_user_template(&tpl)?;

            // 同步更新所有使用该模板且指纹等于旧指纹的对象
            // P111: 仅需 template_id 筛选候选，随后逐个 load_object，走 metadata-only 查询。
            let objects = vault.list_object_metadata(account_id, None, None, false, false)?;
            for obj in objects {
                if obj.template_id.as_deref() != Some(&tpl.id) {
                    continue;
                }
                let mut record = match vault.load_object(&obj.id)? {
                    Some(r) => r,
                    None => continue,
                };
                if record.template_hash.as_deref() != Some(&old_hash) {
                    continue;
                }
                record.template_hash = Some(new_hash.clone());
                record.updated_at = chrono::Utc::now().to_rfc3339();
                record.version += 1;
                vault.save_object(&record)?;
            }
        }
    }

    Ok(migrated_count)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_zh_templates() {
        let registry = SystemTemplateRegistry::load_for_locale("zh-CN").unwrap();
        assert!(!registry.templates.is_empty());
        assert!(registry.templates.contains_key("passport"));
        let passport = registry.get("passport").unwrap();
        assert_eq!(passport.name_fallback, "护照");
    }

    #[test]
    fn test_load_en_templates() {
        let registry = SystemTemplateRegistry::load_for_locale("en-US").unwrap();
        assert!(!registry.templates.is_empty());
        assert!(registry.templates.contains_key("passport"));
        let passport = registry.get("passport").unwrap();
        assert_eq!(passport.name_fallback, "Passport");
    }
}
