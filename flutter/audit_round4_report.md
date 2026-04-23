# SoloSoul Flutter 第四轮深度诊断报告：架构深层结构

> 生成时间：2026-04-23  
> 前提：三轮修复共 110+ 项问题已完成  
> 范围：`flutter/` 目录  
> 方法：修复 rollout 验证 + SOLID 原则审查 + 深层架构分析  
> 核心关切：**可持续的长期开发**

---

## 一、第三轮修复验证

### 1.1 验证通过的修复

| 修复项 | 验证结果 | 证据 |
|--------|---------|------|
| Repository 层删除 | ✅ 完整 | `core/repositories/` 目录已空，0 处 import |
| Professional SensitivityLevel 动态化 | ✅ 完整 | `professional_page.dart` 中 0 处硬编码 `public` |
| `state = state;` hack 移除 | ✅ 完整 | 全代码库 grep 无匹配 |
| `accountsVersion` 版本号机制 | ✅ 完整 | `AuthNotifier` 中 4 处递增，3 个 getter |
| `accountsProvider` 迁移到 `AsyncNotifier` | ✅ 完成 | 使用 `AsyncNotifierProvider<AccountsNotifier>` |
| ProfileSectionState 推广 | ⚠️ 93% | 13/14 Section 已迁移，`_ContactSectionState` 仍是 `ConsumerState` |
| 性能基准测试 | ✅ 存在 | `test/benchmark/crypto_benchmark.dart` (3284 bytes) 和 `storage_benchmark.dart` (5357 bytes) |
| lint 规则启用 | ✅ 已启用 | `analysis_options.yaml` 中 3 条规则已激活 |

### 1.2 仍存在问题的修复

#### json_serializable：注解已加，但手写代码未替换 — Adoption: 0%

**Claimed:** 23 models migrated  
**Actual:**
- **22 个模型**有 `@JsonSerializable()`（18 在 `profile_storage_service.dart`，3 在 `field_history_models.dart`，1 在 `rust_vault_service.dart`）
- **24 个模型**仍有手写 `factory fromJson()` 和 `toJson()`
- **0 个模型**调用生成的 `_$ClassFromJson` / `_$ClassToJson`
- `dart analyze` 报告 **46 个 `unused_element` warnings** 在 `.g.dart` 文件上，确认生成代码是死代码

**风险**：
- 生成的 `.g.dart` 中使用了 **`DateTime.parse`**（14 处），而非 `DateTime.tryParse`
- 如果存储的日期字段损坏，生成的 `fromJson` 会抛出 `FormatException`
- 手写代码已经处理了 null safety，生成的代码却是严格 cast（`as String`）
- **维护两份序列化逻辑** = 未来修改时可能改 A 忘 B

**建议**：
- 将所有 `factory XxxData.fromJson()` 改为 `=> _$XxxDataFromJson(json)`
- 将所有 `toJson()` 改为 `=> _$XxxDataToJson(this)`
- 删除手写序列化逻辑，或标记为 `@deprecated`

---

#### `AccountsNotifier` 仍有副作用读取，`_accountsVersion` 是死代码

```dart
class AccountsNotifier extends AsyncNotifier<List<AccountInfo>> {
  @override
  Future<List<AccountInfo>> build() async {
    ref.watch(authNotifierProvider.notifier);  // ← 副作用读取，notifier 对象永不变
    return ref.read(authNotifierProvider.notifier).getAccountsSortedByRecent();
  }
}
```

**`_accountsVersion` 状态：**
- 声明在 `auth_provider.dart:542`
- 在 3 处递增（lines 614, 662, 964）
- **没有任何 provider 监听它**

`_accountsVersion` 是纯粹的**死代码**——写了但没用。`accountsProvider` 仍然使用旧的 `ref.watch(authNotifierProvider.notifier)` hack，只是换了语法。

**建议**：将 `_accountsVersion` 提升为 provider：`final accountsVersionProvider = Provider<int>((ref) => ref.watch(authNotifierProvider.notifier).accountsVersion);`，`AccountsNotifier` 监听 `accountsVersionProvider`。

