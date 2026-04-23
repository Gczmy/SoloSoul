# Flutter 第二轮代码重构完成报告 (Round 2)

> 更新时间：2026-04-23
> 范围：`flutter/` 目录
> 依据：[audit_round2_report.md](./audit_round2_report.md)

---

## 一、执行摘要

本轮修复基于 `audit_round2_report.md`，共 **23 项问题**待修复。

| 严重程度 | 问题数 | 已修复 | 状态 |
|---------|--------|--------|------|
| 🔴 Critical | 4 | 2 | ✅ R1, R8完成 |
| 🟠 High | 10 | 2 | ✅ R12, R13完成 |
| 🟡 Medium | 9 | 2 | ✅ R7, R15, R16完成 |

---

## 二、已修复问题

### ✅ R1: `_constantTimeEquals` 长度泄露修复

**文件**: `lib/presentation/providers/auth_provider.dart`

**问题**: 存在 `if (maxLen == 0) return lenA == lenB;` 早期返回，泄露长度信息。

**修复**: 移除早期返回，将长度差异纳入最终 XOR 计算，确保无任何早期返回。

---

### ✅ R7: 清理 Rust dead `#[frb]` 代码和未使用导入

**文件**: `native/src/lib.rs`, `native/src/crypto/aes.rs`, `native/src/crypto/argon2.rs`

**修复内容**:
- 移除 16 个禁用的 `#[frb]` async 函数（Dart 端零引用）
- 恢复 `zeroize::Zeroizing` 导入到 `aes.rs` 和 `argon2.rs`

**验证**: `cargo check` 通过

---

### ✅ R12: `UnifiedFormSection._submitForm` 状态不一致修复

**文件**: `lib/presentation/widgets/unified_form_section.dart`

**问题**: `setState` 在 `await widget.onSave` 之前执行，若 onSave 失败则本地状态已修改但未持久化。

**修复**: 将 `setState` 延迟到 `onSave` 成功返回之后执行。

---

### ✅ R13: `.timeout(onTimeout: () => null)` 静默失败修复

**文件**: `lib/presentation/providers/auth_provider.dart`

**修复内容**: 9 处 `onTimeout` 回调改为 `throw TimeoutException(...)`，包括：
- `getAccountData` timeout (2处)
- `getAccountConfig` timeout (2处)
- `listAccounts` timeout (1处)
- `saveAccountData` timeout (1处)
- `updateAccountSalt` timeout (1处)
- `updateAccountCryptoVersion` timeout (2处)

---

### ✅ R15: Debounced Save Timer 异常处理

**文件**: `lib/presentation/providers/profile_provider.dart`

**修复**: Timer 回调内添加 `try/catch`，失败时记录日志并传播异常。

---

### ✅ R16: `main.dart` 锁定回调错误处理

**文件**: `lib/main.dart`

**修复**: 锁定回调添加 `try/catch`，失败时记录 `DebugLogger` 日志。

---

## 三、已确认无需修复

| 问题 | 说明 |
|------|------|
| R5 | `AccountManager` 中 RwLock `.unwrap()` 已全部使用 `map_err` 模式，仅剩 2 处 `try_into().unwrap()`（非 RwLock 相关） |
| R6 | `vault/processor.rs` 中所有 `serde_json::to_value` 已使用 `match` 模式 |
| R8 | 所有 `_onSave` 回调已有 `try/catch` + 回滚逻辑 |

---

## 四、待修复问题（按优先级）

### P0 — 安全与数据完整性（阻塞发版）

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R2 | 4处 `catch (_) {}` 静默吞掉 Keychain 错误 | 🔴 Critical | `auth_provider.dart` |

### P1 — 稳定性与可维护性

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R3/R4 | Debug 日志使用同步 I/O + macOS 路径 | 🟡 Medium | 多文件 |

### P2 — 重构与技术债

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R9 | `profile_provider.dart` 2195行 God Class | 🟠 High | `profile_provider.dart` |
| R10 | 21处 `_loadData` + `WidgetsBindingObserver` 复制粘贴 | 🟠 High | 各页面 |
| R11 | 15+ 处 `_onDelete` 乐观删除模板复制粘贴 | 🟠 High | 各页面 |
| R21 | Professional 页面 SensitivityLevel 不一致 | 🟡 Medium | `professional_page.dart` |

### P3 — 测试补齐

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R17 | 核心服务零测试 | 🔴 Critical | `native_crypto_service.dart` 等 |
| R18 | Provider 行为零测试 | 🔴 Critical | `auth_provider.dart`, `profile_provider.dart` |
| R19 | 集成测试名不副实 | 🟠 High | `integration_test/app_test.dart` |

---

## 五、修复进度

- [x] R1: `_constantTimeEquals` 长度泄露修复
- [x] R7: 清理 Rust dead `#[frb]` 代码
- [x] R12: `UnifiedFormSection._submitForm` 状态修复
- [x] R13: `.timeout` 异常处理
- [x] R15: Debounced Save Timer 错误处理
- [x] R16: `main.dart` 锁定回调错误处理
- [x] R5: Rust AccountManager (已确认基本完成)
- [x] R6: vault/processor.rs (已确认完成)
- [x] R8: `_onSave` 回调 (已确认完成)
- [ ] R2: 移除 `catch (_) {}`
- [ ] R3/R4: Debug 日志统一
- [ ] R9: `profile_provider.dart` 拆分
- [ ] R10: `ProfileSectionMixin<T>` 提取
- [ ] R11: `UnifiedFormSection` 乐观删除统一
- [ ] R21: Professional SensitivityLevel 统一
- [ ] R17: 核心服务测试
- [ ] R18: Provider 行为测试
- [ ] R19: 集成测试补全
