# SoloSoul Flutter 代码修复进度报告

> 本文件跟踪 `FLUTTER_CODE_REVIEW_REPORT.md` 中各项修复的完成状态。

---

## 已完成的修复项

### 2026-04-28 批次 1：安全 & 泄漏 + 静态分析清零

- ✅ **P0-1** `search_provider.dart` — SearchNotifier 改用 `ref.onDispose()` 取消 `_debounceTimer`，防止 Provider 销毁后 Timer 泄漏。
- ✅ **P1-8** `native_vault_service.dart` — 删除未使用字段 `_derivedKeyB64`、`_androidInitialized`。
- ✅ **P1-9** `fallback_secure_storage.dart` — 删除未使用常量 `_metaKey`。
- ✅ **P1-12** `object_editor_page.dart` — 删除未使用的私有方法 `_buildCharacterCounter`。
- ✅ **P1-10** `property_editor_factory.dart` — 将弃用的 `value` 参数替换为 `initialValue`。
- ✅ **P2-12** `native_vault_service.dart` — 删除 `_androidSaveProfile` 中不必要的 `name == null` 检查。
- ✅ **P0-8** `profile/financial/travel/professional_page.dart` — 移除 15 处冗余的 `as dynamic` 类型转换。
- ✅ **P1-6** `settings_page.dart` — 为空 catch 块添加 `DebugLogger` 错误日志，并将 `on Exception` 显式化。
- ✅ **P1-5** `login_page.dart` — 在异步操作后调用 `_promptRestoreIfEmpty` 前添加 `if (!mounted) return;`。
- ✅ **P1-11** `data_management_page.dart` / `settings_page.dart` — 修复 9 处 `use_build_context_synchronously`：统一使用 `if (!mounted) return;` 提前返回模式。
- ✅ **P2-3** 多处 `prefer_const_constructors` / `prefer_const_declarations` / `prefer_const_literals_to_create_immutables` — 批量添加 `const` 优化。
- ✅ **P1-7** 多处 `avoid_catches_without_on_clauses` — `backup_service.dart`、`native_vault_service.dart`、`profile_storage_service.dart`、`sensitivity_models.dart`、`data_management_page.dart` 全部 catch 块显式使用 `on Exception`。
- ✅ **P1-9+** `profile_storage_service.dart` — 重命名局部函数 `_prop` → `prop`、`_sens` → `sens`，消除 `no_leading_underscores_for_local_identifiers`。
- ✅ **P1-9+** `unified_object_service.dart` — `_SectionMeta` → `SectionMeta`，修复 `library_private_types_in_public_api`。
- ✅ **P1-9+** `object_card.dart` — 将 `final doDelete = () async {}` 等匿名函数变量改为正式方法声明 `Future<void> doDelete() async {}`，修复 `prefer_function_declarations_over_variables` 和 `empty_statements`。
- ✅ **P1-9+** `search_filters.dart` — `_chipTheme` → `chipTheme`，修复 `no_leading_underscores_for_local_identifiers`。
- ✅ **P1-9+** `settings_page.dart` — `_tryBiometric` → `tryBiometric`，修复同名 lint。
- ✅ **P1-9+** 测试文件 — 删除 `profile_provider_test.dart` / `profile_data_test.dart` 中的未使用变量；为 `test_ffi.dart`、`scripts/cleanup_orphaned_accounts.dart`、`test/benchmark/*.dart` 添加 `// ignore_for_file: avoid_print`。
- ✅ **P1-9+** `profile_storage_service.g.dart` — 添加 `// ignore_for_file: unused_element` 以抑制生成代码中的已知未引用警告。

### 2026-04-28 批次 2：性能优化

- ✅ **P1-4** `app_theme.dart` — `showOverlaySnackBar()` 添加 `context.mounted` 检查，并将 `OverlayEntry` 引用改为可空类型，延迟回调中安全移除。
- ✅ **P1-4** `home_page.dart` — `_showTopOverlay()` 在 `_MainDashboardState` 中保存 `OverlayEntry?` 引用，在 `dispose()` 中主动清理，避免 overlay 泄漏。
- ✅ **P0-6** `operation_log_page.dart` — 将 `build()` 中的 `WidgetsBinding.addPostFrameCallback` 密码验证逻辑移到 `initState()`，消除每帧 build 都注册回调导致的潜在卡顿和重建循环。
- ✅ **P1-1** `profile_provider.dart` — `ProfileIdentity`/`ProfileTravel`/`ProfileFinancial`/`ProfileProfessional` 四个 section provider 均改用 `select` 精确监听对应子状态，避免跨 section 不必要的重建。
- ✅ **P2-2** `predefined_object_section.dart` — 缓存 `.join(', ')` 结果到局部变量，避免同一字符串计算两次。
- ✅ **P2-10** `field_history_view.dart` / `field_history_dialog.dart` / `history_change_tile.dart` — 提取共享工具函数 `formatRelativeTime()` 和 `formatRelativeTimeShort()` 到 `presentation/utils/format_relative_time.dart`，消除 4 处重复的时间格式化逻辑。
- ✅ **P2-8** `trash_page.dart` — 删除 5 处无意义的 `setState(() {})`（Provider 状态变化已自动触发重建）。

