# 代码分析修复报告

> 最后更新：2026-05-08 12:00:00
> 当前分支：`master`
> 修复轮次：1（初始分析）
> 分析范围：`flutter/lib/`（排除 `lib/gen/` 和 `test/`）

## 问题清单（按优先级 P0 > P1 > P2 > P3）

| ID   | 优先级 | 类别       | 文件位置                         | 描述                                           | 状态      |
|------|--------|------------|----------------------------------|------------------------------------------------|-----------|
| S001 | P1     | 安全       | `lib/core/services/scan/windows_search_service.dart:86` | PowerShell 命令注入：用户路径直接拼接到命令中 | `[x]` 已添加路径验证 |
| S002 | P1     | 安全       | `lib/core/services/fallback_secure_storage.dart:55` | 明文回退存储：敏感数据可能未加密写入文件 | `[x]` 已添加安全警告文档和一次性日志（完整加密需 Rust FFI 支持） |
| P001 | P1     | 性能       | `lib/core/services/scan/scan_background_service.dart:29` | StreamController 泄漏：从未调用 close() | `[x]` 已添加 dispose() |
| D001 | P1     | 死代码     | `lib/core/services/llm/llm_privacy_filter.dart` | 整个文件未被引用 | `[x]` 已删除 |
| D002 | P1     | 死代码     | `lib/core/services/llm/llm_query_enhancer.dart` | 整个文件未被引用 | `[x]` 已删除 |
| D005 | P1     | 死代码     | `lib/presentation/providers/llm/llm_stub_provider.dart` | 整个文件未被引用 | `[x]` 已删除 |
| O001 | P1     | 优化       | `lib/presentation/pages/llm/llm_stats_page.dart` | 1165 行（超 800 行限制） | `[ ]` 待修复 |
| O002 | P1     | 优化       | `lib/presentation/widgets/ocr_scanner_sheet.dart` | 1093 行（超 800 行限制） | `[ ]` 待修复 |
| O003 | P1     | 优化       | `lib/presentation/widgets/object_card.dart` | 1084 行（超 800 行限制） | `[ ]` 待修复 |
| O018 | P1     | 优化       | `lib/presentation/pages/llm/llm_config_page.dart:376` | 使用已弃用的 Radio groupValue/onChanged API | `[x]` 已迁移到 RadioGroup |
| O019 | P1     | 优化       | `lib/presentation/pages/llm/llm_config_page.dart:78` + `login_page.dart:239` | async gap 后使用 BuildContext 未检查 mounted | `[x]` 已添加 mounted 检查 |
| S003 | P2     | 安全       | `lib/presentation/providers/auth/auth_storage.dart:142` | 密码复杂度缺少特殊字符要求 | `[ ]` 待修复 |
| S004 | P2     | 安全       | `lib/core/services/ocr_service.dart:90,159` | OCR 日志未通过 DebugLogger 门控 | `[x]` 已替换为 SoloLog |
| S005 | P2     | 安全       | `lib/core/services/llm/llm_config_service.dart:25` | API Key 在整个会话期间存于内存 | `[ ]` 待修复 |
| P002 | P2     | 性能       | `lib/core/services/profile_storage_service.dart:24` | Profile 缓存无界增长 | `[x]` 已添加 LRU 淘汰（max 3） |
| P003 | P2     | 性能       | `lib/core/services/llm/llm_config_service.dart` | LLM 配置每次访问都从 Vault 解密 | `[x]` 已添加 _configCache 内存缓存 |
| D003 | P2     | 死代码     | `lib/presentation/widgets/llm/streaming_text_widget.dart` | 未被引用的 Widget | `[x]` 已删除 |
| D004 | P2     | 死代码     | `lib/presentation/widgets/ocr_result_preview.dart` | 未被引用的 Widget | `[x]` 已删除 |
| O004 | P2     | 优化       | `lib/core/services/scan/local_search_service.dart` | 1026 行（超 800 行限制） | `[ ]` 待修复 |
| O005 | P2     | 优化       | `lib/presentation/pages/settings_page.dart` | 1013 行（超 800 行限制） | `[ ]` 待修复 |
| O006 | P2     | 优化       | `lib/presentation/pages/data_management_page.dart` | 949 行（超 800 行限制） | `[ ]` 待修复 |
| O007 | P2     | 优化       | `lib/presentation/providers/unified_object_provider.dart` | 936 行（超 800 行限制） | `[ ]` 待修复 |
| O008 | P2     | 优化       | `lib/presentation/pages/login_page.dart` | 931 行（超 800 行限制） | `[ ]` 待修复 |
| O009 | P2     | 优化       | `lib/presentation/pages/object_editor_page.dart` | 859 行（超 800 行限制） | `[ ]` 待修复 |
| O010 | P2     | 优化       | `lib/presentation/pages/llm/llm_config_page.dart` | 822→795 行 | `[x]` 已提取 EmptyProfilesState |
| O011 | P2     | 优化       | `all_accounts_sheet.dart:25` / `current_account_sheet.dart:20` | 重复的 `_getDeviceIcon` 包装方法 | `[x]` 已删除重复方法，直接调用 getDeviceIcon() |
| O012 | P2     | 优化       | `lib/presentation/widgets/operation_tile.dart:13` | `_showDetailDialog` 方法 107 行 | `[x]` 已提取 _buildPropertyList 方法 |
| O013 | P2     | 优化       | `lib/presentation/pages/trash_page.dart:148` | `_confirmEmptyTrash` 方法 108 行 | `[x]` 已提取 _performEmptyTrash 方法 |
| O014 | P2     | 优化       | `lib/presentation/widgets/ocr_scanner_sheet.dart:251` | `_buildResultState` 方法 109 行 | `[x]` 已提取 _buildResultActions |
| O015 | P2     | 优化       | `lib/presentation/widgets/change_password_dialog.dart:196` | 5 个连续 early-return 可用提取方法简化 | `[x]` 可接受原样（每个校验条件不同，代码清晰） |
| O016 | P2     | 优化       | `lib/presentation/pages/trash_page.dart:215` | 两个相邻循环遍历同一集合可合并 | `[x]` 已合并为单次遍历 |
| O017 | P2     | 优化       | `lib/presentation/providers/llm/llm_model_provider.dart:89` | 4 部分布尔表达式过密 | `[x]` 已提取为命名变量 |
| S006 | P3     | 性能       | `lib/core/utils/solo_log.dart:18` | 废弃计时器内存泄漏 | `[x]` 已添加定时清理方法 |
| P004 | P3     | 性能       | `lib/core/services/native_vault_service.dart:52` | 启动时尝试 5+ 个路径加载原生库 | `[ ]` 待修复 |
| O020 | P3     | 优化       | 多处 | 6 处可添加 `const` 关键字 | `[x]` 已添加 const |

