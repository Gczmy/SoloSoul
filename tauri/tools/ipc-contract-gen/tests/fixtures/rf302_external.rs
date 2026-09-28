#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    // P042: IPC 载荷字段名统一为 typeId（与 ObjectRecord 同步载荷一致，前端一套词汇）
    #[serde(rename = "typeId")]
    pub collection_type: String,
    #[serde(rename = "sectionType")]
    pub section_type: String,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    /// 插件合约类型 ID（用于 plugin-template 兼容）。旧记录缺失时由 `default` 填充为 `None`。
    #[serde(
        rename = "contractTypeId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_type_id: Option<String>,
    #[serde(
        rename = "templateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub template_hash: Option<String>,
    #[serde(
        rename = "ignoredTemplateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ignored_template_hash: Option<String>,
    #[serde(rename = "iconName")]
    pub icon_name: String,
    /// 所属父对象 ID（自定义页面子对象），无父级时为 None（P112 附件树按 parent 分组用）。
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// First few property key-value pairs for card previews
    pub properties: serde_json::Value,
    /// Per-field sensitivity overrides: field_name -> sensitivity_level
    #[serde(rename = "propertyLabels", skip_serializing_if = "Option::is_none")]
    pub property_labels: Option<serde_json::Value>,
    pub tags: Vec<String>,
    /// 该对象是否包含（未软删的）附件——供导出范围树等 UI 判断是否展示附件展开图标。
    /// 由 `properties.__attachments` 推导；metadata-only 路径（properties 未解密）为 false。
    /// 注意：本结构体为逐字段显式 rename（无 rename_all），必须显式 camelCase 序列化。
    #[serde(rename = "hasAttachments", default)]
    pub has_attachments: bool,
    /// 对象各字段的敏感度等级集合（去重、按 public < internal < sensitive < critical 排序）。
    /// 反映字段级敏感度分布（区别于 `sensitivity_level` 记录级）；供导出范围树等 UI 展示徽章。
    /// 由 `list_objects` 等解密路径从 property_labels / __fields / 模板定义推导；
    /// metadata-only 路径（properties 未解密）为空数组。
    #[serde(
        rename = "sensitivityLevels",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub sensitivity_levels: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SharedLeaf {
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossCrate {
    pub nested: crate::SharedLeaf,
}
