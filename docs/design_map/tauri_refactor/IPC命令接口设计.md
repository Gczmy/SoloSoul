# IPC 命令接口设计

> **文档定位**: Tauri 前后端 IPC 通信的完整接口规范，包含所有命令签名、参数类型、返回值、错误码。
>
> **阅读对象**: 前后端开发者、API 设计者。
>
> **前置知识**: 需先阅读 `ADR-004-IPC通信与状态管理方案.md`。

---

## 目录

- [接口设计原则](#接口设计原则)
- [认证模块](#认证模块)
- [Vault 模块](#vault-模块)
- [Profile 模块](#profile-模块)
- [UnifiedObject 模块](#unifiedobject-模块)
- [搜索模块](#搜索模块)
- [设置模块](#设置模块)
- [导入导出模块](#导入导出模块)
- [备份模块](#备份模块)
- [插件模块](#插件模块)
- [OCR 模块](#ocr-模块)
- [LLM 模块](#llm-模块)
- [同步模块](#同步模块)
- [日志模块](#日志模块)
- [系统模块](#系统模块)
- [事件定义](#事件定义)
- [TypeScript 类型生成](#typescript-类型生成)

---

## 接口设计原则

1. **命令名 snake_case**: `profile_get`, `vault_unlock`
2. **参数名 camelCase**: 前端传入时自动转换
3. **返回值统一 Result**: `Ok(T)` / `Err(String)`
4. **敏感数据不出后端**: 密钥、密码绝不返回
5. **批量操作用数组**: 减少 IPC 往返
6. **进度通知用 Channel**: 不阻塞 UI

---

## 认证模块

```rust
/// 引导流程：检查是否已有账户
/// 返回：true = 已有账户（跳转到登录），false = 首次使用（跳转到引导）
#[tauri::command]
pub async fn auth_check_has_account() -> Result<bool, String>;

/// 创建首个账户（Bootstrap）
#[tauri::command]
pub async fn auth_bootstrap(
    account_name: String,
    password: String,
) -> Result<AccountInfo, String>;

/// 登录（解锁 Vault）
#[tauri::command]
pub async fn auth_login(
    account_id: String,
    password: String,
) -> Result<AccountInfo, String>;

/// 登出（锁定 Vault）
#[tauri::command]
pub async fn auth_logout() -> Result<(), String>;

/// 获取当前账户信息
#[tauri::command]
pub async fn auth_get_current_account() -> Result<Option<AccountInfo>, String>;
```

### 类型定义

```typescript
interface AccountInfo {
  id: string;           // acc_xxx
  name: string;
  createdAt: string;    // ISO 8601
}
```

---

## Vault 模块

```rust
/// 解锁 Vault
#[tauri::command]
pub async fn vault_unlock(
    account_id: String,
    password: String,
) -> Result<(), String>;

/// 锁定 Vault（擦除内存密钥）
#[tauri::command]
pub async fn vault_lock() -> Result<(), String>;

/// 获取 Vault 状态
#[tauri::command]
pub async fn vault_get_state() -> Result<VaultState, String>;

/// 修改密码
#[tauri::command]
pub async fn vault_change_password(
    old_password: String,
    new_password: String,
) -> Result<(), String>;

/// 删除账户
#[tauri::command]
pub async fn vault_delete_account(
    account_id: String,
    password: String,
) -> Result<(), String>;

/// 列出所有账户
#[tauri::command]
pub async fn vault_list_accounts() -> Result<Vec<AccountSummary>, String>;

/// 设置默认账户
#[tauri::command]
pub async fn vault_set_default_account(account_id: String) -> Result<(), String>;
```

### 类型定义

```typescript
enum VaultState {
  Uninitialized = 'uninitialized',
  Locked = 'locked',
  Unlocked = 'unlocked',
}

interface AccountSummary {
  id: string;
  name: string;
  isDefault: boolean;
  createdAt: string;
  lastAccessedAt: string;
}
```

---

## Profile 模块

```rust
/// 获取完整 Profile
#[tauri::command]
pub async fn profile_get(account_id: String) -> Result<ProfileData, String>;

/// 更新完整 Profile
#[tauri::command]
pub async fn profile_update(
    account_id: String,
    data: ProfileData,
) -> Result<(), String>;

/// 获取指定分区数据
#[tauri::command]
pub async fn profile_get_section(
    account_id: String,
    section_type: String,
) -> Result<SectionData, String>;

/// 更新指定分区
#[tauri::command]
pub async fn profile_update_section(
    account_id: String,
    section_type: String,
    data: SectionData,
) -> Result<(), String>;

/// 更新单个字段
#[tauri::command]
pub async fn profile_update_field(
    account_id: String,
    section_type: String,
    field_key: String,
    field_value: FieldValue,
) -> Result<(), String>;

/// 获取字段历史
#[tauri::command]
pub async fn profile_get_field_history(
    account_id: String,
    section_type: String,
    field_key: String,
) -> Result<Vec<FieldHistoryEntry>, String>;
```

---

## UnifiedObject 模块

```rust
/// 列出对象（支持过滤和排序）
#[tauri::command]
pub async fn unified_object_list(
    account_id: String,
    filter: Option<ObjectFilter>,
    sort: Option<SortConfig>,
) -> Result<Vec<ObjectSummary>, String>;

/// 获取单个对象详情
#[tauri::command]
pub async fn unified_object_get(
    account_id: String,
    object_id: String,
) -> Result<UnifiedObject, String>;

/// 创建对象
#[tauri::command]
pub async fn unified_object_create(
    account_id: String,
    data: UnifiedObjectInput,
) -> Result<UnifiedObject, String>;

/// 更新对象
#[tauri::command]
pub async fn unified_object_update(
    account_id: String,
    object_id: String,
    data: UnifiedObjectInput,
) -> Result<UnifiedObject, String>;

/// 删除对象（软删除）
#[tauri::command]
pub async fn unified_object_delete(
    account_id: String,
    object_id: String,
) -> Result<(), String>;

/// 永久删除对象
#[tauri::command]
pub async fn unified_object_permanently_delete(
    account_id: String,
    object_id: String,
) -> Result<(), String>;

/// 获取对象的分区数据
#[tauri::command]
pub async fn unified_object_get_section_data(
    account_id: String,
    object_id: String,
    section_type: String,
) -> Result<SectionData, String>;

/// 更新对象的字段
#[tauri::command]
pub async fn unified_object_update_field(
    account_id: String,
    object_id: String,
    section_type: String,
    field_key: String,
    field_value: FieldValue,
) -> Result<(), String>;

/// 设置敏感度级别
#[tauri::command]
pub async fn unified_object_set_sensitivity(
    account_id: String,
    object_id: String,
    sensitivity_level: String,
) -> Result<(), String>;
```

### 类型定义

```typescript
interface ObjectFilter {
  sectionType?: string;
  sensitivityLevel?: string;
  keyword?: string;
}

interface SortConfig {
  field: string;
  direction: 'asc' | 'desc';
}

interface ObjectSummary {
  id: string;
  name: string;
  sectionType: string;
  sensitivityLevel: SensitivityLevel;
  updatedAt: string;
}

interface UnifiedObject {
  id: string;
  name: string;
  sectionType: string;
  sectionData: Record<string, unknown>;
  sensitivityLevel: SensitivityLevel;
  createdAt: string;
  updatedAt: string;
}

interface UnifiedObjectInput {
  name: string;
  sectionType: string;
  sectionData: Record<string, unknown>;
  sensitivityLevel?: SensitivityLevel;
}
```

---

## 搜索模块

```rust
/// 统一搜索
#[tauri::command]
pub async fn search_unified(
    account_id: String,
    query: String,
    options: SearchOptions,
) -> Result<SearchResult, String>;

/// 高级搜索
#[tauri::command]
pub async fn search_advanced(
    account_id: String,
    criteria: Vec<SearchCriterion>,
) -> Result<SearchResult, String>;
```

```typescript
interface SearchOptions {
  sectionTypes?: string[];
  sensitivityLevels?: string[];
  limit?: number;
  offset?: number;
}

interface SearchCriterion {
  field: string;
  operator: 'eq' | 'contains' | 'gt' | 'lt' | 'between';
  value: unknown;
}

interface SearchResult {
  items: SearchResultItem[];
  total: number;
  hasMore: boolean;
}

interface SearchResultItem {
  objectId: string;
  name: string;
  sectionType: string;
  matchedField?: string;
  matchedValue?: string;
  relevance: number;
}
```

---

## 设置模块

```rust
/// 获取所有设置
#[tauri::command]
pub async fn settings_get_all() -> Result<AppSettings, String>;

/// 获取单个设置
#[tauri::command]
pub async fn settings_get(key: String) -> Result<Option<SettingValue>, String>;

/// 更新设置
#[tauri::command]
pub async fn settings_update(
    key: String,
    value: SettingValue,
) -> Result<(), String>;

/// 重置为默认值
#[tauri::command]
pub async fn settings_reset_to_default() -> Result<(), String>;
```

```typescript
interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  locale: string;
  autoLockTimeoutMinutes: number;
  biometricEnabled: boolean;
  hapticFeedback: boolean;
  confirmDelete: boolean;
  developerMode: boolean;
  ocrEnginePath?: string;
  llmConfig?: LlmConfig;
  syncConfig?: SyncConfig;
  exportConfig?: ExportConfig;
  backupConfig?: BackupConfig;
}
```

---

## 导入导出模块

```rust
/// 导出数据
#[tauri::command]
pub async fn export_import_export(
    format: String,           // "json" | "csv" | "vcard"
    scope: ExportScope,
    password: Option<String>, // 加密导出
) -> Result<String, String>;  // 返回文件路径

/// 导入数据
#[tauri::command]
pub async fn export_import_import(
    file_path: String,
    format: String,
    merge_strategy: String,   // "merge" | "replace" | "skip"
) -> Result<ImportResult, String>;

/// 获取支持的格式
#[tauri::command]
pub async fn export_import_get_formats() -> Result<Vec<ExportFormatInfo>, String>;
```

---

## 备份模块

```rust
/// 列出备份
#[tauri::command]
pub async fn backup_list(account_id: String) -> Result<Vec<BackupInfo>, String>;

/// 创建备份
#[tauri::command]
pub async fn backup_create(
    account_id: String,
    note: Option<String>,
) -> Result<BackupInfo, String>;

/// 恢复备份
#[tauri::command]
pub async fn backup_restore(
    account_id: String,
    backup_id: String,
) -> Result<(), String>;

/// 删除备份
#[tauri::command]
pub async fn backup_delete(
    account_id: String,
    backup_id: String,
) -> Result<(), String>;
```

---

## 插件模块

```rust
/// 列出插件
#[tauri::command]
pub async fn plugin_list() -> Result<Vec<PluginInfo>, String>;

/// 安装插件
#[tauri::command]
pub async fn plugin_install(file_path: String) -> Result<PluginInfo, String>;

/// 卸载插件
#[tauri::command]
pub async fn plugin_uninstall(plugin_id: String) -> Result<(), String>;

/// 运行插件
#[tauri::command]
pub async fn plugin_run(
    plugin_id: String,
    params: Option<serde_json::Value>,
) -> Result<PluginRunResult, String>;

/// 授权字段访问
#[tauri::command]
pub async fn plugin_approve_consent(
    session_id: String,
    approved_fields: Vec<String>,
) -> Result<(), String>;
```

---

## OCR 模块

```rust
/// 初始化 OCR 引擎
#[tauri::command]
pub async fn ocr_initialize() -> Result<OcrEngineStatus, String>;

/// 扫描图片（流式进度）
#[tauri::command]
pub async fn ocr_scan(
    image_path: String,
    channel: tauri::ipc::Channel<OcrProgress>,
) -> Result<OcrResult, String>;

/// 获取 OCR 引擎状态
#[tauri::command]
pub async fn ocr_get_status() -> Result<OcrEngineStatus, String>;
```

```typescript
interface OcrProgress {
  stage: 'loading' | 'detecting' | 'recognizing' | 'parsing';
  progress: number;  // 0-100
  message: string;
}

interface OcrResult {
  text: string;
  mrzResult?: MrzResult;
  confidence: number;
}
```

---

## LLM 模块

```rust
/// 获取 LLM 配置
#[tauri::command]
pub async fn llm_get_config() -> Result<LlmConfig, String>;

/// 更新 LLM 配置
#[tauri::command]
pub async fn llm_update_config(config: LlmConfig) -> Result<(), String>;

/// 发送消息（流式响应）
#[tauri::command]
pub async fn llm_send_message(
    messages: Vec<LlmMessage>,
    channel: tauri::ipc::Channel<LlmChunk>,
) -> Result<(), String>;

/// 获取用量统计
#[tauri::command]
pub async fn llm_get_usage() -> Result<LlmUsageStats, String>;
```

---

## 同步模块

```rust
/// 获取本机同步信息
#[tauri::command]
pub async fn sync_get_local_info() -> Result<LocalSyncInfo, String>;

/// 发现设备
#[tauri::command]
pub async fn sync_discover(
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String>;

/// 与设备同步（流式进度）
#[tauri::command]
pub async fn sync_with_device(
    device_address: String,
    pairing_key: String,
    channel: tauri::ipc::Channel<SyncProgress>,
) -> Result<SyncResult, String>;

/// 获取同步历史
#[tauri::command]
pub async fn sync_get_logs() -> Result<Vec<SyncLogEntry>, String>;
```

---

## 日志模块

```rust
/// 获取最近日志
#[tauri::command]
pub async fn log_get_recent(
    limit: Option<usize>,
    level: Option<String>,
) -> Result<Vec<LogEntry>, String>;

/// 导出日志
#[tauri::command]
pub async fn log_export(output_path: String) -> Result<(), String>;
```

---

## 系统模块

```rust
/// 获取应用信息
#[tauri::command]
pub async fn system_get_app_info() -> Result<AppInfo, String>;

/// 检查最新版本
#[tauri::command]
pub async fn system_check_version() -> Result<VersionCheckResult, String>;

/// 打开外部链接
#[tauri::command]
pub async fn system_open_url(url: String) -> Result<(), String>;
```

```typescript
interface AppInfo {
  appName: string;
  version: string;
  buildNumber: string;
  rustCoreVersion: string;
  schemaVersion: number;
  os: string;
  arch: string;
  kdfMemoryKb: number;
  kdfIterations: number;
  aesMode: string;
  vaultStatus: VaultState;
  biometricEnabled: boolean;
  autoLockMinutes: number;
}

interface VersionCheckResult {
  currentVersion: string;
  latestVersion: string | null;
  hasUpdate: boolean;
  releaseNotes?: string;
  downloadUrl?: string;
}
```

---

## 事件定义

```rust
// src-tauri/src/ipc/events.rs

/// Vault 锁定事件（广播）
pub const EVENT_VAULT_LOCKED: &str = "vault-locked";

/// Vault 解锁事件（广播）
pub const EVENT_VAULT_UNLOCKED: &str = "vault-unlocked";

/// 主题变更事件
pub const EVENT_THEME_CHANGED: &str = "theme-changed";

/// 语言变更事件
pub const EVENT_LOCALE_CHANGED: &str = "locale-changed";

/// 全局错误事件
pub const EVENT_ERROR: &str = "app-error";

/// 数据变更事件（某个对象被修改）
pub const EVENT_DATA_CHANGED: &str = "data-changed";

/// 设置变更事件
pub const EVENT_SETTINGS_CHANGED: &str = "settings-changed";

/// 插件会话事件
pub const EVENT_PLUGIN_SESSION: &str = "plugin-session";
```

```typescript
// 前端监听示例
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
  const unlisten = listen('vault-locked', () => {
    // 处理 Vault 锁定
    navigate('/login');
  });
  return () => { unlisten.then(f => f()); };
}, []);
```

---

## TypeScript 类型生成

### tauri-specta 配置

```rust
// src-tauri/src/lib.rs
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![/* commands */])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 构建脚本 build.rs 或单独 binary
#[cfg(feature = "specta")]
fn generate_bindings() {
    use tauri_specta::{collect_commands, ts};
    
    ts::export(
        collect_commands![
            auth_check_has_account,
            auth_bootstrap,
            auth_login,
            auth_logout,
            vault_unlock,
            vault_lock,
            // ... 所有命令
        ],
        "../src/lib/ipc.ts",
    ).unwrap();
}
```

### 生成的 TypeScript 文件

```typescript
// src/lib/ipc.ts（自动生成，不要手动修改）

import { invoke } from '@tauri-apps/api/core';

// ----- 类型定义 -----
export interface AccountInfo { id: string; name: string; createdAt: string; }
export interface AccountSummary { id: string; name: string; isDefault: boolean; createdAt: string; lastAccessedAt: string; }
export type VaultState = 'uninitialized' | 'locked' | 'unlocked';
// ... 更多类型

// ----- 命令函数 -----
export async function authCheckHasAccount(): Promise<boolean> {
  return await invoke('auth_check_has_account');
}

export async function authBootstrap(accountName: string, password: string): Promise<AccountInfo> {
  return await invoke('auth_bootstrap', { accountName, password });
}

export async function authLogin(accountId: string, password: string): Promise<AccountInfo> {
  return await invoke('auth_login', { accountId, password });
}

export async function authLogout(): Promise<void> {
  return await invoke('auth_logout');
}

export async function vaultUnlock(accountId: string, password: string): Promise<void> {
  return await invoke('vault_unlock', { accountId, password });
}

export async function vaultLock(): Promise<void> {
  return await invoke('vault_lock');
}

// ... 所有命令
```

---

## 命令总数统计

| 模块 | 命令数 | 说明 |
|------|--------|------|
| 认证 (auth) | 4 | 登录/登出/引导 |
| Vault (vault) | 6 | 解锁/锁定/密码管理 |
| Profile (profile) | 5 | Profile CRUD |
| UnifiedObject (unified_object) | 8 | 对象管理 |
| 搜索 (search) | 2 | 统一/高级搜索 |
| 设置 (settings) | 4 | 设置读写 |
| 导入导出 (export_import) | 3 | 数据迁移 |
| 备份 (backup) | 4 | 备份管理 |
| 插件 (plugin) | 5 | 插件生命周期 |
| OCR (ocr) | 3 | OCR 引擎 |
| LLM (llm) | 4 | AI 对话 |
| 同步 (sync) | 4 | 设备同步 |
| 日志 (log) | 2 | 日志查看 |
| 系统 (system) | 3 | 系统信息 |
| **总计** | **61** | — |

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*对应重构阶段：Phase 4（功能迁移）*