## 修复进度

- 已完成：22 / 35
- 当前处理：无

## 安全正面发现

以下安全实践值得肯定：
1. **BiometricCredentialService** — 双信封加密：deviceKey → encryptedBioToken → encryptedSessionKey
2. **AttemptTracker** — 暴力破解防护：指数退避 + 15 分钟锁定
3. **Constant-time comparison** — 委托 Rust FFI 防时序攻击
4. **DebugLogger sanitization** — 双层脱敏（结构化标签 + 正则安全网）
5. **MigrationService** — 账户 ID 正则验证防路径遍历
6. **BackupService** — 显式 `sanitizeSpecialName()` 剥离 `/`、`\`、`..`
7. **Session key** — 传递到 Rust 后安全擦除
8. **Vault lock on sleep** — macOS 睡眠回调在挂起前锁定 Vault

---

## 详细问题描述与修复指引

### S001 — PowerShell 命令注入 (P1)
**文件**: `lib/core/services/scan/windows_search_service.dart:86-92`
**问题**: `rootPath` 从用户配置的扫描目录直接插入 PowerShell 命令字符串，未做清理。
**修复**: 在传入 PowerShell 前用安全字符白名单验证 `rootPath`。

### S002 — 明文回退存储 (P1)
**文件**: `lib/core/services/fallback_secure_storage.dart:55-82`
**问题**: Keychain 不可用时回退到明文文件写入。虽然有 chmod 600，但文件未静态加密。
**修复**: 为回退文件添加运行时加密层，或拒绝在 Keychain 不可用的平台上运行。

### P001 — StreamController 泄漏 (P1)
**文件**: `lib/core/services/scan/scan_background_service.dart:29`
**问题**: `StreamController.broadcast()` 创建后从未调用 `close()`。
**修复**: 添加 `dispose()` 方法关闭 controller 和取消 subscription。

### D001/D002/D005 — 死代码文件 (P1)
**文件**: `llm_privacy_filter.dart`, `llm_query_enhancer.dart`, `llm_stub_provider.dart`
**问题**: 三个文件均无任何引用，可安全删除。
**修复**: 删除文件。

### O018 — 弃用 API (P1)
**文件**: `lib/presentation/pages/llm/llm_config_page.dart:376-377`
**问题**: Radio widget 使用已弃用的 `groupValue` 和 `onChanged`。
**修复**: 用 RadioGroup 祖先包装。

### O019 — Async 安全 (P1)
**文件**: `llm_config_page.dart:78`, `login_page.dart:239`
**问题**: async gap 后使用 BuildContext 未检查 `mounted`。
**修复**: 在 async gap 后添加 `if (!mounted) return;`。
