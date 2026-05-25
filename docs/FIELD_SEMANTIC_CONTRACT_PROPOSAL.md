# 字段语义契约提案（Field Semantic Contract Proposal）

> **状态**：已批准，进入实施阶段  
> **版本**：v3.1（最终审查修订版）  
> **作者**：AI Assistant  
> **日期**：2026-05-22  
> **审查评分**：9/10 — 设计成熟、考虑周全  
> **关联审查**：SEMANTIC_FIELD_IDENTITY_PROPOSAL — 已否决并归档  

---

## 1. 背景与审查结论

### 1.1 上一版方案（语义化字段标识）的问题

上一版提案试图让"用户看到的名称 = 插件读取的路径"。审查意见揭示了其致命缺陷：

| 审查问题 | 根本原因 |
|---------|---------|
| 字段标识不稳定 | 用户修改显示标签直接导致插件路径失效 |
| 通用插件无法编写 | 插件必须猜测用户的字段命名习惯 |
| 运行时性能差 | 语义路径需要 O(N) 全账户扫描 |
| 调试困难 | Unicode 规范化引入隐式别名 |
| 向后兼容混乱 | 预定义字段无法用新路径访问 |

**核心教训**：在有第三方插件的系统中，**存储 key 的稳定性比界面直观性重要得多**。

### 1.2 本方案审查结论

经审查，本方案（字段语义契约）被确认为**可行的主线方向**，理由：

- **稳定性与易用性兼顾**：存储 key 机器生成且锁定，用户只看到显示标签和语义类型
- **插件可通用**：插件请求 `pet.name` 而非 `宠物狗.昵称`，跨用户、跨语言通用
- **性能可控**：语义类型查找 O(K)（K=section 内字段数，通常 ≤ 20）
- **向后兼容干净**：预定义字段完全不受影响
- **实施复杂度适中**：改动集中，预估 2～3 周

### 1.3 敏感度审查机制的引入

在 v2.0 基础上，进一步审查发现关键安全问题：**插件不能对任何字段的敏感度等级做假设**。用户可能修改默认敏感度，插件必须显式声明所需权限，用户在安装/运行时审查并授权。

因此 v3.0 引入以下核心变更：
- 插件 manifest 统一为 `field_access` 列表，每项声明所需字段 + 敏感度上限
- 安装/运行时执行敏感度审查流程
- Host 每次字段访问都校验声明和实际敏感度

---

## 2. 不变原则

以下原则在任何情况下都不可动摇：

1. **存储 key 锁定**：机器生成，一旦创建永不改变。插件依赖的字段路径永远稳定。
2. **显示标签自由编辑**：用户可随时修改，不影响任何插件。
3. **预定义字段保持现状**：`identity.fullName`、`bankAccount.bankName` 等已有路径继续工作，官方插件零变更。
4. **用户不直接操作机器 key**：UI 层面默认隐藏机器 key，用户只与"显示标签"和"语义类型"交互。
5. **语义类型是可选增强**：即使不存在语义类型，基础体验（key + label）仍然完整可用。
6. **插件不得假设敏感度**：用户有权修改任何字段的敏感度，插件必须显式声明并等待授权。
7. **最小权限原则**：插件只能访问 `field_access` 中声明的字段，且实际敏感度不得超过声明上限。

---

## 3. 核心设计：字段语义契约

### 3.1 三层属性体系

每个字段拥有三层互相独立的属性：

```
┌─────────────────────────────────────────────────────────────────┐
│  字段的三层属性                                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ① 存储 Key（Storage Key）【底层，默认隐藏】                        │
│     → 机器生成的全局唯一标识符，如 "auto_a3f7d2e1"                 │
│     → 基于 UUID v4 前 8 位，确保不与任何预定义 key 冲突            │
│     → 一旦创建永不改变，不因 section 移动、重命名而变化            │
│     → 用户删除字段后重建同名字段，生成全新 key                     │
│     → 在审计日志、插件审查弹窗、高级模式下可见                     │
│                                                                  │
│  ② 显示标签（Display Label）【用户层，随时可改】                    │
│     → 用户看到的名称，如 "昵称"、"宝贝名字"                        │
│     → 随时可编辑，不影响插件                                       │
│     → 通过 propertyLabels 或 PropertyDefinition.name 存储          │
│                                                                  │
│  ③ 语义类型（Semantic Type）【契约层，插件依赖】                    │
│     → 标准化的字段语义分类，如 "pet.name"                          │
│     → 从官方语义类型库中选择                                       │
│     → 插件通过语义类型请求数据，不依赖具体 key 或显示标签          │
│     → 同一 section 内允许重复绑定，但插件只能识别第一个            │
│     → 未指定语义类型的字段对插件不可见（除非用底层 key）           │
│                                                                  │
│  ④ 敏感度（Sensitivity）【安全层，用户可控】                        │
│     → 用户可自由修改，继承自语义类型默认值但可覆盖                 │
│     → 插件必须显式声明所需敏感度上限，经用户审查后授权              │
│     → 运行时 Host 严格校验：实际敏感度 ≤ 插件声明上限              │
│     → 用户修改敏感度时记录审计日志                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 机器 key 的可见性策略（折中方案）

**原则：默认隐藏，特定场景下可见。**

| 场景 | 可见性 | 展示方式 |
|------|--------|---------|
| 普通字段编辑器 | ❌ 隐藏 | 用户只显示标签和语义类型 |
| 插件审查弹窗 | ✅ 可展开 | 每行字段旁的 `ⓘ` 图标，点击/悬停显示 `"标识符：auto_a3f7d2e1"` |
| 字段编辑器（高级模式） | ✅ 可见 | 全局设置"显示技术标识符"开关开启后，显示标签下方小字灰色显示机器 key |
| 审计日志 | ✅ 可见 | 格式：`插件 X 读取字段"昵称"(auto_a3f7d2e1)` |
| 调试/开发者工具 | ✅ 可见 | 完整显示 |

**UI 示例（高级模式）**：
```
昵称
─────────────────────
标识符：auto_a3f7d2e1
语义类型：🐕 宠物名字 (pet.name)
敏感度：公开 🔽
```

### 3.3 语义类型注册表

语义类型是一个预定义的标准化分类系统。

```dart
/// 预定义的语义类型
class SemanticFieldType {
  /// 标准化标识符
  /// 官方类型：短 ID，如 "pet.name"、"person.birth_date"
  /// 第三方扩展：命名空间格式，如 "com.example.veterinarian.license"
  final String id;

