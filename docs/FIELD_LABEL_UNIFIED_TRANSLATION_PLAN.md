# 字段标签统一翻译方案：Section 前缀精确化

> 状态：已设计，待实施  
> 相关 Plan：`/Users/zzc/.kimi/plans/shadowcat-sam-alexander-quasar.md`

---

## 一、问题诊断

### 1.1 现象

授权对话框中 section 前缀与页面/Sidebar 分区名称不一致。例如：

- `identity.number` → 对话框显示"**身份**.号码"，但页面分区是"**身份信息**"
- `idCard.number` → 对话框显示"**身份证**.号码"，但页面分区是"**身份证件**"
- `employment.company` → 对话框显示"**职业**.公司"，但页面分区是"**就业**"

### 1.2 根因：三套独立翻译体系

系统中存在三套相互独立的 ARB 翻译体系：

| 体系 | 使用位置 | ARB 命名示例 | identity 中文 | idCard 中文 |
|------|---------|-------------|--------------|------------|
| **体系 A** | Sidebar / 页面标题 | `profileIdentity`、`profileIdentityDocuments` | **身份信息** | **身份证件** |
| **体系 B** | 模板选择页面 | `templateProfileIdentityName`、`templateProfileIdCardName` | **身份信息** | **身份证件** |
| **体系 C** | 授权对话框 section 前缀 | `sectionIdentity`、`sectionIdCard` | **身份** | **身份证** |

体系 C 与 A/B 不一致，且独立维护导致无法自动同步。

### 1.3 页面名 vs 分区名的混淆

上一版方案（以 `SectionRendererRegistry.l10nTitle` 为来源）解决了预设 section 的不一致问题，但引入了新问题：

| 字段路径 | 显示 | 问题 |
|---------|------|------|
| `financial.accountNumber` | **财务**.账号 | `financial` 是**页面名**，实际分区是 `financial_bank_account`（**银行账户**） |
| `financial.taxIdNumber` | **财务**.税号 | 实际分区是 `financial_tax_id`（**税务识别号**） |
| `digitalAccounts.email` | **数字账户**.邮箱 | 实际应属于 `profile_contact`（**联系信息**） |

**核心诉求**：前缀必须是**分区名**，不能是页面名。

### 1.4 根因：字段路径约定与 `fieldPrefix` 不完全一致

`SectionRendererRegistry` 中 `financial` 相关的 preset：

```dart
'financial_bank_account': fieldPrefix: 'bankAccount',  // 分区：银行账户
'financial_card':         fieldPrefix: 'card',         // 分区：卡片
'financial_tax_id':       fieldPrefix: 'taxId',        // 分区：税务识别号
```

但插件/Vault 中使用的字段路径是 `financial.accountNumber`、`financial.taxIdNumber`，第一段是页面名 `financial`，不是分区名 `bankAccount`/`taxId`。

因此 `getSectionLabelByFieldPrefix('financial')` 返回 null，fallback 到 `sectionFinancial` → "财务"。

---

## 二、方案设计

### 2.1 核心思路

> **利用 `SemanticTypeRegistry` 已有的字段路径 → 语义类型映射，从语义类型推断分区。**
>
> `SemanticTypeRegistry._fieldPathToSemanticType` 已经将 `financial.accountNumber` 映射到 `financial.account_number`。语义类型的第二段（`account_number`、`tax_id`、`card_number` 等）可以精确推断所属分区。

### 2.2 解析链路

```
_getFieldDisplayName('financial.accountNumber')
  ├─ fieldLabel = FieldLabelResolver.resolve('financial.accountNumber')
  │   └─ "账号"
  │
  ├─ sectionKey = 'financial'
  ├─ SectionRendererRegistry.getSectionLabelByFieldPrefix('financial')
  │   └─ 无匹配（没有 fieldPrefix: 'financial'）
  │
  ├─ SemanticTypeRegistry.resolveByFieldPath('financial.accountNumber')
  │   └─ 静态映射: 'financial.accountNumber' → 'financial.account_number'
  │   └─ 语义类型: SemanticFieldType(id: 'financial.account_number', ...)
  │
  ├─ 提取语义类型第二段: 'account_number'
  ├─ _semanticSuffixToFieldPrefix['account_number'] → 'bankAccount'
  │
  ├─ SectionRendererRegistry.getSectionLabelByFieldPrefix('bankAccount')
  │   └─ 匹配 preset: 'financial_bank_account' → l10n.financialBankAccounts
  │   └─ "银行账户"
  │
  └─ 返回: "银行账户.账号"  ✅
```

同样：
- `financial.taxIdNumber` → 语义类型 `financial.tax_id` → 第二段 `tax_id` → `taxId` → "税务识别号".税号 ✅
- `financial.cardNumber` → 语义类型 `financial.card_number` → 第二段 `card_number` → `card` → "卡片".卡号 ✅

### 2.3 具体实施步骤

#### 步骤 1：新增语义后缀 → `fieldPrefix` 映射

在 `SectionRendererRegistry` 中新增：

