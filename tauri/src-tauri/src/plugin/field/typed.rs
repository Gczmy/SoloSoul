use super::helpers::*;
use super::{FieldResolver, PluginError, VaultStore};
use std::sync::Arc;

impl FieldResolver {
    // ── Stage 4-B typed-lookup ──────────────────────────────────────────

    /// 解析 field_id → (contract_type_id, prop_path)。双查路径：
    ///
    /// 1. **PRIMARY**：从 Vault 拉 UserTemplate 列表，find t where t.id == alias
    ///    AND t.contract_type_id.is_some()。
    /// 2. **SECONDARY**：从 self.contracts 找 c where c.type_id_aliases.contains(alias)。
    /// 3. 都 miss → Ok(None)（交给 caller：legacy fallback / typed InvalidField）
    pub fn parse_typed_field(
        &self,
        field_id: &str,
    ) -> Result<Option<(String, String)>, PluginError> {
        let (alias, prop_path) = parse_type_property(field_id)
            .ok_or_else(|| PluginError::InvalidField(format!("不支持的字段路径: {}", field_id)))?;

        // PRIMARY：UserTemplate 反查（user-space 真实 anchor）
        if let (Some(vault), Some(account_id)) = (self.vault.as_ref(), self.account_id.as_ref()) {
            let templates = vault
                .list_user_templates(account_id)
                .map_err(|e| PluginError::ExecutionFailed(format!("读取模板失败: {}", e)))?;
            if let Some(t) = templates
                .iter()
                .find(|t| t.id == alias && t.contract_type_id.is_some())
            {
                return Ok(Some((t.contract_type_id.clone().unwrap(), prop_path)));
            }
        }

        // SECONDARY：manifest contracts aliases 反查
        if let Some(c) = self
            .contracts
            .iter()
            .find(|c| c.type_id_aliases.iter().any(|a| a == &alias))
        {
            return Ok(Some((c.type_id.clone(), prop_path)));
        }

        Ok(None)
    }

    /// Typed-lookup 解析字段值（Stage 4-B 核心路径）
    pub(crate) fn resolve_typed(
        &self,
        field_id: &str,
        vault: &Arc<VaultStore>,
        account_id: &str,
    ) -> Result<String, PluginError> {
        // 1. 解析 typed 路径
        let parsed = self.parse_typed_field(field_id)?;
        let (ctid, prop_path) =
            parsed.ok_or_else(|| PluginError::InvalidField("typed 路径不能解析".into()))?;

        // 2. 按 contract_type_id 反查 UserTemplate（优先 contract_type_id，fallback t.id）
        let alias = parse_type_property(field_id)
            .map(|(a, _)| a)
            .unwrap_or_default();
        let templates = vault
            .list_user_templates(account_id)
            .map_err(|e| PluginError::ExecutionFailed(format!("读取模板失败: {}", e)))?;
        let template = templates
            .iter()
            .find(|t| t.contract_type_id.as_deref() == Some(&ctid))
            .or_else(|| templates.iter().find(|t| t.id == alias))
            .ok_or_else(|| {
                PluginError::InvalidField(format!(
                    "未找到 contract_type_id={} 或 id={} 的用户类型",
                    ctid, alias
                ))
            })?
            .clone();

        // 3. 按 template.contract_type_id 反查对象（不依赖 type_id，因为对象
        //    type_id 可能为 section/category 而非模板 ID）
        let target_ctid = template.contract_type_id.clone();
        let target_tpl_id = template.id.clone();
        let all_objects = vault
            .list_objects(account_id, None, None, None, false, false)
            .map_err(|e| PluginError::ExecutionFailed(format!("查询对象失败: {}", e)))?;
        let mut objects: Vec<_> = all_objects
            .into_iter()
            .filter(|o| {
                o.contract_type_id.as_deref() == target_ctid.as_deref()
                    || o.template_id.as_deref() == Some(&target_tpl_id)
                    || o.collection_type == alias
            })
            .collect();
        if objects.is_empty() {
            return Ok(String::new());
        }
        objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // 4. contract_field gate：仅标记了 contract_field=true 的属性可读
        let prop_first = prop_path.split('.').next().unwrap_or("");
        let prop = template
            .properties
            .iter()
            .find(|p| p.id == prop_first)
            .ok_or_else(|| {
                PluginError::InvalidField(format!("contract {} 没有属性 {}", ctid, prop_first))
            })?;
        if prop.contract_field != Some(true) {
            return Err(PluginError::InvalidField(format!(
                "属性 {} 未声明为 contract_field（gate 拒绝）",
                prop_first
            )));
        }

        Ok(extract_property(&objects[0].properties, &prop_path))
    }

    /// Typed-lookup 获取字段元数据（Stage 4-B）
    pub(crate) fn field_metadata_typed(
        &self,
        field_id: &str,
        vault: &Arc<VaultStore>,
        account_id: &str,
    ) -> Result<(String, String), PluginError> {
        // 复用 parse_typed_field 获取 ctid（保持与 resolve_typed 一致）
        let parsed = self.parse_typed_field(field_id)?;
        let (ctid, prop_path) = parsed.ok_or_else(|| {
            PluginError::InvalidField(format!("typed 路径不能解析字段元数据: {}", field_id))
        })?;

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
            .iter()
            .find(|t| t.contract_type_id.as_deref() == Some(&ctid))
            .or_else(|| {
                let alias = parse_type_property(field_id)
                    .map(|(a, _)| a)
                    .unwrap_or_default();
                templates.iter().find(|t| t.id == alias)
            })
            .ok_or_else(|| PluginError::InvalidField(format!("未找到类型: {}", ctid)))?
            .clone();

        let property = template
            .properties
            .into_iter()
            .find(|p| p.id == prop_first)
            .ok_or_else(|| {
                PluginError::InvalidField(format!("类型 {} 中未找到属性: {}", ctid, prop_first))
            })?;

        // contract_field gate
        if property.contract_field != Some(true) {
            return Err(PluginError::InvalidField(format!(
                "属性 {} 未声明为 contract_field（gate 拒绝）",
                prop_first
            )));
        }

        let label = property.name;
        let sensitivity = property
            .sensitivity_level
            .unwrap_or_else(|| "internal".to_string());
        Ok((label, sensitivity))
    }
}