  /// 多语言显示名称
  /// {"zh": "宠物名字", "en": "Pet Name", "ja": "ペットの名前"}
  final Map<String, String> labels;

  /// 多语言说明（用于 UI 引导）
  final Map<String, String> descriptions;

  /// 所属分类
  final String category; // "person", "pet", "financial", "contact", "travel", "generic", "custom"

  /// 建议的属性类型
  final PropertyType suggestedType;

  /// 默认敏感度（用户可覆盖）
  final SensitivityLevel defaultSensitivity;

  /// 图标名称
  final String iconName;

  /// 该语义类型首次引入的版本（用于兼容性检查）
  final String minAppVersion;
}
```

**初始语义类型库（50 个核心类型）**：

| 语义类型 ID | 中文标签 | 英文标签 | 分类 | 建议类型 | 默认敏感度 |
|------------|---------|---------|------|---------|-----------|
| `person.name` | 姓名 | Full Name | person | text | public |
| `person.nickname` | 昵称 | Nickname | person | text | public |
| `person.given_name` | 名 | Given Name | person | text | public |
| `person.family_name` | 姓 | Family Name | person | text | public |
| `person.birth_date` | 出生日期 | Date of Birth | person | date | sensitive |
| `person.gender` | 性别 | Gender | person | select | public |
| `person.nationality` | 国籍 | Nationality | person | text | public |
| `pet.name` | 宠物名字 | Pet Name | pet | text | public |
| `pet.breed` | 品种 | Breed | pet | text | public |
| `pet.species` | 物种 | Species | pet | select | public |
| `pet.birth_date` | 出生日期 | Birth Date | pet | date | public |
| `pet.weight` | 体重 | Weight | pet | number | public |
| `pet.color` | 毛色 | Color | pet | text | public |
| `pet.vet_name` | 兽医姓名 | Veterinarian | pet | text | public |
| `pet.vet_phone` | 兽医电话 | Vet Phone | pet | text | sensitive |
| `financial.account_number` | 账号 | Account Number | financial | text | critical |
| `financial.bank_name` | 银行名称 | Bank Name | financial | text | public |
| `financial.swift_code` | SWIFT 代码 | SWIFT Code | financial | text | sensitive |
| `financial.iban` | IBAN | IBAN | financial | text | critical |
| `financial.card_number` | 卡号 | Card Number | financial | text | critical |
| `financial.card_cvv` | CVV | CVV | financial | text | critical |
| `financial.card_expiry` | 有效期 | Expiry Date | financial | date | critical |
| `financial.tax_id` | 税号 | Tax ID | financial | text | critical |
| `contact.phone` | 电话号码 | Phone Number | contact | text | sensitive |
| `contact.email` | 邮箱 | Email | contact | text | internal |
| `contact.address` | 地址 | Address | contact | text | sensitive |
| `contact.emergency_contact` | 紧急联系人 | Emergency Contact | contact | text | sensitive |
| `travel.passport_number` | 护照号码 | Passport Number | travel | text | critical |
| `travel.visa_number` | 签证号码 | Visa Number | travel | text | critical |
| `travel.flight_number` | 航班号 | Flight Number | travel | text | public |
| `travel.hotel_name` | 酒店名称 | Hotel Name | travel | text | public |
| `travel.check_in_date` | 入住日期 | Check-in Date | travel | date | public |
| `professional.company` | 公司 | Company | professional | text | public |
| `professional.position` | 职位 | Position | professional | text | public |
| `professional.department` | 部门 | Department | professional | text | public |
| `professional.start_date` | 入职日期 | Start Date | professional | date | public |
| `professional.end_date` | 离职日期 | End Date | professional | date | public |
| `education.institution` | 学校 | Institution | education | text | public |
| `education.degree` | 学位 | Degree | education | select | public |
| `education.major` | 专业 | Major | education | text | public |
| `education.graduation_date` | 毕业日期 | Graduation Date | education | date | public |
| `generic.note` | 备注 | Note | generic | text | public |
| `generic.url` | 网址 | URL | generic | url | public |
| `generic.date` | 日期 | Date | generic | date | public |
| `generic.number` | 数字 | Number | generic | number | public |
| `generic.tag` | 标签 | Tag | generic | text | public |
| `generic.attachment` | 附件 | Attachment | generic | relation | public |
| `custom.untyped` | 其他 | Other | custom | text | public |

**扩展机制**：
- Phase 1：硬编码在 App 中，随版本更新
- Phase 3：支持从远程 CDN 热更新语义类型库
- 第三方插件可声明自定义语义类型（使用命名空间格式如 `com.example.xxx`）
- 插件 manifest 中可声明 `required_semantic_type_versions` 指定最低版本要求

### 3.4 机器 Key 生成策略

**采用策略**：全局唯一 UUID 前缀

```dart
String generateMachineKey() {
  // 格式：auto_{uuid_v4前8位}
  // 示例：auto_a3f7d2e1
  final uuid = Uuid().v4();
  return 'auto_${uuid.substring(0, 8)}';
}
```

**设计理由**：
- `auto_` 前缀明确标识这是机器生成的 key，不会与任何预定义字段（如 `fullName`）冲突
- UUID 前 8 位提供 2^32 级别的唯一性，实际碰撞概率可忽略
- key 完全无意义，不透露任何字段信息，最安全
- 全局唯一，即使跨 section 引用也不会有冲突
- 删除后重建生成全新 key，避免历史数据污染

**约束**：
- 长度限制：13 字符（`auto_` + 8 位）
- 字符集：`[a-z0-9_]`
- 与预定义 key 的冲突检查：预定义 key 不含 `auto_` 前缀，天然不冲突

### 3.5 数据模型变更

#### 3.5.1 `PropertyDefinition`（Schema 层）

```dart
@JsonSerializable(explicitToJson: true)
class PropertyDefinition {
  /// 机器 key（稳定标识符）
  /// 预定义字段：保持现有英文 camelCase（如 "fullName"）
  /// 自定义字段：自动生成，如 "auto_a3f7d2e1"
  final String id;

