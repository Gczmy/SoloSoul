# SoloSoul Riverpod 2.x → 3.x + Flutter 升级方案报告

> 生成日期：2026-04-24  
> 当前 Flutter：3.41.6 (Dart 3.11.4)  
> 当前 Riverpod：2.6.1  
> 目标 Riverpod：3.0.x (latest stable)

---

## 一、执行摘要

**结论：建议采用「分阶段升级」策略。**

- **Flutter**：当前 3.41.6 已是 2026 Q1 最新稳定版，**无需升级**。可等待 3.44 (2026年5月) 或 Flutter 4.0 (2026年中)。
- **Riverpod**：3.0 稳定版已于 2025年9月发布。升级收益明显（API 简化、自动重试、Mutation 支持），但存在 **中等规模 breaking changes**，预估需要 **1.5～2 天** 的迁移工作量。

---

## 二、当前状态分析

### 2.1 Flutter 版本评估

| 维度 | 现状 | 评估 |
|------|------|------|
| 当前版本 | Flutter 3.41.6 / Dart 3.11.4 | 2026年3月发布，属于最新稳定版 |
| 下一个稳定版 | Flutter 3.44 (预计 2026年5月) | 分支已于 2026-04-07 截止 |
| 远期版本 | Flutter 4.0 (预计 2026年中) | 重大版本，Material/Cupertino 解耦 |
| Impeller | iOS/Android 已默认稳定 | 无性能担忧 |

**建议**：Flutter 保持 3.41.6，待 3.44 发布后再评估是否升级。当前版本完全满足 Riverpod 3.0 的兼容性要求（Riverpod 3.0 要求 Dart ≥ 3.0）。

### 2.2 Riverpod 使用盘点

| 特性 | 使用数量 | 文件位置 |
|------|---------|---------|
| `@riverpod` + Code Generation | **29 处** | `profile_provider.dart`, `auth_notifier.dart`, `sensitivity_provider.dart` 等 |
| `AsyncNotifier` | **大量使用** | `AuthNotifier`, `ProfileNotifier`, `AccountsNotifier` 等 |
| `StateNotifier` + `StateNotifierProvider` | **4 个类 + 3 处 Provider** | `AuthStateNotifier`, `SensitivePageAccessNotifier`, `FieldHistoriesNotifier`, `DebugModeNotifier` |
| `ChangeNotifierProvider` | **1 处** | `operation_log_provider.dart` |
| `StateProvider` (含 `.family`) | **2 处** | `settings_page.dart`, `entry_card_widget.dart` |
| `Provider.family` | **1 处** | `sensitivity_based_visibility_widget.dart` |
| `AutoDisposeNotifier` (生成的) | **大量** | 所有 `.g.dart` 文件中 |
| `ProviderRef` 子类 | **0 处直接使用** | 仅出现在生成的 `.g.dart` 中 |

### 2.3 关键依赖链

```
flutter_riverpod: ^2.6.1
riverpod_annotation: ^2.6.1      ─┐
riverpod_generator: ^2.6.2       ─┼── 这三者必须同步升级
state_notifier: ^1.0.0          ─┘   Riverpod 3.0 内置 Notifier，可移除
```

---

## 三、Riverpod 3.0 Breaking Changes 影响评估

### 🔴 高影响（必须修改）

#### 1. `StateNotifierProvider` → `AsyncNotifier` / `Notifier`

**影响范围**：4 个 `StateNotifier` 子类 + 3 处 Provider 声明

| 类名 | 当前定义 | 建议迁移目标 |
|------|---------|-------------|
| `AuthStateNotifier` | `StateNotifier<AuthState>` | `Notifier<AuthState>` |
| `SensitivePageAccessNotifier` | `StateNotifier<SensitivePageAccessState>` | `Notifier<SensitivePageAccessState>` |
| `FieldHistoriesNotifier` | `StateNotifier<FormHistories>` | `Notifier<FormHistories>` |
| `DebugModeNotifier` | `StateNotifier<bool>` | `Notifier<bool>` |

**迁移示例**：

