# SoloSoul 本地搜索与 Vault 自动导入方案

> 状态：设计文档  
> 优先级：P1  
> 目标版本：v1.2.0  
> 基于：xiaoyaosearch 架构借鉴 + SoloSoul Unified Object Model  

---

## 1. 需求概述

在 SoloSoul Flutter 客户端中新增**本地内容搜索与智能导入**功能：

1. **本地搜索**：用户指定搜索路径（或默认热门路径），系统高效扫描本地文件，提取潜在个人信息（简历、证件、护照、银行文件等）。
2. **结构化预览**：扫描结果以 JSON 结构化展示，提供预览窗口供用户逐条确认/修改。
3. **一键导入 Vault**：用户确认后，自动创建对应的 `UnifiedObject`（Page / Section / Item），填充到 Vault 数据库并加密持久化。

**核心约束**：
- 完全本地执行，零网络依赖，符合零知识架构。
- 不引入 Python / FastAPI / 向量索引等重型依赖。
- 复用现有 FRB + Vault 体系，不破坏安全模型。

---

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Flutter UI 层 (Dart)                               │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│  │ 搜索配置页面  │ → │ 扫描执行页面  │ → │ 预览确认页面  │ → │ 导入结果页 │ │
│  │(路径/类型/规则)│    │(进度/日志)   │    │(字段级确认)  │    │(成功/冲突) │ │
│  └──────────────┘    └──────────────┘    └──────────────┘    └───────────┘ │
│         │                   │                   │                           │
│         ▼                   ▼                   ▼                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              LocalSearchService (Dart, dart:io + Process)               ││
│  │   • 分层搜索策略（热门路径 → 元数据过滤 → 内容指纹）                     ││
│  │   • 平台命令适配（mdfind / where / find / everything）                  ││
│  │   • 文件内容解析（txt / json / csv / pdf / xlsx / docx）                ││
│  │   • 正则指纹匹配（身份证 / 手机号 / 邮箱 / 护照号 / 银行卡号）            ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼ ScanResult (JSON)                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              ScanImportService (Dart)                                   ││
│  │   • 映射到 ObjectTypeDefinition                                         ││
│  │   • 冲突检测（空字段填充 / 差异对比 / 新条目询问）                         ││
│  │   • 批量创建 UnifiedObject + PropertyValue                              ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼ List<UnifiedObject>                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              UnifiedObjectNotifier (Dart, Riverpod)                     ││
│  │   • createObject() / addChild()                                         ││
│  │   • _saveDebounced() → ProfileStorageService                            ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼ JSON bytes                                                        │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              RustVaultService → FRB / C FFI                             ││
│  │   • frbEncryptBytes() → AES-256-GCM                                     ││
│  │   • frbSaveProfile() → SQLite (profiles.data BLOB)                      ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

**关键决策**：搜索与解析纯 Dart 实现，不引入 Rust 侧改动（除非后续需要高性能解析器）。导入走现有的 `UnifiedObjectNotifier` 管线，无需新增 FRB 函数。

---

## 3. 搜索层设计（Dart 侧）

### 3.1 分层搜索策略（避免全盘扫描）

借鉴 xiaoyaosearch 的增量索引思想，但简化为**无索引的实时分层搜索**，适合低频、按需触发场景。

```dart
class LocalSearchService {
  /// 第一层：热门路径直击
  static const List<String> _kHotPaths = [
    '~/Documents',
    '~/Desktop',
    '~/Downloads',
  ];

  /// 第二层：元数据过滤（扩展名 + 文件名关键词）
  static const List<String> _kTargetExtensions = [
    '.pdf', '.docx', '.xlsx', '.csv', '.json', '.txt', '.md',
  ];
  static const List<String> _kFilenameKeywords = [
    'resume', 'cv', '简历', 'passport', '护照', 'id_card', '身份证',
    'bank', '银行', 'card', '证书', 'credential', 'profile',
  ];

  /// 第三层：内容指纹正则
  static final Map<String, RegExp> _kContentFingerprints = {
    'id_card': RegExp(r'[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]'),
    'phone': RegExp(r'1[3-9]\d{9}'),
    'email': RegExp(r'[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'),
    'passport': RegExp(r'[EG]\d{8}'), // 中国护照示例
    'bank_card': RegExp(r'\d{16,19}'),
  };
}
```

