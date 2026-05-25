# 语义化字段标识提案（Semantic Field Identity Proposal）

> **状态**：草案待审  
> **作者**：AI Assistant  
> **日期**：2026-05-22  
> **关联组件**：Flutter 客户端、Rust 插件 Host、Unified Object Model  

---

## 1. 问题陈述

### 1.1 当前现状

SoloSoul 的 Unified Object Model（UOM）采用三层标识体系：

| 层级 | 当前实现 | 示例 |
|------|---------|------|
| **存储 Key** | `UnifiedObject.properties` 的 Map key | `fullName`、`bank_name`、`nickname` |
| **显示标签** | `propertyLabels[key]` 或 `translateFieldLabel()` | `"全名"`、`"银行名称"`、`"昵称"` |
| **Schema 定义** | `PropertyDefinition.id` + `PropertyDefinition.name` | `id="bankName"`, `name="银行名称"` |

**预定义字段**（如 `fullName`、`bankName`）有稳定的英文 key，插件通过 `identity.full_name`、`bankAccount.bankName` 等路径读取，工作良好。

**用户自定义字段**时，当前 UI（`ObjectEditorPage`）要求用户理解两个概念：
- **Key**：底层存储标识符（英文 camelCase/snake_case）
- **显示标签**：用户看到的名称

实际上，当前 UI 存在明显的体验断层：新添加的字段 `key` 初始为空字符串，用户没有直观的输入框来设置 key，导致自定义字段的保存和使用存在障碍。

### 1.2 核心矛盾

用户期望的交互：
> 用户用母语创建"宠物狗"分区 → 添加"昵称"字段 → 填入"可可" → 插件直接读取 `宠物狗.昵称` 得到"可可"

当前实现的问题：
1. **双重定义负担**：用户需要同时维护 key 和显示标签
2. **语言壁垒**：为了让插件读取，用户可能需要使用英文 key，与母语体验割裂
3. **插件路径不透明**：用户不知道插件会用什么路径来读取自己的字段
4. **UI 断层**：当前编辑器中 key 无法直接输入，新字段 key 为空导致保存异常

### 1.3 多语言融合的矛盾点

| 场景 | 问题 |
|------|------|
| 通用插件（如地址格式化器） | 需要稳定的跨语言标识符，当前预定义字段已解决 |
| 用户私有插件/个人自动化 | 用户希望用母语路径，但当前系统不支持 Unicode key 作为插件路径 |
| 社区共享模板 | 模板中的字段在不同语言环境下 key 不一致，导致插件无法通用 |

**关键洞察**：
- 通用插件**不应依赖**用户自定义字段（数据结构不确定）
- 用户私有插件/个人自动化**完全可以用**用户母语的字段路径
- 问题本质是：系统没有官方支持"以用户可见名称作为字段标识符"

---

## 2. 目标

1. **用户零负担**：用户只需用自己的语言输入字段名称，系统自动处理标识符
2. **插件路径自然**：插件可以直接使用用户可见的 `分区.字段` 路径读取数据
3. **多语言原生支持**：中文、日文、阿拉伯文等任意 Unicode 字段名都是一等公民
4. **向后兼容**：现有预定义字段的英文 key 和插件生态不受影响
5. **开发者可控**：高级用户/开发者仍可手动指定稳定的机器 key

---

## 3. 方案设计：语义化字段标识（Semantic Field Identity, SFI）

### 3.1 核心原则

> **"用户看到的名称，就是插件读取的名称"**

取消"底层 key"与"显示标签"的强制分离。用户输入的字段名称经过规范化后直接作为：
- 存储 key（`properties` 的 Map key）
- 默认显示标签（`propertyLabels` 可选覆盖）
- 插件字段路径的末端标识符

### 3.2 三层标识符的重新定位

