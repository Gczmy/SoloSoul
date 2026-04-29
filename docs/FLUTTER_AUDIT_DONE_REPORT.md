# Flutter 代码库审计 — 修复完成记录

> 每次完成一个修复项后在此追加记录。

---

## 2026-04-29 修复批次

### C-1: login_page.dart — 空名字索引崩溃
- **修复**: `account.name[0].toUpperCase()` → `account.name.isNotEmpty ? account.name[0].toUpperCase() : '?'`
- **提交**: 含于本次推送

### H-1: main.dart — 缺少 ErrorWidget.builder
- **修复**: 在 `runApp()` 前配置 `ErrorWidget.builder`，记录错误日志并显示友好降级 UI
- **提交**: 含于本次推送

### H-2: search_result_tile.dart — 全量 watch accountStyleProvider
- **修复**: `ref.watch(accountStyleProvider)` → `ref.watch(accountStyleProvider.select((s) => s.value?.displayMode))`
- **提交**: 含于本次推送

### H-3: history_sheet.dart — flatten+sort 重复计算
- **修复**: 将 `HistoryChangeItem` 移到 `field_history_models.dart`，在 `FieldHistoriesNotifier` 中添加 `allChangesSorted` getter 预计算扁平排序列表；`history_sheet.dart` 直接读取，消除 FutureBuilder.builder 内三重嵌套循环
- **提交**: 含于本次推送

### H-5: data_management_page.dart — 3 处 dialog controller 泄漏
- **修复**: 将 `TextEditingController` 创建移到 `showDialog` 外，在 dialog 返回后调用 `controller.dispose()`
- **提交**: 含于本次推送

### H-7: keychain_service.dart — 异常捕获过窄
- **修复**: 所有 `on PlatformException catch` 改为 `on Exception catch`，防止 `MissingPluginException` / `FormatException` 逃逸
- **提交**: 含于本次推送

### M-1: unified_object_provider.dart — 双重 bang
- **修复**: `object!.parentId!` → 使用局部变量 `parentId` 做空检查后安全访问
- **提交**: 含于本次推送

### M-2: entry_card_widget.dart — itemData 无 null 检查
- **修复**: `widget.itemData!.forEach(...)` → 先检查 `itemData == null` 则提前 return，再安全调用 `forEach`
- **提交**: 含于本次推送

### M-3: version_sheet.dart — 字符串索引无 guard
- **修复**: `Platform.operatingSystem[0]` → 添加 `isNotEmpty` 检查
- **提交**: 含于本次推送

### M-4: biometric_settings_widget.dart — dialog controller 泄漏
- **修复**: 将 `TextEditingController` 创建移到 `showDialog` 外，用 `try/finally` 确保 `controller.dispose()`
- **提交**: 含于本次推送

### M-5: sensitive_value_widget.dart — 3 个 provider watch
- **修复**: `ref.watch(accountStyleProvider).value` → `ref.watch(accountStyleProvider.select((s) => s.value?.displayMode))`，只监听 displayMode
- **提交**: 含于本次推送

### M-6: sensitivity_based_visibility_widget.dart — 全局验证状态 watch
- **修复**: 添加 `.select((v) => v)` 缩小重建范围
- **提交**: 含于本次推送

### M-7: object_workspace_page.dart — 全量 watch cache
- **修复**: `ref.watch(unifiedObjectCacheProvider)` → `select` 只监听 `objectById`, `itemChildren`, `workspaceChildren`, `rootObjects`
- **提交**: 含于本次推送

### L-3: profile_storage_service.dart — unawaited saveProfile 错误丢失
- **修复**: `.catchError` 中添加日志记录并返回 `false`
- **提交**: 含于本次推送

---

## 2026-04-29 修复批次 — Round 3（缓存优化 + 竞态修复）

### C-2: property_editor_factory.dart — 4 处 controller 泄漏
- **修复**: `_TextEditor`, `_NumberEditor`, `_RelationEditor`, `_UrlEditor` 从 `StatelessWidget` 转为 `StatefulWidget`，在 `dispose()` 中释放 `TextEditingController`
- **提交**: `1fe50a2`

### C-3: object_editor_page.dart — fire-and-forget + setState 无 mounted
- **修复**: `_saveObject()` 改为 `Future<void>` 并包 `try/catch`；所有 `await` 后 `setState` 前添加 `if (mounted)` 检查
- **提交**: `1fe50a2`

### C-4: native_vault_service.dart — Android/Windows 初始化 race
- **修复**: 添加 `_initFuture` 跟踪异步初始化状态；所有 Android/Windows async 公共方法开头 `await _ensureInitialized()`；`_androidRequest` 添加 null guard 防止未初始化访问
- **提交**: `bfc7ea1`

### H-4: sensitivity_settings_page.dart — build 中 4 次过滤排序
- **修复**: 在 State 中添加 `_cachedEffectiveFields` 和 `_cachedSections`，通过 `_getEffectiveFields()` / `_getFilteredSections()` 缓存计算结果，只在 registry/accountStyle/searchQuery 变化时重新计算
- **提交**: `bfc7ea1`

### H-6: trash_page.dart — for 循环内动态 ref.watch
- **修复**: 在 `sensitivity_provider.dart` 中新增 `trashItemSensitivityMapProvider` 聚合所有 item type 的灵敏度；`trash_page.dart` 改为 watch 单个 provider；同时添加 `_getFilteredTrash()` 缓存过滤结果
- **提交**: `bfc7ea1`

