# SoloSoul Flutter 第五轮深度诊断报告：可变性与架构债务

> 生成时间：2026-04-23
> 前提：四轮修复共 150+ 项问题已完成，debug_logger kDebugMode / autoDispose / catch(e) 栈轨迹 / @override / @deprecated 已修复
> 范围：`flutter/` 目录
> 方法：数据模型可变性审查 + 依赖分析 + 架构债务量化
> 核心关切：**数据完整性与长期可维护性**

---

## 一、第五轮修复验证

### 1.1 本轮验证通过的修复

| 修复项 | 验证结果 | 证据 |
|--------|---------|------|
| DebugLogger kDebugMode 保护 | ✅ 完整 | `debug_logger.dart` init/log 方法添加 `if (!kDebugMode) return;` |
| 14 个 ItemsProvider .autoDispose | ✅ 完整 | `profile_provider.dart` 中 14 处改为 `.autoDispose` |
| catch(e) 栈轨迹记录 | ✅ 完整 | `auth_provider.dart` 8 处改为 `catch (e, st)` |
| @override 注解缺失 | ✅ 完整 | `profile_storage_service.dart` 14 处 `IdentifiableItem.id` 添加 @override |
| @deprecated 无消息 | ✅ 完整 | `field_history_models.dart`/`rust_vault_service.dart`/`profile_storage_service.dart` 50 处添加消息 |

### 1.2 仍存在的未修复问题

#### ProfileData 等核心模型完全可变 — 🔴 Critical

**问题**：`ProfileData` / `TravelData` / `FinancialData` / `ProfessionalData` / `IdentityData` 所有字段非 `final`，List 字段可直接 `.add`/`.remove`/`.clear`。

```dart
// profile_storage_service.dart 中的数据结构
class ProfileData {
  String? id;
  String accountId;
  Profile? profile;           // 可变
  TravelData? travel;         // 可变
  FinancialData? financial;   // 可变
  ProfessionalData? professional;  // 可变
  IdentityData? identity;     // 可变
  DateTime createdAt;
  DateTime updatedAt;
}

class TravelData {
  List<PassportData> passports = [];      // 可变 List
  List<VisaData> visas = [];              // 可变 List
  List<TravelHistoryData> travelHistory = [];  // 可变 List
}
```

**`emptyAllTrash` 直接修改传入对象**：

```dart
// profile_provider.dart
void emptyAllTrash() {
  for (final travel in travelItems) {
    profile.travel!.passports.removeWhere((p) => p.isDeleted);  // ← 直接 mutation
    profile.travel!.visas.removeWhere((v) => v.isDeleted);
    profile.travel!.travelHistory.removeWhere((t) => t.isDeleted);
  }
  // ...
  notifyListeners();  // 触发保存
}
```

这是**副作用**，不是纯函数。问题：
1. 调用方无法预测状态变化
2. 无法 undo/redo（没有旧状态快照）
3. 与 `copyWith` 模式矛盾

**`restoreItem` / `permanentDeleteItem` 同样 mutation**：

```dart
// profile_provider.dart
void restoreItem(String section, String id) {
  switch (section) {
    case 'travel':
      final prof = profile.travel!;
      final index = prof.passports.indexWhere((p) => p.id == id);
      if (index != -1) {
        prof.passports[index] = prof.passports[index].copyWith(isDeleted: false);
        // ← 实际上 .copyWith() 创建新对象，但 profile.travel 仍是同一个引用
        // 如果有其他地方持有 prof.travel 引用，会看到旧状态
      }
  }
}
```

**建议**：
- 引入 `freezed` / immutable pattern
- 所有修改操作返回新对象：`ProfileData emptyAllTrash(ProfileData profile) => ...`
- 消除所有直接 mutation

---

#### ProfileSectionEditor 巨型 switch — 🟠 26 case