```
┌─────────────────────────────────────────────────────────────────┐
│  新的字段标识体系                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   用户输入层        │  "昵称"、"Pet Name"、"ニックネーム"           │
│   （Display Label） │  → 经过 Unicode NFC 规范化 + 去空格          │
│                     │  → 成为 Semantic Identifier                 │
│                     │                                             │
│   语义标识层        │  `昵称`、`Pet Name`、`ニックネーム`           │
│   （Semantic ID）   │  → 存储 key、默认显示名、插件路径末端         │
│                     │                                             │
│   覆盖显示层        │  `propertyLabels[semanticId]`               │
│   （Optional）      │  → 用户后续修改显示名时不改变语义标识符        │
│                     │                                             │
│   机器标识层        │  `_internal_{uuid}`（可选，高级模式）         │
│   （Advanced）      │  → 开发者手动指定的稳定机器 key               │
│                     │                                             │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 数据模型变更

#### 3.3.1 `PropertyDefinition`（Schema 层）

```dart
@JsonSerializable(explicitToJson: true)
class PropertyDefinition {
  /// 语义标识符：用户输入的字段名称（规范化后）
  /// 例如："昵称"、"petName"、"Bank Name"
  final String id;

  /// 显示名称覆盖：如果非空，UI 显示此值而非 id
  /// 例如 id="nickname"，name="昵称"
  /// 允许用户在不改变插件路径的前提下修改显示名
  final String? name;

  final PropertyType type;
  final Map<String, dynamic>? config;
  final bool required;
  final int order;

  /// 【新增】语义路径别名：允许为字段注册额外的插件可读路径
  /// 例如：["pet.name", "animal.nickname"]
  /// 用于向后兼容和开发者定义的别名
  final List<String>? semanticAliases;
}
```

**兼容性**：
- 现有数据 `name` 字段有值（如 `"Full Name"`），升级时：
  - `id` 保持原值（英文 key）
  - `name` 保持原值作为显示覆盖
  - 新增 `semanticAliases` 包含原 `id`

#### 3.3.2 `UnifiedObject`（数据层）

无需结构性变更。`properties` 的 key 直接存储语义标识符（Unicode 字符串）。

```dart
// 示例：用户创建的"宠物狗"分区下的 Item
UnifiedObject(
  name: '可可',
  typeId: 'custom_pet_dog',
  parentId: '__section_pet_dog',
  properties: {
    '昵称': TextProperty(text: '可可', sensitivity: SensitivityLevel.public),
    '品种': TextProperty(text: '金毛寻回犬', sensitivity: SensitivityLevel.public),
    '出生日期': DateProperty(isoDate: '2020-03-15', sensitivity: SensitivityLevel.public),
  },
  // propertyLabels 为空，因为 id 本身就是用户想要的显示名
)
```

#### 3.3.3 【新增】Section 语义路径注册表

```dart
/// 全局注册表：建立 section 名称 → section ID 的映射
/// 用于插件路径解析时快速定位 section
class SectionSemanticRegistry {
  /// 内置 section 的固定路径映射（向后兼容）
  static const Map<String, String> _builtInPaths = {
    'identity': '__section_identity',
    'contact': '__section_contact',
    'passport': '__section_passport',
    'bankAccount': '__section_bank_account',
    // ... 其他预定义 section
  };

  /// 运行时动态 section 名称索引
  /// 键：section 的规范化名称（小写 + NFC）
  /// 值：section 的 UnifiedObject ID
  final Map<String, String> _dynamicIndex = {};

  void rebuildIndex(List<UnifiedObject> allObjects) {
    _dynamicIndex.clear();
    for (final obj in allObjects) {
      if (obj.typeId == 'collection' || obj.typeId == 'page') {
        final normalized = _normalizeSectionName(obj.name);
        _dynamicIndex[normalized] = obj.id;
      }
    }
  }

  String? resolveSectionId(String name) {
    // 1. 先查内置映射
    if (_builtInPaths.containsKey(name)) return _builtInPaths[name];
    // 2. 再查动态索引（不区分大小写）
    return _dynamicIndex[_normalizeSectionName(name)];
  }