```dart
// Before (Riverpod 2.x)
class AuthStateNotifier extends StateNotifier<AuthState> {
  AuthStateNotifier() : super(AuthState.initial);
  void setLoading() => state = AuthState.loading;
}

final authStateProvider = StateNotifierProvider<AuthStateNotifier, AuthState>(
  (ref) => AuthStateNotifier(),
);

// After (Riverpod 3.x)
class AuthStateNotifier extends Notifier<AuthState> {
  @override
  AuthState build() => AuthState.initial;
  void setLoading() => state = AuthState.loading;
}

final authStateProvider = NotifierProvider<AuthStateNotifier, AuthState>(
  AuthStateNotifier.new,
);
```

**工作量**：约 30 分钟（4 个类 + 对应的 import 和 consumer 更新）。

#### 2. `ChangeNotifierProvider` → `Notifier`

**影响范围**：1 处 (`operation_log_provider.dart`)

`OperationLogService` 如果使用了 `notifyListeners()`，需要改为 `Notifier` 的 `state = ...` 模式。

**注意**：如果 `OperationLogService` 是 Flutter 的 `ChangeNotifier` 且被多处监听（非 Riverpod 机制），可能需要保留 `ChangeNotifier` 并改用 `Provider` 包裹。

#### 3. `StateProvider` → `NotifierProvider`

**影响范围**：2 处

```dart
// Before
final debugModeProvider = StateNotifierProvider<DebugModeNotifier, bool>((ref) {
  return DebugModeNotifier();
});

// After
@riverpod
class DebugModeNotifier extends _$DebugModeNotifier {
  @override
  bool build() => false;
  void toggle() => state = !state;
}
```

#### 4. `AutoDispose` 接口统一

**影响范围**：所有生成的 `.g.dart` 文件（约 15+ 个）

Riverpod 3.0 移除了 `AutoDisposeNotifier` / `AutoDisposeAsyncNotifier` 等独立类型，统一为 `Notifier` / `AsyncNotifier`（`autoDispose` 行为通过参数控制）。

**好消息**：由于项目使用 `@riverpod` 代码生成，**只需升级 `riverpod_generator` 后重新运行 `build_runner`**，生成的代码会自动适配新 API。无需手动修改 `.g.dart` 文件。

### 🟡 中影响（需要验证）

#### 5. `AsyncValue.value` 行为变更

**Riverpod 3.0 变化**：
- `AsyncValue.valueOrNull` 被移除，其行为合并到 `.value`
- `AsyncValue.value` 在 `AsyncError` 状态下现在返回 `null`（以前会 throw）

**项目影响**：
- 搜索结果显示项目中 **没有直接使用 `.valueOrNull`**
- 但 `AsyncValue.value` 的使用需要检查：如果代码依赖了 "error 时 throw" 的行为，会静默失败

**建议操作**：全局搜索 `AsyncValue` 的使用模式，确认是否有 `.value` 调用。

#### 6. 默认 `==` 值过滤

**Riverpod 3.0 变化**：所有 provider 现在使用 `==` 比较新旧值，如果相等则不通知 listener。

**项目风险**：
- 项目中的模型类（如 `ProfileData`, `IdentityData` 等）使用的是手动定义的 `@JsonSerializable` 类，**没有实现 `==` 和 `hashCode`**
- 这意味着如果更新一个对象的新实例（字段相同但引用不同），Riverpod 3.0 会认为值变了并通知重建（行为不变）
- **但如果未来实现了 `==`，行为会变化**——当前无需担心，但需要在模型类文档中标注

**建议**：暂不修改模型类。如果未来使用 `freezed` 生成模型，默认会带 `==`，那时需要注意。

#### 7. 自动重试 (Automatic Retry)

**Riverpod 3.0 变化**：失败的 provider 现在默认会自动重试。

**项目风险**：
- `AuthNotifier`, `ProfileNotifier` 等 `AsyncNotifier` 如果在初始化时失败（如 Vault 解锁失败、文件读取失败），会自动重试
- 对于 SoloSoul 这种安全应用，**自动重试可能导致密码验证逻辑被重复调用**，或让错误状态闪烁

