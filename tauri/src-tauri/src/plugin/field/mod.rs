//! 字段解析器 — 插件读取 Vault 字段的真实实现
//!
//! 支持字段路径：
//! - `<typeId>.count`              返回该类型对象数量
//! - `<typeId>[<index>].<prop>`    返回指定对象的属性值
//! - `<typeId>.<prop>`             返回第一个对象的属性值（便捷写法）
//!
//! 属性支持嵌套路径，如 `primary_passport.number`。
//!
//! ## Stage 4-B typed-lookup
//!
//! 当插件 manifest 声明了 `contracts` 字段时，`resolve()` 和 `field_metadata()` 会
//! 通过 `resolve_typed` 路径反查 `UserTemplate.contract_type_id` 和
//! `TemplateProperty.contract_field` gate，不再依赖字符串前缀匹配。

use super::manifest::PluginContractBinding;
use super::PluginError;
use solosoul_vault::VaultStore;
use std::sync::Arc;

/// 字段解析器
#[derive(Clone, Default)]
pub struct FieldResolver {
    vault: Option<Arc<VaultStore>>,
    account_id: Option<String>,
    allowed_patterns: Vec<String>,
    /// Stage 4 typed-lookup 契约绑定锚点（由 PluginManager::run 在构造时填充）
    contracts: Vec<PluginContractBinding>,
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
            contracts: Vec::new(),
        }
    }

    /// Stage 4-B：minimal-injection（仅 contracts；vault 由后续 with_vault 注入）
    pub fn with_contracts(contracts: Vec<PluginContractBinding>) -> Self {
        Self {
            vault: None,
            account_id: None,
            allowed_patterns: Vec::new(),
            contracts,
        }
    }

    /// Stage 4-B：combined inject（vault + contracts 一次性；PluginManager::run 主力路径）
    pub fn with_vault_and_contracts(
        vault: Arc<VaultStore>,
        account_id: String,
        allowed_patterns: Vec<String>,
        contracts: Vec<PluginContractBinding>,
    ) -> Self {
        Self {
            vault: Some(vault),
            account_id: Some(account_id),
            allowed_patterns,
            contracts,
        }
    }

    // ── 公共 API ────────────────────────────────────────────────────────

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

        // .count 始终走 legacy 路径
        if let Some(type_id) = field_id.strip_suffix(".count") {
            if type_id.is_empty() {
                return Err(PluginError::InvalidField("类型 ID 为空".to_string()));
            }
            let objects = vault
                .list_objects(account_id, Some(type_id), None, None, false, false)
                .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
            return Ok(objects.len().to_string());
        }

        // Stage 4-B：typed lookup（当 manifest 声明了 contracts 时）
        if !self.contracts.is_empty() {
            return self.resolve_typed(field_id, vault, account_id);
        }

        // Legacy 路径：尝试 <typeId>[<index>].<prop>
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

        // Legacy 路径：尝试 <typeId>.<prop>（默认取第一个对象）
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

        // Stage 4-B：typed lookup
        if !self.contracts.is_empty() {
            return self.field_metadata_typed(field_id, vault, account_id);
        }

        // Legacy 路径
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

// ── Sub-modules ─────────────────────────────────────────────

pub(crate) mod helpers;
#[cfg(test)]
pub mod tests;
pub(crate) mod typed;

pub(crate) use helpers::*;