**搜索流程**：

```
开始搜索
  │
  ├── 第一层：枚举 _kHotPaths（~3 个目录）
  │     ├── macOS: mdfind -onlyin <path> <query>（毫秒级）
  │     ├── Windows: dir /s /b + 过滤，或 everything SDK
  │     └── Linux: find <path> -type f -name "*..."
  │
  ├── 第二层：扩展名 + 文件名关键词过滤
  │     └── 快速排除 90%+ 无关文件
  │
  ├── 第三层：按需内容指纹扫描
  │     └── 仅读取匹配目标扩展名的文件头/全文
  │         └── 正则命中则保留，未命中则丢弃
  │
  └── 输出：List<ScannedFile>
```

### 3.2 平台适配命令

| 平台 | 机制 | 命令 / API |
|------|------|-----------|
| macOS | Spotlight CLI | `mdfind -onlyin <path> "kMDItemTextContent == '*resume*'"` |
| macOS | 文件枚举 | `find <path> -type f \( -iname '*.pdf' -o -iname '*.docx' \)` |
| Windows | PowerShell | `Get-ChildItem -Recurse -Include *.pdf,*.docx` |
| Windows | Everything SDK | 通过 `everything.dll` 或 CLI `es.exe` |
| Linux | find / locate | `find <path> -type f -iname '*.pdf'` |

> **注意**：`mdfind` 和 Everything 都利用系统已有索引，无需自建索引，避免全盘扫描。

### 3.3 文件内容解析器（Dart 实现）

```dart
abstract class ContentParser {
  /// 返回纯文本内容（用于正则匹配）
  Future<String?> extractText(String filePath);
}

class TextContentParser implements ContentParser { /* txt/md/json/csv */ }
class PdfContentParser implements ContentParser { /* pdf: 调用 pdf_text 包或 Rust FFI */ }
class OfficeContentParser implements ContentParser { /* docx/xlsx: 调用 archive + xml 解析 */ }
```

**解析策略（借鉴 xiaoyaosearch 的分块思想）**：
- 文本文件：一次性读取（限制 1MB）。
- PDF / Office：逐页/逐 sheet 提取，每块限制 5000 字符，优先匹配前 3 块（大多数个人信息在文档前半部分）。
- 超大文件（>10MB）：仅读取前 1MB，跳过。

### 3.4 增量扫描支持（可选优化）

记录每次扫描的 `{path, mtime, size, contentHash}` 到 SQLite 或 `flutter_secure_storage`，下次扫描时：
- `mtime` + `size` 未变 → 跳过内容读取。
- 仅对变更文件重新解析。

这与 xiaoyaosearch 的 `scan_changes()` 机制一致，但存储在 Vault metadata 表中。

---

## 4. 数据模型设计

### 4.1 ScanResult（扫描结果 JSON）

```dart
/// 顶层扫描结果
@freezed
class ScanResult with _$ScanResult {
  const factory ScanResult({
    required ScanMeta meta,
    required List<ScanSection> sections,
  }) = _ScanResult;

  factory ScanResult.fromJson(Map<String, dynamic> json) =>
      _$ScanResultFromJson(json);
}

/// 扫描元信息
@freezed
class ScanMeta with _$ScanMeta {
  const factory ScanMeta({
    required String scanId,          // UUID v4
    required int createdAt,          // Unix timestamp (ms)
    required String sourceFile,      // 原始文件路径
    required double confidence,      // 0.0 - 1.0（正则命中项 / 总字段数）
    String? fileType,                // pdf / docx / xlsx / txt
  }) = _ScanMeta;

  factory ScanMeta.fromJson(Map<String, dynamic> json) =>
      _$ScanMetaFromJson(json);
}

/// 扫描出的 Section（对应 Vault 中的 Section）
@freezed
class ScanSection with _$ScanSection {
  const factory ScanSection({
    required String section,         // 机器标识：identity / education / passport / bank
    required String display,         // 人-readable：个人信息 / 教育经历 / 护照信息 / 银行账户
    required List<ScanField> fields,
  }) = _ScanSection;

  factory ScanSection.fromJson(Map<String, dynamic> json) =>
      _$ScanSectionFromJson(json);
}

/// 扫描出的单个字段
@freezed
class ScanField with _$ScanField {
  const factory ScanField({
    required String key,             // 机器标识：fullName / idCard / institution
    required String value,           // 提取到的值
    required SensitivityLevel sensitivity,
    double? confidence,              // 该字段单独的可信度
  }) = _ScanField;

  factory ScanField.fromJson(Map<String, dynamic> json) =>
      _$ScanFieldFromJson(json);
}
```

