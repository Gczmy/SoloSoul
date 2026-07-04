//! 字段解析器 — 插件读取 Vault 字段的真实实现
//!
//! 支持字段路径：
//! - `<typeId>[<index>].<prop>`    返回指定对象的属性值
//! - `<typeId>.<prop>`             返回第一个对象的属性值（便捷写法）
//!
//! ## Phase 5：count 移至插件侧
//!
//! `.count` 已从软件侧删除。插件应通过 SDK `list_objects()` 批量获取指定类型的
//! 所有对象（返回 JSON 数组），在插件内部完成计数和属性提取。详见 `list_objects()` 方法。
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
use solosoul_vault::{TemplateProperty, UserTemplate, VaultStore};
use std::sync::Arc;

/// 附件列表项（用于 list_attachments）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AttItem {
    id: String,
    object_id: String,
    file_name: String,
    mime_type: String,
    size_bytes: u64,
}

/// 带附件的对象摘要
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AttObject {
    object_id: String,
    object_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template_name: Option<String>,
    attachments: Vec<AttItem>,
}

/// 页面级附件分组
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AttPage {
    page_id: Option<String>,
    page_name: String,
    objects: Vec<AttObject>,
}

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

    /// 获取 Vault 引用（供 Host Functions 使用）
    pub fn vault_ref(&self) -> Option<&Arc<VaultStore>> {
        self.vault.as_ref()
    }

    /// 获取账户 ID 引用（供 Host Functions 使用）
    pub fn account_id_ref(&self) -> Option<&String> {
        self.account_id.as_ref()
    }

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

    fn resolve_typed(
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
                // 通过 template_id 匹配（避免 contract_type_id 跨模板污染）。
                // 对于无 template_id 的旧对象，回退到 contract_type_id 匹配。
                o.template_id.as_deref() == Some(&target_tpl_id)
                    || (o.template_id.is_none()
                        && o.contract_type_id.as_deref() == target_ctid.as_deref())
                    || o.collection_type == alias
            })
            .collect();
        if objects.is_empty() {
            return Ok(String::new());
        }
        objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // 4. 通过 role binding 查询：新版 contract_bindings 优先，旧版 contract_field 兜底
        let prop_first = prop_path.split('.').next().unwrap_or("");
        let prop = Self::find_property_for_role(&template, &ctid, prop_first).ok_or_else(|| {
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
    fn field_metadata_typed(
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

        let property =
            Self::find_property_for_role(&template, &ctid, &prop_first).ok_or_else(|| {
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

        // __name__ 特殊路径：返回对象名称（系统字段，非 properties 内的自定义属性）
        if self.is_name_field(field_id) {
            // Stage 4-B typed-lookup：当插件声明 contracts 时，同 .count 一样需要
            // 通过 contract_type_id 查询对象，而非 type_id。
            if !self.contracts.is_empty() {
                let alias = field_id.find('.').map(|dot| &field_id[..dot]).unwrap_or("");
                if let Some((ctid, _)) = self.parse_typed_field(&format!("{}.dummy", alias))? {
                    let all = vault
                        .list_objects(account_id, None, None, None, false, false)
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e))
                        })?;
                    let mut objects: Vec<_> = all
                        .into_iter()
                        .filter(|o| {
                            // 通过 template_id 匹配（避免 contract_type_id 跨模板污染）。
                            // 对于无 template_id 的旧对象，回退到 contract_type_id 匹配。
                            o.template_id.as_deref() == Some(alias)
                                || (o.template_id.is_none()
                                    && o.contract_type_id.as_deref() == Some(&ctid))
                                || o.collection_type == alias
                        })
                        .collect();
                    if objects.is_empty() {
                        return Ok(String::new());
                    }
                    objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));

                    // 检查是否带下标
                    if let Some((_, index, _)) = parse_indexed_field(field_id) {
                        return Ok(objects
                            .get(index)
                            .ok_or_else(|| {
                                PluginError::InvalidField(format!("索引越界: {}", field_id))
                            })?
                            .name
                            .clone());
                    }
                    return Ok(objects[0].name.clone());
                }
            }

            // Legacy __name__ 路径
            if let Some((type_id, index, _)) = parse_indexed_field(field_id) {
                let objects = vault
                    .list_objects(account_id, Some(&type_id), None, None, false, false)
                    .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
                let mut objects = objects;
                objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                let record = objects
                    .get(index)
                    .ok_or_else(|| PluginError::InvalidField(format!("索引越界: {}", field_id)))?;
                return Ok(record.name.clone());
            }
            if let Some((type_id, _)) = parse_type_property(field_id) {
                let objects = vault
                    .list_objects(account_id, Some(&type_id), None, None, false, false)
                    .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
                if objects.is_empty() {
                    return Ok(String::new());
                }
                let mut objects = objects;
                objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                return Ok(objects[0].name.clone());
            }
        }

        // Stage 4-B：typed lookup（当 manifest 声明了 contracts 时）
        if !self.contracts.is_empty() {
            return self.resolve_typed(field_id, vault, account_id);
        }

        // legacy_field_parse feature 已移除 — 插件必须声明 contracts
        Err(PluginError::InvalidField(
            "Legacy field parsing is disabled. Plugins must declare contracts for typed-lookup access.".into(),
        ))
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

        // __name__ 元数据
        if let Some((_, _, prop_path)) = parse_indexed_field(field_id) {
            if prop_path == "__name__" {
                return Ok(("名称".to_string(), "public".to_string()));
            }
        }
        if let Some((_, prop_path)) = parse_type_property(field_id) {
            if prop_path == "__name__" {
                return Ok(("名称".to_string(), "public".to_string()));
            }
        }

        // Stage 4-B：typed lookup
        if !self.contracts.is_empty() {
            return self.field_metadata_typed(field_id, vault, account_id);
        }

        // legacy_field_parse feature 已移除 — 插件必须声明 contracts
        Err(PluginError::InvalidField(
            "Legacy field parsing is disabled. Plugins must declare contracts for typed-lookup access.".into(),
        ))
    }

    // ── __name__ 路径辅助 ─────────────────────────────────────────────

    /// 判断 field_id 是否为 `__name__` 路径
    fn is_name_field(&self, field_id: &str) -> bool {
        if let Some((_, _, prop_path)) = parse_indexed_field(field_id) {
            return prop_path == "__name__";
        }
        if let Some((_, prop_path)) = parse_type_property(field_id) {
            return prop_path == "__name__";
        }
        false
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

    /// 列出指定类型的所有对象，返回 JSON 数组（Phase 5，替代 .count）
    ///
    /// 插件通过 SDK `list_objects()` 调用此方法。返回的 JSON 数组每个元素包含：
    /// - `id`: 对象 ID
    /// - `name`: 对象名称
    /// - `properties`: 对象属性 JSON 对象
    ///
    /// 插件在本地完成计数（`objects.len()`）和属性提取，不再需要 .count 字段。
    pub fn list_objects(&self, type_id: &str) -> Result<String, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;

        // typed-lookup：当插件声明 contracts 时，通过 contract_type_id/template_id 匹配对象
        if !self.contracts.is_empty() {
            if let Some((ctid, _)) = self.parse_typed_field(&format!("{}.dummy", type_id))? {
                let all = vault
                    .list_objects(account_id, None, None, None, false, false)
                    .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
                let mut objects: Vec<_> = all
                    .into_iter()
                    .filter(|o| {
                        // 通过 template_id（即 type_id 别名）匹配对象，避免跨模板污染。
                        // 多个模板可能共享同一 contract_type_id，因此不能仅用 contract_type_id 过滤。
                        // 对于无 template_id 的旧对象，回退到 contract_type_id 匹配。
                        o.template_id.as_deref() == Some(type_id)
                            || (o.template_id.is_none()
                                && o.contract_type_id.as_deref() == Some(&ctid))
                            || o.collection_type == type_id
                    })
                    .collect();
                objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                let items: Vec<serde_json::Value> = objects
                    .iter()
                    .map(|o| {
                        serde_json::json!({
                            "id": o.id,
                            "name": o.name,
                            "properties": o.properties,
                        })
                    })
                    .collect();
                return Ok(serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string()));
            }
        }

        // Legacy 路径：按 type_id 直接查询
        let objects = vault
            .list_objects(account_id, Some(type_id), None, None, false, false)
            .map_err(|e| PluginError::ExecutionFailed(format!("查询 Vault 失败: {}", e)))?;
        let items: Vec<serde_json::Value> = objects
            .iter()
            .map(|o| {
                serde_json::json!({
                    "id": o.id,
                    "name": o.name,
                    "properties": o.properties,
                })
            })
            .collect();
        Ok(serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string()))
    }

    /// 列出所有可水印的附件（图片/PDF），按页面 → 对象分组返回 JSON。
    pub fn list_attachments(&self) -> Result<String, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;

        let objects = vault
            .list_objects(account_id, None, None, None, false, false)
            .map_err(|e| PluginError::ExecutionFailed(format!("查询对象失败: {}", e)))?;

        let mut page_objects: Vec<solosoul_vault::ObjectSummary> = Vec::new();
        let mut section_groups: std::collections::BTreeMap<
            String,
            Vec<solosoul_vault::ObjectSummary>,
        > = std::collections::BTreeMap::new();

        for obj in &objects {
            if obj.collection_type == "page" {
                page_objects.push(obj.clone());
            } else {
                section_groups
                    .entry(obj.section_type.clone())
                    .or_default()
                    .push(obj.clone());
            }
        }

        let mut pages: Vec<AttPage> = Vec::new();
        let mut child_ids_assigned: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for page_obj in &page_objects {
            let children = vault
                .list_objects(account_id, None, Some(&page_obj.id), None, false, false)
                .unwrap_or_default();
            for child in &children {
                child_ids_assigned.insert(child.id.clone());
            }
            let objects = Self::collect_attachment_objects(vault, &children);
            if !objects.is_empty() {
                pages.push(AttPage {
                    page_id: Some(page_obj.id.clone()),
                    page_name: page_obj.name.clone(),
                    objects,
                });
            }
        }

        for (section, objs) in &section_groups {
            let filtered: Vec<_> = objs
                .iter()
                .filter(|o| !child_ids_assigned.contains(&o.id))
                .cloned()
                .collect();
            let objects = Self::collect_attachment_objects(vault, &filtered);
            if !objects.is_empty() {
                pages.push(AttPage {
                    page_id: None,
                    page_name: section.clone(),
                    objects,
                });
            }
        }

        let tree = serde_json::json!({ "pages": pages });
        Ok(tree.to_string())
    }

    fn collect_attachment_objects(
        vault: &Arc<VaultStore>,
        summaries: &[solosoul_vault::ObjectSummary],
    ) -> Vec<AttObject> {
        let ids: Vec<String> = summaries.iter().map(|s| s.id.clone()).collect();
        let records = vault.load_objects_batch(&ids).ok().unwrap_or_default();
        let template_cache: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
        summaries
            .iter()
            .filter_map(|summary| {
                let record = records.get(&summary.id)?;
                let template_name = record.template_id.as_ref().and_then(|tid| {
                    let mut cache = template_cache.borrow_mut();
                    cache.get(tid).cloned().unwrap_or_else(|| {
                        let name = vault
                            .load_user_template(tid)
                            .ok()
                            .flatten()
                            .map(|t| t.name.clone());
                        cache.insert(tid.clone(), name.clone());
                        name
                    })
                });
                let attachments: Vec<AttItem> = record
                    .properties
                    .get("__attachments")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| {
                        let id = v.get("id")?.as_str()?;
                        let file_name = v.get("fileName")?.as_str()?;
                        let mime_type = v.get("mimeType")?.as_str()?;
                        let size_bytes = v.get("sizeBytes")?.as_u64()?;
                        if !matches!(
                            mime_type,
                            "application/pdf"
                                | "image/png"
                                | "image/jpeg"
                                | "image/jpg"
                                | "image/webp"
                                | "image/gif"
                        ) {
                            return None;
                        }
                        Some(AttItem {
                            id: id.to_string(),
                            object_id: summary.id.clone(),
                            file_name: file_name.to_string(),
                            mime_type: mime_type.to_string(),
                            size_bytes,
                        })
                    })
                    .collect();
                if attachments.is_empty() {
                    None
                } else {
                    Some(AttObject {
                        object_id: summary.id.clone(),
                        object_name: summary.name.clone(),
                        template_name,
                        attachments,
                    })
                }
            })
            .collect()
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
    use solosoul_vault::{
        ContractRoleBinding, ObjectRecord, PropertyType, TemplateProperty, UserTemplate,
        VaultConfig,
    };
    use tempfile::TempDir;

    fn test_vault(account_id: &str) -> (TempDir, Arc<VaultStore>) {
        let tmp = TempDir::new().unwrap();
        let config =
            VaultConfig::new(account_id, tmp.path().to_path_buf()).with_data_key([0u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (tmp, Arc::new(vault))
    }

    #[test]
    #[ignore = "legacy field_parse feature removed — requires contracts"]
    fn test_field_metadata() {
        let account_id = "acc_test_meta";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            contract_type_id: None,
            id: "address".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![
                TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "street".to_string(),
                    name: "街道".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("private".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
                TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
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
            contract_type_id: None,
            id: "address".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![
                TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "street".to_string(),
                    name: "街道".to_string(),
                    prop_type: PropertyType::Text,
                    sensitivity_level: Some("internal".to_string()),
                    sensitive: None,
                    options: None,
                    deprecated_at: None,
                },
                TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
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
            contract_type_id: None,
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

    // ── Stage 4-B typed-lookup 单元测试 ─────────────────────────────────

    /// typed-lookup happy path：UserTemplate + ObjectRecord 都标 contract
    #[test]
    fn test_resolve_typed_happy_path() {
        let account_id = "acc_typed_happy";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![TemplateProperty {
                contract_field: Some(true),
                contract_bindings: None,
                id: "street".to_string(),
                name: "街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            }],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now),
        };
        vault.save_user_template(&template).unwrap();

        let record = ObjectRecord {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr_1".to_string(),
            account_id: account_id.to_string(),
            type_id: "addr".to_string(),
            section_type: "identity".to_string(),
            name: "家".to_string(),
            icon_name: "map-pin".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"street": "长安街1号"}),
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

        // 使用 SECONDARY alias 路径：alias "addr" → contract_type_id "com.solosoul.address/v1"
        let contracts = vec![PluginContractBinding {
            type_id: "com.solosoul.address/v1".to_string(),
            type_id_aliases: vec!["addr".to_string()],
            ..Default::default()
        }];
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            account_id.to_string(),
            vec!["addr.*".to_string()],
            contracts,
        );

        let result = resolver.resolve("addr.street").unwrap();
        assert_eq!(result, "长安街1号");
    }

    /// typed-lookup：用户未建契约模板 → InvalidField
    #[test]
    fn test_resolve_typed_missing_template() {
        let account_id = "acc_typed_missing_tpl";
        let (_tmp, vault) = test_vault(account_id);

        let contracts = vec![PluginContractBinding {
            type_id: "com.solosoul.address/v1".to_string(),
            type_id_aliases: vec!["address".to_string()],
            ..Default::default()
        }];
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            account_id.to_string(),
            vec![],
            contracts,
        );

        let result = resolver.resolve("address.street");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("contract_type_id"));
    }

    /// typed-lookup：property 上 contract_field != Some(true) → InvalidField（gate）
    #[test]
    fn test_resolve_typed_contract_field_false() {
        let account_id = "acc_typed_gate_false";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: None,
            properties: vec![TemplateProperty {
                contract_field: None, // 未标记为 contract_field
                contract_bindings: None,
                id: "street".to_string(),
                name: "街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            }],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now.clone()),
        };
        vault.save_user_template(&template).unwrap();

        let record = ObjectRecord {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr_1".to_string(),
            account_id: account_id.to_string(),
            type_id: "addr".to_string(),
            section_type: "identity".to_string(),
            name: "家".to_string(),
            icon_name: "map-pin".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"street": "长安街1号"}),
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

        let contracts = vec![PluginContractBinding {
            type_id: "com.solosoul.address/v1".to_string(),
            type_id_aliases: vec!["addr".to_string()],
            ..Default::default()
        }];
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            account_id.to_string(),
            vec![],
            contracts,
        );

        let result = resolver.resolve("addr.street");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("没有角色") || err.contains("gate") || err.contains("contract_field"),
            "Expected role binding rejection error, got: {}",
            err
        );
    }

    #[test]
    #[ignore = "legacy field_parse feature removed — requires contracts"]
    fn test_resolve_legacy_unchanged() {
        let account_id = "acc_legacy_unchanged";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            contract_type_id: None,
            id: "address".to_string(),
            account_id: account_id.to_string(),
            name: "地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "street".to_string(),
                name: "街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            }],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now),
        };
        vault.save_user_template(&template).unwrap();

        let record = ObjectRecord {
            contract_type_id: None,
            id: "addr_1".to_string(),
            account_id: account_id.to_string(),
            type_id: "address".to_string(),
            section_type: "identity".to_string(),
            name: "家".to_string(),
            icon_name: "map-pin".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"street": "长安街1号"}),
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

        // 不传 contracts → 走 legacy 路径
        let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);

        let result = resolver.resolve("address.street").unwrap();
        assert_eq!(result, "长安街1号");

        // list_objects 替代 .count
        let json = resolver.list_objects("address").unwrap();
        let items: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"].as_str().unwrap(), "家");
        assert_eq!(
            items[0]["properties"]["street"].as_str().unwrap(),
            "长安街1号"
        );
    }

    /// 新版 contract_bindings 解析：用户模板字段通过 contract_bindings 绑定到 plugin role
    #[test]
    fn test_resolve_typed_with_contract_bindings() {
        let account_id = "acc_bindings";
        let (_tmp, vault) = test_vault(account_id);

        let now = chrono::Utc::now().to_rfc3339();
        let template = UserTemplate {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr_tpl".to_string(),
            account_id: account_id.to_string(),
            name: "临时地址".to_string(),
            icon_id: Some("map-pin".to_string()),
            properties: vec![TemplateProperty {
                contract_field: None,
                contract_bindings: Some(vec![ContractRoleBinding {
                    contract_type_id: "com.solosoul.address/v1".to_string(),
                    role_id: "street".to_string(),
                }]),
                id: "specificAddress".to_string(),
                name: "具体地址".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            }],
            category: Some("identity".to_string()),
            created_at: now.clone(),
            updated_at: Some(now.clone()),
        };
        vault.save_user_template(&template).unwrap();

        let record = ObjectRecord {
            contract_type_id: Some("com.solosoul.address/v1".to_string()),
            id: "addr_1".to_string(),
            account_id: account_id.to_string(),
            type_id: "addr_tpl".to_string(),
            section_type: "identity".to_string(),
            name: "家".to_string(),
            icon_name: "map-pin".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"specificAddress": "123 Main St"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: now.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let contracts = vec![PluginContractBinding {
            type_id: "com.solosoul.address/v1".to_string(),
            type_id_aliases: vec!["addr_tpl".to_string()],
            ..Default::default()
        }];
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            account_id.to_string(),
            vec!["addr_tpl.*".to_string()],
            contracts,
        );

        // 插件请求 street role，通过 role binding 映射到 specificAddress 字段
        let result = resolver.resolve("addr_tpl.street").unwrap();
        assert_eq!(result, "123 Main St");
    }

    /// parse_typed_field SECONDARY alias 路径（无 vault 模式）
    #[test]
    fn test_parse_typed_field_secondary_alias() {
        let contracts = vec![PluginContractBinding {
            type_id: "com.solosoul.address/v1".to_string(),
            type_id_aliases: vec!["address".to_string()],
            ..Default::default()
        }];
        let resolver = FieldResolver::with_contracts(contracts);

        let res = resolver.parse_typed_field("address.street").unwrap();
        assert_eq!(
            res,
            Some(("com.solosoul.address/v1".to_string(), "street".to_string()))
        );

        // miss
        let res = resolver.parse_typed_field("unknown.field").unwrap();
        assert_eq!(res, None);
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