### H-2 (revisited): search_result_tile.dart — 精确 watch revealedFields
- **修复**: 将 `ref.watch(accountStyleProvider.select((s) => s.value?.displayMode))` 改为 watch `revealedFields` 和 `isSensitiveAccessGrantedProvider`，确保 tile 在字段 reveal/hide 时正确重建
- **提交**: `bfc7ea1`

### M-7 (revisited): predefined_object_section.dart — 缓存 fieldDefs
- **修复**: 将 `ConsumerWidget` 转为 `ConsumerStatefulWidget`，添加 `_cachedFieldDefs` 缓存 schema 解析结果（只依赖 immutable 的 `typeId`）
- **提交**: `bfc7ea1`

### M-6 (revisited): sensitivity_based_visibility_widget.dart — effectiveFieldLevelProvider select
- **修复**: `effectiveFieldLevelProvider` 中 `ref.watch(accountStyleProvider).value` → `ref.watch(accountStyleProvider.select((s) => s.value))`
- **提交**: `bfc7ea1`

### L-1: test/ — const ProfileData info
- **修复**: `ProfileData()` → `const ProfileData()` 在 `profile_data_test.dart` 和 `profile_provider_test.dart`
- **提交**: `9d531af`

### L-2: biometric_credential_service_test.dart — deprecated setMockMethodCallHandler
- **修复**: 替换为 `TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler`
- **提交**: `9d531af`

---

## 2026-04-29 重构 — 全面移除 Legacy 代码，完全迁移至 UnifiedObject

**提交**: `5dabedc`
**变更**: +266 / −12,830 行

### 删除的旧模型类（20+ 个）
- `IdentityData`, `ContactData`/`ContactEntry`, `AddressData`, `IdCardData`
- `TravelData`, `PassportData`, `VisaData`, `TravelHistoryData`
- `FinancialData`, `BankAccountData`, `CardData`, `TaxIdData`
- `ProfessionalData`, `EducationData`, `EmploymentData`, `SkillData`, `LanguageData`, `AwardData`
- `DeletedItemInfo`, `ItemTypeMeta`

### 删除的纯 legacy 文件
- `lib/presentation/providers/profile_section_editor.dart`
- `lib/presentation/providers/services/section_mutators.dart`
- `lib/presentation/providers/services/operation_log_aggregator.dart`
- `lib/core/services/log_section_config.dart`

### 核心文件修改

**`lib/core/models/profile_data.dart`**
- `ProfileData` 移除 `identity`/`travel`/`financial`/`professional` 字段
- 仅保留 `unifiedObjects` + `schemaVersion`

**`lib/core/services/profile_storage_service.dart`**
- 删除 `_migrateProfileDataToUnified`（~600 行）、`_migrateLegacyToUnified`
- 删除 `getDeletedItems`、`_calculateRestoreItem`、`_calculatePermanentDeleteItem`
- 删除 `purgeOldDeletedItems`、`calculateEmptyTrash`、`emptyAllTrash`
- 简化 `_migrateIfNeeded` 为仅版本号递增

**`lib/presentation/providers/profile_provider.dart`**
- 删除 `ProfileIdentity`/`ProfileTravel`/`ProfileFinancial`/`ProfileProfessional`
- 删除 14 个 dead item providers：`EducationItems`、`PassportItems`、`BankAccountItems` 等
- 删除 `updateIdentity`/`updateTravel`/`updateFinancial`/`updateProfessional` 方法
- 删除 `softDelete`/`restore`/`permanentDelete`/`emptyAllTrash` 方法（已委托给 `UnifiedObjectNotifier`）

**`lib/presentation/pages/trash_page.dart`**
- 删除 `TrashLegacyEntry` 和 `_TrashItemCard`（~877 行）
- 删除所有 `DeletedItemInfo` 相关方法（`_restoreItem`、`_confirmRestore`、`_showDetail`、`_itemHasHistory` 等）
- Trash 现在仅显示 `TrashUnifiedEntry`（unified objects）

**`lib/presentation/providers/search_provider.dart`**
- 重写 `_performSearch`：从遍历 legacy `ProfileData` 字段改为遍历 `unifiedObjectProvider`
- 搜索范围：`object.name`、`object.typeId`、`object.properties` 中的文本值
- 通过 `parentId` 链解析 section 名称

**`lib/presentation/providers/services/trash_manager.dart`**
- 重写为 unified-object coordinator
- `softDelete` → `deleteDefaultItem`/`deleteObject`
- `restore` → `restoreObject`/`restoreDefaultItem`
- `permanentDelete` → `permanentlyDeleteObject`
- `emptyAllTrash` → 遍历 deleted objects 逐一 `permanentlyDeleteObject`

**`lib/presentation/providers/sensitivity_provider.dart`**
- 删除 `trashItemSensitivityMapProvider`（不再需要，trash 已 unified-only）

### 删除的测试文件
- `test/unit/profile_data_test.dart`
- `test/unit/profile_version_test.dart`
- `test/unit/profile_provider_test.dart`
- `test/unit/presentation/providers/profile_providers_test.dart`
- `test/unit/presentation/providers/profile_provider_test.dart`
- `test/benchmark/storage_benchmark.dart`
- `integration_test/app_test.dart`

### 验证结果
- `dart analyze --fatal-infos`：**0 issues**
- 生产代码编译：**完全通过**