### 4.2 示例 JSON 输出

```json
{
  "meta": {
    "scanId": "scan-a1b2c3d4",
    "createdAt": 1714800000000,
    "sourceFile": "/Users/xxx/Documents/resume.pdf",
    "confidence": 0.92,
    "fileType": "pdf"
  },
  "sections": [
    {
      "section": "identity",
      "display": "个人信息",
      "fields": [
        { "key": "fullName", "value": "张三", "sensitivity": "public", "confidence": 0.95 },
        { "key": "idCard", "value": "110101199001011234", "sensitivity": "critical", "confidence": 0.99 },
        { "key": "phone", "value": "13800138000", "sensitivity": "sensitive", "confidence": 0.98 },
        { "key": "email", "value": "zhangsan@example.com", "sensitivity": "internal", "confidence": 0.97 }
      ]
    },
    {
      "section": "education",
      "display": "教育经历",
      "fields": [
        { "key": "institution", "value": "清华大学", "sensitivity": "public", "confidence": 0.90 },
        { "key": "degree", "value": "硕士", "sensitivity": "public", "confidence": 0.88 },
        { "key": "major", "value": "计算机科学与技术", "sensitivity": "public", "confidence": 0.85 }
      ]
    },
    {
      "section": "passport",
      "display": "护照信息",
      "fields": [
        { "key": "passportNumber", "value": "E12345678", "sensitivity": "critical", "confidence": 0.99 },
        { "key": "nationality", "value": "CHN", "sensitivity": "public", "confidence": 1.0 }
      ]
    }
  ]
}
```

### 4.3 与 SoloSoul 现有模型映射

| ScanResult 层级 | SoloSoul 映射 |
|-----------------|--------------|
| `ScanResult` | 一次扫描任务，对应一个导入会话 |
| `ScanSection.section` | `ObjectTypeDefinition.id`（如 `profile_identity`, `travel_passport`） |
| `ScanSection.display` | `UnifiedObject.name`（对象名称） |
| `ScanField.key` | `PropertyDefinition.id` |
| `ScanField.value` | `PropertyValue.text` / `PropertyValue.number` 等 |
| `ScanField.sensitivity` | `PropertyValue.sensitivity`（需映射到 `sensitivity_enums.dart` 的枚举） |

---

## 5. 导入管线设计

### 5.1 导入流程（4 步）

```
扫描完成
  │
  ├── 步骤 1：字段映射（ScanImportService）
  │     ├── 根据 section 查找/匹配 ObjectTypeDefinition
  │     ├── 根据 key 查找/匹配 PropertyDefinition
  │     └── 未匹配字段标记为 "待手动分配"
  │
  ├── 步骤 2：冲突检测
  │     ├── 空字段 + confidence > 0.8 → 自动填充（绿色标记）
  │     ├── 已有值且相同 → 跳过（灰色标记）
  │     └── 已有值且不同 → 冲突（红色标记，用户必须选择）
  │
  ├── 步骤 3：用户确认（UI 预览页面）
  │     ├── 逐 Section 展示卡片
  │     ├── 每字段显示：扫描值 → 目标字段 → 敏感度标签
  │     └── 用户可：全选/取消、修改值、修改目标字段、删除字段
  │
  └── 步骤 4：批量写入 Vault
        ├── 对每个确认的 Section：
        │   ├── 若 Vault 中无此 Section → 创建新 UnifiedObject
        │   ├── 若已存在 → 更新 properties（YMap insert）
        │   └── 记录操作日志（OperationEntry）
        ├── UnifiedObjectNotifier._saveDebounced() → 加密持久化
        └── 返回 ScanImportResult
```

### 5.2 ScanImportService（核心 Dart 类）

