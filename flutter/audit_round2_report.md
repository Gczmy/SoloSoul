# SoloSoul Flutter 第二轮深度诊断报告

> 生成时间：2026-04-23  
> 前提：基于 `audit_report.md` 的 70+ 项修复已完成（见 `audit_done_report.md`）  
> 范围：`flutter/` 目录（Dart + Rust native + 平台配置）  
> 方法：修复验证 + 回归检测 + 未覆盖领域深挖

---

## 一、修复验证结论

### ✅ 已验证完成的关键修复

| 修复项 | 验证结果 | 说明 |
|--------|---------|------|
| C1 UTF-16 长度修复 | ✅ 正确 | `native_vault_service.dart:176` 使用 `utf8.encode(requestJson).length` |
| C2 密码编码统一 | ✅ 正确 | `native_crypto_service.dart:223,285` 统一使用 `utf8.encode(password)` |
| C3 Rust catch_unwind | ✅ 正确 | `native/src/lib.rs:441-471` 包裹正确，所有权无泄漏 |
| C6/C7 Argon2id 参数 | ✅ 正确 | 默认值升级到 `65536KB / 3 iterations` |
| C9/C10 accountsProvider | ✅ 正确 | 使用 `ref.watch`，监听 AuthState 变化 |
| C11 setState + mounted | ⚠️ 部分 | catch 块中已添加，但 **try 块中的 setState 仍无 mounted 保护** |
| C13 macOS 路径 | ⚠️ 部分 | `login_page.dart` 已改，但 `native_vault_service.dart:27` debug 日志仍用 `~/Library/Logs/` |
| C15 双保存修复 | ✅ 正确 | `historyAwareOnSave` 不再调用 `_onAccountSave` |
| H1/H2 内存清零 | ✅ 正确 | `rust_vault_service.dart` 和 `native_crypto_service.dart` 已添加清零 |
| H3 Keychain 错误传播 | ⚠️ 部分 | `_writeSecure()` 已 `rethrow`，但 **多个调用点仍用 `catch (_) {}` 静默吞掉** |
| H7 路由常量 | ✅ 正确 | 已创建 `AppRoutes` 类 |
| H8 ProfileNotifier dispose | ✅ 正确 | 已添加 `override dispose()` 取消 timer |

---

## 二、修复引入的回归与新问题（🔴 Critical / 🟠 High）

### R1. 🔴 `_constantTimeEquals` 引入新的时序攻击向量

**文件**：`lib/presentation/providers/auth_provider.dart:30-39`

**修复前**：标准字符串 `==` 比较（不安全，但行为简单）。
**修复后**：
```dart
bool _constantTimeEquals(String a, String b) {
  if (a.length != b.length) {  // ← 致命：长度不等时立即返回
    return false;
  }
  var result = 0;
  for (var i = 0; i < a.length; i++) {
    result |= a.codeUnitAt(i) ^ b.codeUnitAt(i);
  }
  return result == 0;
}
```

**问题**：`a.length != b.length` 的**早期返回**泄露了长度信息。攻击者可以通过尝试不同长度的输入，测量响应时间差异，逐字节推断出正确哈希的长度。

**正确做法**：
- 方案 A：对输入进行固定长度的 HMAC/哈希后再比较，比较时强制遍历固定长度
- 方案 B：若必须比较不同长度字符串，先填充到统一长度再遍历，或始终遍历 `max(a.length, b.length)`
- 方案 C：使用 `package:cryptography` 提供的 `constantTimeBytesEquality`

**建议**：删除此函数，改用 `Uint8List` 级别的常数时间比较，且**禁止任何早期返回**。

---

### R2. 🟠 `_writeSecure` 改为 `rethrow`，但 4 个关键调用点仍静默吞掉

**文件**：`lib/presentation/providers/auth_provider.dart`

`_writeSecure()` 已正确改为 `rethrow`（line 180），但以下调用点仍用 `catch (_) {}`：

| 行号 | 上下文 | 影响 |
|------|--------|------|
| `908` | `unlockVault` 中 V1→V2 迁移后保存 | 迁移成功但 Keychain 写入失败时，用户以为成功，下次登录发现数据丢失 |
| `935` | `unlockVault` 中 Rust 账户迁移到 Keychain | 同上 |
| `944` | `unlockVault` 中更新账户 salt | 密码修改后 salt 未保存，下次无法解锁 |
| `950` | `unlockVault` 中更新 crypto version | 版本标记未保存，可能导致重复迁移 |

**问题**：这些路径在解锁流程中，Keychain 写入失败被静默忽略后，用户界面显示"解锁成功"，但持久化状态已损坏。下次启动时可能触发错误迁移或无法识别账户。

