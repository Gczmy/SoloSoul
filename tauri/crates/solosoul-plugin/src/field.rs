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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

/// P004: 单次插件运行内的惰性缓存——`list_user_templates`/`list_objects` 均为
/// 全表 AES 解密，K 个字段 × N 对象会放大为 K×N 次解密。缓存在 `FieldResolver`
/// 生命周期（= 单次插件运行）内复用结果，同一字段的重复查询不再触库。
/// `Arc<Mutex<..>>` 使 `FieldResolver` 保持 Send + Sync（被 Arc 包裹跨线程共享）。
#[derive(Default)]
struct FieldCache {
    templates: Option<Vec<UserTemplate>>,
    all_objects: Option<Vec<solosoul_vault::ObjectSummary>>,
    objects_by_type: HashMap<String, Vec<solosoul_vault::ObjectSummary>>,
}

/// 字段解析器
#[derive(Clone, Default)]
pub struct FieldResolver {
    vault: Option<Arc<VaultStore>>,
    account_id: Option<String>,
    allowed_patterns: Vec<String>,
    /// Stage 4 typed-lookup 契约绑定锚点（由 PluginManager::run 在构造时填充）
    contracts: Vec<PluginContractBinding>,
    /// P001: 附件静态加密密钥（`Some` 时插件复制附件到工作区前解密；
    /// `None` 保持旧明文行为，兼容无会话密钥的测试/未解锁场景）。
    attachment_key: Option<[u8; 32]>,
    /// P004: 惰性缓存（templates / 全量对象 / 按 type_id 的对象）
    cache: Arc<Mutex<FieldCache>>,
}

impl FieldResolver {
    /// 创建空解析器（测试或 Vault 未解锁时使用）
    pub fn new() -> Self {
        Self::default()
    }