```dart
class ScanImportService {
  final UnifiedObjectNotifier _objectNotifier;
  final FieldRegistry _fieldRegistry;

  ScanImportService(this._objectNotifier, this._fieldRegistry);

  /// 将 ScanResult 映射为待导入的候选对象列表
  Future<List<ImportCandidate>> mapScanResult(ScanResult result);

  /// 检测与现有 Vault 数据的冲突
  Future<List<ImportConflict>> detectConflicts(List<ImportCandidate> candidates);

  /// 执行导入（用户确认后调用）
  Future<ScanImportResult> executeImport(
    List<ImportCandidate> confirmedCandidates, {
    ConflictResolution defaultResolution = ConflictResolution.skip,
  });
}

class ImportCandidate {
  final ScanSection source;
  final UnifiedObject? existingObject;  // null = 需要新建
  final List<ImportFieldCandidate> fields;
  bool isSelected;
}

class ImportFieldCandidate {
  final ScanField source;
  final PropertyDefinition? targetProperty;
  final PropertyValue? existingValue;
  final ImportAction suggestedAction;  // autoFill / skip / conflict
  ImportAction userAction;             // 用户最终选择
}

enum ImportAction { autoFill, skip, overwrite, createNew }

class ScanImportResult {
  final int itemsCreated;
  final int itemsUpdated;
  final int fieldsWritten;
  final int fieldsSkipped;
  final List<String> warnings;
}
```

### 5.3 写入 Vault 的具体逻辑

**创建新对象**：
```dart
final newObject = await _objectNotifier.createObject(
  typeId: _mapSectionToTypeId(section.section), // e.g., 'travel_passport'
  name: section.display,
  parentId: targetPageId,  // e.g., '__page_travel'
  properties: {
    for (final f in confirmedFields)
      f.targetProperty.id: PropertyValue.text(
        text: f.source.value,
        sensitivity: f.source.sensitivity,
      ),
  },
);
```

**更新现有对象**：
```dart
final updated = existingObject.copyWith(
  properties: {
    ...existingObject.properties,
    for (final f in confirmedFields)
      f.targetProperty.id: PropertyValue.text(
        text: f.source.value,
        sensitivity: f.source.sensitivity,
      ),
  },
  updatedAt: DateTime.now().millisecondsSinceEpoch,
);
_objectNotifier.updateObject(updated);
```

> 自动保存：`_saveDebounced()` 会在 300ms 后自动触发 `ProfileStorageService.saveProfile()`，无需手动调用。

---

## 6. UI 页面设计

### 6.1 页面流程

```
HomePage / ObjectWorkspacePage
  │
  ├── 点击 "本地搜索导入" FAB / 菜单项
  │
  ▼
┌─────────────────────┐
│  LocalSearchConfigPage │  搜索配置
│  ───────────────────  │
│  [ ] 使用默认热门路径  │  ━━ ~/Documents, ~/Desktop, ~/Downloads
│  [ ] 自定义路径        │  ━━ 文件夹选择器
│  ───────────────────  │
│  文件类型：            │
│  [✓] PDF  [✓] Word   │
│  [✓] Excel [✓] TXT   │
│  ───────────────────  │
│  扫描深度：            │
│  [ ] 仅文件名         │
│  [✓] 文件名+内容指纹  │
│  [ ] 全文解析（慢）    │
│  ───────────────────  │
│  [  开始扫描  ]        │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  LocalSearchProgressPage │  扫描进度
│  ───────────────────  │
│  扫描中... /Users/xxx/Documents │
│  [████████░░░░░░░░░░] 45%      │
│  已发现 12 个候选文件           │
│  命中 3 个有效文档              │
│  [  取消  ]                     │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  ScanPreviewPage    │  预览确认（核心页面）
│  ───────────────────  │
│  📄 resume.pdf        │
│  ┌─────────────────┐ │
│  │ 个人信息 (identity)│ │
│  │ 全名: 张三 ✓      │ │  ← 绿色 = 自动填充
│  │ 身份证: 1101... ✓ │ │  ← 关键字段 = 自动标记 critical
│  │ 手机: 138... ✓    │ │
│  └─────────────────┘ │
│  ┌─────────────────┐ │
│  │ 教育经历         │ │
│  │ 学校: 清华 ✓     │ │
│  │ 学位: 硕士 ⚠     │ │  ← 黄色 = Vault 已有不同值
│  └─────────────────┘ │
│  [✓] 全选当前文件    │
│  [  确认导入选中项  ] │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  ScanImportResultPage │ 导入结果
│  ───────────────────  │
│  ✅ 导入成功          │
│  新建 2 个条目        │
│  更新 1 个条目        │
│  写入 8 个字段        │
│  跳过 1 个冲突字段    │
│  [  前往查看  ] [关闭] │
└─────────────────────┘
```

