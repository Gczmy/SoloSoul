# 07 — IPC 命令接口完整规范

> **前置阅读**：`03_项目结构与Rust_Workspace.md`、`06_数据库与服务层.md`
> **Manifesto 对齐**：最少惊喜 | 安全默认
> **当前状态**：已全部实现。命令以 `tauri/src-tauri/permissions/solo-soul/default.toml`（ACL 白名单）与 `tauri/src-tauri/src/lib.rs`（handler 注册）为**权威来源**；本规范为设计期文档，个别命令的增删（如 P002 移除 `get_state`/`delete_account`/`object_restore`/`object_purge`）以 ACL/handler 为准。

---

## 1. 接口设计原则与实现变更

| 原则 | 说明 |
|------|------|
| 命令名 snake_case | `object_list`、`attachment_save` |
| 前缀简化 | 核心域不强制加模块前缀（`auth_login`→`login`，`vault_unlock`→`unlock`） |
| UnifiedObject 重命名 | 所有 `unified_object_*` 已实装为 `object_*` |
| 敏感度模块废弃 | Sensitivity 在实现中降级为 Object 的字段属性，不设独立 IPC 模块 |
| 参数/返回值 camelCase | 类型在 `src/lib/ipc.ts` 手工维护，Rust snake_case → TS camelCase 手工转换 |
| 返回值统一 `Result<T, String>` | 错误消息中文，用户可理解 |
| 敏感数据不出后端 | 密钥、密码绝不通过 IPC 返回 |
| 进度通知用 Event | LLM 流式、OCR 下载、导入导出等长操作通过 Tauri Event 推送 |

---

## 2. 命令总览（约 155 个）

### 核心数据（Core）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 认证 (Auth) | 5 | `check_has_account`、`bootstrap`、`login`、`logout`、`get_current_account` |
| 保险箱 (Vault) | 8 | `unlock`、`lock`、`change_password`、`vault_list_accounts`、`vault_update_hint`、`get_vault_stats`、`verify_password`、`unlock_with_password`（`get_state`、`delete_account` 已随 P002 移除） |
| 生物识别 (Biometric) | 5 | `biometric_check_availability`、`biometric_save_credential`、`biometric_unlock`、`biometric_delete_credential`、`biometric_test` |
| 档案 (Profile) | 6 | `profile_load`、`profile_save`、`profile_get_section`、`profile_update_field`、`profile_delete`、`profile_list` |
| 对象 (Object) | 6 | `object_list`、`object_get`、`object_create`、`object_update`、`object_delete`、`object_trash_list`（`object_restore`、`object_purge` 已随 P002 移除，回收站恢复/物理删除统一走 `trash_restore`/`trash_permanent_delete`） |
| 模板 (Template) | 7 | `template_create`、`template_update`、`template_delete`、`template_restore`、`template_get`、`template_list`、`template_check_field_usage`（`template_save_from_object` 死命令已删，从对象保存模板统一走 `template_create`） |

### 扩展功能（Extensions）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 回收站 (Trash) | 5 | `trash_restore`、`trash_permanent_delete`、`trash_get_detail`、`trash_get_retention`、`trash_set_retention` |
| 页面 (Page) | 1 | `page_delete` |
| 快照 (Snapshot) | 5 | `snapshot_list`、`snapshot_get`、`snapshot_get_data`、`snapshot_rollback`、`snapshot_count_batch` |
| 附件 (Attachment) | 14 | `attachment_list`、`attachment_save`、`attachment_download`、`attachment_rename`、`attachment_cleanup_orphans`、`attachment_copy_to_vault` |
| 搜索 (Search) | 2 | `search_unified`、`search_advanced` |
| 设置 (Settings) | 4 | `ui_get_preferences`、`ui_update_preference`、`user_data_get_preferences`、`user_data_update_preference` |

### 导入导出与备份（Portability）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 导入导出 (Export/Import) | 9 | `export_get_scope_tree`、`export_estimate_size`、`export_execute`、`export_get_attachments`、`import_parse_package`、`import_get_password_hint`、`import_decrypt_preview`、`import_execute`、`import_execute_advanced` |
| 备份 (Backup) | 5 | `backup_list`、`backup_create`、`backup_restore`、`backup_delete`、`inspect_backup` |

