# 08 — IPC 命令接口完整规范

> **前置阅读**：`04_Rust_Crate拆分与后端架构.md`、`07_数据库_服务层_Repository迁移.md`
> **Manifesto 对齐**：最少惊喜 | 安全默认
> **源文档**：`tauri_refactor/IPC命令接口设计.md`
>
> **[警告] 术语迁移（审批通过）**：本文档中的命令名最终应使用新术语。
> `unified_object_*` → `object_*`。开发时直接使用新命名。
> 详见文档 23 的术语规范。

---

## 1. 接口设计原则

| 原则 | 说明 |
|------|------|
| 命令名 snake_case | `profile_get`, `vault_unlock` |
| 参数/返回值 camelCase | tauri-specta 自动转换 Rust snake_case → TS camelCase |
| 返回值统一 `Result<T, String>` | 错误消息为中文，用户可理解 |
| 敏感数据不出后端 | 密钥、密码绝不通过 IPC 返回 |
| 批量操作用数组 | 减少 IPC 往返次数 |
| 进度通知用 Channel | 不阻塞 UI（OCR、同步、导出等长操作） |

---

## 2. 命令总览（66 个）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 认证 (auth) | 4 | `auth_bootstrap`, `auth_login`, `auth_logout`, `auth_check_has_account` |
| Vault (vault) | 6 | `vault_unlock`, `vault_lock`, `vault_change_password`, `vault_list_accounts` |
| 生物识别 (biometric) | 5 | `biometric_check_availability`, `biometric_save_credential`, `biometric_unlock`, `biometric_delete_credential`, `biometric_test` |
| Profile (profile) | 5 | `profile_get`, `profile_update`, `profile_update_field` |
| UnifiedObject (unified_object) | 8 | `unified_object_list`, `_create`, `_update`, `_delete` |
| 搜索 (search) | 2 | `search_unified`, `search_advanced` |
| 设置 (user_data) | 4 | `user_data_get_preferences`, `user_data_update_preference` |
| 敏感度 (sensitivity) | 4 | `sensitivity_get_field`, `sensitivity_update_field`, `sensitivity_get_log` |
| 导入导出 (export_import) | 3 | `export_import_export`, `export_import_import` |
| 备份 (backup) | 4 | `backup_list`, `backup_create`, `backup_restore`, `backup_delete` |
| 插件 (plugin) | 7 | `plugin_list`, `plugin_install`, `plugin_run`, `plugin_consent_response` |
| OCR (ocr) | 3 | `ocr_initialize`, `ocr_scan` (流式), `ocr_get_status` |
| LLM (llm) | 4 | `llm_send_message` (流式), `llm_get_config` |
| 同步 (sync) | 4 | `sync_discover`, `sync_with_device` (流式) |
| 日志 (log) | 2 | `log_get_recent`, `log_export` |
| 系统 (system) | 3 | `system_get_app_info`, `system_check_version` |

---

## 3. 认证模块

```rust
#[tauri::command]
pub async fn auth_check_has_account() -> Result<bool, String>;
// 返回 true → 跳转登录；false → 跳转引导

#[tauri::command]
pub async fn auth_bootstrap(account_name: String, password: String) -> Result<AccountInfo, String>;

#[tauri::command]
pub async fn auth_login(account_id: String, password: String) -> Result<AccountInfo, String>;

#[tauri::command]
pub async fn auth_logout() -> Result<(), String>;
```

```typescript
interface AccountInfo {
  id: string;        // acc_xxx
  name: string;
  createdAt: string; // ISO 8601
}
```

---

## 4. Vault 模块

```rust
#[tauri::command]
pub async fn vault_unlock(account_id: String, password: String) -> Result<(), String>;

#[tauri::command]
pub async fn vault_lock() -> Result<(), String>;
// 擦除内存密钥 + 关闭数据库 + 广播 "vault-locked" 事件

#[tauri::command]
pub async fn vault_get_state() -> Result<VaultState, String>;

#[tauri::command]
pub async fn vault_change_password(old_password: String, new_password: String) -> Result<(), String>;

#[tauri::command]
pub async fn vault_delete_account(account_id: String, password: String) -> Result<(), String>;
// 物理删除所有数据 + 附件

#[tauri::command]
pub async fn vault_list_accounts() -> Result<Vec<AccountSummary>, String>;

#[tauri::command]
pub async fn vault_set_default_account(account_id: String) -> Result<(), String>;
```