  static String _normalizeSectionName(String name) {
    return name.trim().toLowerCase().normalizeNfc();
  }
}
```

### 3.4 UI 层变更（ObjectEditorPage）

#### 3.4.1 新字段添加流程

```
用户点击 "+" 添加字段
    │
    ▼
┌─────────────────────────────┐
│ 弹出轻量输入框：              │
│ "字段名称（如：昵称）"        │
│ [________________]           │
│ [取消]        [确认]         │
└─────────────────────────────┘
    │
    ▼
系统处理：
  1. Unicode NFC 规范化
  2. 去除前后空格
  3. 限制长度 64 字符
  4. 检查同 section 内是否重复
  5. 生成 _PropertyField(key: "昵称", displayLabel: "昵称")
    │
    ▼
UI 显示：
  ┌────────────────────────┐
  │ 昵称                    │  ← 只读显示 semantic id（小字灰色）
  │ ┌────────────────────┐ │
  │ │ 昵称               │ │  ← 可编辑显示标签（默认 = semantic id）
  │ └────────────────────┘ │
  │ [text ▼] [Public ▼] [🗑]│
  └────────────────────────┘
```

#### 3.4.2 编辑已有字段

- **修改显示标签**：只更新 `propertyLabels[semanticId]`，不改变插件路径
- **修改语义标识符**：视为"重命名字段"，系统提示"此操作会改变插件读取路径"
  - 更新 `properties` 的 key
  - 迁移 `propertyLabels`
  - 记录操作日志

#### 3.4.3 高级/开发者模式（可选）

为高级用户和开发者提供"插件标识符"输入框：

```
┌─────────────────────────────────────┐
│ 字段名称：昵称                         │
│ 插件标识符（可选）：nickname           │  ← 默认隐藏，展开显示
│ 显示标签：昵称                         │
└─────────────────────────────────────┘
```

- 如果填写了插件标识符，系统同时注册：
  - `properties["昵称"]` — 主存储
  - `semanticAliases: ["nickname"]` — 别名
- 插件可以用 `宠物狗.昵称` 或 `宠物狗.nickname` 读取

### 3.5 插件 Host 层变更（Rust）

#### 3.5.1 字段路径解析升级

当前 Rust Host 的字段路径解析逻辑（`extract_from_unified_object_model`）：

```rust
// 当前：直接取 field_id 的最后一段作为 property_key
let property_key = match field_id {
    "identity.full_name" => "fullName",
    // ... 预定义映射
    _ => field_id.split('.').last().unwrap_or(field_id),
};
```

**升级后的解析流程**：

```rust
/// 解析插件请求的字段路径
/// 
/// 支持的路径格式：
/// 1. 预定义路径（向后兼容）：`identity.full_name`、`bankAccount.bankName`
/// 2. 语义路径（新）：`宠物狗.昵称`、`Pet Dog.Pet Name`
/// 3. 数组索引（向后兼容）：`address[0].street`
/// 4. 通配符（向后兼容）：`travel.*`
fn resolve_field_path(field_id: &str, json_value: &serde_json::Value) -> Option<String> {
    // 阶段 1：预定义字段快速路径（保持现有逻辑不变）
    if let Some(legacy_value) = extract_legacy_field(field_id, json_value) {
        return Some(legacy_value);
    }

    // 阶段 2：解析语义路径
    // 格式："section_name.field_name" 或 "section_name[索引].field_name"
    let parts: Vec<&str> = field_id.split('.').collect();
    if parts.len() < 2 {
        // 单段路径：全局搜索所有非 page/collection 对象的 properties
        return find_property_in_all_objects(parts[0], json_value);
    }

    let section_name = parts[0];
    let field_name = parts.last().unwrap();

    // 2.1 定位 section
    let section_id = resolve_section_id(section_name, json_value)?;
    
    // 2.2 在 section 的子对象中查找字段
    // 策略 A：如果 section 本身有 properties（Schema 定义），先匹配
    // 策略 B：遍历 section.childrenIds，在子对象的 properties 中匹配 field_name
    find_field_in_section(section_id, field_name, json_value)
}