  /// 显示名称
  /// 预定义字段：从 ARB 读取
  /// 自定义字段：用户输入的字段名称
  final String name;

  final PropertyType type;
  final Map<String, dynamic>? config;
  final bool required;
  final int order;

  /// 【新增】语义类型标识符
  /// 如 "pet.name"、"person.birth_date"
  /// 为 null 表示未指定语义类型（对插件不可见）
  final String? semanticType;

  /// 【新增】机器 key 生成方式标记
  /// true = 机器自动生成（auto_ 前缀）
  /// false = 预定义/手动指定
  final bool isAutoKey;

  /// 【新增】敏感度级别
  /// 继承自语义类型的默认值，但允许用户覆盖
  @override
  final SensitivityLevel sensitivity;
}
```

#### 3.5.2 `UnifiedObject`（数据层）

`UnifiedObject.properties` 继续使用机器 key。语义类型存储在 Section 的 schema 中。

```json
{
  "id": "section_pet_dog",
  "name": "宠物狗",
  "typeId": "collection",
  "properties": {
    "auto_a3f7d2e1": {"type": "text", "text": "", "sensitivity": "public"},
    "auto_b2e18f4a": {"type": "text", "text": "", "sensitivity": "public"}
  },
  "propertyLabels": {
    "auto_a3f7d2e1": "昵称",
    "auto_b2e18f4a": "品种"
  },
  "__semanticTypes": {
    "auto_a3f7d2e1": "pet.name",
    "auto_b2e18f4a": "pet.breed"
  }
}
```

**兼容性**：
- `__semanticTypes` 是可选字段，旧数据不存在时默认为空 Map
- 预定义 section 不设置 `__semanticTypes`（官方插件继续用现有路径）
- 未来若给预定义字段也标注语义类型，可增量添加，不影响现有插件

#### 3.5.3 【新增】插件级字段映射

为支持"将现有字段绑定到语义类型以满足插件需求"的场景，引入插件级映射表：

```dart
/// 每个已安装插件的字段映射配置
@jsonSerializable
class PluginFieldMapping {
  /// 插件 ID
  final String pluginId;

  /// 语义类型 → 机器 key 的映射
  /// 示例：{"pet.name": "auto_a3f7d2e1", "pet.breed": "auto_b2e18f4a"}
  final Map<String, String> semanticTypeToKey;