```typescript
enum VaultState { Uninitialized = 'uninitialized', Locked = 'locked', Unlocked = 'unlocked' }

interface AccountSummary {
  id: string; name: string; isDefault: boolean;
  createdAt: string; lastAccessedAt: string;
}
```

---

## 5. 生物识别模块（新增）

```rust
/// 检查当前平台是否支持生物识别
#[tauri::command]
pub async fn biometric_check_availability() -> Result<BiometricAvailability, String>;

/// 保存生物识别凭证（会话密钥写入 OS Keychain）
/// 需验证主密码
#[tauri::command]
pub async fn biometric_save_credential(
    account_id: String,
    password: String,
) -> Result<(), String>;

/// 通过生物识别解锁 Vault
/// 弹出系统对话框，验证后从 Keychain 读取会话密钥
#[tauri::command]
pub async fn biometric_unlock(account_id: String) -> Result<(), String>;

/// 删除生物识别凭证（从 Keychain 删除会话密钥）
/// 需验证主密码
#[tauri::command]
pub async fn biometric_delete_credential(
    account_id: String,
    password: String,
) -> Result<(), String>;

/// 测试生物识别是否正常工作
/// 弹出系统对话框，不执行解锁
#[tauri::command]
pub async fn biometric_test(account_id: String) -> Result<bool, String>;
```

```typescript
enum BiometryType { touchId = 'touchId', faceId = 'faceId', iris = 'iris', windowsHello = 'windowsHello' }

interface BiometricAvailability {
  available: boolean;
  biometryType?: BiometryType;
  error?: string;
}
```

**安全约束**：

| 约束 | 要求 |
|------|------|
| 密码仅在 `biometric_save_credential` / `biometric_delete_credential` 时接收 | 绝不通过返回值或事件泄露 |
| 会话密钥不出 Rust | 前端永不接触原始会话密钥 |
| 凭证不可导出 | OS Keychain 存储时标记为 `ThisDeviceOnly` |
| 密码变更自动过期 | 详见文档 21 第 4.4 节 |

---

## 6. Profile 模块

```rust
#[tauri::command] pub async fn profile_get(account_id: String) -> Result<ProfileData, String>;
#[tauri::command] pub async fn profile_update(account_id: String, data: ProfileData) -> Result<(), String>;
#[tauri::command] pub async fn profile_get_section(account_id: String, section_type: String) -> Result<SectionData, String>;
#[tauri::command] pub async fn profile_update_section(account_id: String, section_type: String, data: SectionData) -> Result<(), String>;
#[tauri::command] pub async fn profile_update_field(account_id: String, section_type: String, field_key: String, field_value: FieldValue) -> Result<(), String>;
#[tauri::command] pub async fn profile_get_field_history(account_id: String, section_type: String, field_key: String) -> Result<Vec<FieldHistoryEntry>, String>;
```

---

## 7. UnifiedObject 模块

```rust
#[tauri::command] pub async fn unified_object_list(account_id: String, filter: Option<ObjectFilter>, sort: Option<SortConfig>) -> Result<Vec<ObjectSummary>, String>;
#[tauri::command] pub async fn unified_object_get(account_id: String, object_id: String) -> Result<UnifiedObject, String>;
#[tauri::command] pub async fn unified_object_create(account_id: String, data: UnifiedObjectInput) -> Result<UnifiedObject, String>;
#[tauri::command] pub async fn unified_object_update(account_id: String, object_id: String, data: UnifiedObjectInput) -> Result<UnifiedObject, String>;
#[tauri::command] pub async fn unified_object_delete(account_id: String, object_id: String) -> Result<(), String>;         // 软删除
#[tauri::command] pub async fn unified_object_permanently_delete(account_id: String, object_id: String) -> Result<(), String>; // 硬删除
#[tauri::command] pub async fn unified_object_get_section_data(account_id: String, object_id: String, section_type: String) -> Result<SectionData, String>;
#[tauri::command] pub async fn unified_object_update_field(account_id: String, object_id: String, section_type: String, field_key: String, field_value: FieldValue) -> Result<(), String>;
#[tauri::command] pub async fn unified_object_set_sensitivity(account_id: String, object_id: String, sensitivity_level: String) -> Result<(), String>;
```

