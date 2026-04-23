# Flutter 第二轮代码重构完成报告 (Round 2)

> 更新时间：2026-04-23
> 范围：`flutter/` 目录
> 依据：[audit_round2_report.md](./audit_round2_report.md)

---

## 一、执行摘要

本轮修复基于 `audit_round2_report.md`，共 **23 项问题**待修复。

| 严重程度 | 问题数 | 已修复 | 状态 |
|---------|--------|--------|------|
| 🔴 Critical | 4 | 0 | 进行中 |
| 🟠 High | 10 | 0 | 进行中 |
| 🟡 Medium | 9 | 1 | ✅ R7完成 |

---

## 二、已修复问题

### ✅ R7: 清理 Rust dead `#[frb]` 代码和未使用导入

**文件**: `native/src/lib.rs`

**修复内容**:
- 移除 16 个禁用的 `#[frb]` async 函数（Dart 端零引用）：
  - `init_account_manager_async`, `init_vault`, `unlock_vault`, `lock_vault`
  - `is_vault_unlocked`, `list_accounts`, `get_vault_stats`
  - `list_profiles`, `create_profile`, `real_save_profile`
  - `real_load_profile`, `real_delete_profile`, `real_list_profiles`
  - `encrypt_data`, `decrypt_data`, `ping`
- 恢复 `zeroize::Zeroizing` 导入到 `aes.rs` 和 `argon2.rs`（上轮错误移除）
- 保留 `FfiVaultStats`、`BridgeAccountInfo`、`BridgeProfileSummary` 结构体（潜在未来 FFI 用途）

**验证**: `cargo check` 通过，仅有 warning 无 error

---

## 三、待修复问题（按优先级）

### P0 — 安全与数据完整性（阻塞发版）

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R1 | `_constantTimeEquals` 长度泄露 | 🔴 Critical | `auth_provider.dart` |
| R2 | 4处 `catch (_) {}` 静默吞掉 Keychain 错误 | 🔴 Critical | `auth_provider.dart` |
| R8 | `_onSave` 回调无 try/catch | 🔴 Critical | 各数据页面 |
| R5 | AccountManager 20处 `.unwrap()` | 🔴 Critical | `account/manager.rs` |

### P1 — 稳定性与可维护性

| # | 问题 | 严重性 | 文件 |
|---|------|--------|------|
| R12 | `UnifiedFormSection._submitForm` 状态不一致 | 🟠 High | `unified_form_section.dart` |
| R13 | `.timeout(onTimeout: () => null)` 隐藏失败 | 🟠 High | 多个文件 |
| R15 | Debounced Save Timer 静默吞异常 | 🟡 Medium | `profile_provider.dart` |
| R6 | `vault/processor.rs` 4处 `.unwrap()` | 🟠 High | `vault/processor.rs` |
| R16 | `main.dart` 锁定回调无错误处理 | 🟡 Medium | `main.dart` |
| R3/R4 | Debug 日志使用同步 I/O | 🟡 Medium | 多文件 |

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

## 四、修复进度

- [x] R7: 清理 Rust dead `#[frb]` 代码和未使用导入
- [ ] R5: Rust AccountManager 20处 `.unwrap()` 改为 `map_err`
- [ ] R6: `vault/processor.rs` 4处 `.unwrap()` 改为 `match`
- [ ] R1: `_constantTimeEquals` 长度泄露修复
- [ ] R2: 移除 4处 `catch (_) {}`
- [ ] R8: `_onSave` 回调添加 try/catch
- [ ] R12: `UnifiedFormSection._submitForm` 状态修复
- [ ] R13: `.timeout` 异常处理
- [ ] R15: Debounced Save Timer 错误处理
- [ ] R16: `main.dart` 锁定回调错误处理
- [ ] R3/R4: Debug 日志统一
- [ ] R9: `profile_provider.dart` 拆分
- [ ] R10: `ProfileSectionMixin<T>` 提取
- [ ] R11: `UnifiedFormSection` 乐观删除统一
- [ ] R21: Professional SensitivityLevel 统一
- [ ] R17: 核心服务测试
- [ ] R18: Provider 行为测试
- [ ] R19: 集成测试补全