```dart
// profile_section_editor.dart
static (ProfileData, bool) markDeleted(...) {
  switch (section) {
    case 'education': return _markDeletedEducation(...);
    case 'bankAccount': return _markDeletedBankAccount(...);
    case 'employment': return _markDeletedEmployment(...);
    case 'skill': return _markDeletedSkill(...);
    case 'taxId': return _markDeletedTaxId(...);
    case 'passport': return _markDeletedPassport(...);
    case 'visa': return _markDeletedVisa(...);
    case 'travelHistory': return _markDeletedTravelHistory(...);
    case 'card': return _markDeletedCard(...);
    case 'contact': return _markDeletedContact(...);
    case 'language': return _markDeletedLanguage(...);
    case 'award': return _markDeletedAward(...);
    case 'idCard': return _markDeletedIdCard(...);
    case 'address': return _markDeletedAddress(...);
    // ... 共 26 个 case
  }
}
```

**对比** `operation_log_aggregator.dart`：17 个 case 的 switch。

**建议**：策略模式 / Map 派发替代 switch。

---

#### 巨型文件 — 🟠 多个文件超过 1000 行

| 文件 | 行数 | 问题 |
|------|------|------|
| `search_page.dart` | 1324 | 41 个 `addResult`，手动索引每个字段 |
| `operation_log_page.dart` | 1631 | 巨型页面 |
| `sensitivity_provider.dart` | 1091 | 巨型 provider |
| `auth_provider.dart` | 1331 | 虽然拆分为 8 个类，但仍在同一文件 |

---

#### Riverpod v1/v2 混用 — 🟡

| 模式 | 使用位置 | 问题 |
|------|---------|------|
| `StateNotifier` | `AuthNotifier`, `ProfileNotifier` | v1 风格，无内置 loading/error |
| `AsyncNotifier` | `AccountsNotifier` | v2 风格，有 `.when()` |

**建议**：统一迁移到 `AsyncNotifier` + `AsyncValue`。

---

#### go_router 已依赖但未迁移 — 🟡

```yaml
# pubspec.yaml
dependencies:
  go_router: ^14.2.0  # 已存在
```

```dart
// main.dart
MaterialApp(
  routes: { ... }  // ← 仍在使用旧 API
)
```

**建议**：迁移到 go_router，享受声明式路由和深层链接支持。

---

## 二、依赖分析

### 2.1 已依赖但未使用的包

| 包 | 版本 | 使用情况 | 建议 |
|----|------|---------|------|
| `freezed` | ^2.5.7 | **0 使用** | 引入 immutable pattern |
| `freezed_annotation` | ^2.4.4 | **0 使用** | 同上 |
| `riverpod_generator` | ^2.6.2 | **0 使用** | 简化 provider 定义 |
| `go_router` | ^14.2.0 | **0 使用** | 迁移导航系统 |

### 2.2 手写 vs 生成代码混用

| 类 | 状态 |
|----|------|
| 22 个模型 | 有 `@JsonSerializable()` + 手写 `fromJson`/`toJson` |
| 生成 `_$XxxFromJson` / `_$XxxToJson` | **死代码**（0 处调用） |
| `ProfileData` | 无 `@JsonSerializable()`，纯手写 |

---

## 三、安全与内存问题

### 3.1 DebugLogger 修复验证 ✅

已修复：`debug_logger.dart` 添加 `kDebugMode` 条件，Release 模式不再写磁盘日志。

### 3.2 内存清理验证 ✅

`SecureAccountStorage` 第 610 行附近有内存清理逻辑，清除 salt 和 verifyKey。

### 3.3 `on Exception catch (e)` 修复验证 ✅

`migrateAccountCryptoVersion` 中的过度捕获已修复为 `catch (e, st)` 并记录栈轨迹。

---

## 四、数据完整性风险

### 4.1 mutation 导致的状态不一致

由于 `restoreItem` 和 `permanentDeleteItem` 直接 mutation 传入对象，如果调用方持有旧引用，可能看到过期状态。

### 4.2 copyWith 与 mutation 混用