### 插件与同步（Plugin & Sync）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 插件 (Plugin) | 11 | `plugin_list_all`、`plugin_list_installed`、`plugin_install`、`plugin_update`、`plugin_uninstall`、`plugin_run`、`plugin_consent_response`、`plugin_dialog_response`、`plugin_list_sessions`、`plugin_audit_log`、`plugin_update_registry` |
| 同步 (Sync) | 8 | `mdns_advertise`、`mdns_discover`、`sync_enable`、`sync_forget_peer`、`sync_get_status`、`sync_trust_peer`、`sync_discover`、`sync_with_device` |

### AI 与自动化（AI & Automation）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| OCR | 10 | `ocr_scan_image`、`ocr_scan_mrz`、`ocr_get_supported_languages`、`ocr_list_available_tiers`、`ocr_get_active_tier`、`ocr_set_active_tier`、`ocr_get_model_status`、`ocr_install_bundled_model`、`ocr_install_bundled_model_with_progress`、`ocr_download_model` |
| 大语言模型 (LLM) | 30+ | `llm_chat`、`llm_send_message`、`llm_get_providers`、`llm_save_provider`、`llm_test_provider`、`llm_set_active_provider`、`llm_list_conversations`、`llm_get_conversation`、`llm_soft_delete_conversation`、`llm_search_guide_chunks`、`llm_rebuild_guide_embeddings`、`llm_get_stats`、`llm_reset_stats`、`llm_set_local_embedding`、`llm_check_embedding_available`、`llm_download_embed_model`、`llm_find_guides`、`llm_check_connection` |
| 指南 (Guide) | 4 | `guide_load_content`、`guide_load_index`、`guide_load_search_index`、`guide_search` |

### 基础设施（Infrastructure）

| 模块 | 命令数 | 典型命令 |
|------|--------|---------|
| 密码学 (Crypto) | 10 | `encrypt_bytes`、`decrypt_bytes`、`encrypt_with_key`、`decrypt_with_key`、`derive_key`、`generate_salt`、`constant_time_compare`、`encrypt_file`、`decrypt_file`、`verify_password` |
| 文件系统 (FS) | 4 | `fs_get_file_size`、`fs_is_dir`、`fs_read_file_as_data_url`、`fs_scan_directory` |
| 系统 (System) | 2 | `get_app_info`、`get_current_account` |
| 日志 (Log) | 3 | `log_get_recent`、`log_export`、`log_write` |

---

## 3. Auth 认证模块

```rust
#[tauri::command]
pub async fn check_has_account() -> Result<bool, String>;
// true → 跳转登录页；false → 跳转引导页

#[tauri::command]
pub async fn bootstrap(account_name: String, password: String) -> Result<AccountInfo, String>;

#[tauri::command]
pub async fn login(account_id: String, password: String) -> Result<AccountInfo, String>;

#[tauri::command]
pub async fn logout() -> Result<(), String>;

#[tauri::command]
pub async fn get_current_account() -> Result<Option<AccountInfo>, String>;
```

> **命名变更**：设计稿中 `auth_login`/`auth_logout`/`auth_bootstrap` → 实现中去掉 `auth_` 前缀。

```typescript
interface AccountInfo {
  id: string;        // acc_xxx
  name: string;
  createdAt: string; // ISO 8601
}
```

---

## 4. Vault 保险箱模块