    /// P004: 缓存读写助手——模板列表（全表解密一次）
    fn cached_templates(&self) -> Result<Vec<UserTemplate>, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PluginError::ExecutionFailed("字段缓存锁已损坏".to_string()))?;
        if cache.templates.is_none() {
            cache.templates = Some(
                vault
                    .list_user_templates(account_id)
                    .map_err(|e| PluginError::ExecutionFailed(format!("读取模板失败: {}", e)))?,
            );
        }
        Ok(cache.templates.clone().unwrap_or_default())
    }

    /// P004: 缓存读写助手——全量对象列表（全表解密一次）
    fn cached_all_objects(&self) -> Result<Vec<solosoul_vault::ObjectSummary>, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PluginError::ExecutionFailed("字段缓存锁已损坏".to_string()))?;
        if cache.all_objects.is_none() {
            cache.all_objects = Some(
                vault
                    .list_objects(account_id, None, None, None, false, false)
                    .map_err(|e| PluginError::ExecutionFailed(format!("查询对象失败: {}", e)))?,
            );
        }
        Ok(cache.all_objects.clone().unwrap_or_default())
    }

    /// P004: 缓存读写助手——按 type_id 的对象列表（全表解密一次，之后内存过滤）
    fn cached_objects_by_type(
        &self,
        type_id: &str,
    ) -> Result<Vec<solosoul_vault::ObjectSummary>, PluginError> {
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let account_id = self
            .account_id
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("未选择账户".to_string()))?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PluginError::ExecutionFailed("字段缓存锁已损坏".to_string()))?;
        if !cache.objects_by_type.contains_key(type_id) {
            let objects = vault
                .list_objects(account_id, Some(type_id), None, None, false, false)
                .map_err(|e| PluginError::ExecutionFailed(format!("查询对象失败: {}", e)))?;
            cache.objects_by_type.insert(type_id.to_string(), objects);
        }
        Ok(cache.objects_by_type[type_id].clone())
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
            attachment_key: None,
            cache: Arc::new(Mutex::new(FieldCache::default())),
        }
    }

    /// P001: 设置附件静态加密密钥（插件复制附件到工作区前解密）。
    pub fn with_attachment_key(mut self, key: [u8; 32]) -> Self {
        self.attachment_key = Some(key);
        self
    }

    /// P001: 获取附件静态加密密钥。
    pub(crate) fn attachment_key_ref(&self) -> Option<&[u8; 32]> {
        self.attachment_key.as_ref()
    }

    /// Stage 4-B：minimal-injection（仅 contracts；vault 由后续 with_vault 注入）
    pub fn with_contracts(contracts: Vec<PluginContractBinding>) -> Self {
        Self {
            vault: None,
            account_id: None,
            allowed_patterns: Vec::new(),
            contracts,
            attachment_key: None,
            cache: Arc::new(Mutex::new(FieldCache::default())),
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
            attachment_key: None,
            cache: Arc::new(Mutex::new(FieldCache::default())),
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
        if self.vault.is_some() && self.account_id.is_some() {
            let templates = self.cached_templates()?;
            if let Some(t) = templates
                .iter()
                .find(|t| t.id == alias && t.contract_type_id.is_some())
            {
                if let Some(ctid) = t.contract_type_id.clone() {
                    return Ok(Some((ctid, prop_path)));
                }
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
        template.properties.iter().find(|p| {
            template.contract_type_id.as_deref() == Some(ctid)
                && p.id == role_id
                && p.contract_field == Some(true)
        })
    }

    /// Typed-lookup 获取字段元数据（Stage 4-B）
    fn field_metadata_typed(&self, field_id: &str) -> Result<(String, String), PluginError> {
        let (alias, _, property) = parse_indexed_field(field_id)
            .or_else(|| parse_type_property(field_id).map(|(a, p)| (a, 0, p)))
            .ok_or_else(|| PluginError::InvalidField(format!("不支持的字段路径: {field_id}")))?;
        let role = property.split('.').next().unwrap_or_default();
        let templates = self.cached_templates()?;
        let exact = templates.iter().find(|t| t.id == alias);
        let mut metadata = Vec::new();
        for template in &templates {
            let selected = exact.map_or_else(
                || {
                    self.contracts.iter().any(|c| {
                        c.type_id_aliases.contains(&alias)
                            && Self::template_supports_contract(template, &c.type_id)
                    })
                },
                |t| t.id == template.id,
            );
            if !selected {
                continue;
            }
            let (fields, _) = self.template_projection(template, &alias);
            for (output, source, whole_role) in fields {
                if output != role
                    || (!whole_role && !self.explicitly_allowed(&format!("{alias}.{property}")))
                {
                    continue;
                }
                if let Some(p) = template.properties.iter().find(|p| p.id == source) {
                    metadata.push((
                        p.name.clone(),
                        p.sensitivity_level
                            .clone()
                            .unwrap_or_else(|| "internal".into()),
                    ));
                }
            }
        }
        // 一个契约别名包含多个用户模板时，授权提示采用其中最严格的敏感度。
        metadata
            .into_iter()
            .max_by_key(|(_, level)| match level.as_str() {
                "public" => 0,
                "internal" => 1,
                "sensitive" => 2,
                _ => 3,
            })
            .ok_or_else(|| PluginError::InvalidField(format!("字段未声明或未绑定: {field_id}")))
    }

    // ── 公共 API ────────────────────────────────────────────────────────

    /// 解析字段值
    pub fn resolve(&self, field_id: &str) -> Result<String, PluginError> {
        normalize_for_permission(field_id)
            .ok_or_else(|| PluginError::InvalidField(format!("非法字段路径: {field_id}")))?;
        let (alias, index, property) = parse_indexed_field(field_id)
            .or_else(|| parse_type_property(field_id).map(|(a, p)| (a, 0, p)))
            .ok_or_else(|| PluginError::InvalidField(format!("不支持的字段路径: {field_id}")))?;
        if self.contracts.is_empty() && property != "__name__" {
            return Err(PluginError::InvalidField("Legacy field parsing is disabled. Plugins must declare contracts for typed-lookup access.".into()));
        }
        // 单字段与批量接口共用投影，空声明、其他契约及未绑定字段均不能绕过。
        let objects: Vec<serde_json::Value> = serde_json::from_str(&self.list_objects(&alias)?)?;
        let Some(object) = objects.get(index) else {
            return Ok(String::new());
        };
        if property == "__name__" {
            return Ok(object["name"].as_str().unwrap_or_default().to_string());
        }
        let role = property.split('.').next().unwrap_or_default();
        if object["properties"].get(role).is_none() {
            return Err(PluginError::InvalidField(format!(
                "字段未声明或未绑定: {field_id}"
            )));
        }
        Ok(extract_property(&object["properties"], &property))
    }

    /// 获取字段元数据（字段标签与敏感度等级）
    ///
    /// 支持路径：
    /// - `<typeId>[<index>].<prop>`
    /// - `<typeId>.<prop>`（默认取第一个对象）
    /// - 嵌套属性取第一级属性名匹配
    pub fn field_metadata(&self, field_id: &str) -> Result<(String, String), PluginError> {
        // 注：Vault/账户解锁状态由缓存助手（cached_*）在首次查询时校验。
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
            return self.field_metadata_typed(field_id);
        }

        // legacy_field_parse feature 已移除 — 插件必须声明 contracts
        Err(PluginError::InvalidField(
            "Legacy field parsing is disabled. Plugins must declare contracts for typed-lookup access.".into(),
        ))
    }

    /// 构建用户数据结构树（仅元数据，不含字段值）
    pub fn build_structure_tree(&self) -> Result<String, PluginError> {
        // 注：Vault/账户解锁状态由缓存助手（cached_*）在首次查询时校验。
        let templates = self.cached_templates()?;

        let types: Vec<serde_json::Value> = templates
            .into_iter()
            .filter_map(|tpl| {
                let (projection, name_allowed) = self.template_projection(&tpl, &tpl.id);
                if !self.contracts.is_empty() && projection.is_empty() && !name_allowed {
                    return None;
                }
                let count = self
                    .cached_objects_by_type(&tpl.id)
                    .map(|list| list.len())
                    .unwrap_or(0);

                let properties: Vec<serde_json::Value> =
                    if self.contracts.is_empty() {
                        tpl.properties.iter().map(|p| serde_json::json!({
                        "id": p.id, "name": p.name, "type": p.prop_type,
                        "sensitivity": p.sensitivity_level.as_deref().unwrap_or("internal"),
                    })).collect()
                    } else {
                        projection.iter().filter_map(|(role, source, _)| {
                        let p = tpl.properties.iter().find(|p| &p.id == source)?;
                        Some(serde_json::json!({"id": role, "name": p.name, "type": p.prop_type,
                            "sensitivity": p.sensitivity_level.as_deref().unwrap_or("internal")}))
                    }).collect()
                    };

                Some(serde_json::json!({
                    "id": tpl.id,
                    "name": tpl.name,
                    "category": tpl.category.unwrap_or_default(),
                    "count": count,
                    "properties": properties
                }))
            })
            .collect();

        let tree = serde_json::json!({ "types": types });
        Ok(tree.to_string())
    }

    /// 列出指定类型的所有对象，返回 JSON 数组（Phase 5，替代 .count）
    ///
    /// 插件通过 SDK `list_objects()` 调用此方法。返回的 JSON 数组每个元素包含：
    /// - `id`: 对象 ID
    /// - `name`: 获准读取时为对象名称，否则为空
    /// - `properties`: 已声明且由用户模板绑定的属性；typed 模式以角色名为键
    ///
    /// 插件在本地完成计数（`objects.len()`）和属性提取，不再需要 .count 字段。
    pub fn list_objects(&self, type_id: &str) -> Result<String, PluginError> {
        let templates = self.cached_templates()?;
        if self.contracts.is_empty() {
            if !self.declares_type(type_id) {
                return Err(PluginError::InvalidField(format!(
                    "类型未在 manifest 中声明: {type_id}"
                )));
            }
            let mut objects = self.cached_objects_by_type(type_id)?;
            objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            let items: Vec<_> = objects.iter().map(|o| serde_json::json!({
                "id": o.id,
                "name": if self.explicitly_allowed(&format!("{type_id}.__name__")) { &o.name } else { "" },
                "properties": project_properties(&o.properties, type_id, &|path| self.explicitly_allowed(path)).unwrap_or_else(|| serde_json::json!({})),
            })).collect();
            return Ok(serde_json::Value::Array(items).to_string());
        }

        // 真实模板 ID 优先；仅 manifest 中的契约别名可选择多个已绑定模板。
        let exact = templates.iter().find(|t| t.id == type_id);
        let candidates: Vec<_> = templates
            .iter()
            .filter(|t| {
                if let Some(exact) = exact {
                    return t.id == exact.id;
                }
                self.contracts.iter().any(|c| {
                    (c.type_id == type_id || c.type_id_aliases.iter().any(|a| a == type_id))
                        && Self::template_supports_contract(t, &c.type_id)
                })
            })
            .collect();
        let mut access = Vec::new();
        for template in candidates {
            let (fields, name_allowed) = self.template_projection(template, type_id);
            if !fields.is_empty() || name_allowed {
                access.push((template, fields, name_allowed));
            }
        }
        if access.is_empty() {
            return Err(PluginError::InvalidField(format!(
                "类型没有已声明且绑定的字段: {type_id}"
            )));
        }
        let mut objects = self.cached_all_objects()?;
        objects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let mut items = Vec::new();
        for object in objects {
            let matches: Vec<_> = access
                .iter()
                .filter(|(t, _, _)| match &object.template_id {
                    Some(id) => id == &t.id,
                    None => {
                        object.collection_type == t.id
                            || (object.contract_type_id.is_some()
                                && object.contract_type_id == t.contract_type_id)
                    }
                })
                .collect();
            // 旧对象无法唯一归属模板时不猜测其绑定，避免共享契约串读。
            if matches.len() != 1 {
                continue;
            }
            let (_, fields, name_allowed) = matches[0];
            let mut projected = serde_json::Map::new();
            for (role, property, whole_role) in fields {
                if let Some(value) = object.properties.get(property) {
                    let path = format!("{type_id}.{role}");
                    let value = if *whole_role {
                        Some(value.clone())
                    } else {
                        project_properties(value, &path, &|p| self.explicitly_allowed(p))
                    };
                    if let Some(value) = value {
                        projected.insert(role.clone(), value);
                    }
                }
            }
            items.push(serde_json::json!({
                "id": object.id,
                "name": if *name_allowed { object.name } else { String::new() },
                "properties": projected,
            }));
        }
        Ok(serde_json::Value::Array(items).to_string())
    }

    fn explicitly_allowed(&self, path: &str) -> bool {
        self.allowed_patterns
            .iter()
            .any(|p| pattern_matches(p, path))
    }

    fn declares_type(&self, alias: &str) -> bool {
        self.allowed_patterns
            .iter()
            .any(|p| p == "*" || p.starts_with("*.") || p.starts_with(&format!("{alias}.")))
    }

    fn template_supports_contract(template: &UserTemplate, ctid: &str) -> bool {
        template.contract_type_id.as_deref() == Some(ctid)
            || template.properties.iter().any(|p| {
                p.contract_bindings
                    .as_ref()
                    .is_some_and(|bs| bs.iter().any(|b| b.contract_type_id == ctid))
            })
    }

    /// 返回 (角色输出名, 用户绑定字段, 是否声明整个角色)，名称单独授权。
    fn template_projection(
        &self,
        template: &UserTemplate,
        alias: &str,
    ) -> (Vec<(String, String, bool)>, bool) {
        let mut fields = Vec::new();
        let mut name_allowed = false;
        for contract in &self.contracts {
            if !Self::template_supports_contract(template, &contract.type_id) {
                continue;
            }
            let mut roles: Vec<String> = contract.roles.iter().map(|r| r.role_id.clone()).collect();
            for prop in &template.properties {
                if template.contract_type_id.as_deref() == Some(&contract.type_id)
                    && prop.contract_field == Some(true)
                {
                    roles.push(prop.id.clone());
                }
                if let Some(bindings) = &prop.contract_bindings {
                    roles.extend(
                        bindings
                            .iter()
                            .filter(|b| b.contract_type_id == contract.type_id)
                            .map(|b| b.role_id.clone()),
                    );
                }
            }
            roles.sort();
            roles.dedup();
            for role in roles {
                let declaration = contract.roles.iter().find(|r| r.role_id == role);
                let property = Self::find_property_for_role(template, &contract.type_id, &role)
                    .or_else(|| {
                        let id = declaration?.default_property_id.as_deref()?;
                        template.properties.iter().find(|p| {
                            p.id == id
                                && p.contract_field == Some(true)
                                && template.contract_type_id.as_deref() == Some(&contract.type_id)
                        })
                    });
                if let Some(property) = property {
                    if declaration.is_some() || self.declares_type(alias) {
                        fields.push((role, property.id.clone(), declaration.is_some()));
                    }
                } else if declaration
                    .is_some_and(|r| r.default_property_id.as_deref() == Some("__name__"))
                {
                    name_allowed = true;
                }
            }
            name_allowed |= self.explicitly_allowed(&format!("{alias}.__name__"));
        }
        (fields, name_allowed)
    }

    /// 列出所有可水印的附件（图片/PDF），按页面 → 对象分组返回 JSON。
    pub fn list_attachments(&self) -> Result<String, PluginError> {
        // 注：Vault/账户解锁状态由缓存助手（cached_*）在首次查询时校验。
        let vault = self
            .vault
            .as_ref()
            .ok_or(PluginError::ExecutionFailed("Vault 未解锁".to_string()))?;
        let objects = self.cached_all_objects()?;

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
            // P004: 从缓存的全量对象中按 parent_id 过滤（避免逐页二次全表解密）
            let children: Vec<_> = objects
                .iter()
                .filter(|o| o.parent_id.as_deref() == Some(page_obj.id.as_str()))
                .cloned()
                .collect();
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
                        let name = vault.load_user_template(tid).ok().flatten().map(|t| t.name);
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

/// 精确声明父字段允许其完整值；只声明子字段时递归投影，绝不带出同级属性。
fn project_properties(
    value: &serde_json::Value,
    path: &str,
    allowed: &impl Fn(&str) -> bool,
) -> Option<serde_json::Value> {
    if allowed(path) {
        return Some(value.clone());
    }
    match value {
        serde_json::Value::Object(map) => {
            let filtered: serde_json::Map<_, _> = map
                .iter()
                .filter_map(|(key, value)| {
                    project_properties(value, &format!("{path}.{key}"), allowed)
                        .map(|v| (key.clone(), v))
                })
                .collect();
            if filtered.is_empty() {
                None
            } else {
                Some(filtered.into())
            }
        }
        serde_json::Value::Array(items) => {
            let projected: Vec<_> = items
                .iter()
                .map(|v| project_properties(v, path, allowed))
                .collect();
            if projected.iter().all(Option::is_none) {
                None
            } else {
                Some(
                    projected
                        .into_iter()
                        .map(|v| v.unwrap_or(serde_json::Value::Null))
                        .collect(),
                )
            }
        }
        _ => None,
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
                    allowed_types: None,
                    max_items: None,
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
                    allowed_types: None,
                    max_items: None,
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
                    allowed_types: None,
                    max_items: None,
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
                    allowed_types: None,
                    max_items: None,
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
            template_hash: None,
            ignored_template_hash: None,
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

    fn projection_fixture() -> (tempfile::TempDir, Arc<VaultStore>, PluginContractBinding) {
        let (dir, vault) = test_vault("acc_projection");
        for id in ["address", "other"] {
            let template: UserTemplate = serde_json::from_value(serde_json::json!({
                "id": id, "accountId": "acc_projection", "name": id, "createdAt": "2026-09-17",
                "contractTypeId": "address/v1",
                "properties": [
                    {"id": "customStreet", "name": "Street", "type": "text", "contractBindings": [{"contractTypeId": "address/v1", "roleId": "street"}]},
                    {"id": "secret", "name": "Secret", "type": "text", "contractField": true},
                    {"id": "nested", "name": "Nested", "type": "text", "contractField": true},
                    {"id": "unbound", "name": "Unbound", "type": "text"}
                ]
            })).unwrap();
            vault.save_user_template(&template).unwrap();
            vault.save_object(&ObjectRecord {
                id: id.into(), account_id: "acc_projection".into(), type_id: id.into(),
                template_id: Some(id.into()), contract_type_id: Some("address/v1".into()), name: "private-name".into(),
                properties: serde_json::json!({"customStreet": "allowed-street", "secret": "private-secret", "unbound": "private-unbound",
                    "nested": {"city": "allowed-city", "secret": "private-nested"}}),
                ..Default::default()
            }).unwrap();
        }
        (
            dir,
            vault,
            PluginContractBinding {
                type_id: "address/v1".into(),
                type_id_aliases: vec!["address".into()],
                ..Default::default()
            },
        )
    }

    #[test]
    fn list_objects_projects_fields_and_rejects_other_templates_or_empty_permissions() {
        let (_dir, vault, contract) = projection_fixture();
        let resolver = FieldResolver::with_vault_and_contracts(
            vault.clone(),
            "acc_projection".into(),
            vec!["address.street".into()],
            vec![contract.clone()],
        );
        let items: serde_json::Value =
            serde_json::from_str(&resolver.list_objects("address").unwrap()).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(
            items[0]["properties"],
            serde_json::json!({"street": "allowed-street"})
        );
        assert_eq!(items[0]["name"], "");
        assert!(!items.to_string().contains("private-"));
        assert!(resolver.list_objects("other").is_err());
        assert!(resolver.resolve("address.secret").is_err());
        assert_eq!(
            resolver.resolve("address.street").unwrap(),
            "allowed-street"
        );
        for contracts in [vec![], vec![contract]] {
            let denied = FieldResolver::with_vault_and_contracts(
                vault.clone(),
                "acc_projection".into(),
                vec![],
                contracts,
            );
            assert!(denied.list_objects("address").is_err());
        }
    }

    #[test]
    fn list_objects_limits_nested_fields_and_typed_wildcards_to_user_bindings() {
        let (_dir, vault, contract) = projection_fixture();
        let resolver = FieldResolver::with_vault_and_contracts(
            vault.clone(),
            "acc_projection".into(),
            vec!["address.nested.city".into()],
            vec![contract.clone()],
        );
        let items: serde_json::Value =
            serde_json::from_str(&resolver.list_objects("address").unwrap()).unwrap();
        assert_eq!(
            items[0]["properties"],
            serde_json::json!({"nested": {"city": "allowed-city"}})
        );
        let wildcard = FieldResolver::with_vault_and_contracts(
            vault.clone(),
            "acc_projection".into(),
            vec!["address.*".into()],
            vec![contract],
        );
        let items: serde_json::Value =
            serde_json::from_str(&wildcard.list_objects("address").unwrap()).unwrap();
        assert!(items[0]["properties"].get("secret").is_some()); // 明确声明通配且已绑定
        assert!(items[0]["properties"].get("unbound").is_none());
        assert_eq!(items[0]["name"], "private-name");
        let legacy = FieldResolver::with_vault(
            vault,
            "acc_projection".into(),
            vec!["address.nested.city".into()],
        );
        let items: serde_json::Value =
            serde_json::from_str(&legacy.list_objects("address").unwrap()).unwrap();
        assert_eq!(
            items[0]["properties"],
            serde_json::json!({"nested": {"city": "allowed-city"}})
        );
        assert!(legacy.list_objects("other").is_err());
    }

    #[test]
    fn roles_only_plugin_uses_custom_bindings_and_role_metadata() {
        let (_dir, vault, mut contract) = projection_fixture();
        contract.roles = vec![
            super::super::manifest::PluginContractRole {
                role_id: "street".into(),
                ..Default::default()
            },
            super::super::manifest::PluginContractRole {
                role_id: "document".into(),
                default_property_id: Some("__name__".into()),
                ..Default::default()
            },
        ];
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            "acc_projection".into(),
            vec![],
            vec![contract],
        );
        let items: serde_json::Value =
            serde_json::from_str(&resolver.list_objects("address").unwrap()).unwrap();
        assert_eq!(
            items[0]["properties"],
            serde_json::json!({"street": "allowed-street"})
        );
        assert_eq!(items[0]["name"], "private-name");
        let tree: serde_json::Value =
            serde_json::from_str(&resolver.build_structure_tree().unwrap()).unwrap();
        assert_eq!(tree["types"][0]["properties"][0]["id"], "street");
        assert!(resolver.resolve("address.secret").is_err());
    }

    #[test]
    fn nested_projection_preserves_array_shape_without_sibling_values() {
        let input =
            serde_json::json!({"contacts": [{"email": "a", "secret": "b"}, {"secret": "c"}]});
        let actual = project_properties(&input, "address", &|p| {
            pattern_matches("address.contacts.email", p)
        })
        .unwrap();
        assert_eq!(
            actual,
            serde_json::json!({"contacts": [{"email": "a"}, null]})
        );
    }

    #[test]
    fn expiry_roles_work_with_a_field_binding_on_another_template_contract() {
        let (_dir, vault, _) = projection_fixture();
        let mut template = vault.load_user_template("address").unwrap().unwrap();
        template.contract_type_id = Some("identity/v1".into());
        template.properties[0].contract_bindings = Some(vec![ContractRoleBinding {
            contract_type_id: "com.solosoul.expiry/guardian/v1".into(),
            role_id: "expiryDate".into(),
        }]);
        vault.save_user_template(&template).unwrap();
        let contract = serde_json::from_value(serde_json::json!({
            "typeId": "com.solosoul.expiry/guardian/v1", "strictContractGate": true,
            "roles": [
                {"roleId": "expiryDate", "defaultPropertyId": "expiryDate"},
                {"roleId": "document", "defaultPropertyId": "__name__"}
            ]
        }))
        .unwrap();
        let resolver = FieldResolver::with_vault_and_contracts(
            vault,
            "acc_projection".into(),
            vec![],
            vec![contract],
        );
        let items: serde_json::Value =
            serde_json::from_str(&resolver.list_objects("address").unwrap()).unwrap();
        assert_eq!(
            items[0]["properties"],
            serde_json::json!({"expiryDate": "allowed-street"})
        );
        assert_eq!(
            resolver.resolve("address.expiryDate").unwrap(),
            "allowed-street"
        );
        assert_eq!(
            resolver.field_metadata("address.expiryDate").unwrap(),
            ("Street".into(), "internal".into())
        );
        let tree: serde_json::Value =
            serde_json::from_str(&resolver.build_structure_tree().unwrap()).unwrap();
        assert_eq!(tree["types"].as_array().unwrap().len(), 1);
        assert_eq!(tree["types"][0]["properties"][0]["id"], "expiryDate");
        assert!(resolver.list_objects("other").is_err());
    }

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
                allowed_types: None,
                max_items: None,
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
            template_hash: None,
            ignored_template_hash: None,
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
        assert!(err.contains("没有已声明且绑定的字段"));
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
                allowed_types: None,
                max_items: None,
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
            template_hash: None,
            ignored_template_hash: None,
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
            err.contains("没有已声明且绑定的字段"),
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
                allowed_types: None,
                max_items: None,
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
            template_hash: None,
            ignored_template_hash: None,
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
                allowed_types: None,
                max_items: None,
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
            created_at: now,
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            template_hash: None,
            ignored_template_hash: None,
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