---

#### lint 规则列在配置中但**未有效执行**

`analysis_options.yaml`:
```yaml
avoid_catches_without_on_clauses: true
unawaited_futures: true
use_build_context_synchronously: true
```

`dart analyze lib/` 输出 **64 issues**，但 **3 条自定义规则的违例数为 0**。

**证据**：`native_crypto_service.dart` 中有 **4 处裸 `catch (e)`**（lines 82, 103, 121, 139），明显应触发 `avoid_catches_without_on_clauses`，但 `dart analyze` 对此文件报告 **"No issues found"**。

**根因**：规则列在配置中但 analyzer 没有正确 flag 违例。可能是 `flutter_lints` 包的默认规则覆盖了自定义规则，或 analyzer 缓存问题。

**建议**：检查 `flutter_lints` 包版本与自定义规则的兼容性；尝试显式 `include: package:flutter_lints/flutter.yaml` 后覆盖规则；或升级到 `package:lints` / `package:very_good_analysis`。

---

#### `contactItemsProvider` + `indexOf` = **静默删除失败（Critical）**

```dart
// profile_page.dart:463
Future<void> _onContactDelete(ContactEntry contact) async {
  final contacts = ref.read(contactItemsProvider);
  final index = contacts.indexOf(contact);   // ← IDENTITY equality
```

`contactItemsProvider` **每次重建都创建新的 `ContactEntry` 实例**（`.map((e) => ContactEntry(...))`），且 `ContactEntry` **未覆盖 `operator ==`**。如果 provider 在 widget build 和用户 tap 之间重建（如其他 section 的 auto-save 触发），`indexOf` 返回 `-1`，删除**静默失败**。

**对比**：`idCards` (line 789) 和 `addresses` (line 1042) 也用了 `indexOf`，但它们的 provider 直接 `.toList()` 现有实例，identity equality 仍有效。**只有 `ContactEntry` broken。**

**Fix**: 替换为 `indexById(contact.id, (c) => c.id)`，与 `financial_page.dart`、`professional_page.dart`、`travel_page.dart` 保持一致。

---

#### `loadProfile`/`saveProfile` 的 `on Exception catch` 导致 **TypeError 崩溃回归（High）**

```dart
// profile_storage_service.dart:2031
} on Exception catch (e) {
  return null;
}
```

`ProfileData.fromJson` 中有 `as String` / `as int` 动态 cast。如果 JSON 损坏，Dart 抛出 **`TypeError`**（`Error` 子类，不是 `Exception`）。`on Exception` **不会捕获**，导致 app 崩溃而非优雅返回 `null`。`saveProfile` (line 2052) 同理。

> **Round 3 之前**：裸 `catch (e)` 会同时捕获 `Exception` 和 `Error`，安全返回 `null`/`false`。

**Fix**: 改为 `catch (e)`（捕获所有 throwable），或分两个 clause：`on Exception catch (e)` + `on Error catch (e)`。

---

#### `BridgeProfileSummary` key-naming drift（High）

| 代码 | 期望 |
|------|------|
| 手写 `fromJson` (`rust_vault_service.dart:27–35`) | snake_case: `created_at`, `updated_at` |
| 生成 `_$BridgeProfileSummaryFromJson` (`rust_vault_service.g.dart:9–17`) | camelCase: `createdAt`, `updatedAt` |

如果未来切换为生成代码，反序列化会直接崩溃。这是手写与生成代码并存时的**命名约定不一致**。

---

#### `widget_test.dart` timer leak — **Round 3 回归（High）**

`ProfileNotifier` (line 1541) 无 `autoDispose`，其 debounce `Timer` (`_saveDebounceTimer`, line 171) 或 `_autoLockTimer` (`main.dart:127`) 在 widget tree 被 tear down 后仍然存在，触发 Flutter test framework 的 invariant violation：