```rust
#[tauri::command]
pub async fn unlock(account_id: String, password: String) -> Result<(), String>;
// 派生密钥 → 解密 Vault → 广播 "vault-unlocked"

#[tauri::command]
pub async fn unlock_with_password(password: String) -> Result<(), String>;
// 无 account_id 参数版本，内部从 config 查找默认账户

#[tauri::command]
pub async fn lock() -> Result<(), String>;
// 擦除内存密钥 + 关闭数据库 + 广播 "vault-locked"

// 注：`get_state` 命令已随 P002 移除（前端零调用）；Vault 状态经 `vault-locked` 事件与
// 前端 authStore 维护，Rust 侧状态判定保留在服务方法 `VaultService::get_vault_state()`。

#[tauri::command]
pub async fn change_password(old_password: String, new_password: String) -> Result<(), String>;

// 注：`delete_account` 命令已随 P002 移除（前端零调用）；账户删除能力保留在
// `VaultService::delete_account()` 服务方法（CLI `/security delete-account` 与 recovery 流程使用）。

#[tauri::command]
pub async fn vault_list_accounts() -> Result<Vec<AccountSummary>, String>;

#[tauri::command]
pub async fn vault_update_hint(account_id: String, hint: String) -> Result<(), String>;

#[tauri::command]
pub async fn get_vault_stats() -> Result<serde_json::Value, String>;
// 返回 Vault 总大小、对象/附件/快照计数

#[tauri::command]
pub async fn verify_password(password: String) -> Result<bool, String>;
```

```typescript
interface AccountSummary {
  id: string; name: string; isDefault: boolean;
  createdAt: string; lastAccessedAt: string;
}
```

---

## 5. Biometric 生物识别模块

```rust
#[tauri::command]
pub async fn biometric_check_availability() -> Result<BiometricAvailability, String>;

#[tauri::command]
pub async fn biometric_save_credential(account_id: String, password: String) -> Result<(), String>;
// 验证主密码 → 会话密钥写入 OS Keychain

#[tauri::command]
pub async fn biometric_unlock(account_id: String) -> Result<(), String>;
// 弹出系统对话框 → 从 Keychain 读取会话密钥 → 解锁 Vault

#[tauri::command]
pub async fn biometric_delete_credential(account_id: String, password: String) -> Result<(), String>;

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
| 密码仅在 `biometric_save_credential`/`biometric_delete_credential` 参数接收 | 绝不通过返回值或事件泄露 |
| 会话密钥不出 Rust | 前端永不接触原始会话密钥 |
| 凭证不可导出 | OS Keychain 标记 `ThisDeviceOnly` |
| 密码变更自动过期 | 修改主密码后生物识别凭证自动失效 |

---

## 6. Profile 档案模块

```rust
#[tauri::command]
pub async fn profile_load(account_id: String) -> Result<ProfileData, String>;

#[tauri::command]
pub async fn profile_save(account_id: String, data: ProfileData) -> Result<(), String>;

#[tauri::command]
pub async fn profile_get_section(account_id: String, section_type: String) -> Result<SectionData, String>;

#[tauri::command]
pub async fn profile_update_field(account_id: String, section_type: String, field_key: String, field_value: FieldValue) -> Result<(), String>;
// 前端配合 profileStore 乐观更新 + 500ms debounce

#[tauri::command]
pub async fn profile_delete(account_id: String) -> Result<(), String>;

#[tauri::command]
pub async fn profile_list() -> Result<Vec<ProfileSummary>, String>;
```

> **命名变更**：设计稿中 `profile_get` → 实现为 `profile_load`，`profile_update` → 实现为 `profile_save`。

---

## 7. Object 对象模块

> ⚠️ 已从 `unified_object_*` 全面更名为 `object_*`。

```rust
#[tauri::command]
pub async fn object_list(account_id: String, filter: Option<ObjectFilter>) -> Result<Vec<ObjectSummary>, String>;

#[tauri::command]
pub async fn object_get(account_id: String, object_id: String) -> Result<ObjectData, String>;

#[tauri::command]
pub async fn object_create(input: ObjectInput) -> Result<ObjectData, String>;

#[tauri::command]
pub async fn object_update(object_id: String, input: ObjectInput) -> Result<ObjectData, String>;

#[tauri::command]
pub async fn object_delete(object_id: String) -> Result<(), String>;
// 软删除 → 进入回收站

#[tauri::command]
pub async fn object_trash_list(account_id: String, since: Option<i64>) -> Result<Vec<TrashItemSummary>, String>;
// 回收站列表（支持时间范围过滤）