  /// Section ID 限定（可选）
  /// 插件可指定从哪个 section 读取语义类型字段
  final String? targetSectionId;
}
```

**关键规则**：
- 映射**不会修改** Section 的 `__semanticTypes`，仅对该插件生效
- 映射优先级高于 `__semanticTypes`：用户主动为特定插件指定的映射应覆盖默认绑定
- 用户可以修改映射而不影响原始字段或其他插件

存储位置：`~/.solosoul/{account_id}/plugin_mappings.json`

**使用场景**：
- 用户已有"宝贝名字"字段（无语义类型），安装宠物提醒插件时：
  - 向导提示："请选择哪个字段代表【宠物名字】"
  - 用户选择现有字段
  - 系统记录：`plugin_mappings["com.solosoul.official.pet-reminder"] = {"pet.name": "auto_xxx"}`
- 该映射仅对此插件生效，不影响其他插件

---

## 4. 插件契约层

### 4.1 插件 Manifest（统一字段声明）

不再区分 `required_semantic_types` 和 `required_fields`，改为统一的 `field_access` 列表。

```json
{
  "plugin_id": "com.solosoul.official.pet-reminder",
  "name": "Pet Reminder",
  "version": "1.0.0",
  "plugin_api_version": "1.2",

  "field_access": [
    {
      "semantic_type": "pet.name",
      "required_sensitivity": "internal",
      "access": "read"
    },
    {
      "semantic_type": "pet.breed",
      "required_sensitivity": "public",
      "access": "read"
    },
    {
      "semantic_type": "pet.birth_date",
      "required_sensitivity": "sensitive",
      "access": "read"
    },
    {
      "key": "identity.full_name",
      "required_sensitivity": "public",
      "access": "read"
    }
  ],

  "network_policy": {
    "block_all_outbound": true
  }
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `semantic_type` | string? | 优先使用语义类型定位字段（如 `"pet.name"`） |
| `key` | string? | 兼容预定义字段的直接存储 key（如 `"identity.full_name"`） |
| `required_sensitivity` | string | 插件要求的敏感度上限：`public` / `internal` / `sensitive` / `critical` |
| `access` | string | 访问模式：`"read"` 或 `"write"` |

**规则**：
- `semantic_type` 和 `key` 二选一，不可同时为空
- 插件声明 `required_sensitivity` 表示：**该字段在用户数据中的实际敏感度必须 ≤ 此等级**
- 若实际敏感度 > `required_sensitivity`，插件无法安全使用，需用户决策

### 4.2 敏感度等级体系

与系统完全一致，不可扩展。采用数值映射便于比较：

```rust
#[repr(u8)]
enum SensitivityLevel {
    Public = 0,
    Internal = 1,
    Sensitive = 2,
    Critical = 3,
}
```

| 等级 | 数值 | 说明 | 默认场景 |
|------|------|------|---------|
| `public` | 0 | 公开数据 | 姓名、国家、宠物品种 |
| `internal` | 1 | 内部数据 | 邮箱、普通地址 |
| `sensitive` | 2 | 敏感数据 | 出生日期、电话号码 |
| `critical` | 3 | 关键数据 | 银行卡号、护照号、密码 |

**运行时校验规则**：
- 若 `actual_sensitivity <= required_sensitivity`：✅ 允许访问
- 若 `actual_sensitivity > required_sensitivity`：⚠️ 根据用户安装时的授权策略处理

### 4.3 向后兼容

- 旧插件（没有 `field_access`，只有 `required_fields` / `optional_fields`）：
  - Host 将 `required_fields` 转换为 `field_access` 项，每项默认 `required_sensitivity: "sensitive"`（最严格的默认值）
  - 首次运行时弹出兼容性警告，要求用户确认
  - 旧插件无法使用语义类型，继续通过现有路径读取预定义字段

- 新插件同时声明 `field_access` 和 `required_fields`：
  - `field_access` 优先处理
  - `required_fields` 作为兜底（转换为敏感度 sensitive 的 read 访问）

---

## 5. 插件安装/运行时的审查流程

### 5.1 安装时审查流程

当用户安装或更新插件时，Host 执行以下步骤：

```
用户触发插件安装
    │
    ▼
1. 字段解析
   对每个 field_access 项：
   - 如果有 semantic_type：在用户数据中查找绑定了该语义类型的字段
   - 如果有 key：直接定位存储 key
   - 记录匹配结果（找到 / 未找到）
    │
    ▼
2. 敏感度比较
   对每个已匹配的字段：
   - 读取该字段在用户 Schema 中定义的 sensitivity（用户可能已修改默认值）
   - 比较：实际敏感度 vs 插件声明的 required_sensitivity
   - 标记状态：✅ 符合 / ⚠️ 超出 / ❓ 字段缺失
    │
    ▼
3. 展示给用户（审查弹窗）
   表格列出每个字段：
   - 字段显示名称（如"宠物名字"）
   - 机器 key（可展开，默认隐藏）
   - 所在分区（如"宠物狗"）
   - 当前敏感度（用户设置的值）
   - 插件要求的敏感度上限
   - 访问模式（读/写）
   - 状态（✅ / ⚠️ / ❓）
    │
    ▼
4. 用户决策
   ├── 拒绝授权 → 插件无法安装
   ├── 降低字段敏感度 → 提供快捷入口修改字段敏感度，重新校验
   ├── 同意忽略超出项 → 插件安装，但超出字段返回 null/错误，记录用户同意
   └── 一键创建缺失字段 → 引导用户创建带正确语义类型的新字段
```

### 5.2 UI 审查弹窗设计

```
┌─────────────────────────────────────────────────────────────────┐
│  🔌 插件 "宠物提醒" 请求访问以下字段                               │
│                                                                  │
│  该插件需要读取您数据中的特定字段。请审查并决定是否授权。          │
│                                                                  │
├──────────────┬──────────────┬────────────┬────────┬────────┬─────┤
│ 字段名称      │ 所在分区      │ 当前敏感度  │ 插件要求 │ 访问   │ 状态 │
├──────────────┼──────────────┼────────────┼────────┼────────┼─────┤
│ 🐕 宠物名字 ⓘ │ 宠物狗        │ public     │ internal│ 读取   │ ✅  │
│ 🐕 品种       │ 宠物狗        │ public     │ public  │ 读取   │ ✅  │
│ 🐕 出生日期 ⓘ │ 宠物狗        │ critical   │ sensitive│ 读取  │ ⚠️  │
│ 👤 全名       │ 身份信息      │ sensitive  │ public  │ 读取   │ ⚠️  │
│ 🐕 体重       │ —             │ —          │ public  │ 读取   │ ❓  │
└──────────────┴──────────────┴────────────┴────────┴────────┴─────┘
│                                                                  │
│ ⚠️ 以下字段的敏感度超出插件要求：                                 │
│    • "出生日期" 当前为 critical，插件仅需要 sensitive             │
│    • "全名" 当前为 sensitive，插件仅需要 public                   │
│    这些字段将无法被插件读取，相关功能可能不可用。                  │
│                                                                  │
│ ❓ 以下字段在您的数据中不存在：                                   │
│    • "体重" — 插件相关功能将不可用                               │
│                                                                  │
│ 点击 ⓘ 图标可查看字段的技术标识符（机器 key）                     │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  [ 修改字段敏感度 ]    [ 一键创建缺失字段 ]                       │
│                                                                  │
│              [ 继续安装（跳过无法访问的字段） ]                   │
│                          [ 取消安装 ]                            │
└─────────────────────────────────────────────────────────────────┘
```

**ⓘ 图标悬停/点击展开**：
```
┌─────────────────────────┐
│  标识符：auto_a3f7d2e1   │
│  语义类型：pet.name      │
│  创建时间：2026-05-20    │
└─────────────────────────┘
```

**状态说明**：

| 状态 | 图标 | 含义 | 处理方式 |
|------|------|------|---------|
| 符合 | ✅ | 实际敏感度 ≤ 插件要求 | 正常授权访问 |
| 超出 | ⚠️ | 实际敏感度 > 插件要求 | 用户可选择降低敏感度或同意忽略 |
| 缺失 | ❓ | 字段在用户数据中不存在 | 用户可创建新字段或同意忽略 |

### 5.3 运行时行为

插件执行期间，每次 `solosoul_request_field` 调用时，Host 执行以下检查：

```rust
fn solosoul_request_field(field_id: &str, ...) -> i32 {
    // 1. 检查该字段是否在 field_access 中声明
    let access_decl = find_field_access_declaration(field_id, &manifest.field_access);
    if access_decl.is_none() {
        log_audit(..., AuditAction::FieldAccessDenied { reason: "Not declared in manifest" });
        return -1; // PermissionDenied
    }
    
    // 2. 定位实际字段
    let (machine_key, section_id, actual_sensitivity) = resolve_field(field_id, ...)?;
    
    // 3. 敏感度校验
    let required = access_decl.required_sensitivity;
    if actual_sensitivity > required {
        // 检查用户安装时的授权策略
        match get_user_authorization_strategy(plugin_id, field_id) {
            AuthStrategy::Deny => {
                log_audit(..., AuditAction::FieldAccessDenied { reason: "Sensitivity exceeded" });
                return -2; // UserDenied
            }
            AuthStrategy::AllowButMask => {
                // 返回脱敏后的数据（如只显示前几位）
                let masked = mask_value(value, actual_sensitivity);
                return write_memory(..., masked);
            }
            AuthStrategy::AllowWithWarning => {
                // 返回完整数据，但记录审计日志
                log_audit(..., AuditAction::FieldAccessGranted { 
                    field: field_id, 
                    confirmed_by_user: true,
                    sensitivity_override: true 
                });
                return write_memory(..., value);
            }
        }
    }
    
    // 4. 正常返回
    write_memory(..., value)
}
```

**用户授权策略（安装时选择）**：

| 策略 | 行为 | 适用场景 |
|------|------|---------|
| `Deny` | 敏感度超出时直接拒绝访问 | 高安全需求用户 |
| `AllowButMask` | 返回脱敏数据（如卡号只显示后 4 位） | 平衡安全与功能 |
| `AllowWithWarning` | 返回完整数据但记录审计日志 | 信任插件，追求功能完整 |

### 5.4 运行时敏感度超出弹窗

如果运行时用户尚未处理敏感度超出问题：

```
┌─────────────────────────────────────────────────┐
│  ⚠️ 插件访问受限                                  │
│                                                  │
│  "宠物提醒" 尝试读取 "出生日期"，但该字段的敏感度   │
│  为 critical，超出插件声明的 sensitive 上限。     │
│                                                  │
│  请选择处理方式：                                  │
│                                                  │
│  ○ 拒绝访问（插件相关功能不可用）                 │
│  ○ 返回脱敏数据（只显示部分信息）                 │
│  ○ 允许访问并记录审计日志                         │
│                                                  │
│  [ 记住我的选择 ]  [ 确认 ]                        │
└─────────────────────────────────────────────────┘
```

---

## 6. 字段缺失处理

如果字段（根据语义类型或 key）在当前用户数据中不存在：

1. **允许插件继续安装**，标记该字段为"缺失"
2. **插件运行时**尝试读取该字段返回 `null`（或 Rust 侧返回空字符串）
3. **用户界面**提示："某些功能不可用，因为缺少字段 XXX"
4. **提供"一键创建缺失字段"引导**：
   - 根据语义类型自动创建字段
   - 自动生成机器 key（`auto_xxx`）
   - 设置语义类型为插件要求的类型
   - 显示标签使用当前语言的默认标签（从语义类型库获取）
   - 敏感度继承语义类型的默认值
   - 创建完成后**自动绑定到该插件**（写入 `plugin_mappings`）
   - 用户可修改显示标签和敏感度后再确认创建

**一键创建流程**：
```
用户点击"一键创建缺失字段"
    │
    ▼
系统生成：
  - machine_key: "auto_f2e9a1b3"
  - semantic_type: "pet.weight"
  - display_label: "体重"（从语义类型库获取当前语言标签）
  - sensitivity: "public"（语义类型默认值）
  - type: "number"（语义类型建议类型）
    │
    ▼
弹窗确认：
  "将在【宠物狗】分区中创建以下字段："
  字段名称：体重（可修改）
  类型：数字
  敏感度：公开 🔽
  语义类型：🐕 体重 (pet.weight)
    │
    ▼
用户确认后：
  1. 创建字段到 Section schema
  2. 写入 __semanticTypes["auto_f2e9a1b3"] = "pet.weight"
  3. 写入 plugin_mappings[plugin_id]["pet.weight"] = "auto_f2e9a1b3"
  4. 刷新审查弹窗，该字段状态变为 ✅
```

---

## 7. Rust Host 实现

### 7.1 语义类型解析（带敏感度校验）

```rust
// ============================================================================
// 语义类型解析（新增模块）
// ============================================================================

/// 单次插件执行期间的缓存
/// 策略：每次插件执行前重建一次（数据量小，开销可忽略）
struct SemanticTypeCache {
    /// section_id -> {semantic_type -> (machine_key, sensitivity)}
    section_index: HashMap<String, HashMap<String, (String, SensitivityLevel)>>,
}

impl SemanticTypeCache {
    fn build(section: &serde_json::Value) -> Option<Self> {
        let section_id = section.get("id").and_then(|id| id.as_str())?;
        let semantic_types = section.get("__semanticTypes").and_then(|st| st.as_object())?;
        let properties = section.get("properties").and_then(|p| p.as_object())?;
        
        let mut index = HashMap::new();
        for (machine_key, st_value) in semantic_types {
            if let Some(st) = st_value.as_str() {
                let sensitivity = properties.get(machine_key)
                    .and_then(|prop| prop.get("sensitivity"))
                    .and_then(|s| s.as_str())
                    .and_then(|s| parse_sensitivity(s))
                    .unwrap_or(SensitivityLevel::Public);
                
                index.entry(st.to_string())
                    .or_insert_with(|| (machine_key.to_string(), sensitivity));
            }
        }
        
        let mut section_index = HashMap::new();
        section_index.insert(section_id.to_string(), index);
        
        Some(Self { section_index })
    }
}

/// 解析语义路径并返回字段信息（含敏感度）
/// 
/// 路径格式："{section_id}.semantic://{semantic_type}" 或 "{section_name}.semantic://{semantic_type}"
fn resolve_semantic_field(
    field_id: &str,
    json_value: &serde_json::Value,
    plugin_mappings: Option<&HashMap<String, String>>,
) -> Option<(String, SensitivityLevel)> {
    let (section_ref, semantic_type) = parse_semantic_path(field_id)?;
    let section_id = resolve_section_reference(section_ref, json_value)?;
    let section = find_section_by_id(&section_id, json_value)?;
    
    // 优先使用插件级映射（用户主动指定 > 默认绑定）
    if let Some(mappings) = plugin_mappings {
        if let Some(machine_key) = mappings.get(semantic_type) {
            let sensitivity = get_field_sensitivity(section, machine_key);
            return Some((machine_key.clone(), sensitivity));
        }
    }
    
    // 使用 section 的 __semanticTypes
    let semantic_types = section.get("__semanticTypes").and_then(|st| st.as_object())?;
    let machine_key = find_machine_key_by_semantic_type(semantic_types, semantic_type)?;
    let sensitivity = get_field_sensitivity(section, &machine_key);
    
    Some((machine_key, sensitivity))
}

fn get_field_sensitivity(section: &serde_json::Value, machine_key: &str) -> SensitivityLevel {
    section.get("properties")
        .and_then(|p| p.as_object())
        .and_then(|props| props.get(machine_key))
        .and_then(|prop| prop.get("sensitivity"))
        .and_then(|s| s.as_str())
        .and_then(|s| parse_sensitivity(s))
        .unwrap_or(SensitivityLevel::Public)
}

fn parse_sensitivity(s: &str) -> Option<SensitivityLevel> {
    match s {
        "public" => Some(SensitivityLevel::Public),
        "internal" => Some(SensitivityLevel::Internal),
        "sensitive" => Some(SensitivityLevel::Sensitive),
        "critical" => Some(SensitivityLevel::Critical),
        _ => None,
    }
}

/// 【新增 Host Function】获取包含指定语义类型的所有 section
fn get_sections_with_semantic_type(
    semantic_type: &str,
    json_value: &serde_json::Value,
) -> Vec<serde_json::Value> {
    // ... 实现同 v2.0
}
```

### 7.2 字段访问统一入口

```rust
/// 统一的字段访问解析函数
/// 处理所有类型的字段请求：预定义 key、语义类型、插件映射
fn resolve_field_access(
    field_id: &str,
    manifest: &PluginManifest,
    plugin_mappings: &PluginFieldMapping,
    json_value: &serde_json::Value,
) -> Result<FieldResolution, FieldAccessError> {
    // 1. 在 manifest.field_access 中查找声明
    let access_decl = manifest.field_access.iter()
        .find(|decl| {
            if field_id.contains("semantic://") {
                let (_, st) = parse_semantic_path(field_id).unwrap_or(("", ""));
                decl.semantic_type.as_deref() == Some(st)
            } else {
                decl.key.as_deref() == Some(field_id)
            }
        })
        .ok_or(FieldAccessError::NotDeclared)?;
    
    // 2. 解析字段位置
    let (machine_key, actual_sensitivity, section_name) = if field_id.contains("semantic://") {
        resolve_semantic_field(field_id, json_value, Some(&plugin_mappings.semantic_type_to_key))
            .map(|(key, sens)| (key, sens, "...".to_string()))
            .ok_or(FieldAccessError::FieldNotFound)?
    } else {
        resolve_legacy_field(field_id, json_value)
            .ok_or(FieldAccessError::FieldNotFound)?
    };
    
    // 3. 敏感度校验
    let required = parse_sensitivity(&access_decl.required_sensitivity)
        .unwrap_or(SensitivityLevel::Sensitive);
    
    Ok(FieldResolution {
        machine_key,
        actual_sensitivity,
        required_sensitivity: required,
        access_mode: access_decl.access.clone(),
        section_name,
    })
}

struct FieldResolution {
    machine_key: String,
    actual_sensitivity: SensitivityLevel,
    required_sensitivity: SensitivityLevel,
    access_mode: String,
    section_name: String,
}

enum FieldAccessError {
    NotDeclared,
    FieldNotFound,
    SensitivityExceeded,
}
```

---

## 8. UI 层设计

### 8.1 字段创建流程（含敏感度）

```
用户点击 "+" 添加字段
    │
    ▼
步骤 1：输入字段名称
  字段名称 [昵称________________]
  
  💡 推荐语义类型：🐕 宠物名字  👤 人物昵称  🏷️ 其他
    │
    ├── 跳过语义类型 → 创建完成（semanticType=null）
    └── 选择语义类型
        │
        ▼
步骤 2：确认语义类型和敏感度
  已选择：🐕 宠物名字 (pet.name)
  
  字段类型：文本
  敏感度：公开 🔽
    └─ 继承自语义类型默认值，可修改
  
  [修改语义类型] [确认创建 ✓]
```

**敏感度选择器**：
```dart
DropdownButton<SensitivityLevel>(
  value: selectedSensitivity,
  items: SensitivityLevel.values.map((level) {
    return DropdownMenuItem(
      value: level,
      child: Row(
        children: [
          Icon(Icons.circle, color: level.color, size: 10),
          const SizedBox(width: 8),
          Text(level.localizedLabel),
          const SizedBox(width: 8),
          Text(
            level == defaultSensitivity ? '(推荐)' : '',
            style: TextStyle(fontSize: 11, color: Colors.grey),
          ),
        ],
      ),
    );
  }).toList(),
  onChanged: (value) => setState(() => selectedSensitivity = value),
)
```

### 8.2 语义类型重复绑定提示

当用户尝试在同一 section 中添加第二个相同语义类型的字段时：

```
┌─────────────────────────────────────────────────┐
│  ⚠️ 语义类型重复                                  │
│                                                  │
│  该分区已有一个“宠物名字”字段（当前为“昵称”）。    │
│                                                  │
│  第二个同语义类型的字段不会被任何插件自动识别。    │
│  只有高级用户才需要这样做。                       │
│                                                  │
│  [ 继续创建（高级用户）]  [ 取消 ]                 │
└─────────────────────────────────────────────────┘
```

### 8.3 字段编辑器（高级模式）

全局设置"显示技术标识符"开关开启后：

```
昵称
─────────────────────
标识符：auto_a3f7d2e1
语义类型：🐕 宠物名字 (pet.name) [修改]
敏感度：公开 🔽
类型：文本
```

---

## 9. 审计日志规范

所有与字段和插件相关的重要操作必须记录审计日志。

| 事件 | 记录内容 | 存储位置 |
|------|---------|---------|
| **字段敏感度修改** | 字段显示标签、机器 key、旧敏感度、新敏感度、操作时间、操作者 | `~/.solosoul/audit/field_audit.log` |
| **字段语义类型修改** | 字段显示标签、机器 key、旧语义类型、新语义类型、操作时间 | `~/.solosoul/audit/field_audit.log` |
| **插件安装授权** | 插件 ID、每个字段的授权决策（同意/拒绝/掩码/降级）、用户选择的策略、时间戳 | `~/.solosoul/audit/plugin_audit.log` |
| **运行时敏感度超出** | 插件 ID、字段显示标签、机器 key、实际敏感度、所需敏感度、用户当时的选择策略、时间戳 | `~/.solosoul/audit/plugin_audit.log` |
| **插件字段访问被拒绝** | 插件 ID、字段标识、拒绝原因（未声明/敏感度超出）、时间戳 | `~/.solosoul/audit/plugin_audit.log` |
| **插件字段访问掩码返回** | 插件 ID、字段标识、掩码类型、时间戳 | `~/.solosoul/audit/plugin_audit.log` |
| **插件级字段映射变更** | 插件 ID、语义类型、旧映射 key、新映射 key、时间戳 | `~/.solosoul/audit/plugin_audit.log` |

**日志格式**：JSON Lines，每行一个 JSON 对象

```json
{"timestamp":"2026-05-22T10:30:00Z","event":"field_sensitivity_changed","field_key":"auto_a3f7d2e1","field_label":"昵称","old_sensitivity":"public","new_sensitivity":"sensitive","account_id":"..."}
{"timestamp":"2026-05-22T10:35:00Z","event":"plugin_field_access_denied","plugin_id":"com.solosoul.pet-reminder","field_key":"auto_a3f7d2e1","field_label":"昵称","reason":"sensitivity_exceeded","required":"internal","actual":"sensitive"}
```

---

## 10. 多语言融合

| 元素 | 处理方式 |
|------|---------|
| **语义类型库** | 每个类型自带多语言标签（zh/en/ja/...），App 根据当前 locale 显示 |
| **用户字段名称** | 用户母语输入，无需翻译 |
| **插件请求** | 插件使用标准化英文 ID（如 `pet.name`），用户完全看不到 |
| **插件安装向导** | 根据当前 locale 显示语义类型的本地化名称 |
| **预定义字段** | 保持现有 ARB 翻译体系，不受影响 |
| **机器 key** | 全局默认隐藏，审计日志和高级模式可见 |
| **敏感度标签** | 通过 ARB 翻译：`public`→"公开"、`sensitive`→"敏感" 等 |

---

## 11. 实施路线图

### Phase 1：语义类型基础设施（7 天）

**Dart 模型**：
- [ ] `PropertyDefinition` 增加 `semanticType: String?`、`isAutoKey: bool`
- [ ] 新增 `SemanticFieldType` 和 `SemanticTypeRegistry`（50 个预定义类型）
- [ ] 机器 key 生成器：`generateMachineKey()`
- [ ] `UnifiedObject` 序列化支持 `__semanticTypes`
- [ ] 新增 `PluginFieldMapping` 模型和持久化
- [ ] 敏感度解析工具函数 + 数值映射 `repr(u8)`

**Rust Host**：
- [ ] 新增 `extract_by_semantic_type()` 函数（含敏感度读取）
- [ ] 新增 `get_sections_with_semantic_type()` 函数
- [ ] 新增 `resolve_field_access()` 统一入口
- [ ] 实现语义类型缓存（每次插件执行前重建一次）
- [ ] 实现敏感度校验逻辑
- [ ] 单元测试覆盖

### Phase 2：UI 改造（7 天）

- [ ] 重构 `ObjectEditorPage._PropertyFieldRow`
  - 隐藏机器 key（默认）/ 高级模式显示
  - 语义类型 Chip（可点击修改）
  - 显示标签输入框
  - 敏感度选择器（继承默认值但可覆盖）
- [ ] 新增 `SemanticTypePickerSheet` 组件
- [ ] 字段创建向导（两步骤 + 智能推荐 + 敏感度确认）
- [ ] 语义类型重复绑定提示弹窗
- [ ] 插件安装审查弹窗 `PluginAccessReviewDialog`
  - 字段解析 + 敏感度比较
  - 状态表格（符合/超出/缺失）
  - ⓘ 图标显示机器 key
  - 用户决策按钮
- [ ] 插件运行时超出弹窗 `PluginSensitivityOverrideDialog`
- [ ] 一键创建缺失字段引导

### Phase 3：插件生态（4 天）

- [ ] 更新 manifest schema：统一 `field_access` 列表
- [ ] 旧 manifest 兼容性转换逻辑
- [ ] 更新 Rust SDK：
  - `get_field("section_id.semantic://pet.name")`
  - `get_sections_with_semantic_type("pet.name")`
- [ ] 更新官方插件文档
- [ ] 新增示例插件：宠物提醒（演示语义类型 + 敏感度审查）

### Phase 4：审计与测试（4 天）

- [ ] 实现审计日志系统（JSON Lines 格式）
- [ ] 现有数据迁移测试
- [ ] 预定义字段回归测试
- [ ] 插件权限流测试（含敏感度超出场景）
- [ ] 性能基准测试
- [ ] 多语言 UI 测试
- [ ] 旧插件兼容性测试
- [ ] 审计日志完整性测试

---

## 12. 风险评估与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 语义类型库不够全面 | 中 | 初始 50 个覆盖 80% 场景；`custom.untyped` 兜底；支持热更新扩展 |
| 用户不理解"语义类型" | 中 | 智能推荐降低选择负担；提供"跳过"选项；图标 + 自然语言描述 |
| 同一 section 内重复绑定语义类型 | 低 | 允许但警告；插件识别第一个；高级用户自行处理 |
| 插件依赖的语义类型在旧版本 App 中不存在 | 低 | manifest 声明 `required_semantic_type_versions`；安装时检查并提示更新 |
| 敏感度审查弹窗过于频繁 | 中 | 安装时一次性审查，运行时仅对未处理项弹出；提供"记住选择"选项 |
| 旧插件兼容性断裂 | 低 | 自动转换 `required_fields` 为 `field_access`（默认 sensitive）；弹出兼容性警告 |
| 性能退化 | 极低 | O(K) 查找 + 执行期缓存；K 通常 < 20 |
| 用户恶意降低敏感度以安装插件 | 中 | 审计日志记录所有敏感度修改；运行时超出访问记录详细日志 |
| 审计日志膨胀 | 低 | JSON Lines 格式，按日期轮转；仅保留 90 天 |

---

## 13. 待决策事项（已全部确定）

| 事项 | 决策 | 理由 |
|------|------|------|
| 机器 key 生成策略 | `auto_{uuid_v4前8位}` | 全局唯一、无意义、天然不与预定义 key 冲突 |
| 机器 key 可见性 | **默认隐藏，特定场景可展开** | 审计/调试/插件审查需要可验证性；普通用户不增加认知负担 |
| 预定义字段是否加语义类型 | **Phase 1 不加** | 保持现有路径稳定；未来可增量添加 |
| 同一 section 内重复语义类型 | **允许，但警告** | 少数场景用户需要；插件识别第一个 |
| 语义类型查找范围 | **必须限定 section** | 避免多 section 歧义；支持 `get_sections_with_semantic_type()` 辅助选择 |
| 语义类型库维护方式 | **Phase 1 硬编码，Phase 3 CDN 热更新** | 快速启动 + 长期可扩展 |
| 插件同时支持两种声明 | **支持，`field_access` 和 `required_fields` 共存** | 新旧插件兼容，混合声明灵活 |
| 语义类型作为可选增强 | **确认，基础模型不变** | 即使去掉语义类型，key+label 体系仍然完整 |
| 敏感度审查机制 | **安装时审查 + 运行时校验** | 最小权限原则；用户完全控制权 |
| 旧插件敏感度默认值 | **sensitive（最严格）** | 安全优先；用户明确授权后才放宽 |
| 用户授权策略 | **Deny / AllowButMask / AllowWithWarning** | 满足不同安全需求 |
| 插件级映射优先级 | **plugin_mappings > __semanticTypes** | 用户主动指定应覆盖默认绑定；不修改 Section schema |
| 缓存失效策略 | **每次插件执行前重建一次** | 数据量小，开销可忽略；无需复杂增量失效逻辑 |
| 敏感度数值映射 | **repr(u8): Public=0, Internal=1, Sensitive=2, Critical=3** | 数值比较安全、明确 |

---

## 14. 附录

### 14.1 与上一版方案对比

| 维度 | 语义化字段标识（已否决） | 字段语义契约 v3.1（本方案） |
|------|----------------------|---------------------|
| 存储 key 稳定性 | ❌ 用户改标签 = 改 key | ✅ 机器 key 锁定（auto_xxx） |
| 插件路径稳定性 | ❌ 低 | ✅ 极高（semantic type 不变） |
| 通用插件可行性 | ❌ 不可行 | ✅ 插件请求语义类型 |
| 性能 | ❌ O(N) 扫描 | ✅ O(K)，K<20，加缓存 |
| 用户学习成本 | ✅ 无（但代价高） | ✅ 无（智能推荐 + 可跳过） |
| 实现复杂度 | 极高 | 中等 |
| 多语言支持 | 表面支持，实际混乱 | ✅ 语义类型自带 i18n |
| 向后兼容 | 混乱 | ✅ 预定义字段完全不变 |
| 与极简方案的关系 | 完全替代 | ✅ 可选增强，基础模型不变 |
| 敏感度安全 | ❌ 无机制 | ✅ 显式声明 + 审查 + 运行时校验 + 审计日志 |
| 机器 key 可验证性 | ❌ 完全隐藏 | ✅ 默认隐藏，审计/审查/高级模式可见 |

### 14.2 与极简方案（Key锁定+标签可编辑）的对比

| 维度 | 极简方案 | 语义契约方案 |
|------|---------|------------|
| 用户自定义字段对插件可见性 | 插件要求用户创建特定 key | 插件声明语义类型，用户选择绑定 |
| 插件读取自定义字段可靠性 | 依赖用户正确输入 key | 依赖系统保证绑定关系 |
| 通用插件编写难度 | 高 | 低 |
| 用户学习成本 | 较低 | 中等（通过 UI 优化可降低） |
| 实现复杂度 | 低 | 中 |
| 是否适合第三方插件市场 | 不适合 | 适合 |
| 敏感度安全控制 | 无 | 显式声明 + 用户审查 + 审计日志 |
| 可验证性/可调试性 | 低 | 高（机器 key 在审计/审查中可见） |

### 14.3 相关文件清单

| 文件 | 变更 | 说明 |
|------|------|------|
| `flutter/lib/core/models/unified_object_model.dart` | 修改 | `PropertyDefinition` 增加 `semanticType`、`isAutoKey` |
| `flutter/lib/core/models/semantic_type_registry.dart` | 新增 | 语义类型注册表 + 50 个预定义类型 |
| `flutter/lib/core/models/plugin_field_mapping.dart` | 新增 | 插件级字段映射模型 |
| `flutter/lib/core/services/machine_key_generator.dart` | 新增 | UUID 机器 key 生成器 |
| `flutter/lib/core/services/audit_log_service.dart` | 新增 | 审计日志服务（JSON Lines） |
| `flutter/lib/presentation/pages/object_editor_page.dart` | 修改 | 字段编辑器增加语义类型 Chip + 选择弹窗 + 敏感度选择器 + 高级模式开关 |
| `flutter/lib/presentation/widgets/semantic_type_picker.dart` | 新增 | 语义类型选择弹窗 |
| `flutter/lib/presentation/widgets/plugin_access_review_dialog.dart` | 新增 | 插件安装审查弹窗（含 ⓘ 图标展开机器 key） |
| `flutter/lib/presentation/widgets/plugin_sensitivity_override_dialog.dart` | 新增 | 运行时敏感度超出弹窗 |
| `flutter/lib/presentation/widgets/semantic_type_duplicate_warning.dart` | 新增 | 语义类型重复绑定提示 |
| `flutter/lib/presentation/widgets/plugin_field_setup_wizard.dart` | 修改 | 增加敏感度比较、审查流程、一键创建缺失字段 |
| `flutter/native/src/plugin/host.rs` | 修改 | 增加 `extract_by_semantic_type()` + 敏感度校验 + 缓存 |
| `flutter/native/src/plugin/access_control.rs` | 新增 | 字段访问控制 + 敏感度比较逻辑 + 策略处理 |
| `flutter/native/src/plugin/manifest.rs` | 修改 | 统一 `field_access` 列表，增加敏感度声明 |
| `SoloSoul_plugin_market/SDK/schema/manifest.schema.json` | 修改 | manifest schema 更新为 v1.2 |
| `SoloSoul_plugin_market/SDK/rust/src/lib.rs` | 修改 | SDK 增加语义类型读取 + 敏感度处理封装 |