```
A Timer is still pending even after the widget tree was disposed.
```

36 个失败测试中，35 个是预存在的 FFI binding 失败，**只有这 1 个是 Round 3 引入的回归**。

**Fix**: `profileNotifierProvider` 和 `*ItemsProvider` 改用 `.autoDispose`，并在 `dispose()` 中 cancel timer。

---

#### 多个 `WidgetsBindingObserver` 浪费 + 跨 section UI 闪烁（Medium）

`ProfileSectionState` mixin 让每个 section 注册自己的 `WidgetsBindingObserver`：
- `travel_page.dart` 3 个 section → 3 个 observer
- `professional_page.dart` 5 个 section → 5 个 observer

`AppLifecycleState.resumed` 时每个 observer 都调用 `loadItems()`（现在都是 no-op），这是纯粹的浪费。

更严重的是：**跨 section save 导致 UI 闪烁**。`_PassportSectionState` 保存时调用 `updateTravelImmediate(travel)`，突变 `profileNotifierProvider`，重建所有 travel-derived provider（`passportItemsProvider`、`visaItemsProvider`、`travelHistoryItemsProvider`）。如果用户正在编辑 visa，form 会丢失焦点/状态。

**Fix**: 删除 `ProfileSectionState` mixin（zombie abstraction），或在 page 级别只注册一个 observer。

---

## 二、深层架构问题（SOLID 原则审查）

### 2.1 依赖倒置原则（DIP）— 🔴 严重违反

**问题**：11+ 个全局单例，零接口抽象

```dart
class NativeCryptoService { static NativeCryptoService get instance { ... } }
class RustVaultService { static RustVaultService get instance { ... } }
class ProfileStorageService { ... }  // 隐式单例
```

**横向依赖链**：
```dart
// rust_vault_service.dart
final nonce = NativeCryptoService.instance.generateSalt();
final encrypted = NativeCryptoService.instance.encrypt(...);
```

**Provider 层直接实例化**：
```dart
class AuthNotifier extends StateNotifier<AuthState> {
  final SecureAccountStorage _storage = SecureAccountStorage.instance;
  final ProfileStorageService _profileStorage = ProfileStorageService.instance;
```

**后果**：
- **无法 Mock**：`auth_provider_test.dart` 只能测试数据结构（`AccountInfo.fromJson`），无法测试 `AuthNotifier.unlockVault()` 的核心逻辑
- **无法并行测试**：单例持有全局可变状态（如 `_encryptionKey`），测试间互相污染
- **平台耦合**：`NativeCryptoService` 在 Android 走 Dart fallback，在 macOS 走 FFI，但调用方无法通过接口切换策略

**建议**：
1. 提取接口：
   ```dart
   abstract class CryptoService {
     Uint8List? deriveKey({required String password, required Uint8List salt, ...});
     Uint8List? encrypt({required Uint8List data, required Uint8List key, ...});
     Uint8List? decrypt({required Uint8List encrypted, required Uint8List key, ...});
   }
   ```
2. 构造函数注入：
   ```dart
   class AuthNotifier extends StateNotifier<AuthState> {
     AuthNotifier({required CryptoService crypto, required VaultService vault, ...});
   }
   ```
3. Riverpod 层负责组装：
   ```dart
   final authNotifierProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
     return AuthNotifier(
       crypto: ref.watch(cryptoServiceProvider),
       vault: ref.watch(vaultServiceProvider),
     );
   });
   ```

---

### 2.2 开闭原则（OCP）— 🔴 严重违反

**问题**：新增一个 Profile Section（如 "Health Records"）需要修改 **10+ 个文件**