/// 通过名称解析 section ID
fn resolve_section_id(name: &str, json_value: &serde_json::Value) -> Option<String> {
    // 1. 内置 section 映射（向后兼容）
    let built_in = match name {
        "identity" | "Identity" => Some("__section_identity"),
        "contact" | "Contact" => Some("__section_contact"),
        "passport" | "Passport" => Some("__section_passport"),
        "bankAccount" | "Bank Account" | "bank_account" => Some("__section_bank_account"),
        // ...
        _ => None,
    };
    if let Some(id) = built_in { return Some(id.to_string()); }

    // 2. 动态 section 查找：遍历 objects，匹配 name 和 typeId
    let objects = json_value
        .get("unified_objects")?
        .get("objects")?
        .as_array()?;
    
    let normalized_name = normalize_name(name);
    for obj in objects {
        let obj_name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let type_id = obj.get("typeId").and_then(|t| t.as_str()).unwrap_or("");
        if (type_id == "collection" || type_id == "page") 
            && normalize_name(obj_name) == normalized_name {
            return obj.get("id").and_then(|id| id.as_str()).map(|s| s.to_string());
        }
    }
    None
}

/// Unicode 名称规范化（Rust 端）
fn normalize_name(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    name.nfc().collect::<String>().trim().to_lowercase()
}
```

#### 3.5.2 字段匹配策略

在 section 内查找字段时，按优先级匹配：

1. **精确匹配**：`properties.get(field_name)`
2. **规范化匹配**：`properties.get(normalize_name(field_name))`
3. **别名匹配**：检查 `semanticAliases` 是否包含 field_name
4. **propertyLabels 反向查找**：如果 section 的 `propertyLabels` 中有 value == field_name，返回对应的 key

```rust
fn find_field_in_section(
    section_id: &str,
    field_name: &str,
    json_value: &serde_json::Value,
) -> Option<String> {
    let objects = json_value
        .get("unified_objects")?
        .get("objects")?
        .as_array()?;

    let normalized_field = normalize_name(field_name);

    // 找到 section 对象
    let section = objects.iter().find(|obj| {
        obj.get("id").and_then(|id| id.as_str()) == Some(section_id)
    })?;

    // 收集该 section 下所有相关对象的 properties
    // （包括 section 自身的 properties 和子对象的 properties）
    let mut all_properties = Vec::new();

    // 1. Section 自身的 properties（Schema 层）
    if let Some(props) = section.get("properties") {
        all_properties.push(props);
    }

    // 2. 子对象的 properties（数据层）
    let child_ids: Vec<&str> = section
        .get("childrenIds")
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    for obj in objects {
        let obj_id = obj.get("id").and_then(|id| id.as_str()).unwrap_or("");
        if child_ids.contains(&obj_id) {
            if let Some(props) = obj.get("properties") {
                all_properties.push(props);
            }
        }
    }

    // 在收集到的 properties 中查找字段
    for props in &all_properties {
        let props_map = props.as_object()?;

        // 策略 1：精确匹配 key
        if let Some(prop) = props_map.get(field_name) {
            return extract_property_value(prop);
        }

        // 策略 2：规范化匹配 key
        for (key, prop) in props_map {
            if normalize_name(key) == normalized_field {
                return extract_property_value(prop);
            }
        }

        // 策略 3：propertyLabels 反向查找
        if let Some(labels) = section.get("propertyLabels").and_then(|l| l.as_object()) {
            for (key, label) in labels {
                if label.as_str().map(|s| normalize_name(s)) == Some(normalized_field.clone()) {
                    if let Some(prop) = props_map.get(key) {
                        return extract_property_value(prop);
                    }
                }
            }
        }
    }

    None
}
```

#### 3.5.3 敏感度解析升级

当前 `resolve_field_sensitivity` 基于硬编码的英文路径。需要增加对语义路径的支持：

```rust
pub(crate) fn resolve_field_sensitivity(field_id: &str) -> SensitivityLevel {
    // 阶段 1：预定义字段（保持现有逻辑）
    match field_id {
        "identity.full_name" => SensitivityLevel::Public,
        "identity.id_card.number" => SensitivityLevel::Critical,
        // ... 现有映射
        _ => {}
    }

    // 阶段 2：基于字段名称的模式匹配
    let lower = field_id.to_lowercase();
    if lower.contains("password") || lower.contains("cvv") || lower.contains("pin") {
        return SensitivityLevel::Critical;
    }
    if lower.contains("number") || lower.contains("account") || lower.contains("card") {
        return SensitivityLevel::Sensitive;
    }
    if lower.contains("name") || lower.contains("country") {
        return SensitivityLevel::Public;
    }

    // 阶段 3：基于属性值中的 sensitivity 字段（运行时）
    // 如果能从 UOM 中读取到该字段的 sensitivity 元数据，使用之
    
    // 默认
    SensitivityLevel::Sensitive
}
```

### 3.6 多语言融合策略

| 字段类型 | 标识方式 | 插件路径 | 多语言处理 |
|---------|---------|---------|-----------|
| **预定义字段** | 英文 key（`fullName`） | `identity.fullName` | `translateFieldLabel()` 提供 i18n 显示 |
| **模板字段** | 英文 key（`bank_name`） | `bankAccount.bank_name` | 模板 `nameKey` 映射到 ARB |
| **用户自定义字段** | 语义标识符（"昵称"） | `宠物狗.昵称` | 用户母语即标识，无需翻译 |
| **开发者自定义字段** | 语义标识符 + 可选别名 | `宠物狗.nickname`（别名） | 开发者定义别名保证跨语言 |

**核心规则**：
1. 用户自定义字段**不需要**多语言翻译——它属于用户个人的数据 Schema
2. 通用插件**只依赖**预定义字段（已有稳定英文 key）
3. 用户私有插件/社区插件**直接使用**用户母语的语义路径
4. 需要跨语言共享的自定义 Schema，由开发者在 `semanticAliases` 中定义英文别名

---

## 4. 实施路线图

### Phase 1：基础支持（P0，2 周）

**目标**：让 Unicode 字段名可以作为存储 key 和插件路径使用

1. **Dart 模型层**
   - [ ] 确认 `UnifiedObject.properties` 使用中文 key 时 JSON 序列化/反序列化无问题
   - [ ] 更新 `PropertyDefinition`：添加 `semanticAliases` 字段
   - [ ] 添加 `SectionSemanticRegistry` 运行时索引

2. **Flutter UI**
   - [ ] 重构 `ObjectEditorPage._PropertyFieldRow`：
     - 将 key 输入与显示标签输入合并为单一"字段名称"输入
     - 新字段默认 `key = displayLabel = 用户输入`
     - 提供"展开高级选项"按钮设置插件别名
   - [ ] 添加字段规范化逻辑（NFC、去空格、长度限制、重复检查）
   - [ ] 更新保存逻辑：`properties` key 使用规范化后的用户输入

3. **Rust Host**
   - [ ] 添加 `unicode-normalization` crate 依赖
   - [ ] 实现 `resolve_section_id()` 动态 section 查找
   - [ ] 实现 `find_field_in_section()` 语义字段查找
   - [ ] 更新 `extract_from_unified_object_model()` 增加语义路径分支

### Phase 2：体验优化（P1，1 周）

1. **字段重命名支持**
   - [ ] UI 支持修改字段语义标识符（提示影响插件路径）
   - [ ] 数据迁移：重命名时更新所有相关 Item 的 properties key

2. **插件路径预览**
   - [ ] 在 `ObjectEditorPage` 中显示字段的"插件可读路径"提示
   - [ ] 例如：小字显示 `"插件读取路径：宠物狗.昵称"`

3. **冲突检测**
   - [ ] 规范化后的字段名在同一 section 内去重
   - [ ] 与预定义字段 key 冲突时提示用户

### Phase 3：生态扩展（P2，2 周）

1. **插件 SDK 更新**
   - [ ] Rust SDK 中 `get_field()` 支持语义路径
   - [ ] 文档更新：插件开发指南中增加"语义路径"章节

2. **官方插件适配**
   - [ ] 评估现有 22 个官方插件是否需要适配
   - [ ] 新增示例插件：演示如何读取用户自定义语义字段

3. **Schema 导出**
   - [ ] 允许用户导出自己的 section Schema（含语义标识符）
   - [ ] 社区分享模板时保留语义标识符

---

## 5. 风险评估与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| **Unicode key 在 Wasm 边界传递问题** | 高 | 测试 UTF-8 编码的字段路径在 Rust ↔ Wasm 传递时的正确性；Host Functions 已使用 UTF-8 字符串，风险较低 |
| **性能退化** | 中 | 语义路径需要遍历 objects 数组查找； mitigation：Dart 端缓存 `SectionSemanticRegistry` 索引，Rust 端在单次插件执行中缓存解析结果 |
| **现有数据损坏** | 高 | 仅新创建/编辑的字段使用新规则；现有数据完全不动；增加 schema 版本标记 |
| **插件兼容性** | 中 | 预定义字段路径保持不变；通用插件无需修改；只有需要读取用户自定义字段的插件才使用新路径 |
| **字段名重复** | 低 | 规范化后同 section 内去重；UI 实时提示冲突 |
| **大小写敏感问题** | 低 | Rust 端解析时使用规范化比较（转小写 + NFC）；预定义字段保持原有大小写敏感逻辑 |

---

## 6. 向后兼容保证

### 6.1 数据兼容

```dart
// 现有数据（英文 key）
{
  "properties": {
    "fullName": {"type": "text", "text": "张三"}
  },
  "propertyLabels": {
    "fullName": "全名"
  }
}

