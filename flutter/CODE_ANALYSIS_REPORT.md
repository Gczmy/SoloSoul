# 代码分析修复报告

> 最后更新：2026-05-31 23:45:00
> 当前分支：`master`
> 修复轮次：1（初始分析）
> 分析范围：`lib/`（排除 `lib/frb/`、`lib/gen/`、`lib/data/`、`lib/domain/`）

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|---|---|---|---|---|---|
| P001 | P0 | 性能 | `lib/presentation/providers/unified_object_notifier.dart:453-466` | `updateObject` 循环内串行 `await deleteAttachment` | `[x]` 已修复 |
| P002 | P0 | 性能 | `lib/presentation/providers/unified_object_notifier.dart:651-699` | `permanentlyDeleteObject`/`permanentlyDeleteMultiple` 循环内串行 I/O | `[x]` 已修复 |
| P003 | P0 | 性能 | `lib/core/models/semantic_type_registry.dart:683-688` | `getType` 对 `_allTypes` 线性遍历 O(n)，应预构建 Map | `[x]` 已修复 |
| P004 | P0 | 性能 | `lib/core/models/semantic_type_registry.dart:795-803` | `recommend` 内 `results.contains(type)` 在循环内 O(m×n) | `[x]` 已修复 |
| P005 | P0 | 重复代码 | `lib/presentation/widgets/password_verification_dialog.dart` | 两个 State 类约 400+ 行重复（`PasswordVerificationDialogContentState` / `BiometricPasswordDialogContentState`） | `[ ]` 待修复（大重构） |
| P006 | P0 | 重复代码 | `lib/presentation/widgets/ocr_scanner_sheet.dart:360-578` | `_pickImage` / `_pickDocument` 90%+ OCR/MRZ/LLM 逻辑重复 | `[ ]` 待修复（大重构） |
| P007 | P1 | 过长函数 | `lib/presentation/pages/plugin_dashboard_page.dart:2006-2291` | `_onRun` 约 285 行，事件流处理 | `[ ]` 待修复 |
| P008 | P1 | 过长函数 | `lib/presentation/pages/object_editor_page.dart:1164-1331` | `_PropertyFieldRow.build` 约 167 行 | `[ ]` 待修复 |
| P009 | P1 | 过长函数 | `lib/presentation/widgets/object_card.dart:688-858` | `build` 约 170 行 | `[ ]` 待修复 |
| P010 | P1 | 过长函数 | `lib/presentation/widgets/ocr_scanner_sheet.dart:717-839` | `_performLlmExtraction` 约 122 行 | `[ ]` 待修复 |
| P011 | P1 | 过长函数 | `lib/presentation/pages/llm/llm_config_page.dart:207-420` | `build` 约 213 行 | `[ ]` 待修复 |
| P012 | P1 | 深层嵌套 | `lib/presentation/pages/plugin_dashboard_page.dart:2006-2291` | `_onRun` 内 `await for` > `switch` > `case` > `if` > `try` > `for` 达 6-7 层 | `[ ]` 待修复 |
| P013 | P1 | 深层嵌套 | `lib/presentation/providers/unified_object_notifier.dart:203-312` | `_migrateDefaultSectionSchemas` 嵌套达 5-6 层 | `[ ]` 待修复 |
| P014 | P1 | 深层嵌套 | `lib/presentation/pages/object_editor_page.dart:548-666` | `_saveObject` 嵌套达 6-7 层 | `[ ]` 待修复 |
| P015 | P1 | 重复代码 | `lib/presentation/pages/plugin_dashboard_page.dart:240-264 / 2550-2574` | 两个 `_getManifest` 方法完全重复 | `[x]` 已修复 |
| P016 | P1 | 重复代码 | `lib/presentation/widgets/object_card.dart:252-604` | `_saveNewItem` / `_saveEditItem` 日志/通知逻辑重复 | `[ ]` 待修复 |
| P017 | P1 | 重复代码 | `lib/presentation/providers/unified_object_notifier.dart:133-312` | `_createDefaultStructure` / `_migrateDefaultSectionSchemas` page/section 创建逻辑重复 | `[ ]` 待修复 |
| P018 | P1 | 性能 | `lib/presentation/providers/unified_object_notifier.dart:109` | `repairOrphanItems` 外层 for 内调用 `objects.any(...)`，O(n²) | `[x]` 已修复 |
| P019 | P1 | 性能 | `lib/core/services/unified_object_service.dart:720-735` | `getDescendantIds` 递归内每次重建 Map，O(n×depth) | `[x]` 误报/设计如此 — Map 在递归函数外部构建 |
| P020 | P1 | 性能 | `lib/core/services/rust_vault_service.dart:520-526` | `saveSettingEncrypted` 含诊断加密调用，每次保存多做一次加密 | `[x]` 已修复 |
| P021 | P2 | 重复代码 | `lib/presentation/pages/sync_page.dart:234-245 / 700-711` | `_hexToBytes` 在两个 State 中重复定义 | `[x]` 已修复 |
| P022 | P2 | 死代码 | `lib/presentation/pages/plugin_dashboard_page.dart:1773` | `_showAccessReview` 中 `locale` 变量获取后未使用 | `[x]` 已修复 |
| P023 | P2 | 死代码 | `lib/presentation/pages/plugin_dashboard_page.dart:1369-1390` | `_typeIcon` / `_typeLabel` 中 `case 'map':` 永远不会触发 | `[x]` 已修复 |
| P024 | P2 | 代理方法 | `lib/presentation/pages/trash_page.dart:374` | `_logSectionForTypeId` 直接代理顶层函数 | `[x]` 已修复 |
| P025 | P2 | 代理方法 | `lib/presentation/widgets/trash/unified_object_trash_card.dart:323` | `_typeColor` 直接代理 `typeColorForId` | `[x]` 已修复 |
| P026 | P2 | 缺失参数 | `lib/core/models/unified_object_model.dart:187` | `ObjectTypeDefinition.copyWith` 缺少 `titlePropertyKey` | `[x]` 已修复 |
| P027 | P2 | 硬编码 | `lib/presentation/pages/scan/scan_preview_page.dart:546` | `_ScanPreviewEmptyState` 标题/副标题硬编码英文 | `[x]` 已修复 |
| P028 | P2 | 性能 | `lib/presentation/widgets/object_card.dart:130-160` | `_template` getter 每次访问都创建新 Map | `[ ]` 待修复 |
| P029 | P2 | 性能 | `lib/core/services/llm/llm_config_service.dart:261-362` | 5 个 getter 重复 "获取 active profile" 逻辑 | `[ ]` 待修复 |
| P030 | P2 | 可简化 | `lib/core/services/unified_object_service.dart:884-998` | `getIconFromName` 114 行巨大 switch，可改为 Map 常量表 | `[ ]` 待修复 |
| P031 | P2 | 可简化 | `lib/core/models/semantic_type_registry.dart:388-471` | `_localizeKey` 83 行 switch，可改为 Map 常量表 | `[ ]` 待修复 |

