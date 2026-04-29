# Flutter 代码库审计报告

> 生成时间: 2026-04-29
> 范围: `flutter/lib/` + `flutter/test/`
> 分析维度: 死代码、性能、错误处理、代码质量

---

## 🔴 Critical（必须立即修复）

### C-1: login_page.dart — 空名字导致崩溃
- **文件**: `lib/presentation/pages/login_page.dart:1189`
- **问题**: `account.name[0].toUpperCase()` 没有检查 `account.name.isNotEmpty`。若账户名为空则崩溃。
- **修复**: 添加空字符串保护: `account.name.isNotEmpty ? account.name[0].toUpperCase() : '?'`

### C-2: property_editor_factory.dart — 4 处 TextEditingController 内存泄漏
- **文件**: `lib/presentation/widgets/property_editor_factory.dart:128,154,323,349`
- **问题**: `_TextEditor`, `_NumberEditor`, `_RelationEditor`, `_UrlEditor` 四个 StatelessWidget 在 `build()` 中创建 `TextEditingController` 且永不 dispose，每次重建泄漏一个 controller。
- **修复**: 将四个 widget 转为 `StatefulWidget`，在 `dispose()` 中释放 controller。或使用 `TextFormField(initialValue: ...)` 替代 controller 方案。

### C-3: object_editor_page.dart — fire-and-forget + setState 无 mounted 检查
- **文件**: `lib/presentation/pages/object_editor_page.dart`
- **问题**:
  1. `void _saveObject() async` 被同步调用，内部多个 await 抛出的异常无人捕获。
  2. `onTap`/`onPressed` 中 `await showModalBottomSheet` / `await showDialog` 后直接 `setState()`，无 `mounted` 检查。
- **修复**:
  1. 改为 `Future<void> _saveObject() async`，body 包 `try/catch`。
  2. 所有 `await` 后 `setState` 前加 `if (mounted)`。

### C-4: native_vault_service.dart — 初始化 race condition（Android/Windows 必崩）
- **文件**: `lib/core/services/native_vault_service.dart:56-88`
- **问题**: `_initialize()` 同步调用 `_initializeAndroid()` / `_initializeWindows()` 且不 await；后续所有 Android/Windows fallback 方法使用 `_profilesDir!` 和 `_fallbackSecureStorage!`。若 vault 方法在异步初始化完成前被调用则崩溃。
- **修复**: 将 `_initialize()` 改为 `Future<void>`，同步 await 平台初始化。或至少在每次使用 `_profilesDir`/`_fallbackSecureStorage` 前做空检查并抛 `StateError`。
- **备注**: 此为 P0-2 已知问题，当前代码硬编码返回 `false`/`null` 作为临时 stub，需彻底修复。

---

## 🟠 High（高优先级）

### H-1: main.dart — 缺少 ErrorWidget.builder
- **文件**: `lib/main.dart`
- **问题**: 未配置 `ErrorWidget.builder`，release 模式下 widget build 崩溃会显示灰屏而非友好降级 UI。
- **修复**: 在 `runApp()` 前设置:
  ```dart
  ErrorWidget.builder = (details) {
    DebugLogger.instance.logError('ERROR_WIDGET', details.exception.toString());
    return const Material(child: Center(child: Text('Something went wrong')));
  };
  ```

### H-2: search_result_tile.dart — 全量 watch accountStyleProvider
- **文件**: `lib/presentation/widgets/search_result_tile.dart:24`
- **问题**: `ref.watch(accountStyleProvider);` 监听整个 provider。搜索结果可能数十条，任何 account style 字段变化都会触发全部 tile 重建。
- **修复**: 改为 `ref.watch(accountStyleProvider.select((s) => s.value?.displayMode));`，只监听实际用到的字段。

### H-3: history_sheet.dart — flatten+sort 在 FutureBuilder.builder 中重复执行
- **文件**: `lib/presentation/widgets/history_sheet.dart:80-103`
- **问题**: `FutureBuilder.builder` 内执行三重嵌套循环展平历史记录再排序。每次父级 rebuild 都会重新计算，即使 snapshot 没变。
- **修复**: 将展平+排序逻辑移到 `Future` 内部（await 返回前完成），或在 provider 层预计算。