**建议**：这些 `catch (_) {}` 必须改为传播错误，或在 `catch` 中设置错误状态并通知用户。

---

### R3. 🟠 Debug 日志仍使用同步 I/O + macOS 路径

**文件**：`lib/core/services/native_vault_service.dart:17-34`

```dart
void _log(String msg) {
  if (!kDebugMode) return;  // Release 已保护
  try {
    final homeDir = Platform.environment['HOME'] ?? '/tmp';
    final logDir = Directory('$homeDir/Library/Logs');  // ← 仍是 macOS 路径
    ...
    logFile.writeAsStringSync(...);  // ← 同步 I/O
  } catch (_) {}
}
```

虽然加了 `kDebugMode`，但：
1. **Debug 模式在 Windows/Linux 上仍会失败**：`Library/Logs` 不存在
2. **同步 `writeAsStringSync` 在 Debug 模式下每次 FFI 调用都触发磁盘 I/O**，可能导致 UI 卡顿（特别是列表查询等高频操作）
3. **未使用 `kDebugMode` 保护的日志仍存在于 `auth_provider.dart`**（见下方 R4）

**建议**：Debug 日志也应使用 `path_provider` + 异步写入，或改用 `dart:developer` 的 `log()` 函数。

---

### R4. 🟡 `auth_provider.dart` 中仍有 20+ 处 `kDebugMode` 保护的同步日志

**文件**：`lib/presentation/providers/auth_provider.dart:231-739`

`createAccount` 和 `unlockVault` 方法中遍布：
```dart
if (kDebugMode) {
  final traceLog = File('${Platform.environment['HOME']}/Library/Logs/flutter_native_vault.log');
  traceLog.writeAsStringSync('...', mode: FileMode.append);
}
```

虽然被 `kDebugMode` 保护，但：
- 每次方法调用触发 3-5 次同步文件 I/O
- 在 Debug 模式下大量调用时（如批量导入、列表渲染）会造成明显 UI jank
- 代码极其冗长，每个 checkpoint 重复 4 行样板代码

**建议**：提取为 `void _debugTrace(String msg)` 辅助函数，统一使用 `dart:developer.log()`。

---

## 三、Rust Native 层新问题

### R5. 🔴 `AccountManager` 20 处 `.unwrap()` 在 `RwLock` 上 — 锁中毒后永久降级

**文件**：`native/src/account/manager.rs`

`catch_unwind` 修复后，panic 不再杀死进程，但如果 panic 发生在持有 `RwLock` 的代码块中，锁会被**中毒（poisoned）**。`AccountManager` 中有 **20 处** `.read().unwrap()` / `.write().unwrap()`：

| 行号 | 代码 | 中毒后果 |
|------|------|---------|
| 117, 128, 145, 172, 248, 476 | `accounts_cache.write/read().unwrap()` | 所有账户缓存操作 panic |
| 256, 261, 486, 491, 550, 556 | `session_key/unlocked_account.write().unwrap()` | 所有解锁/密钥操作 panic |
| 346, 358, 506, 542 | `vault_store.read/write().unwrap()` | 所有 Vault 存储操作 panic |
| 680, 687, 693 | `session_key/unlocked_account.read().unwrap()` | 状态查询 panic |

**影响**：一旦任何 panic 导致锁中毒，`vault_request_ffi` 的 `catch_unwind` 会捕获 panic 并返回 "Internal error"，但锁保持中毒状态。**所有后续调用永久失败**，直到 App 重启。

**建议**：将所有 `.unwrap()` 替换为 `.map_err(|e| format!("Lock poisoned: {}", e))?`，通过 `Result` 传播错误。

---

### R6. 🟠 `vault/processor.rs` 仍有 4 处 `serde_json::to_value(...).unwrap()`

**文件**：`native/src/vault/processor.rs:191,255,290,537`

```rust
serde_json::to_value(&profiles).unwrap()  // 等
```

虽然 `vault_request_ffi` 路径有 `catch_unwind`，但：
- 这些 panic 被捕获后返回不透明的 `"Internal error"`，Dart 无法区分是序列化失败、数据库错误还是其他问题
- 如果未来启用 `#[frb]` 路径（`vault_request` 同步函数），没有 `catch_unwind`，会直接 abort Flutter isolate

**建议**：替换为 `match` 或 `?` 运算符，返回可解析的 JSON 错误。

---

### R7. 🟡 大量死代码（`#[frb]` 禁用函数 + 未使用导入）

**文件**：`native/src/lib.rs:104-385`