// 新数据（语义标识符）
{
  "properties": {
    "全名": {"type": "text", "text": "张三"}
  }
  // propertyLabels 为空或不存在
}
```

- 现有数据读取：不受影响
- 现有数据编辑：保持原有英文 key，但允许用户重命名为语义标识符
- 新创建字段：默认使用语义标识符

### 6.2 插件兼容

- 所有使用预定义字段路径的官方插件：**零变更**
- 新插件使用语义路径：**需要用户数据也是语义标识符**
- 混合使用：插件可以同时声明 `required_fields: ["identity.full_name", "宠物狗.昵称"]`

### 6.3 API 兼容

- FRB 接口 `frb_plugin_execute` 参数不变
- Host Functions 签名不变
- 仅内部解析逻辑升级

---

## 7. 示例场景

### 场景 A：用户创建宠物档案

```
用户操作：
  1. 创建分区 "宠物狗"（系统自动分配 ID）
  2. 添加字段 "昵称"（key="昵称", label="昵称"）
  3. 添加字段 "品种"（key="品种", label="品种"）
  4. 添加字段 "出生日期"（key="出生日期", label="出生日期"）
  5. 创建条目，name="可可"
     - 昵称 = "可可"
     - 品种 = "金毛寻回犬"
     - 出生日期 = "2020-03-15"