**建议**：全局禁用自动重试，保持与 2.x 一致的行为：

```dart
// main.dart
void main() {
  runApp(
    ProviderScope(
      retry: (retryCount, error) => null, // 永不自动重试
      child: const MyApp(),
    ),
  );
}
```

### 🟢 低/无影响

| Breaking Change | 项目使用情况 | 结论 |
|----------------|------------|------|
| `Ref` 子类移除 (`FutureProviderRef` 等) | 仅出现在 `.g.dart` 中 | 重新生成代码即可 |
| `FamilyNotifier` 移除 | 未使用 | 无影响 |
| `ProviderObserver` 签名变化 | 未使用 | 无影响 |
| `overrideWithProvider` / `overrideWithValue` 移除 | 未使用 | 无影响 |
| `StreamProvider` 暂停行为变化 | 未使用 `StreamProvider` | 无影响 |
| `provider.future` / `provider.state` 移除 | 未直接使用 | 无影响 |

---

## 四、依赖升级矩阵

### 4.1 必须升级

| 包 | 当前版本 | 目标版本 | 说明 |
|----|---------|---------|------|
| `flutter_riverpod` | ^2.6.1 | ^3.0.0 | 核心状态管理 |
| `riverpod_annotation` | ^2.6.1 | ^3.0.0 | 注解支持 |
| `riverpod_generator` | ^2.6.2 | ^3.0.0 | 代码生成器 |
| `hooks_riverpod` | 未使用 | — | 如需使用 Hooks，单独添加 |

### 4.2 可移除

| 包 | 当前版本 | 建议 |
|----|---------|------|
| `state_notifier` | ^1.0.0 | **移除**。Riverpod 3.0 内置 `Notifier`，不再依赖此包 |

### 4.3 需验证兼容性

| 包 | 当前版本 | 风险 |
|----|---------|------|
| `flutter_rust_bridge` | ^2.0.0 | 低。与 Riverpod 无直接耦合 |
| `go_router` | ^14.2.0 | 低。仅依赖 `BuildContext` / `ref`，不受 Riverpod 升级影响 |
| `flutter_secure_storage` | ^9.2.2 | 无。独立功能 |
| `freezed` / `freezed_annotation` | ^2.5.7 / ^2.4.4 | 中。`freezed` 生成的类带 `==`，结合 Riverpod 3.0 的 `updateShouldNotify` 需要确认行为 |
| `local_auth` | ^2.3.0 | 无 |
| `flutter_animate` | ^4.5.0 | 无 |

---

## 五、推荐升级方案（分阶段）

### 阶段 0：准备（30 分钟）

1. **创建独立分支**：`git checkout -b upgrade/riverpod-3`
2. **冻结功能开发**：升级期间不合并新功能
3. **备份 pubspec.lock**：`cp pubspec.lock pubspec.lock.backup`
4. **确认测试通过**：运行 `flutter test` 确保基线绿色

### 阶段 1：依赖升级 + 代码生成（1 小时）

1. 修改 `pubspec.yaml`：

```yaml
dependencies:
  flutter_riverpod: ^3.0.0
  riverpod_annotation: ^3.0.0
  # state_notifier: ^1.0.0   # ← 删除此行

dev_dependencies:
  riverpod_generator: ^3.0.0
```

2. 运行 `flutter pub get`
3. 运行 `dart run build_runner build --delete-conflicting-outputs`
4. 检查生成的 `.g.dart` 文件是否有编译错误

### 阶段 2：`StateNotifier` → `Notifier` 迁移（1 小时）

按优先级迁移：

1. `AuthStateNotifier` + `authStateProvider`
2. `SensitivePageAccessNotifier` + `sensitivePageAccessProvider`
3. `FieldHistoriesNotifier` + `fieldHistoriesProvider`
4. `DebugModeNotifier` + `debugModeProvider`

每迁移一个，运行 `dart analyze` 验证。

### 阶段 3：`ChangeNotifierProvider` + `StateProvider` 迁移（30 分钟）