---

## 修复进度

- 已完成：**14 / 31**
- 当前处理：**P005 — password_verification_dialog 共享 UI 组件化**

---

## 详细问题描述与修复指引

### P001 — `updateObject` 循环内串行 I/O

**文件**：`lib/presentation/providers/unified_object_notifier.dart:453-466`

**代码片段**：
```dart
for (final fileId in removedFileIds) {
  await AttachmentStorageService().deleteAttachment(
    accountId: accountId,
    fileId: fileId,
  );
}
```

**影响**：当对象有大量附件被移除时，串行删除导致显著延迟。每个 `await` 都阻塞事件循环。

**修复方案**：
```dart
await Future.wait(
  removedFileIds.map((fileId) => AttachmentStorageService().deleteAttachment(
    accountId: accountId,
    fileId: fileId,
  )),
);
```

---

### P002 — 永久删除时循环内串行 I/O

**文件**：`lib/presentation/providers/unified_object_notifier.dart:651-699`

**代码片段**：
```dart
// permanentlyDeleteObject
for (final a in object.attachments) {
  await AttachmentStorageService().deleteAttachment(...);
}
// permanentlyDeleteMultiple
for (final id in objectIds) { ... await deleteAttachments(...) ... }
```

**影响**：批量删除多个对象时，每个附件串行删除，性能极差。