数据存储：
  Section（collection）:
    id: "obj_xxx", name: "宠物狗", typeId: "collection"
    properties: {
      "昵称": TextProperty(...),
      "品种": TextProperty(...),
      "出生日期": DateProperty(...)
    }

  Item:
    id: "obj_yyy", name: "可可", parentId: "obj_xxx"
    properties: {
      "昵称": TextProperty(text: "可可"),
      "品种": TextProperty(text: "金毛寻回犬"),
      "出生日期": DateProperty(isoDate: "2020-03-15")
    }

插件 manifest：
  {
    "required_fields": ["宠物狗.昵称", "宠物狗.品种"]
  }

插件读取：
  get_field("宠物狗.昵称") → "可可"
  get_field("宠物狗.品种") → "金毛寻回犬"
```

### 场景 B：通用插件读取预定义字段（不变）

```
插件 manifest：
  {
    "required_fields": ["identity.full_name", "address.street"]
  }

插件读取：
  get_field("identity.full_name") → "张三"
  get_field("address.street") → "长安街1号"
```

### 场景 C：用户修改显示标签但不改变插件路径

```
初始：
  properties: {"邮箱": TextProperty(...)}
  propertyLabels: null

用户将显示标签改为 "Email Address"：
  properties: {"邮箱": TextProperty(...)}
  propertyLabels: {"邮箱": "Email Address"}