---

## 8. 敏感度模块（新增）

```rust
#[tauri::command] pub async fn sensitivity_get_field(field_id: String) -> Result<String, String>;
#[tauri::command] pub async fn sensitivity_get_map() -> Result<SensitivityMap, String>;
#[tauri::command] pub async fn sensitivity_update_field(field_id: String, new_level: String, password: String, reason: Option<String>) -> Result<(), String>;
#[tauri::command] pub async fn sensitivity_get_log(limit: Option<usize>) -> Result<Vec<SensitivityLogEntry>, String>;
```

---

## 9. 事件定义（Rust → 前端推送）

```rust
pub const EVENT_VAULT_LOCKED: &str = "vault-locked";
pub const EVENT_VAULT_UNLOCKED: &str = "vault-unlocked";
pub const EVENT_THEME_CHANGED: &str = "theme-changed";
pub const EVENT_LOCALE_CHANGED: &str = "locale-changed";
pub const EVENT_DATA_CHANGED: &str = "data-changed";
pub const EVENT_SETTINGS_CHANGED: &str = "settings-changed";
pub const EVENT_BIOMETRIC_ENABLED: &str = "biometric-enabled";
pub const EVENT_BIOMETRIC_DISABLED: &str = "biometric-disabled";
pub const EVENT_BIOMETRIC_CREDENTIAL_EXPIRED: &str = "biometric-credential-expired";
```

```typescript
// 前端监听
import { listen } from '@tauri-apps/api/event';
useEffect(() => {
  const unlisten = listen('vault-locked', () => navigate('/login'));
  return () => { unlisten.then(f => f()); };
}, []);
```

---

## 10. tauri-specta 类型生成

```rust
// 构建脚本自动生成 TypeScript 类型
#[cfg(feature = "specta")]
fn generate_bindings() {
    use tauri_specta::{collect_commands, ts};
    ts::export(collect_commands![/* 所有命令 */], "../src/lib/ipc.ts").unwrap();
}
```

生成的 `src/lib/ipc.ts`（自动生成，不可手动修改）:
```typescript
export async function authLogin(accountId: string, password: string): Promise<AccountInfo> {
  return await invoke('auth_login', { accountId, password });
}
```

---

## 11. 安全约束

| 约束 | 要求 |
|------|------|
| 密码不通过 IPC 返回 | `auth_bootstrap` 和 `auth_login` 接收密码，但绝不返回 |
| 密钥不出 Rust | 前端永不接触 `[u8; 32]` 密钥 |
| 敏感度查询不验证 | `sensitivity_get_field` 无需密码（用于渲染判断） |
| 敏感度修改需密码 | `sensitivity_update_field` 必须传入密码 |
| 生物识别密码验证 | `biometric_save_credential` / `biometric_delete_credential` 需主密码验证 |
| 错误不泄露内部状态 | 不返回 "table x not found"，返回 "数据加载失败" |

---

## 12. 完成标准

### P0（必须）
- [ ] 全部 66 个命令编译通过（空实现即可）
- [ ] tauri-specta 生成 TypeScript 类型文件
- [ ] IPC 调用可从前端 `invoke` 到 Rust 并返回
- [ ] 密码/密钥不通过任何 IPC 返回值泄露

### P1（重要 — 事件与生物识别）
- [ ] `vault-locked` 事件可被前端监听并触发导航
- [ ] 新增 5 个 `biometric_*` 命令编译通过并可用
- [ ] `biometric_check_availability` 返回正确的平台生物识别能力
- [ ] `biometric_unlock` 通过系统对话框验证后从 Keychain 读取会话密钥
- [ ] `biometric-enabled` / `biometric-disabled` / `biometric-credential-expired` 事件可被前端监听

---

*文档版本：v2.1 (priority-refactored)*
*创建日期：2026-06-05*
*最后更新：2026-06-07*
*对应开发阶段：Phase 1-2（IPC 层）*