// 注：`object_restore` / `object_purge` 命令已随 P002 移除（前端零调用），
// 回收站恢复/物理删除统一走 `trash_restore` / `trash_permanent_delete`。
```

---

## 8. Template 模板模块

```rust
#[tauri::command]
pub async fn template_create(name: String, icon_id: Option<String>, category: Option<String>, properties: Vec<TemplateProperty>, contract_type_id: Option<String>) -> Result<String, String>;

#[tauri::command]
pub async fn template_update(template_id: String, name: Option<String>, icon_id: Option<String>, category: Option<String>, properties: Option<Vec<TemplateProperty>>) -> Result<(), String>;

#[tauri::command]
pub async fn template_delete(template_id: String) -> Result<(), String>;

#[tauri::command]
pub async fn template_restore(trash_id: String) -> Result<(), String>;

#[tauri::command]
pub async fn template_get(template_id: String) -> Result<UserTemplate, String>;

#[tauri::command]
pub async fn template_list() -> Result<Vec<UserTemplate>, String>;

#[tauri::command]
pub async fn template_check_field_usage(template_id: String, field_key: String) -> Result<FieldUsage, String>;
// 统计字段使用情况（活跃数/软删除数），用于删除字段前的安全确认
```

---

## 9. Attachment 附件模块

```rust
#[tauri::command]
pub async fn attachment_list(object_id: String, include_trash: bool) -> Result<Vec<AttachmentMeta>, String>;

#[tauri::command]
pub async fn attachment_save(object_id: String, file_path: String) -> Result<AttachmentMeta, String>;
// 复制文件到 attachments/ 目录，元数据写入对象 properties.__attachments

#[tauri::command]
pub async fn attachment_rename(attachment_id: String, new_name: String) -> Result<AttachmentMeta, String>;

#[tauri::command]
pub async fn attachment_delete(attachment_id: String) -> Result<(), String>;
// 物理删除附件文件和元数据

#[tauri::command]
pub async fn attachment_soft_delete(attachment_id: String) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_restore(attachment_id: String) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_batch_soft_delete(ids: Vec<String>) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_batch_restore(ids: Vec<String>) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_batch_delete(ids: Vec<String>) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_count_batch(object_ids: Vec<String>) -> Result<Vec<AttachmentCount>, String>;

#[tauri::command]
pub async fn attachment_copy_to_vault(object_id: String, file_path: String) -> Result<String, String>;
// 从外部路径复制文件到 Vault 附件目录

#[tauri::command]
pub async fn attachment_list_all() -> Result<Vec<AttachmentMeta>, String>;
// 全局列出所有对象的附件

#[tauri::command]
pub async fn attachment_download(attachment_id: String, target_path: String) -> Result<(), String>;

#[tauri::command]
pub async fn attachment_cleanup_orphans() -> Result<usize, String>;
// 清理未被任何对象引用的孤立附件文件，返回清理数量
```

---

## 10. Snapshot 快照模块

```rust
#[tauri::command]
pub async fn snapshot_list(object_id: String) -> Result<Vec<SnapshotMeta>, String>;

#[tauri::command]
pub async fn snapshot_get(snapshot_id: String) -> Result<SnapshotDetail, String>;

#[tauri::command]
pub async fn snapshot_get_data(snapshot_id: String) -> Result<serde_json::Value, String>;
// 获取快照的完整数据内容

#[tauri::command]
pub async fn snapshot_rollback(snapshot_id: String) -> Result<ObjectData, String>;
// 回滚对象到指定快照状态

#[tauri::command]
pub async fn snapshot_count_batch(object_ids: Vec<String>) -> Result<Vec<SnapshotCount>, String>;
// 批量查询多个对象的快照数量
```

---

## 11. LLM 大语言模型模块（部分核心命令）

```rust
// 对话管理
#[tauri::command] pub async fn llm_chat(conversation_id: String, message: String, provider_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_send_message(conversation_id: String, message: String, provider_id: String, model: String, temperature: Option<f32>) -> Result<(), String>;
// 流式响应通过 Tauri Event "llm-stream-chunk" 推送到前端

