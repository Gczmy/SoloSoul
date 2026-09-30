//! 对象创建的共享记录构建与模板投影。创建严格读错返回；旧辅助保留 best-effort 契约。

use serde_json::{Map, Value};
use solosoul_vault::{ObjectRecord, PropertyType, UserTemplate, VaultStore};

use super::{template_fingerprint, validate_dynamic_groups};

/// GUI/CLI 在边界决定 ID、类型、页面和显示默认值；构建器不推断宿主策略。
#[derive(Debug, Clone)]
pub struct CreateRecordInput {
    pub id: String,
    pub type_id: String,
    pub section_type: String,
    pub name: String,
    pub icon_name: String,
    pub parent_id: Option<String>,
    pub properties: Value,
    pub template_id: Option<String>,
    pub template_type: Option<String>,
}

/// 一次读取模板后构建完整记录，不保存对象或修改父页面。
/// 真正缺失模板保持兼容，读取失败原样返回，不能丢失字段语义后继续创建。
pub fn build_create_record(
    vault: &VaultStore,
    account_id: &str,
    input: CreateRecordInput,
    now: &str,
) -> Result<ObjectRecord, String> {
    let template = input
        .template_id
        .as_deref()
        .map(|id| vault.load_user_template(id))
        .transpose()?
        .flatten();
    let mut properties = input.properties;
    let mut property_labels = None;
    let mut contract_type_id = None;
    let mut template_hash = None;
    if let Some(template) = template {
        let (labels, fields) = project_template_properties(&template);
        property_labels = labels;
        contract_type_id = template.contract_type_id.clone();
        let hash = template_fingerprint(&template);
        inject_property_fields(&mut properties, &fields);
        inject_template_name(&mut properties, &template.name);
        if let Some(values) = properties.as_object_mut() {
            values.insert("__templateHash".into(), Value::String(hash.clone()));
        }
        template_hash = Some(hash);
        // 仅真实模板分支校验新继承的约束；无/缺模板不收紧已有自定义 properties。
        validate_dynamic_groups(&properties)?;
    }
    Ok(ObjectRecord {
        id: input.id,
        account_id: account_id.to_string(),
        type_id: input.type_id,
        section_type: input.section_type,
        name: input.name,
        icon_name: input.icon_name,
        parent_id: input.parent_id,
        children_ids: vec![],
        properties,
        property_labels,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: input.template_id,
        template_type: input.template_type,
        contract_type_id,
        template_hash,
        ignored_template_hash: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        version: 1,
    })
}

/// 字段定义与敏感度副本的唯一投影，严格创建和兼容 getter 共用。
pub fn project_template_properties(template: &UserTemplate) -> (Option<Value>, Value) {
    let mut labels = Map::new();
    let mut fields = Map::new();
    for property in &template.properties {
        if let Some(level) = &property.sensitivity_level {
            labels.insert(property.id.clone(), Value::String(level.clone()));
        }
        let mut field = Map::new();
        field.insert("name".into(), Value::String(property.name.clone()));
        field.insert(
            "type".into(),
            Value::String(property.prop_type.as_str().into()),
        );
        if let Some(options) = &property.options {
            field.insert("options".into(), serde_json::json!(options));
        }
        if let Some(deprecated_at) = &property.deprecated_at {
            field.insert("deprecatedAt".into(), Value::String(deprecated_at.clone()));
        }
        if let Some(contract_field) = property.contract_field {
            field.insert("contractField".into(), Value::Bool(contract_field));
        }
        if property.prop_type == PropertyType::DynamicGroup {
            if let Some(allowed) = &property.allowed_types {
                field.insert(
                    "allowedTypes".into(),
                    serde_json::json!(allowed.iter().map(PropertyType::as_str).collect::<Vec<_>>()),
                );
            }
            if let Some(max_items) = property.max_items {
                field.insert("maxItems".into(), serde_json::json!(max_items));
            }
        }
        fields.insert(property.id.clone(), Value::Object(field));
    }
    let labels = if labels.is_empty() {
        None
    } else {
        Some(Value::Object(labels))
    };
    let fields = if fields.is_empty() {
        Value::Null
    } else {
        Value::Object(fields)
    };
    (labels, fields)
}

/// 恢复等既有路径仅补 contract，不重新注入字段、标签或模板指纹。
pub fn inherit_contract_type_id(vault: &VaultStore, template_id: Option<&str>) -> Option<String> {
    template_id.and_then(|id| {
        vault
            .load_user_template(id)
            .ok()
            .flatten()
            .and_then(|template| template.contract_type_id)
    })
}

fn inherit_template_properties(
    vault: &VaultStore,
    template_id: Option<&str>,
) -> (Option<Value>, Value) {
    let template = template_id.and_then(|id| vault.load_user_template(id).ok().flatten());
    template
        .as_ref()
        .map(project_template_properties)
        .unwrap_or((None, Value::Null))
}

/// 兼容读取辅助：缺失或读错返回 None；严格创建不使用此吞错入口。
pub fn inherit_property_labels(vault: &VaultStore, template_id: Option<&str>) -> Option<Value> {
    inherit_template_properties(vault, template_id).0
}

/// 兼容读取辅助：缺失、读错或空模板返回 null。
pub fn inherit_property_fields(vault: &VaultStore, template_id: Option<&str>) -> Value {
    inherit_template_properties(vault, template_id).1
}

/// 纯注入：只跳过 null，保留旧调用方对空对象或其他 JSON 值的既有行为。
pub fn inject_property_fields(properties: &mut Value, fields: &Value) {
    if !fields.is_null() {
        if let Some(values) = properties.as_object_mut() {
            values.insert("__fields".into(), fields.clone());
        }
    }
}

fn inject_template_name(properties: &mut Value, name: &str) {
    if let Some(values) = properties.as_object_mut() {
        values.insert("__templateName".into(), Value::String(name.to_string()));
    }
}

/// 兼容元信息辅助只写模板名称，不隐式写 hash 或重新投影字段。
pub fn inject_template_meta(vault: &VaultStore, template_id: Option<&str>, properties: &mut Value) {
    if let Some(template) = template_id.and_then(|id| vault.load_user_template(id).ok().flatten()) {
        inject_template_name(properties, &template.name);
    }
}

#[cfg(test)]
mod tests;
