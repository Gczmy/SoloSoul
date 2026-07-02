use super::helpers::*;
use super::{FieldResolver, PluginError, VaultStore};
use solosoul_vault::{TemplateProperty, UserTemplate};
use std::sync::Arc;

/// 查找提供指定 contract role 的模板属性。
/// 优先新版 contract_bindings；其次回退 legacy contract_field + 字段 ID 匹配。
/// 若多个字段绑定到同一 role，取第一个匹配项，并在 tracing 中记录 warning。
fn find_property_for_role<'a>(
    template: &'a UserTemplate,
    ctid: &str,
    role_id: &str,
) -> Option<&'a TemplateProperty> {
    // 1. 新版：字段声明了 contract_bindings 且包含 (ctid, role_id)
    let all_matches: Vec<&TemplateProperty> = template
        .properties
        .iter()
        .filter(|p| {
            p.contract_bindings.as_ref().is_some_and(|bs| {
                bs.iter()
                    .any(|b| b.contract_type_id == ctid && b.role_id == role_id)
            })
        })
        .collect();

    if !all_matches.is_empty() {
        if all_matches.len() > 1 {
            tracing::warn!(
                "contract {} 的角色 {} 被 {} 个字段绑定，取第一个（id={}），请检查模板配置",
                ctid,
                role_id,
                all_matches.len(),
                all_matches[0].id,
            );
        }
        return Some(all_matches[0]);
    }

    // 2. 旧版兼容：字段 ID 等于 role_id 且 contract_field == true
    template
        .properties
        .iter()
        .find(|p| p.id == role_id && p.contract_field == Some(true))
}

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

        // 4. 通过 role binding 查询：新版 contract_bindings 优先，旧版 contract_field 兜底
        let prop_first = prop_path.split('.').next().unwrap_or("");
        let prop = find_property_for_role(&template, &ctid, prop_first).ok_or_else(|| {
            PluginError::InvalidField(format!(
                "contract {} 没有角色 {} 的绑定字段",
                ctid, prop_first
            ))
        })?;

        // 若 role 绑定到了不同字段 id，将 prop_path 中的 role 前缀替换为实际字段 id
        let actual_prop_path = if prop.id != prop_first {
            let suffix = &prop_path[prop_first.len()..];
            format!("{}{}", prop.id, suffix)
        } else {
            prop_path.to_string()
        };

        Ok(extract_property(&objects[0].properties, &actual_prop_path))
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

        // 通过 role binding 查询
        let property = find_property_for_role(&template, &ctid, &prop_first).ok_or_else(|| {
            PluginError::InvalidField(format!(
                "contract {} 没有角色 {} 的绑定字段",
                ctid, prop_first
            ))
        })?;

        let label = property.name.clone();
        let sensitivity = property
            .sensitivity_level
            .clone()
            .unwrap_or_else(|| "internal".to_string());
        Ok((label, sensitivity))
    }
}
