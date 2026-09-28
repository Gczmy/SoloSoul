#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectData {
    pub id: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub name: String,
    // P042: IPC 载荷统一为 typeId（与 ObjectRecord 同步载荷一致）
    #[serde(rename = "typeId")]
    pub collection_type: String,
    pub properties: serde_json::Value,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    #[serde(rename = "propertyLabels")]
    pub property_labels: Option<serde_json::Value>,
    // P006: ObjectData 补齐 tags——此前 TS 声明 tags? 永为 undefined，
    // updateObject 用 undefined 覆盖摘要 tags，详情页标签成死渲染路径。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<String>,
    #[serde(rename = "contractTypeId")]
    pub contract_type_id: Option<String>,
    #[serde(rename = "templateHash")]
    pub template_hash: Option<String>,
    #[serde(rename = "ignoredTemplateHash")]
    pub ignored_template_hash: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonMaps {
    pub payload: serde_json::Value,
    pub counts: std::collections::HashMap<String, usize>,
    pub nested: std::collections::HashMap<String, Vec<Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defaulted_items: Vec<String>,
}