**修复方案**：两处均改为 `Future.wait` 并行删除。注意错误处理：即使个别文件删除失败，也不应中断整个流程。

---

### P003 — `getType` 线性遍历 O(n)

**文件**：`lib/core/models/semantic_type_registry.dart:683-688`

**代码片段**：
```dart
SemanticFieldType? getType(String id) {
  for (final t in _allTypes) {
    if (t.id == id) return t;
  }
  return null;
}
```

**影响**：每次字段解析都线性扫描类型列表。虽然 `_allTypes` 不大，但在大规模对象解析时会累积。

**修复方案**：
```dart
static final Map<String, SemanticFieldType> _typeById = {
  for (final t in _allTypes) t.id: t,
};

SemanticFieldType? getType(String id) => _typeById[id];
```

---

### P004 — `recommend` 内 `contains` O(m×n)

**文件**：`lib/core/models/semantic_type_registry.dart:795-803`

**代码片段**：
```dart
for (final type in candidates) {
  if (!results.contains(type)) {
    results.add(type);
  }
}
```

**影响**：`results` 增长后 `contains` 变为线性扫描，嵌套循环导致 O(m×n)。

**修复方案**：使用 `Set<SemanticFieldType>` 收集结果。

---

### P005 — 密码验证对话框 400+ 行重复

**文件**：`lib/presentation/widgets/password_verification_dialog.dart`

**问题**：`PasswordVerificationDialogContentState` 和 `BiometricPasswordDialogContentState` 共享：
- `TextEditingController`, `FocusNode`
- `_errorMessage`, `_isVerifying`, `_hasError`
- `_onFocusChanged`, `_onTextChanged`, `_verify()`
- `_iconColor getter`
- build 中 80% 的 UI 结构

**修复方案**：
1. 提取 `BasePasswordDialogState<T extends StatefulWidget>` 抽象类
2. 将公共字段和方法下沉到基类
3. `BiometricPasswordDialogContentState` 只需扩展 biometric 按钮和 `_tryBiometric`

---

### P006 — OCR 扫描 `_pickImage` / `_pickDocument` 90% 重复

**文件**：`lib/presentation/widgets/ocr_scanner_sheet.dart:360-578`

**问题**：两个方法从 Step 1 (OCR) 到 Step 3 (LLM) 的代码几乎完全相同，仅输入源不同。

**修复方案**：
```dart
Future<void> _pickImage() async {
  final bytes = await _captureImageBytes();
  if (bytes != null) await _processOcrBytes(bytes, isPdf: false);
}

Future<void> _pickDocument() async {
  final bytes = await _pickDocumentBytes();
  if (bytes != null) await _processOcrBytes(bytes, isPdf: true);
}
```

---

### P007 — `_onRun` 285 行事件流处理

**文件**：`lib/presentation/pages/plugin_dashboard_page.dart:2006-2291`

**问题**：插件事件流处理包含巨大 switch-case 和多种内联逻辑。

**修复方案**：将每个 `case` 提取为独立方法：`_handleDialogRequest`, `_handleConsentRequest`, `_handleBatchEnd`, `_showResultDialog`。

---

### P008 — `_PropertyFieldRow.build` 167 行

**文件**：`lib/presentation/pages/object_editor_page.dart:1164-1331`

**修复方案**：提取 `_buildKeyInput`, `_buildTypeDropdown`, `_buildSensitivityMenu`, `_buildDeleteButton`。

---

### P009 — `object_card.dart` build 170 行

**文件**：`lib/presentation/widgets/object_card.dart:688-858`

**修复方案**：提取 `_buildHeader`, `_buildItemsList`, `_buildAddItemButton`, `_buildCollapseButton`。

---

### P010 — `_performLlmExtraction` 122 行

**文件**：`lib/presentation/widgets/ocr_scanner_sheet.dart:717-839`

**修复方案**：拆分为 `_activateModel`, `_buildExtractionPrompt`, `_parseLlmResponse`, `_restoreLlmConfig`。

---

### P011 — `llm_config_page.dart` build 213 行

**文件**：`lib/presentation/pages/llm/llm_config_page.dart:207-420`