以下函数全部被标记为 `// #[frb] // disabled`，Dart 端零引用：
- `init_account_manager_async`, `init_vault`, `unlock_vault`, `lock_vault`
- `is_vault_unlocked`, `list_accounts`, `get_vault_stats`
- `list_profiles`, `create_profile`, `real_save_profile`, `real_load_profile`, `real_delete_profile`
- `ping`

加上未使用导入：`Zeroize`（`argon2.rs`, `aes.rs`）、`serde::de::Error`（`profile.rs`）、`hex_encode`（`store.rs:454`）。

**建议**：清理所有 dead `#[frb]` 代码和未使用导入，减少维护负担。

---

## 四、深层代码质量问题（上次超时未覆盖）

### R8. 🔴 `_onSave` 回调无 try/catch — 数据不一致风险

**文件**：所有数据页面

所有 Section 的保存回调在更新本地状态后，直接 `await provider.updateXxx()`，**没有任何错误捕获**。如果保存失败，UI 状态已与持久化状态不一致。

**关键行号**：
- `profile_page.dart:572`, `936`, `1234`
- `financial_page.dart:310`, `596`, `877`
- `travel_page.dart:324`, `657`, `941`
- `professional_page.dart:301`, `695`, `933`, `1156`, `1390`

**建议**：所有 `_onSave` 回调必须包裹 try/catch，保存失败时回滚本地 `_items` 状态并显示错误。

---

### R9. 🔴 `profile_provider.dart` 2,195 行 God Class

**文件**：`lib/presentation/providers/profile_provider.dart`

这是整个 Flutter 代码库中最大的业务逻辑文件之一，包含：

| 职责 | 行号范围 | 问题 |
|------|---------|------|
| `ProfileNotifier` 核心（加载/保存） | 24-175 | 应与变更日志分离 |
| 变更摘要方法（4个 `_summarizeXxxChanges`） | 219-378 | 应与 notifier 分离 |
| 变更日志方法（`_logIdentityChanges` 等） | 381-789 | 过长的日志逻辑内联在 provider 中 |
| `softDelete` / `restore` / `permanentDelete` | 1004-1568 | 应与加载/保存分离 |
| `_markItemDeleted` / `_markItemRestored` | 1055-1833 | **300+ 行完全对称的 switch 块**，仅 `isDeleted` 和 `deletedAt` 不同 |

**`_markItemDeleted` / `_markItemRestored` 是代码异味典范**：
- 两个方法各自包含 12 个 switch case
- 每处仅 `isDeleted: true/false` 和 `deletedAt: now/null` 不同
- 新增一个 Section（如 Health）时，需要在这两个方法的 12 个 case 中各添加 2 处代码 = 24 处修改

**建议**：拆分为 5+ 个文件；`_markItemDeleted`/`_markItemRestored` 使用映射表或泛型方法统一处理。

---

### R10. 🟠 21 处 `_loadData` + `WidgetsBindingObserver` 完全复制粘贴

**文件**：`profile_page.dart`, `financial_page.dart`, `travel_page.dart`, `professional_page.dart`

每个页面的每个 Section 都包含完全相同的生命周期样板：

```dart
@override
void initState() {
  super.initState();
  WidgetsBinding.instance.addObserver(this);
  _loadData();
}
@override
void didChangeAppLifecycleState(AppLifecycleState state) {
  if (state == AppLifecycleState.resumed) _loadData();
}
@override
void dispose() {
  WidgetsBinding.instance.removeObserver(this);
  super.dispose();
}
void _loadData() {
  final xxx = ref.read(profileNotifierProvider)?.xxx;
  setState(() { _items = [...]; });
}
```

**重复位置**：
- `profile_page.dart` — Contact, IdCard, Address（3 处）
- `financial_page.dart` — BankAccount, Card, TaxId（3 处）
- `travel_page.dart` — Passport, Visa, TravelHistory（3 处）
- `professional_page.dart` — Education, Employment, Skills, Language, Award（5 处）
- 加上 `trash_page.dart`, `operation_log_page.dart` 等共 **21 处**

**建议**：提取 `ProfileSectionMixin<T>` 或基类 `AutoReloadSectionState<T>`。

---

### R11. 🟠 15+ 处 `_onDelete` 乐观删除模板复制粘贴

每个 Section 的删除逻辑完全一致，仅字符串和 itemType 不同：

```dart
setState(() { _items.removeAt(index); });
try {
  await notifier.softDelete(...);
} catch (e) {
  if (mounted) setState(() { _items.insert(index, item); });
  showOverlaySnackBar(...);
}
OperationNotification.show(...);
```

**建议**：在 `UnifiedFormSection` 内部统一处理乐观删除、回滚和通知。