// 对话生命周期
#[tauri::command] pub async fn llm_list_conversations(account_id: String) -> Result<Vec<ConversationSummary>, String>;
#[tauri::command] pub async fn llm_get_conversation(conversation_id: String) -> Result<Conversation, String>;
#[tauri::command] pub async fn llm_save_conversation(conversation_id: String, messages: Vec<Message>) -> Result<(), String>;
#[tauri::command] pub async fn llm_delete_conversation(conversation_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_soft_delete_conversation(conversation_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_restore_conversation(conversation_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_rename_conversation(conversation_id: String, new_name: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_list_trash(account_id: String) -> Result<Vec<ConversationSummary>, String>;
#[tauri::command] pub async fn llm_permanent_delete(conversation_id: String) -> Result<(), String>;

// 提供商管理
#[tauri::command] pub async fn llm_get_providers(account_id: String) -> Result<Vec<LlmProvider>, String>;
#[tauri::command] pub async fn llm_save_provider(account_id: String, provider: LlmProvider) -> Result<(), String>;
#[tauri::command] pub async fn llm_delete_provider(account_id: String, provider_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_test_provider(account_id: String, provider_id: String) -> Result<TestResult, String>;
#[tauri::command] pub async fn llm_set_active_provider(account_id: String, provider_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_get_api_key(account_id: String, provider_id: String) -> Result<String, String>;
#[tauri::command] pub async fn llm_accept_risk(account_id: String) -> Result<(), String>;

// 本地 Embedding 与 RAG
#[tauri::command] pub async fn llm_set_local_embedding(enabled: bool, model_name: Option<String>) -> Result<(), String>;
#[tauri::command] pub async fn llm_check_embedding_available() -> Result<bool, String>;
#[tauri::command] pub async fn llm_get_embed_models() -> Result<Vec<EmbedModelInfo>, String>;
#[tauri::command] pub async fn llm_download_embed_model(model_name: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_search_guide_chunks(account_id: String, query: String, language: String, top_k: Option<usize>) -> Result<Vec<GuideChunk>, String>;
#[tauri::command] pub async fn llm_rebuild_guide_embeddings(language: String) -> Result<(), String>;

// 统计
#[tauri::command] pub async fn llm_get_stats(account_id: String) -> Result<LlmUsageStats, String>;
#[tauri::command] pub async fn llm_reset_stats(account_id: String) -> Result<(), String>;
#[tauri::command] pub async fn llm_persist_stats(account_id: String) -> Result<(), String>;

// 指南检索
#[tauri::command] pub async fn llm_find_guides(query: String) -> Result<Vec<GuideReference>, String>;
#[tauri::command] pub async fn llm_check_connection(provider_id: String) -> Result<ConnectionStatus, String>;

// 系统提示与 AI 功能开关
#[tauri::command] pub async fn llm_set_system_prompt_switch(enabled: bool) -> Result<(), String>;
#[tauri::command] pub async fn llm_set_ai_features(account_id: String, features: AiFeatures) -> Result<(), String>;
#[tauri::command] pub async fn llm_get_config(account_id: String) -> Result<LlmConfig, String>;
```

---

## 12. OCR 模块

```rust
#[tauri::command]
pub async fn ocr_scan_image(file_path: String) -> Result<OcrResult, String>;

#[tauri::command]
pub async fn ocr_scan_mrz(file_path: String) -> Result<MrzResult, String>;
// 身份证/MRTD 机读区识别，未检测到 MRZ 时自动 fallback 到通用 OCR

#[tauri::command]
pub async fn ocr_get_supported_languages() -> Result<Vec<String>, String>;

#[tauri::command]
pub async fn ocr_list_available_tiers() -> Result<Vec<OcrTierInfo>, String>;
// tiny / small / medium 三档

#[tauri::command]
pub async fn ocr_get_active_tier() -> Result<String, String>;

#[tauri::command]
pub async fn ocr_set_active_tier(tier: String) -> Result<(), String>;

#[tauri::command]
pub async fn ocr_get_model_status(tier: String) -> Result<OcrModelStatus, String>;

#[tauri::command]
pub async fn ocr_install_bundled_model() -> Result<(), String>;
// 安装打包内置的 small 模型

