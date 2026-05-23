## 6. 字段路径到 UnifiedObject 的映射层

SoloSoul 已完成 Unified Object Model（v3），所有用户数据以 `UnifiedObject` + `PropertyValue` 形式存储。插件通过**逻辑字段路径**（如 `"identity.full_name"`）请求数据，Rust Host 负责将其映射到 Vault 的实际查询。

### 6.1 映射表（内置默认值）

```rust
/// 插件字段路径 -> Vault 查询映射
/// 此表随 UnifiedObject Schema 演进同步更新
const FIELD_MAP: &[(&str, VaultQuery)] = &[
    ("identity.full_name", VaultQuery::Property {
        object_type: "identity",
        property_key: "full_name",
        tag: None,
    }),
    ("identity.id_card.number", VaultQuery::Property {
        object_type: "identity",
        property_key: "id_card_number",
        tag: None,
    }),
    ("travel.primary_passport.number", VaultQuery::Property {
        object_type: "passport",
        property_key: "number",
        tag: Some("primary"),
    }),
    ("travel.passports.*.number", VaultQuery::Property {
        object_type: "passport",
        property_key: "number",
        tag: None, // 返回所有护照号列表
    }),
    ("identity.contact.emails", VaultQuery::Property {
        object_type: "identity",
        property_key: "emails",
        tag: None,
    }),
    ("identity.contact.phones", VaultQuery::Property {
        object_type: "identity",
        property_key: "phones",
        tag: None,
    }),
];
```

### 6.2 映射规则

| 插件字段路径模式 | 含义 | 示例 |
|-----------------|------|------|
| `identity.full_name` | 精确匹配单个属性 | `"张三"` |
| `travel.primary_passport.number` | 带标签的精确匹配 | `"E12345678"` |
| `travel.passports.*.number` | 通配匹配，返回列表 | `["E12345678", "E87654321"]` |

> **扩展性**：未来支持用户自定义字段别名，通过 `SETTING_{accountId}` 中的 `plugin_field_mappings` 配置覆盖默认映射。