---

### R12. 🟠 `UnifiedFormSection._submitForm` 状态不一致风险

**文件**：`lib/presentation/widgets/unified_form_section.dart:295-344`

```dart
setState(() {
  _items.insert(0, createdItem as T);  // 本地状态已修改
  _mode = 'idle';
});
await widget.onSave(createdItem, values, editingItem);  // 如果这里抛出异常...
```

如果 `onSave` 抛出异常，表单已回到 `idle` 模式，但 `_items` 中已包含未持久化的数据。

**建议**：将 `setState` 的本地修改延迟到 `onSave` 成功返回后执行。

---

### R13. 🟠 静默 `.timeout(onTimeout: () => null)` 隐藏失败

| 文件 | 行号 | 问题 |
|------|------|------|
| `auth_provider.dart:755` | `getAccountData(...).timeout(..., onTimeout: () => null)` | 无法区分"数据不存在"和"读取超时" |
| `auth_provider.dart:785` | 同上 | 同上 |
| `auth_provider.dart:796` | `Future.delayed(...).timeout(..., onTimeout: () => null)` | Rust 配置获取超时返回 null |
| `operation_log_page.dart:184` | `.timeout(..., onTimeout: () => null)` | 日志加载失败静默忽略 |

**影响**：超时会被误判为"数据不存在"，触发不必要的迁移或降级逻辑。

**建议**：`onTimeout` 应抛出 `TimeoutException`，由调用方明确处理。

---

### R14. 🟠 `OperationLogService.addEntry` 20 处 fire-and-forget

**文件**：`lib/presentation/providers/profile_provider.dart:574-684`

```dart
OperationLogService.instance.addEntry(OperationLogger.logXxx(...));
```

无 `await`，无 `try/catch`。如果日志服务失败（如磁盘满），异常会向上传播并打断当前操作流程。

**建议**：使用 `unawaited(Future.microtask(() => ...))` 或 try/catch 包裹。

---

### R15. 🟡 Debounced Save 定时器静默吞掉异常

**文件**：`lib/presentation/providers/profile_provider.dart:169-172`

```dart
_saveDebounceTimer = Timer(_kSaveDebounceDuration, () async {
  _saveDebounceTimer = null;
  await doSave();  // 异常未被捕获，在 Timer 回调中静默丢失
});
```

用户保存失败时没有任何反馈。

**建议**：Timer 回调内部 try/catch，并通过另一个 StateNotifier 或回调将失败信息反馈到 UI。

---

### R16. 🟡 `main.dart` macOS 锁定回调无错误处理

**文件**：`lib/main.dart:80-82`

```dart
NativeChannelService.setLockCallback(() {
  ref.read(authNotifierProvider.notifier).lockVault();
});
```

如果 `lockVault()` 抛出异常，NativeChannel 回调会崩溃。

**建议**：用 try/catch 包裹回调体。

---

## 五、测试覆盖灾难性缺口

### R17. 🔴 核心服务零测试

| 服务文件 | 行数 | 测试文件 | 实际覆盖 |
|----------|------|----------|---------|
| `native_crypto_service.dart` | 486 | 无 | **0%** |
| `profile_storage_service.dart` | 2,662 | 无 | **0%**（仅实例非空检查） |
| `native_vault_service.dart` | 424 | 无 | **0%** |
| `rust_vault_service.dart` | 466 | `rust_vault_service_test.dart` | 仅数据模型，无加密/FFI 行为测试 |

**关键未测试路径**：
- AES-256-GCM 加密/解密往返（Dart fallback 路径）
- PBKDF2 派生密钥正确性
- Profile 序列化/反序列化（尤其软删除标记）
- `RustVaultService` 的 session key 生命周期
- `NativeVaultService.request()` 的 JSON 编解码和错误处理

### R18. 🔴 Provider 行为零测试

| Provider | 测试文件 | 实际测试内容 | 行为覆盖 |
|----------|----------|-------------|---------|
| `AuthNotifier` | `auth_provider_test.dart` | 仅数据类（AccountInfo, DeviceInfo） | **0%** |
| `ProfileNotifier` | `profile_provider_test.dart` | 仅数据模型（ProfileData 等） | **0%** |
| `AccountStyleNotifier` | 无 | — | **0%** |
| `FormFieldRegistryNotifier` | `sensitivity_provider_test.dart` | ✅ 有测试 | 良好 |

**关键未测试行为**：
- `AuthNotifier.createAccount` / `unlockVault` / `changePassword` / `deleteAccount`
- `AuthNotifier` 的 V1→V2 迁移路径
- `ProfileNotifier` 的 debounce 逻辑和并发加载防护
- `ProfileNotifier.softDelete` / `restore` / `permanentDelete`