插件读取：
  get_field("宠物狗.邮箱") → 正常工作（key 未变）

UI 显示：
  显示 "Email Address"（来自 propertyLabels）
```

### 场景 D：开发者定义稳定别名

```
用户创建字段：
  字段名称："手机号"
  插件别名（高级模式）："phone_number"

数据存储：
  properties: {"手机号": TextProperty(...)}
  semanticAliases: ["phone_number"]

插件可用路径：
  "联系人.手机号" → 匹配精确 key
  "联系人.phone_number" → 匹配 semanticAlias
```

---

## 8. 待决策事项

1. **字段名长度限制**：当前显示标签限制 24 字符，语义标识符是否放宽到 64 字符？
2. **特殊字符处理**：是否允许 `!@#$%` 等特殊字符在字段名中？建议限制为字母、数字、空格、常用标点。
3. **预定义字段是否开放语义化**：是否允许用户将 `fullName` 的显示标签重命名为"全名"后，插件也能用 `identity.全名` 读取？
4. **Section 名称冲突**：如果用户创建了名为 "identity" 的自定义 section，与内置 section 冲突时如何处理？
5. **性能优化策略**：`SectionSemanticRegistry` 是每次插件执行时重建，还是 Dart 端持久化缓存？

---

## 9. 附录

### 9.1 相关文件清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `flutter/lib/core/models/unified_object_model.dart` | 修改 | `PropertyDefinition` 增加 `semanticAliases` |
| `flutter/lib/presentation/pages/object_editor_page.dart` | 重写 | 字段编辑 UI 改为语义化标识 |
| `flutter/lib/core/services/unified_object_service.dart` | 新增 | `SectionSemanticRegistry` |
| `flutter/native/src/plugin/host.rs` | 修改 | 字段路径解析增加语义路径分支 |
| `flutter/native/src/plugin/manifest.rs` | 无修改 | 保持现有 `required_fields` 格式 |
| `flutter/lib/presentation/utils/format_field_label.dart` | 无修改 | 预定义字段 i18n 保持现有逻辑 |
| `SoloSoul_plugin_market/SDK/rust/src/lib.rs` | 可选修改 | SDK 增加语义路径示例 |

### 9.2 参考实现：规范化函数

```dart
// Dart 端
String normalizeFieldIdentity(String input) {
  // 1. Unicode NFC 规范化
  var normalized = input.trim();
  // 2. 去除首尾空白
  // 3. 内部连续空格合并为单个空格
  normalized = normalized.replaceAll(RegExp(r'\s+'), ' ');
  // 4. 长度限制
  if (normalized.length > 64) {
    normalized = normalized.substring(0, 64);
  }
  return normalized;
}
```

```rust
// Rust 端
use unicode_normalization::UnicodeNormalization;

fn normalize_field_identity(input: &str) -> String {
    let mut s = input.nfc().collect::<String>();
    s = s.trim().to_string();
    // 合并连续空格
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }
    result.truncate(64);
    result
}
```