```dart
// auth_provider.dart
final prof = profile.travel!;
prof.passports[index] = prof.passports[index].copyWith(isDeleted: false);
// ↑ prof 是 profile.travel 的引用
// ↑ 但 profile.travel 本身是 profile 的引用
// 如果有第三方持有 profile 引用，会看到 passport 被修改，但 travel 引用不变
```

---

## 五、第五轮优先路线图

### P0 — 引入 freezed / immutable pattern

1. **为 `ProfileData` 添加 `@freezed`**
   - 所有字段变为 final
   - `copyWith` 变为纯函数
   - `emptyAllTrash` / `restoreItem` / `permanentDeleteItem` 返回新对象

2. **替换所有手写 `fromJson`/`toJson`**
   - 22 个模型切换到生成代码
   - 删除手写序列化逻辑

### P0 — 消除 mutation

3. **`emptyAllTrash` 改为纯函数**
   ```dart
   ProfileData emptyAllTrash(ProfileData profile) {
     return profile.copyWith(
       travel: profile.travel?.copyWith(
         passports: profile.travel!.passports.where((p) => !p.isDeleted).toList(),
       ),
       // ...
     );
   }
   ```

4. **`restoreItem` / `permanentDeleteItem` 改为纯函数**

### P1 — 策略模式替代 switch

5. **ProfileSectionEditor switch → Map 派发**
   ```dart
   final _mutators = {
     'education': _markDeletedEducation,
     'bankAccount': _markDeletedBankAccount,
     // ...
   };
   ```

6. **operation_log_aggregator switch → 同上**

### P1 — 巨型文件拆分

7. **search_page.dart** (1324 行) → 拆分为 `SearchController` + `SearchResults` + `SearchFilters`
8. **operation_log_page.dart** (1631 行) → 拆分为 `LogListView` + `LogDetailView` + `LogFilterSheet`
9. **sensitivity_provider.dart** (1091 行) → 拆分为多个专注的 provider

### P2 — go_router 迁移

10. **main.dart 路由迁移**
    - `MaterialApp.routes` → `GoRouter`
    - 利用 go_router 的声明式路由和深层链接

### P2 — Riverpod v2 统一

11. **StateNotifier → AsyncNotifier**
    - `AuthNotifier` → `AuthAsyncNotifier`
    - `ProfileNotifier` → `ProfileAsyncNotifier`

### P3 — riverpod_generator 采用

12. **使用 `@riverpod` 注解简化 provider 定义**

---

## 六、可持续性评分（第五轮）

| 改进项 | 评分 | 理由 |
|--------|------|------|
| freezed / immutable pattern | ⭐⭐⭐⭐⭐ | 消除 mutation 副作用，数据完整性保障 |
| 消除 mutation | ⭐⭐⭐⭐⭐ | 纯函数化，undo/redo 可行 |
| 策略模式替代 switch | ⭐⭐⭐⭐ | 消除 26/17 case switch，可扩展性 |
| 巨型文件拆分 | ⭐⭐⭐ | 降低认知负担，提高可维护性 |
| go_router 迁移 | ⭐⭐⭐ | 声明式路由，深层链接支持 |
| Riverpod v2 统一 | ⭐⭐⭐ | 统一异步模式，减少样板代码 |
| riverpod_generator 采用 | ⭐⭐ | 简化 provider 定义，但需改造现有代码 |

---

## 七、已修复问题汇总（四轮累计）

| 轮次 | 修复数 | 主要问题 |
|------|--------|---------|
| Round 1 | 30+ | Repository 层删除、敏感性动态化 |
| Round 2 | 25+ | State 管理规范化、内存泄漏修复 |
| Round 3 | 30+ | json_serializable、lint 规则启用 |
| Round 4 | 25+ | AccountsNotifier 副作用、ProfileSectionState 僵尸抽象 |
| Round 5 (本轮) | 5 | kDebugMode、autoDispose、catch(e) 栈轨迹、@override、@deprecated |