1. `operationLogProvider`：`ChangeNotifierProvider` → `NotifierProvider` 或保留 `Provider`
2. `entry_card_widget.dart`：`StateProvider.family` → `@riverpod` class
3. `settings_page.dart`：`StateNotifierProvider` → `@riverpod` class

### 阶段 4：全局配置与清理（30 分钟）

1. **禁用自动重试**：在 `main.dart` 的 `ProviderScope` 中添加 `retry: (retryCount, error) => null`
2. **移除 `state_notifier` 包**：确认所有 import 已清理
3. **运行 `dart analyze --fatal-infos`**：修复所有 warning
4. **运行 `flutter test`**：修复失败的测试

### 阶段 5：回归测试（2 小时）

1. **单元测试**：`flutter test test/unit/`
2. **Widget 测试**：`flutter test test/widget/`
3. **集成测试**：`flutter test integration_test/`
4. **手动验证**：
   - 创建账户 → 解锁 → 保存 Profile → 锁定 → 重新解锁
   - 敏感页面访问（1 分钟超时）
   - 操作日志记录
   - 设置页面切换

---

## 六、风险与缓解措施

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|---------|
| `build_runner` 生成代码与 3.0 不兼容 | 中 | 高 | 升级 `riverpod_generator` 到 3.0，删除所有 `.g.dart` 后重新生成 |
| `StateNotifier` 迁移遗漏 | 中 | 中 | 使用 `grep -rn "StateNotifier" lib/` 全局检查 |
| 自动重试导致密码验证逻辑异常 | 高 | 高 | **全局禁用自动重试**，保持 2.x 行为 |
| `AsyncValue.value` 行为变化导致静默失败 | 低 | 中 | 全局搜索 `AsyncValue` 使用，添加 regression test |
| `flutter_rust_bridge` 与新版 Flutter 不兼容 | 低 | 高 | 单独验证 FFI 端到端测试 |
| macOS Release 构建签名问题 | 低 | 高 | 升级后在真实机器上测试 Keychain 读写 |

---

## 七、回滚方案

如果升级后出现无法快速修复的问题：

```bash
# 1. 回滚代码
git checkout master

# 2. 回滚依赖
# 恢复 pubspec.yaml 和 pubspec.lock.backup

# 3. 清理并重建
rm -rf pubspec.lock
mv pubspec.lock.backup pubspec.lock
flutter pub get
dart run build_runner build --delete-conflicting-outputs
```

---

## 八、时间线估算

| 阶段 | 预估时间 | 负责人 |
|------|---------|--------|
| 准备 + 分支 | 30 min | Developer |
| 依赖升级 + 代码生成 | 1 hour | Developer |
| StateNotifier 迁移 | 1 hour | Developer |
| ChangeNotifier/StateProvider 迁移 | 30 min | Developer |
| 全局配置 + 清理 | 30 min | Developer |
| 测试修复 + 回归 | 2 hours | Developer + QA |
| **总计** | **~6 小时 (1 工作日)** | |

---

## 九、Flutter 升级建议（独立决策）

当前 Flutter 3.41.6 已满足 Riverpod 3.0 的所有兼容性要求。**不建议在 Riverpod 升级的同时升级 Flutter**，以免问题来源难以定位。

| 时机 | 建议操作 |
|------|---------|
| 现在 | 保持 Flutter 3.41.6，仅升级 Riverpod |
| 2026年5月 | Flutter 3.44 发布后评估升级（Impeller 改进、Wasm 支持） |
| 2026年中 | Flutter 4.0 发布后评估（Material/Cupertino 解耦，可能有 breaking changes） |

---

## 十、参考资源

- [Riverpod 3.0 Migration Guide](https://riverpod.dev/docs/3.0_migration)
- [Riverpod 3.0 Changelog](https://pub.dev/packages/riverpod/changelog)
- [Riverpod 3.0 What's New](https://riverpod.dev/docs/whats_new)
- [Flutter 3.41 Release Notes](https://docs.flutter.dev/release/release-notes)

---

*报告结束。如需执行此升级计划，请确认后进入阶段 0。*