### H-4: sensitivity_settings_page.dart — build 中 4 次过滤+排序
- **文件**: `lib/presentation/pages/sensitivity_settings_page.dart:94-124`
- **问题**: `_buildSettingsView()` 每次 build 执行排序、映射和 4 次 `where+toList`。字段列表可能上百条。
- **修复**: 将 `buildEffectiveFields()` 结果缓存到 provider/notifier 中，只在 registry 或 accountStyle 变化时重新计算。

### H-5: data_management_page.dart — 3 处对话框 TextEditingController 未 dispose
- **文件**: `lib/presentation/pages/data_management_page.dart:257,349,423`
- **问题**: `showDialog` builder 内创建 `TextEditingController` 未释放。
- **修复**: 在对话框内部使用 `StatefulBuilder` 或自定义 `StatefulWidget` 管理 controller 生命周期。

### H-6: trash_page.dart — build 中 for 循环内动态 ref.watch
- **文件**: `lib/presentation/pages/trash_page.dart:252-259`
- **问题**: `_buildTrashContent` 在 `for` 循环中调用 `ref.watch(effectiveSensitivityProvider(fieldId))`。Riverpod 不保证动态 watch 数量变化时的正确行为。
- **修复**: 在 provider/notifier 中预计算灵敏度映射，widget 只 watch 一个派生 provider。

### H-7: keychain_service.dart — 异常捕获范围过窄
- **文件**: `lib/core/services/keychain_service.dart:25,74,97,123`
- **问题**: 只 catch `PlatformException`，`MissingPluginException` / `FormatException` 会逃逸。
- **修复**: 扩展 catch 为 `Exception`。

---

## 🟡 Medium（中等优先级）

### M-1: unified_object_provider.dart — 双重 bang 操作
- **文件**: `lib/presentation/providers/unified_object_provider.dart:35,331`
- **问题**: `profile!.unifiedObjects!` 和 `object!.parentId!` 双重 bang。
- **修复**: 使用局部变量做空检查后安全访问。

### M-2: entry_card_widget.dart — itemData 无 null 检查
- **文件**: `lib/presentation/widgets/entry_card_widget.dart:205`
- **问题**: `widget.itemData!.forEach(...)` 无前置 null 检查。
- **修复**: 使用 `widget.itemData?.forEach(...)` 或提前 return。

### M-3: version_sheet.dart — 字符串索引无 guard
- **文件**: `lib/presentation/widgets/settings/version_sheet.dart:141`
- **问题**: `Platform.operatingSystem[0].toUpperCase()` 无空检查（虽然实际上不会为空）。
- **修复**: 添加 `isNotEmpty` guard。

### M-4: biometric_settings_widget.dart — 对话框 controller 未 dispose
- **文件**: `lib/presentation/widgets/biometric_settings_widget.dart:58`
- **问题**: `_showPasswordDialog` 内 `TextEditingController()` 未释放。
- **修复**: 在对话框关闭时 dispose 或使用 `initialValue`。

### M-5: sensitive_value_widget.dart — 3 个 provider watch
- **文件**: `lib/presentation/widgets/sensitive_value_widget.dart:147-152`
- **问题**: 每个敏感字段都 watch `accountStyleProvider` + `effectiveSensitivityProvider` + `isSensitiveAccessGrantedProvider`。
- **修复**: 至少给 `accountStyleProvider` 加 `select`。

### M-6: sensitivity_based_visibility_widget.dart — 全局验证状态 watch
- **文件**: `lib/presentation/widgets/sensitivity_based_visibility_widget.dart:142`
- **问题**: `ref.watch(isSensitiveAccessGrantedProvider)` 导致所有实例在验证过期/授予时同时重建。
- **修复**: 在事件处理器中用 `ref.read`，UI 只监听轻量版本。