### R19. 🟠 集成测试名不副实

**文件**：`integration_test/app_test.dart`（170 行）

实际仅测试：
- 应用启动后是否显示 "SoloSoul" 文本
- 页面导航后标题是否存在
- OCR 对话框是否弹出

AGENTS.md 声称的 "FFI 端到端（创建账户/保存/加载/删除 Profile）" 在代码中**完全不存在**。

---

## 六、不一致的模式（设计债务）

### R20. 🟡 `ConsumerStatefulWidget` 泛滥 — 24 个 vs 4 个 `ConsumerWidget`

绝大多数 Section 使用 `ConsumerStatefulWidget` + `WidgetsBindingObserver` 仅为了本地列表缓存（`_items`）和生命周期监听。这些完全可以用 `ConsumerWidget` + Riverpod provider 驱动状态来替代。

### R21. 🟡 SensitivityLevel 查询不一致

- `financial_page.dart` / `travel_page.dart`：统一使用 `ref.watch(effectiveSensitivityProvider('xxx'))`
- `professional_page.dart` 的 `_EducationSection`、`_SkillsSection`、`_LanguageSection`：**大量字段硬编码为 `SensitivityLevel.public`**
- `_EmploymentSection`、`_AwardSection`：混合使用

用户自定义的敏感度设置在 Professional 页面可能不生效。

### R22. 🟡 错误展示方式不一致

| 场景 | 方式 |
|------|------|
| 大多数 Section | `showOverlaySnackBar`（自定义 overlay） |
| `trash_page.dart` | `AlertDialog` |
| `login_page.dart` | `ScaffoldMessenger.showSnackBar` |
| `settings_page.dart` | `AlertDialog` + `ScaffoldMessenger` 混合 |

### R23. 🟡 `accountsProvider` 使用副作用 hack

```dart
final accountsProvider = FutureProvider<List<AccountInfo>>((ref) async {
  final notifier = ref.read(authNotifierProvider.notifier);
  ref.watch(authNotifierProvider);  // 仅为了强制重算
  return notifier.getAccountsSortedByRecent();
});
```

以及 `AuthNotifier.selectAccount` 中的 `state = state;`  hack。

**建议**：使用 Riverpod v2 的 `AsyncNotifier` 或明确的状态变化触发机制。

---

## 七、优先修复路线图（第二轮）

### P0 — 安全与数据完整性（阻塞发版）

1. **[R1]** 修复 `_constantTimeEquals` 的长度泄露（禁止早期返回，或改用 HMAC 后固定长度比较）
2. **[R2]** 移除 `auth_provider.dart` 中 4 处 `catch (_) {}`（line 908, 935, 944, 950），让 Keychain 错误正确传播
3. **[R8]** 为所有 `_onSave` 回调添加 try/catch + 失败回滚
4. **[R5]** Rust `AccountManager` 中 20 处 `.unwrap()` 改为 `map_err` + `?`

### P1 — 稳定性与可维护性

5. **[R12]** `UnifiedFormSection._submitForm` 延迟本地状态修改到 `onSave` 成功后
6. **[R13]** `.timeout(onTimeout: () => null)` 改为抛出 `TimeoutException`
7. **[R15]** Debounced Save Timer 添加 try/catch + 错误反馈
8. **[R6]** `vault/processor.rs` 中 4 处 `serde_json::to_value(...).unwrap()` 改为 `match`
9. **[R16]** `main.dart` 锁定回调添加 try/catch
10. **[R3/R4]** 统一 debug 日志为 `dart:developer.log()`，移除所有 `writeAsStringSync`

### P2 — 重构与技术债

11. **[R9]** 拆分 `profile_provider.dart` 为 5+ 个文件
12. **[R10]** 提取 `ProfileSectionMixin<T>` 统一 21 处 `_loadData` + `WidgetsBindingObserver`
13. **[R11]** 在 `UnifiedFormSection` 内统一处理乐观删除
14. **[R7]** 清理 Rust dead `#[frb]` 代码和未使用导入
15. **[R21]** 统一 Professional 页面的 SensitivityLevel 查询为动态 provider

### P3 — 测试补齐

16. **[R17]** 为 `native_crypto_service.dart` 添加 Dart fallback 加密/解密单元测试
17. **[R18]** 为 `AuthNotifier` 和 `ProfileNotifier` 添加行为测试（使用 mock storage）
18. **[R19]** 补全集成测试：账户创建 → 保存 Profile → 加载 Profile → 删除账户 的完整 FFI 端到端测试