| 文件 | 修改点 |
|------|--------|
| `profile_storage_service.dart` | 新增 `HealthData` 类，修改 `ProfileData` |
| `profile_provider.dart` | 新增 `updateHealth()`、`_logHealthChanges()`、`_summarizeHealthChanges()` |
| `profile_provider.dart` | `_addLogEntry()` switch 新增 case |
| `profile_provider.dart` | `_findIndexById()` switch 新增 case |
| `profile_provider.dart` | `_getItemLabel()` switch 新增 case |
| `profile_provider.dart` | `emptyAllTrash()` 遍历 health 条目 |
| `profile_provider.dart` | 新增 `healthItemsProvider` |
| `profile_section_editor.dart` | `markDeleted`/`markRestored`/`getItem` switch 新增 case |
| `profile_section_editor.dart` | 新增 `_markDeletedHealth`、`_markRestoredHealth`、`_getItemHealth` |
| `log_section_config.dart` | 新增 health 映射 |
| `health_page.dart` | 新建页面，重写 12 个回调 |

**ProfileSectionEditor 的巨型 switch**：
```dart
static (ProfileData, bool) markDeleted({...}) {
  switch (section) {
    case 'travel': return _markDeletedTravel(...);
    case 'financial': return _markDeletedFinancial(...);
    case 'professional': return _markDeletedProfessional(...);
    case 'profile': return _markDeletedProfile(...);
  }
  return (current, false);  // 新增 section 无法扩展
}
```

**UnifiedFormSection 的使用方也未受益**：
每个 Section 仍需手动传入 12 个回调参数（`onSave`、`onDelete`、`historyAwareOnSave`、`onDidDelete`、`onDeleteFailed` 等），形状完全相同，只是类型 `T` 不同。

**建议**：
1. **Section 元数据注册制**：定义 `ProfileSectionConfig`，新增 Section 只需注册一个新配置
2. **策略模式替代 switch**：`Map<String, SectionMutator>` 查找，无需修改已有代码
3. **UnifiedFormSection 自动处理历史记录**：内部检查 `historyConfig != null`，自动调用 `FieldHistoriesNotifier.recordSnapshot()`

---

### 2.3 单一职责原则（SRP）— 🔴 严重违反

**AuthNotifier** (~1100 行) 混合了 8 种职责：
1. 认证状态机（locked/unlocked/loading）
2. 账户存储管理（SecureAccountStorage 读写）
3. 密码学操作（deriveKey）
4. Vault 生命周期（unlock/lock）
5. 密码修改（7 步复杂流程）
6. V1/V2 加密版本迁移
7. Rust → Keychain 账户迁移
8. 设备跟踪与操作元数据

**ProfileNotifier** (~1500 行) 混合了 8 种职责：
1. 数据加载与保存（debounce）
2. 领域模型更新（4 个 updateXxx）
3. 变更日志生成（4 个 `_logXxxChanges`）
4. 变更摘要生成（4 个 `_summarizeXxxChanges`）
5. 软删除 / 恢复 / 永久删除
6. 垃圾回收（`emptyAllTrash`）
7. 索引查找（`_findIndexById`）
8. 操作日志记录（`_addLogEntry`，20+ case switch）

**建议拆分**：
- `AuthNotifier` → `AuthStateNotifier` + `VaultUnlockService` + `MigrationService` + `PasswordService` + `AccountManager`
- `ProfileNotifier` → `ProfilePersistenceNotifier` + `SectionMutator`（每域一个）+ `OperationLogAggregator` + `TrashManager`

---

### 2.4 泛型类型安全 — 🟠 运行时 cast 风险

**`UnifiedFormSection<T>` 中存在 `as dynamic` 和 `as T`**：

```dart
// unified_form_section.dart:329-330
String _getItemId(T item) {
  if (widget.itemIdExtractor != null) {
    return widget.itemIdExtractor!(item);
  }
  return (item as dynamic).id as String? ?? '';  // 绕过泛型
}

// unified_form_section.dart:357-361
if (widget.itemFactory != null && editingItem != null) {
  createdItem = widget.itemFactory!(
    values,
    id: (editingItem as dynamic).id as String?,  // 绕过泛型
  );
}

// unified_form_section.dart:370-378
setState(() {
  if (wasAdding) {
    _items.insert(0, createdItem as T);  // 运行时 cast
  } else {
    _items[_editingIndex] = createdItem as T;  // 运行时 cast
  }
});
```

