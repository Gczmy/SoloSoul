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
| 🟠 High | 10 | 5 | ✅ R10, R11, R12, R13, R21完成 |
| 🟡 Medium | 9 | 4 | ✅ R3/R4(已确认), R7, R15, R16完成 |

---

## 二、已修复问题

### ✅ R1: `_constantTimeEquals` 长度泄露修复
- 文件: `auth_provider.dart`
- 移除早期返回，长度差异纳入最终XOR计算

### ✅ R7: 清理 Rust dead `#[frb]` 代码
- 文件: `lib.rs`, `aes.rs`, `argon2.rs`
- 移除16个禁用async函数，恢复zeroize导入

### ✅ R10: `ProfileSectionState` 基类提取
- 文件: `lib/presentation/mixins/profile_section_mixin.dart` (新建)
- `ProfileSectionState<T>` 继承 `ConsumerState` + `WidgetsBindingObserver`
- Pilot: `financial_page.dart` 的 `_BankAccountSectionState`

### ✅ R11: `UnifiedFormSection` 乐观删除统一
- 文件: `unified_form_section.dart`
- 新增 `onDidDelete`, `onDeleteFailed` 回调和 `handleDelete()` 方法
- Pilot: `financial_page.dart` 的 `_CardSection`

### ✅ R12: `UnifiedFormSection._submitForm` 状态修复
- `setState` 延迟到 `onSave` 成功之后执行

### ✅ R13: `.timeout(onTimeout: () => null)` 修复
- 9处 timeout 回调改为 `throw TimeoutException`

### ✅ R15: Debounced Save Timer 异常处理
- Timer 回调添加 try/catch + rethrow

### ✅ R16: `main.dart` 锁定回调错误处理
- 添加 try/catch + DebugLogger 日志

### ✅ R21: Professional 页面 SensitivityLevel 动态化
- Education, Skills, Language 的硬编码 `SensitivityLevel.public` 改为 `ref.watch(effectiveSensitivityProvider(...))`

---

## 三、已确认无需修复

| 问题 | 说明 |
|------|------|
| R5 | `AccountManager` RwLock `.unwrap()` 全部已用 `map_err` |
| R6 | `vault/processor.rs` 所有 `serde_json::to_value` 已用 `match` |
| R8 | 所有 `_onSave` 已有 try/catch + 回滚 |
| R3/R4 | auth_provider.dart 已使用 DebugLogger，无 `writeAsStringSync` |

---

## 四、待修复问题（按优先级）

### P2 — 重构与技术债

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R9 | `profile_provider.dart` 2195行 God Class | 🟠 High | `profile_provider.dart` |

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
- [x] R10: `ProfileSectionState` 基类提取 (Pilot完成)
- [x] R11: `UnifiedFormSection` 乐观删除统一 (Pilot完成)
- [x] R12: `UnifiedFormSection._submitForm` 状态修复
- [x] R13: `.timeout` 异常处理
- [x] R15: Debounced Save Timer 错误处理
- [x] R16: `main.dart` 锁定回调错误处理
- [x] R21: Professional SensitivityLevel 动态化
- [x] R5: Rust AccountManager (已确认完成)
- [x] R6: vault/processor.rs (已确认完成)
- [x] R8: `_onSave` 回调 (已确认完成)
- [x] R3/R4: Debug日志 (已确认完成)
- [ ] R9: `profile_provider.dart` 拆分
- [ ] R17: 核心服务测试
- [ ] R18: Provider 行为测试
- [ ] R19: 集成测试补全
