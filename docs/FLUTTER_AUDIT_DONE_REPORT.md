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

## 剩余待办项（本轮未修复）

| 编号 | 严重度 | 文件 | 描述 | 原因 |
|------|--------|------|------|------|
| C-2 | 🔴 Critical | property_editor_factory.dart | 4 处 controller 泄漏 | 需将 StatelessWidget 转为 StatefulWidget，改动较大 |
| C-3 | 🔴 Critical | object_editor_page.dart | fire-and-forget + setState 无 mounted | 涉及方法签名变更和多处回调修改，影响面广 |
| C-4 | 🔴 Critical | native_vault_service.dart | Android/Windows 初始化 race | 已知 P0-2 问题，当前 stub 实现，需架构级修复 |
| H-4 | 🟠 High | sensitivity_settings_page.dart | build 中 4 次过滤排序 | 需引入缓存机制或重构为派生 provider，非 trivial |
| H-6 | 🟠 High | trash_page.dart | for 循环内动态 ref.watch | 需重构为派生 provider，涉及灵敏度映射预计算 |
| L-1 | 🔵 Low | test/ | const ProfileData info | 仅 info，非阻塞 |
| L-2 | 🔵 Low | biometric_credential_service_test.dart | deprecated setMockMethodCallHandler | 仅 info，非阻塞 |