### 6.2 预览页面关键交互

- **敏感度自动标记**：身份证、护照号 → `critical`；手机号 → `sensitive`；姓名/学校 → `public`。
- **冲突高亮**：若 Vault 中已有同名 section 且字段值不同，红色高亮并显示差异对比。
- **批量操作**：全选/反选、一键忽略所有冲突、一键接受所有自动填充。
- **敏感数据遮罩**：使用现有 `SensitiveValueWidget`，`critical` 字段默认遮罩，点击后通过 `password_verification_dialog.dart` 解锁。

---

## 7. 与现有代码的集成点

### 7.1 新增文件清单

| 文件 | 职责 |
|------|------|
| `flutter/lib/core/services/local_search_service.dart` | 本地搜索核心（命令调用、文件过滤、内容解析） |
| `flutter/lib/core/services/content_parser_service.dart` | 多格式文件内容提取 |
| `flutter/lib/core/services/scan_import_service.dart` | 扫描结果映射、冲突检测、批量导入 |
| `flutter/lib/core/models/scan_result_model.dart` | `ScanResult` / `ScanSection` / `ScanField` 模型（Freezed） |
| `flutter/lib/presentation/pages/local_search_config_page.dart` | 搜索配置页面 |
| `flutter/lib/presentation/pages/local_search_progress_page.dart` | 扫描进度页面 |
| `flutter/lib/presentation/pages/scan_preview_page.dart` | 扫描结果预览/确认页面 |
| `flutter/lib/presentation/pages/scan_import_result_page.dart` | 导入结果页面 |
| `flutter/lib/presentation/providers/local_search_provider.dart` | 搜索状态管理（Riverpod） |
| `flutter/lib/presentation/widgets/scan_section_card.dart` | 预览页面中的 Section 卡片 |
| `flutter/lib/presentation/widgets/scan_field_row.dart` | 预览页面中的字段行（含冲突标记） |

### 7.2 修改的现有文件

| 文件 | 修改内容 |
|------|---------|
| `flutter/lib/core/services/unified_object_service.dart` | 在 `_kBuiltinTypes` 中确认扫描目标类型已存在（`profile_identity`, `travel_passport`, `financial_bank_account` 等） |
| `flutter/lib/presentation/providers/unified_object_provider.dart` | 确保 `createObject()` / `updateObject()` 支持批量调用后的自动保存 |
| `flutter/lib/presentation/widgets/app_sidebar.dart` | 在菜单/FAB 中添加 "本地搜索导入" 入口 |
| `flutter/lib/presentation/router/app_router.dart` | 添加新页面路由 |

### 7.3 无需修改的文件（复用现有能力）

| 文件 | 复用能力 |
|------|---------|
| `flutter/lib/core/services/rust_vault_service.dart` | 加密保存复用 `frbEncryptBytes` + `frbSaveProfile` |
| `flutter/lib/core/services/profile_storage_service.dart` | 加载/保存 Profile 复用现有管线 |
| `flutter/lib/presentation/widgets/sensitive_value_widget.dart` | 敏感数据遮罩复用 |
| `flutter/lib/presentation/widgets/password_verification_dialog.dart` | 关键字段解锁复用 |
| `flutter/lib/presentation/models/sensitivity_models.dart` | 字段敏感度注册表复用/扩展 |

---

## 8. 安全与隐私考量

1. **扫描范围可控**：默认仅扫描 `~/Documents` 等热门路径，绝不自动扫描全盘或系统目录。
2. **内存安全**：扫描出的文本内容在预览后即丢弃，不持久化到临时文件。
3. **敏感数据不落地**：解析出的身份证号、护照号等仅在内存中流转，最终通过 FRB 加密后才写入 SQLite。
4. **用户最终控制权**：任何数据进入 Vault 前必须经过用户显式确认，禁止静默自动导入。
5. **操作日志**：每次导入生成 `OperationEntry`，记录来源文件、创建/更新的对象 ID、字段数量，支持审计。
6. **权限最小化**：文件读取使用 `dart:io` 标准文件 API，无需额外权限（除 macOS 沙盒需用户授权 Documents 目录）。