**修复方案**：提取 `_buildBackendSection`, `_buildLocalSection`, `_buildCloudSection`, `_buildTestSection`。

---

### P012-P014 — 深层嵌套

分别对应 P007、P013（`_migrateDefaultSectionSchemas`）、P014（`_saveObject`）。

**通用修复方案**：
- 使用 early return（卫语句）减少嵌套
- 将循环体提取为独立方法
- 将复杂分支提取为私有方法

---

### P015 — `_getManifest` 重复定义

**文件**：`lib/presentation/pages/plugin_dashboard_page.dart:240-264 / 2550-2574`

**问题**：`_PluginDashboardPageState` 和 `_PluginCard` 中定义了两个完全相同的 `_getManifest`。

**修复方案**：提取为顶层私有函数 `_getManifestFromData`。

---

### P016 — `_saveNewItem` / `_saveEditItem` 重复

**文件**：`lib/presentation/widgets/object_card.dart:252-604`

**问题**：两者都包含：计算 name、调用保存、记录日志、显示通知。

**修复方案**：提取 `_saveItemAndLog({String? itemId, ...})` 统一流程。

---

### P017 — 默认结构创建逻辑重复

**文件**：`lib/presentation/providers/unified_object_notifier.dart:133-312`

**问题**：`_createDefaultStructure` 和 `_migrateDefaultSectionSchemas` 都包含创建默认 page/section 的相同逻辑。

**修复方案**：提取 `_createPageObject` 和 `_createSectionObject` 工厂方法。

---

### P018 — `repairOrphanItems` O(n²)

**文件**：`lib/presentation/providers/unified_object_notifier.dart:109`

**修复方案**：预构建 `Set<String> existingIds` 用于快速存在性判断。

---

### P019 — `getDescendantIds` 递归重建 Map

**文件**：`lib/core/services/unified_object_service.dart:720-735`

**修复方案**：将 `Map<String, UnifiedObject>` 作为参数传入递归辅助函数。

---

### P020 — `saveSettingEncrypted` 诊断代码影响性能

**文件**：`lib/core/services/rust_vault_service.dart:520-526`

**代码片段**：
```dart
final test = await frb.frbEncryptBytes(data: Uint8List.fromList([0]));
```

**修复方案**：移除或条件编译（仅在 debug 模式执行）。

---

### P021 — `_hexToBytes` 重复

**文件**：`lib/presentation/pages/sync_page.dart:234-245 / 700-711`

**修复方案**：提取为顶层工具函数 `List<int>? hexToBytes(String hex)`。

---

### P022 — 未使用变量 `locale`

**文件**：`lib/presentation/pages/plugin_dashboard_page.dart:1773`

**修复方案**：删除 `locale` 变量，同一方法内使用 `languageCode`。

---

### P023 — 不可达 `case 'map':`

**文件**：`lib/presentation/pages/plugin_dashboard_page.dart:1369-1390`

**修复方案**：移除 `'map'` case，或补全 `MapResultCard` 渲染器。

---

### P024-P025 — 代理方法

直接内联调用顶层函数即可。

---

### P026 — `copyWith` 缺少参数

**文件**：`lib/core/models/unified_object_model.dart:187`

**修复方案**：补全 `titlePropertyKey`。

---

### P027 — 硬编码英文

**文件**：`lib/presentation/pages/scan/scan_preview_page.dart:546`

**修复方案**：添加 ARB 键并替换。

---

### P028 — `_template` getter 每次重建 Map

**文件**：`lib/presentation/widgets/object_card.dart:130-160`

**修复方案**：缓存结果，在 `widget.object` 或 `widget.itemTemplate` 变化时重新计算。

---

### P029 — LLM 配置 5 个 getter 重复逻辑

**文件**：`lib/core/services/llm/llm_config_service.dart:261-362`

**修复方案**：提取 `_getActiveProfileOrFallback()` 辅助方法。

---

### P030-P031 — 巨大 switch 改为 Map

分别对应 `getIconFromName` 和 `_localizeKey`。

**修复方案**：
```dart
static const Map<String, IconData> _iconByName = {
  'work': Icons.work,
  'person': Icons.person,
  // ...
};
```