### 2026-04-28 批次 3：工程化与常量提取

- ✅ **P1-14** `pubspec.yaml` — 版本号从 `1.0.0+1` 更新为 `1.3.0+1`，与 CHANGELOG 一致。
- ✅ **P1-15** `pubspec.yaml` — 将 `crypto: ^3.0.3` 从 `dev_dependencies` 移至 `dependencies`（Android 运行时 Dart 密码学回退必需）。
- ✅ **P2-1** `app_theme.dart` — 新增语义化 UI 常量：`kNotificationDuration`、`kOverlayDuration`、`kPasswordHintDelay`、`kPagePadding`、`kDefaultBorderRadius`、`kDefaultMaxVisibleItems`。
- ✅ **P2-1** 多处页面文件 — 批量替换通知 duration 和 page padding 的魔法数字为 `AppTheme` 常量（`travel_page.dart`、`professional_page.dart`、`financial_page.dart`、`profile_page.dart`、`security_settings_page.dart`、`login_page.dart`、`data_management_page.dart`、`home_page.dart` 等）。

### 关键里程碑

- ✅ **`flutter analyze --fatal-infos --fatal-warnings` 现已全部通过**（lib/ + test/ 无任何 warning/info/error）。

---

## 待办修复项（需专门重构迭代）

以下事项因涉及大规模架构重构或需要更深入的测试验证，建议在独立迭代中处理：

### 阶段 A：列表性能与 Widget 分解

- ⏳ **P0-3** `trash_page.dart` — `ListView` → `CustomScrollView` + `SliverList` 或统一数据模型 + `ListView.builder`。当前 `ListView` 使用 spread operator 预构建所有卡片，但 Trash 数据模型混合了 `UnifiedObject` 和 `DeletedItemInfo` 两种类型，需要统一抽象层才能实现真正的延迟构建。
- ⏳ **P0-4** `app_sidebar.dart` — `ListView` → `ListView.builder`。Sidebar 包含固定导航项、条件 Divider、自定义页面树（支持展开/折叠/拖拽），结构复杂。实际导航项数量通常 < 30，性能影响有限。
- ⏳ **P0-7** `object_card.dart` — `build()` 方法约 520 行，需拆分为 `_ObjectCardHeader`、`_ObjectCardPropertiesList`、`_ObjectCardHistorySection` 等子 Widget。此重构与 **P1-2**（`fieldHistoriesProvider` 细粒度 `select`）密切相关，只有分解为独立子 Widget 后，每个子 Widget 才能独立 `watch` 特定历史字段。

### 阶段 B：Isolate 与计算密集型任务

- ⏳ **P0-5** `search_provider.dart` — `_performSearch()` 在主线程遍历所有 Profile 字段执行字符串匹配。需要：① 将搜索逻辑提取为纯函数；② 使用 `Isolate.run` 或 `compute()` 在后台 isolate 执行；③ 处理搜索结果回传和 UI 更新。
- ⏳ **P2-4** 全局 Isolate 缺失 — 搜索、大文件加密/解密、备份列表构建均需 offload 到 isolate。

### 阶段 C：Provider 架构修复

- ⏳ **P1-3** `operation_log_provider.dart` — `OperationLogService` extends `ChangeNotifier`，但 `OperationLogServiceNotifier`（Riverpod `Notifier`）未监听其 `notifyListeners()`，导致 `ref.watch(operationLogProvider)` 实际上不响应日志变化。修复方案：将 `NotifierProvider` 改为 `ChangeNotifierProvider`，或让 `Notifier` 监听单例并维护版本号状态。
- ⏳ **P1-2** `object_card.dart` — 需配合 P0-7 的 Widget 分解才能有效实现 `select`。

### 阶段 D：God File 拆分

- ⏳ **P0-2** `profile_storage_service.dart`（3291 行，24 个类）— 将数据模型提取到 `core/models/profile_data/` 目录。
- ⏳ **P2-6** `settings_page.dart`（1952 行，17 个类）— 将各 Sheet（`_DebugLogSheet`、`_VersionSheet`、`_CurrentAccountSheet` 等）提取到 `widgets/settings/`。
- ⏳ **P2-7** `home_page.dart`（1243 行，12 个类）— 将 `_PageEditor`、`_IconPicker`、`_DashedPlaceholder`、`_DashedBorderPainter` 提取到 `widgets/home/`。

### 阶段 E：代码重复与通用化

- ⏳ **P2-9** 4 个页面结构复制粘贴 — `profile_page.dart`、`financial_page.dart`、`travel_page.dart`、`professional_page.dart` 共享相同的 Scaffold + AppBar + HeaderActionButtons + PredefinedObjectSection 模式，可提取 `ProfileSectionPage` 通用页面。
- ⏳ **P2-11** `sensitivity_models.dart` — `FieldRegistry` 是 630 行手写字段映射。可考虑按 section 拆分为多个 const list，或迁移到 JSON 配置。

### 阶段 F：测试与资产

- ⏳ **P2-5** 测试覆盖率严重不足 — 118 个库文件 vs 16 个测试文件。核心服务（`backup_service`、`biometric_service`、`native_crypto_service` 等）无任何测试。