---

## 9. 实现步骤（Roadmap）

### Phase 1：基础扫描能力（P1，预计 3 天）
- [ ] 创建 `ScanResult` 数据模型（Freezed + JSON）。
- [ ] 实现 `LocalSearchService`：
  - [ ] macOS `mdfind` 命令封装。
  - [ ] `find` / `dir` 跨平台回退。
  - [ ] 扩展名 + 文件名关键词过滤。
- [ ] 实现基础 `ContentParserService`：
  - [ ] `.txt` / `.md` / `.json` / `.csv` 文本提取。
  - [ ] `.pdf` 文本提取（集成 `pdf_text` 包）。
- [ ] 实现正则指纹匹配器（身份证、手机号、邮箱、护照号、银行卡号）。
- [ ] 编写单元测试（表驱动，Mock 文件系统）。

### Phase 2：预览与导入管线（P1，预计 4 天）
- [ ] 实现 `ScanImportService`：
  - [ ] `mapScanResult()` → `ImportCandidate`。
  - [ ] `detectConflicts()` 对比现有 Vault 数据。
  - [ ] `executeImport()` 批量创建/更新 `UnifiedObject`。
- [ ] 实现 `LocalSearchConfigPage` + `LocalSearchProgressPage`。
- [ ] 实现 `ScanPreviewPage`（核心页面，含冲突高亮、敏感度标记）。
- [ ] 实现 `ScanImportResultPage`。
- [ ] 集成 `SensitiveValueWidget` 到预览字段。
- [ ] Widget 测试：预览页面渲染、冲突标记交互。

### Phase 3：Office 格式支持与优化（P2，预计 3 天）
- [ ] `.docx` 内容解析（`archive` + `xml` 包）。
- [ ] `.xlsx` 内容解析（`excel` 包或 CSV 回退）。
- [ ] 增量扫描缓存（记录 `{path, mtime, size}` 到 Vault metadata）。
- [ ] Windows Everything SDK 集成（性能优化）。
- [ ] 集成测试：端到端扫描 → 预览 → 导入 → Vault 验证。

### Phase 4：高级功能（P3，后续版本）
- [ ] AI 辅助字段映射（本地 LLM，如 Ollama，可选）。
- [ ] 更多文件格式（图片 OCR 复用现有 `OcrService`）。
- [ ] 扫描任务调度（定时增量扫描）。

---

## 10. 参考与借鉴

| xiaoyaosearch 设计 | SoloSoul 适配 |
|-------------------|--------------|
| `FileScanner` 多线程扫描 | Dart `async` + `Future.wait` 并发，隔离为 Isolate 避免阻塞 UI |
| `scan_changes()` 增量更新 | 记录 `{path, mtime, size}` 到 Vault metadata，下次扫描跳过未变更 |
| `ContentParser` 多格式解析 | Dart 包 + 平台命令，复用现有 `OcrService` 处理图片 |
| `ChunkIndexService` 三层索引 | **不引入** Faiss / Whoosh，改用系统索引（mdfind / Everything）+ 实时扫描 |
| `SearchResult` Pydantic 模型 | 改造为 `ScanResult` Freezed 模型，适配 `PropertyValue` 体系 |
| 混合搜索（语义+全文） | **第一阶段不做**，仅关键词+正则；后续可选本地嵌入模型 |

---

## 11. 结论

本方案通过**分层搜索策略**（系统索引 + 元数据过滤 + 内容指纹）避免了全盘扫描，通过**结构化 JSON 中间层**（`ScanResult` → `ScanSection` → `ScanField`）与 SoloSoul 的 `UnifiedObject` + `PropertyValue` 体系无缝衔接，通过**预览确认 → 批量导入**的流程确保用户始终拥有最终控制权。

全部实现可在 Dart 侧完成，零 Rust 改动，零后端依赖，完美契合 SoloSoul 本地优先、零知识、最小依赖的架构哲学。