#[tauri::command]
pub async fn ocr_install_bundled_model_with_progress(tier: String) -> Result<(), String>;
// 带进度推送的安装，通过 Event "ocr-install-progress" 推送 { tier, progress, done, error? }

#[tauri::command]
pub async fn ocr_download_model(tier: String) -> Result<(), String>;
```

---

## 13. Crypto 密码学模块

```rust
// 使用当前会话密钥
#[tauri::command] pub async fn encrypt_bytes(data: Vec<u8>) -> Result<Vec<u8>, String>;
#[tauri::command] pub async fn decrypt_bytes(data: Vec<u8>) -> Result<Vec<u8>, String>;

// 使用自定义密钥
#[tauri::command] pub async fn encrypt_with_key(key: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, String>;
#[tauri::command] pub async fn decrypt_with_key(key: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, String>;

// 密钥派生
#[tauri::command] pub async fn derive_key(password: Vec<u8>, salt: Vec<u8>, memory_mib: u32, iterations: u32, parallelism: u32) -> Result<Vec<u8>, String>;

// 工具函数
#[tauri::command] pub async fn generate_salt(length: u32) -> Vec<u8>;
#[tauri::command] pub async fn constant_time_compare(a: Vec<u8>, b: Vec<u8>) -> bool;
#[tauri::command] pub async fn verify_password(password: String) -> Result<bool, String>;

// 文件加解密
#[tauri::command] pub async fn encrypt_file(source: String, dest: String) -> Result<(), String>;
#[tauri::command] pub async fn decrypt_file(source: String, dest: String) -> Result<(), String>;
```

---

## 14. FS 文件系统模块

```rust
#[tauri::command]
pub async fn fs_get_file_size(path: String) -> Result<u64, String>;

#[tauri::command]
pub async fn fs_is_dir(path: String) -> Result<bool, String>;

#[tauri::command]
pub async fn fs_read_file_as_data_url(path: String) -> Result<String, String>;
// 读取文件并返回 base64 Data URL

#[tauri::command]
pub async fn fs_scan_directory(path: String) -> Result<Vec<FsEntry>, String>;
```

---

## 15. Plugin 插件模块

```rust
#[tauri::command] pub async fn plugin_list_all() -> Result<Vec<PluginInfo>, String>;
#[tauri::command] pub async fn plugin_list_installed() -> Result<Vec<InstalledPlugin>, String>;
#[tauri::command] pub async fn plugin_install(plugin_id: String) -> Result<(), String>;
#[tauri::command] pub async fn plugin_update(plugin_id: String) -> Result<(), String>;
#[tauri::command] pub async fn plugin_uninstall(plugin_id: String) -> Result<(), String>;
#[tauri::command] pub async fn plugin_run(plugin_id: String, input: serde_json::Value) -> Result<serde_json::Value, String>;
#[tauri::command] pub async fn plugin_consent_response(session_id: String, consent: PluginConsent) -> Result<(), String>;
#[tauri::command] pub async fn plugin_dialog_response(session_id: String, response: serde_json::Value) -> Result<(), String>;
#[tauri::command] pub async fn plugin_list_sessions() -> Result<Vec<PluginSession>, String>;
#[tauri::command] pub async fn plugin_audit_log(plugin_id: String, limit: Option<usize>) -> Result<Vec<PluginAuditEntry>, String>;
#[tauri::command] pub async fn plugin_update_registry() -> Result<(), String>;
```

---

## 16. 事件定义（Rust → 前端推送）

### 生命周期事件

```rust
pub const EVENT_VAULT_LOCKED: &str = "vault-locked";
pub const EVENT_VAULT_UNLOCKED: &str = "vault-unlocked";
pub const EVENT_THEME_CHANGED: &str = "theme-changed";
pub const EVENT_LOCALE_CHANGED: &str = "locale-changed";
pub const EVENT_DATA_CHANGED: &str = "data-changed";
pub const EVENT_SETTINGS_CHANGED: &str = "settings-changed";
```

### 生物识别事件

```rust
pub const EVENT_BIOMETRIC_ENABLED: &str = "biometric-enabled";
pub const EVENT_BIOMETRIC_DISABLED: &str = "biometric-disabled";
pub const EVENT_BIOMETRIC_CREDENTIAL_EXPIRED: &str = "biometric-credential-expired";
```

### 流式推送事件

| 事件名 | Payload | 说明 |
|--------|---------|------|
| `llm-stream-chunk` | `{ conversationId, chunk, isDone, error? }` | LLM 流式对话逐字推送 |
| `ocr-install-progress` | `{ tier, progress (0–100), done, error? }` | OCR 模型下载进度 |
| `export-progress` | `{ percent, message }` | 导入导出进度 |
| `import-progress` | `{ percent, message }` | 导入进度 |
| `sync-progress` | `{ deviceId, percent, message }` | 设备同步进度 |

```typescript
// 前端监听示例
import { listen } from '@tauri-apps/api/event';
useEffect(() => {
  const unlisten = listen('vault-locked', () => navigate('/login'));
  return () => { unlisten.then(f => f()); };
}, []);
```

---

## 17. tauri-specta 类型生成

系统使用 `tauri-specta` 为所有 `#[tauri::command]` 自动生成 TypeScript Binding。生成的 `src/lib/ipc.ts` 不可手动修改。