**风险**：
- `TypeError` 在运行时被抛出，不会被 `on Exception catch (e)` 捕获（因为 `TypeError` 是 `Error` 子类）
- 这意味着 cast 失败会导致 **未捕获的异常**，App 直接崩溃

**建议**：
1. 引入 `IdentifiableItem` 接口：
   ```dart
   abstract class IdentifiableItem { String get id; }
   class ContactEntry implements IdentifiableItem { ... }
   ```
2. `UnifiedFormSection<T extends IdentifiableItem>` 消除 `as dynamic`
3. `itemIdExtractor` 改为必填参数，彻底消灭 fallback

---

### 2.5 异常处理策略 — 🟠 `on Exception` 的盲区

全代码库 **48+ 处**使用 `on Exception catch (e)`：

```dart
} on Exception catch (e) {
  showOverlaySnackBar(context, content: 'Failed: $e');
}
```

**不会被捕获的错误**：
- `TypeError`（泛型 cast 失败）
- `StackOverflowError`
- `OutOfMemoryError`
- `ArgumentError`
- `StateError`（setState after dispose）

**风格也不统一**：
```dart
// native_crypto_service.dart:82
} catch (e) {  // 捕获所有，包括 Error
  throw Exception('Failed to bind: $e');
}
```

**建议**：
1. 业务层使用 `catch (e, st)` 捕获所有可恢复错误
2. 区分 `Exception`（业务错误，展示给用户）和 `Error`（编程错误，上报/崩溃）
3. 或定义 `Result<T>` 类型，彻底消除异常作为控制流

---

## 三、异步模式混乱

同一数据流有三种不同的异步抽象：

| 抽象 | 使用位置 | 问题 |
|------|---------|------|
| `AsyncNotifier` + `AsyncValue` | `accountsProvider` | ✅ 最新模式，有 `.when()` 错误分支 |
| `StateNotifier` + 手动 `try/catch` | `profileNotifierProvider` | 无内置 loading/error 状态，手动管理 `_isLoading` |
| `Future<void>` + `setState` | `UnifiedFormSection.onSave` | 错误通过回调传播，UI 手动触发 SnackBar |

**`ProfileNotifier.loadProfile()` 的防御式编程**：
```dart
Future<void> loadProfile() async {
  final authState = _ref.read(authNotifierProvider);
  if (authState != AuthState.unlocked) return;  // 第1次检查
  ...
  final authStateBeforeLoad = _ref.read(authNotifierProvider);
  if (authStateBeforeLoad != AuthState.unlocked) return;  // 第2次检查
  final profile = await _storage.loadProfile(accountId);  // 异步 IO
  final authStateAfterLoad = _ref.read(authNotifierProvider);
  if (authStateAfterLoad != AuthState.unlocked) return;  // 第3次检查
}
```

这种"检查-等待-再检查"说明缺乏统一的异步原子性保证。如果 `AuthNotifier.lockVault()` 能取消所有正在进行的 `ProfileNotifier` 操作（通过 `CancelToken`），就不需要重复检查。

**建议**：
- `ProfileNotifier` 迁移到 `AsyncNotifier<ProfileData?>`
- `UnifiedFormSection` 暴露 `AsyncValue<void>` 操作状态，让 UI 自动处理 loading/error/success

---

## 四、僵尸抽象

### `ProfileSectionState` 已是历史包袱

```dart
abstract class ProfileSectionState<T extends ConsumerStatefulWidget>
    extends ConsumerState<T> with WidgetsBindingObserver {
```

设计意图：`WidgetsBindingObserver` + `loadItems()` 手动刷新数据。

**实际运行方式**：所有数据通过 `ref.watch(educationItemsProvider)` 自动刷新，`loadItems()` 变成空函数。