```dart
/// 语义类型 ID 的第二段 → fieldPrefix 映射。
///
/// 当字段路径第一段是页面名（如 `financial`）而非分区名（如 `bankAccount`）时，
/// 通过语义类型的第二段精确推断该字段属于哪个分区。
static const Map<String, String> _semanticSuffixToFieldPrefix = {
  // financial → bankAccount
  'account_number': 'bankAccount',
  'bank_name': 'bankAccount',
  'swift_code': 'bankAccount',
  'iban': 'bankAccount',
  // financial → card
  'card_number': 'card',
  'card_cvv': 'card',
  'card_expiry': 'card',
  // financial → taxId
  'tax_id': 'taxId',
  // 按需扩展
};
```

#### 步骤 2：新增 `getSectionLabelForFieldPath`

```dart
/// 根据完整字段路径获取对应的分区显示标签。
///
/// 优先直接匹配 [fieldPrefix]；若字段路径使用页面名前缀（如 `financial.*`），
/// 则通过 [SemanticTypeRegistry] 的语义类型映射推断实际分区。
static String? getSectionLabelForFieldPath(
  String fieldPath,
  AppLocalizations l10n,
) {
  // 1. 直接匹配 fieldPrefix（如 identity.*、passport.* 等）
  final prefix = fieldPath.split('.').first;
  final direct = getSectionLabelByFieldPrefix(prefix, l10n);
  if (direct != null) return direct;

  // 2. 通过语义类型推断分区（处理 financial.* 等页面级前缀）
  final semanticType = SemanticTypeRegistry.resolveByFieldPath(fieldPath);
  if (semanticType != null) {
    final suffix = semanticType.id.split('.').last;
    final inferredPrefix = _semanticSuffixToFieldPrefix[suffix];
    if (inferredPrefix != null) {
      return getSectionLabelByFieldPrefix(inferredPrefix, l10n);
    }
  }

  return null;
}
```

#### 步骤 3：修改 `_getFieldDisplayName`

```dart
String _getFieldDisplayName(BuildContext context, String fieldId) {
  final l10n = AppLocalizations.of(context);
  final label = FieldLabelResolver.resolve(fieldId);

  if (fieldId.contains('.')) {
    // 使用 getSectionLabelForFieldPath 替代 getSectionLabelByFieldPrefix
    final sectionLabel = SectionRendererRegistry.getSectionLabelForFieldPath(
          fieldId, l10n,
        ) ??
        translateFieldLabel(fieldId.split('.').first, l10n);
    if (sectionLabel != formatFieldLabel(fieldId.split('.').first) &&
        sectionLabel != label) {
      return '$sectionLabel.$label';
    }
  }

  return label;
}
```

### 2.4 匹配情况验证

| 字段路径 | 语义类型 | 第二段 | 推断 fieldPrefix | 分区显示 |
|---------|---------|--------|----------------|---------|
| `identity.fullName` | `person.name` | `name` | 直接匹配 `identity` | **身份信息** |
| `passport.number` | `travel.passport_number` | `passport_number` | 直接匹配 `passport` | **护照** |
| `financial.accountNumber` | `financial.account_number` | `account_number` | `bankAccount` | **银行账户** |
| `financial.bankName` | `financial.bank_name` | `bank_name` | `bankAccount` | **银行账户** |
| `financial.taxIdNumber` | `financial.tax_id` | `tax_id` | `taxId` | **税务识别号** |
| `digitalAccounts.email` | `contact.email` | `email` | 无映射 | fallback → "数字账户" |

---

## 三、改动范围

| 文件 | 改动 |
|------|------|
| `presentation/widgets/section_renderer_registry.dart` | 新增 `_semanticSuffixToFieldPrefix` 映射表、新增 `getSectionLabelForFieldPath` 方法 |
| `presentation/widgets/plugin_consent_dialog.dart` | `_getFieldDisplayName` 改用 `getSectionLabelForFieldPath` |

---

## 四、唯一真理来源确认

```
字段路径
  ├─ 直接匹配 fieldPrefix ──→ SectionRendererRegistry.l10nTitle
  │                              （身份/护照/地址等大多数情况）
  │
  └─ 页面级前缀（financial.*）
       └─ SemanticTypeRegistry.resolveByFieldPath ──→ 语义类型
              └─ 提取第二段 ──→ _semanticSuffixToFieldPrefix
                     └─ 推断 fieldPrefix ──→ SectionRendererRegistry.l10nTitle
                            （银行账户/卡片/税务识别号）
```

**所有分区名称最终都收敛到 `SectionRendererRegistry.l10nTitle`**，不存在独立的页面级翻译来源。

---

## 五、测试验证清单

| 场景 | 期望结果 |
|------|---------|
| `identity.number` | **身份信息**.号码 |
| `idCard.number` | **身份证件**.号码 |
| `employment.company` | **就业**.公司 |
| `financial.accountNumber` | **银行账户**.账号 |
| `financial.taxIdNumber` | **税务识别号**.税号 |
| `passport.number` | **护照**.号码 |
| `contact.email` | **联系信息**.邮箱 |
| `address.street` | **地址**.街道 |