示例：

```typescript
// Rust 命令: pub async fn login(account_id: String, password: String) -> Result<AccountInfo, String>;
// 自动生成 TS 函数:
export async function login(accountId: string, password: string): Promise<AccountInfo> {
  return await invoke('login', { accountId, password });
}

// Rust 命令: pub async fn object_list(account_id: String, filter: Option<ObjectFilter>) -> Result<Vec<ObjectSummary>, String>;
// 自动生成 TS 函数:
export async function objectList(accountId: string, filter: ObjectFilter | null): Promise<ObjectSummary[]> {
  return await invoke('object_list', { accountId, filter });
}
```

> `tauri-specta` 自动完成 Rust `snake_case` → TypeScript `camelCase` 转换，命令调用仍使用原始 snake_case 字符串。

---

## 18. 安全约束

| 约束 | 要求 |
|------|------|
| 密码接收范围 | 仅 `bootstrap`、`login`、`unlock`、`unlock_with_password`、`change_password`、`verify_password` 及生物识别绑定可通过参数接收密码（`delete_account` 命令已随 P002 移除） |
| 零流出原则 | 前端永不接收 Rust 内存中的密钥（`[u8; 32]` 主密钥），全部收敛在后端操作 |
| 敏感度控制 | 已并入 Object 数据节点，前端按 `SensitivityLevel` 标志位局部模糊/锁定，不再涉及独立 IPC |
| 错误不泄露内部状态 | 不返回 "table x not found"，返回中文用户可理解消息如"数据加载失败" |
| 命令权限 | 需 Vault 解锁才可调用的命令在 Rust 层检查 `AppState.ensure_unlocked()` |

---

## 19. 完成标准

### P0（必须）
- [x] 全部约 155 个命令编译通过
- [x] `tauri-specta` 生成 TypeScript 类型文件
- [x] IPC 调用可从前端 `invoke` 到 Rust 并返回
- [x] 密码/密钥不通过任何 IPC 返回值泄露

### P1（重要 — 事件与生物识别）
- [x] `vault-locked` / `vault-unlocked` 事件可被前端监听并触发导航/状态同步
- [x] 5 个 `biometric_*` 命令编译通过且可用
- [x] `biometric_check_availability` 返回正确的平台生物识别能力
- [x] `biometric_unlock` 通过系统对话框验证后从 Keychain 读取会话密钥
- [x] `biometric-enabled` / `biometric-disabled` / `biometric-credential-expired` 事件可被前端监听

### P2（实现完成确认）
- [x] 流式事件（LLM stream / OCR progress / export-import progress）正确推送
- [x] 附件模块 14 个命令全部可用（含批量操作和孤儿清理）
- [x] 模板模块 `template_check_field_usage` 返回正确字段使用统计

---

*文档版本：v3.0（实现后补充）*
*创建日期：2026-06-05*
*最后更新：2026-06-25*
*对应开发阶段：Phase 1-2（IPC 层），已全部实现*