### M-7: object_workspace_page.dart — 全量 watch cache
- **文件**: `lib/presentation/pages/object_workspace_page.dart:36`
- **问题**: `ref.watch(unifiedObjectCacheProvider)` 监听整个缓存对象。
- **修复**: 用 `select` 只监听 `rootObjects` 或需要的派生状态。

---

## 🔵 Low（低优先级 / 代码质量）

### L-1: 测试文件中的 `const ProfileData()` / `const IdentityData()`
- **文件**: `test/unit/profile_data_test.dart`, `test/unit/profile_provider_test.dart`
- **问题**: `IdentityData` 已移除 `const` 构造函数，但测试文件中仍用 `const ProfileData()`（空参数，实际上仍合法但 linter 提示）。
- **修复**: 根据 linter 建议加回 `const` 或保持现状（仅 info，非阻塞）。

### L-2: biometric_credential_service_test.dart — deprecated `setMockMethodCallHandler`
- **文件**: `test/unit/core/services/biometric_credential_service_test.dart`
- **问题**: 使用已废弃的 `setMockMethodCallHandler`。
- **修复**: 替换为 `tester.binding.defaultBinaryMessenger.setMockMethodCallHandler`。

### L-3: profile_storage_service.dart 中的 `unawaited(saveProfile(...))`
- **文件**: `lib/core/services/profile_storage_service.dart:992`
- **问题**: save 失败时错误静默丢失。
- **修复**: 附加 `.catchError` 记录日志。

---

## 📋 待办清单汇总

| 编号 | 严重度 | 文件 | 描述 | 状态 |
|------|--------|------|------|------|
| C-1 | 🔴 Critical | login_page.dart | 空名字索引崩溃 | ⬜ 待办 |
| C-2 | 🔴 Critical | property_editor_factory.dart | 4 处 controller 泄漏 | ⬜ 待办 |
| C-3 | 🔴 Critical | object_editor_page.dart | fire-and-forget + setState 无 mounted | ⬜ 待办 |
| C-4 | 🔴 Critical | native_vault_service.dart | Android/Windows 初始化 race | ⬜ 待办 |
| H-1 | 🟠 High | main.dart | 缺少 ErrorWidget.builder | ⬜ 待办 |
| H-2 | 🟠 High | search_result_tile.dart | 全量 watch accountStyleProvider | ⬜ 待办 |
| H-3 | 🟠 High | history_sheet.dart | flatten+sort 重复计算 | ⬜ 待办 |
| H-4 | 🟠 High | sensitivity_settings_page.dart | build 中 4 次过滤排序 | ⬜ 待办 |
| H-5 | 🟠 High | data_management_page.dart | 3 处 dialog controller 泄漏 | ⬜ 待办 |
| H-6 | 🟠 High | trash_page.dart | for 循环内动态 ref.watch | ⬜ 待办 |
| H-7 | 🟠 High | keychain_service.dart | 异常捕获过窄 | ⬜ 待办 |
| M-1 | 🟡 Medium | unified_object_provider.dart | 双重 bang | ⬜ 待办 |
| M-2 | 🟡 Medium | entry_card_widget.dart | itemData 无 null 检查 | ⬜ 待办 |
| M-3 | 🟡 Medium | version_sheet.dart | 字符串索引无 guard | ⬜ 待办 |
| M-4 | 🟡 Medium | biometric_settings_widget.dart | dialog controller 泄漏 | ⬜ 待办 |
| M-5 | 🟡 Medium | sensitive_value_widget.dart | 3 个 provider watch | ⬜ 待办 |
| M-6 | 🟡 Medium | sensitivity_based_visibility_widget.dart | 全局验证状态 watch | ⬜ 待办 |
| M-7 | 🟡 Medium | object_workspace_page.dart | 全量 watch cache | ⬜ 待办 |
| L-1 | 🔵 Low | test/ | const ProfileData info | ⬜ 待办 |
| L-2 | 🔵 Low | biometric_credential_service_test.dart | deprecated setMockMethodCallHandler | ⬜ 待办 |
| L-3 | 🔵 Low | profile_storage_service.dart | unawaited saveProfile 错误丢失 | ⬜ 待办 |