```dart
// profile_page.dart:433-438
class _ContactSectionState extends ConsumerState<_ContactSection> {
  @override void initState() { super.initState(); _loadData(); }
  void _loadData() {} // No-op
}

// profile_page.dart:756
class _IdCardSectionState extends ProfileSectionState<_IdCardSection> {
  @override void loadItems() {} // No-op
}
```

**建议**：废弃 `ProfileSectionState` 基类，改用 Hook 或纯 Provider 驱动。`_ContactSectionState` 已经是事实上的无继承模式。

---

## 五、第四轮优先路线图

### P0 — 消除双份序列化逻辑

1. **替换手写 `fromJson`/`toJson` 为生成版本**
   - 将 20 个手写 `factory XxxData.fromJson()` 改为 `=> _$XxxDataFromJson(json)`
   - 将 20 个手写 `toJson()` 改为 `=> _$XxxDataToJson(this)`
   - 删除手写序列化逻辑，或标记为 `@deprecated`
   - **风险**：生成的代码使用 `DateTime.parse`，需评估是否添加 custom converter

### P0 — 修复 `AccountsNotifier` 副作用读取

2. **将 `_accountsVersion` 提升为 provider**
   - 创建 `final accountsVersionProvider = Provider<int>((ref) => ref.watch(authNotifierProvider.notifier).accountsVersion);`
   - `AccountsNotifier.build()` 改为 `ref.watch(accountsVersionProvider)`

### P1 — 泛型安全

3. **引入 `IdentifiableItem` 接口**
   - 所有数据模型实现 `String get id`
   - `UnifiedFormSection<T extends IdentifiableItem>` 消除 `as dynamic`
   - `itemIdExtractor` 改为必填参数

### P1 — 异常处理统一

4. **所有 `on Exception catch (e)` 改为 `catch (e, st)`**
   - 区分业务异常（展示给用户）和编程错误（上报）
   - 或引入 `Result<T>` 类型

### P2 — God Class 拆分

5. **AuthNotifier 拆分**
   - `AuthStateNotifier`：纯状态机
   - `VaultUnlockService`：Rust FFI 解锁
   - `MigrationService`：V1→V2、Rust→Keychain
   - 目标：每个类 < 200 行

6. **ProfileNotifier 拆分**
   - `ProfilePersistenceNotifier`：加载/保存/debounce
   - `OperationLogAggregator`：变更检测与摘要
   - `TrashManager`：软删除/恢复/垃圾回收
   - 目标：每个类 < 300 行

### P2 — 消除僵尸抽象

7. **废弃 `ProfileSectionState`**
   - 将剩余的 `_ContactSectionState` 改为纯 `ConsumerState`
   - 删除 `ProfileSectionState` 基类
   - 如需要生命周期监听，提供 `useAppLifecycleResumed()` Hook

### P3 — lint 违例清理

8. **修复 64 个 `dart analyze` issues**
   - 17 处 `unused_catch_clause`：改为 `catch (e)` 并在日志中使用 `e`
   - 2 处 `dead_code`：删除不可达代码
   - 1 处 `unused_element_parameter`：删除未使用参数

---

## 六、可持续性评分（第四轮）

| 改进项 | 评分 | 理由 |
|--------|------|------|
| 消除双份序列化 | ⭐⭐⭐⭐⭐ | 消除维护隐患，降低日期解析风险 |
| 修复 `AccountsNotifier` 副作用 | ⭐⭐⭐⭐⭐ | 修复 provider 不刷新的 bug |
| 引入 `IdentifiableItem` | ⭐⭐⭐⭐ | 编译期类型安全，消除 `TypeError` 崩溃 |
| 统一异常处理 | ⭐⭐⭐⭐ | 防止 `TypeError` 等静默崩溃 |
| God Class 拆分 | ⭐⭐⭐ | 降低复杂度，但工作量大 |
| 废弃 `ProfileSectionState` | ⭐⭐⭐ | 清理僵尸抽象，降低认知负担 |
| lint 违例清理 | ⭐⭐ | 低成本，保持 CI 绿色 |
