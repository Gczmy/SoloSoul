import i18n from './i18n';
import {
  readBackendError,
  readLegacyObjectError,
  readLegacyLlmError,
  normalizePluginError,
} from './backendErrorWire';
import type { BackendErrorCode } from './backendErrorWire';
import { resolveI18nPrefix } from './utils';

// ─── P029: 两套后端错误库合并——Rust 静态错误映射（原 rustErrors.ts）与
// 前缀 token 解析（原 backendError.ts）统一在本模块。单一入口：
// ① resolveBackendErrorMessage：先查前缀 token，未命中回退 Rust 精确/前缀映射；
// ② translateRustError：直接返回 i18n key（供需 key 判断的调用方，如 BootstrapPage）。

const BACKEND_ERROR_KEYS = {
  VAULT_LOCKED: 'common:vault_locked',
  VAULT_BUSY: 'common:backend_vault_busy',
  SESSION_EXPIRED: 'common:backend_session_expired',
  INTERNAL_ERROR: 'common:backend_operation_failed',
  OBJECT_NAME_REQUIRED: 'common:backend_object_name_required',
  OBJECT_NAME_TOO_LONG: 'common:backend_object_name_too_long',
  OBJECT_PAYLOAD_TOO_LARGE: 'common:backend_object_payload_too_large',
  OBJECT_NOT_FOUND: 'common:backend_object_not_found',
  OBJECT_ID_EXISTS: 'common:backend_object_id_exists',
  OBJECT_VALIDATION_FAILED: 'common:backend_object_validation_failed',
  OBJECT_READ_FAILED: 'common:object_load_failed',
  OBJECT_WRITE_FAILED: 'common:object_save_failed',
  OBJECT_TEMPLATE_MISSING: 'common:backend_object_template_missing',
  OBJECT_TEMPLATE_NOT_FOUND: 'common:backend_object_template_not_found',
  OBJECT_TEMPLATE_READ_FAILED: 'common:backend_object_template_read_failed',
  SNAPSHOT_READ_FAILED: 'common:history_load_failed',
  SNAPSHOT_NOT_FOUND: 'common:backend_snapshot_not_found',
  SNAPSHOT_INVALID: 'common:backend_snapshot_invalid',
  SNAPSHOT_OWNERSHIP_MISMATCH: 'common:backend_snapshot_ownership_mismatch',
  SNAPSHOT_ROLLBACK_FAILED: 'common:rollback_failed',
  LLM_INVALID_REQUEST: 'common:backend_llm_invalid_request',
  LLM_PROVIDER_NOT_CONFIGURED: 'common:backend_llm_provider_not_configured',
  LLM_PROVIDER_DISABLED: 'common:backend_llm_provider_disabled',
  LLM_PROVIDER_NOT_REGISTERED: 'common:backend_llm_provider_not_registered',
  LLM_CONFIRMATION_CANCELLED: 'common:backend_llm_confirmation_cancelled',
  LLM_CONFIRMATION_TIMEOUT: 'common:backend_llm_confirmation_timeout',
  LLM_PROVIDER_READ_FAILED: 'common:backend_llm_provider_read_failed',
  LLM_PROVIDER_WRITE_FAILED: 'common:backend_llm_provider_write_failed',
  LLM_NETWORK_FAILED: 'common:backend_llm_network_failed',
  LLM_TIMEOUT: 'common:backend_llm_timeout',
  LLM_PROVIDER_REJECTED: 'common:backend_llm_provider_rejected',
  LLM_PROVIDER_UNAVAILABLE: 'common:backend_llm_provider_unavailable',
  LLM_RATE_LIMITED: 'common:backend_llm_rate_limited',
  LLM_RESPONSE_INVALID: 'common:backend_llm_response_invalid',
  LLM_CONVERSATION_NOT_FOUND: 'common:backend_llm_conversation_not_found',
  LLM_CONVERSATION_READ_FAILED: 'common:backend_llm_conversation_read_failed',
  LLM_CONVERSATION_WRITE_FAILED: 'common:backend_llm_conversation_write_failed',
  LLM_REPLY_SAVE_FAILED: 'common:backend_llm_reply_save_failed',
  LLM_CONTEXT_READ_FAILED: 'common:backend_llm_context_read_failed',
  LLM_USAGE_FAILED: 'common:backend_llm_usage_failed',
  LLM_GUIDE_FAILED: 'common:backend_llm_guide_failed',
  LLM_EMBEDDING_FAILED: 'common:backend_llm_embedding_failed',
  BACKUP_INVALID_NAME: 'common:backend_backup_invalid_name',
  BACKUP_NOT_FOUND: 'common:backend_backup_not_found',
  BACKUP_READ_FAILED: 'common:backend_backup_read_failed',
  BACKUP_WRITE_FAILED: 'common:backend_backup_write_failed',
  BACKUP_INVALID_PACKAGE: 'common:backend_backup_invalid_package',
  BACKUP_UNSUPPORTED_VERSION: 'common:backend_backup_unsupported_version',
  BACKUP_RESTORE_FAILED: 'common:backend_backup_restore_failed',
  BACKUP_RESTORE_PARTIAL: 'common:backend_backup_restore_partial',
  BACKUP_METADATA_FAILED: 'common:backend_backup_metadata_failed',
  TRANSFER_INVALID_PATH: 'common:backend_transfer_invalid_path',
  TRANSFER_TASK_UNCONFIRMED: 'common:backend_transfer_task_unconfirmed',
  EXPORT_PASSWORD_REQUIRED: 'common:backend_export_password_required',
  EXPORT_PASSWORD_MATCHES_MASTER: 'common:backend_export_password_matches_master',
  EXPORT_PASSWORD_CHECK_FAILED: 'common:backend_export_password_check_failed',
  EXPORT_SCOPE_EMPTY: 'common:backend_export_scope_empty',
  EXPORT_OBJECT_NOT_FOUND: 'common:backend_export_object_not_found',
  EXPORT_ATTACHMENT_TOO_LARGE: 'common:backend_export_attachment_too_large',
  EXPORT_TOO_LARGE: 'common:backend_export_too_large',
  EXPORT_FORMAT_UNSUPPORTED: 'common:backend_export_format_unsupported',
  EXPORT_READ_FAILED: 'common:backend_export_read_failed',
  EXPORT_WRITE_FAILED: 'common:backend_export_write_failed',
  EXPORT_RENDER_FAILED: 'common:backend_export_render_failed',
  EXPORT_FAILED: 'common:backend_export_failed',
  IMPORT_FILE_MISSING: 'common:backend_import_file_missing',
  IMPORT_INVALID_PACKAGE: 'common:backend_import_invalid_package',
  IMPORT_MANIFEST_MISSING: 'common:backend_import_manifest_missing',
  IMPORT_SALT_MISSING: 'common:backend_import_salt_missing',
  IMPORT_DECRYPT_FAILED: 'common:backend_import_decrypt_failed',
  IMPORT_PASSWORD_REQUIRED: 'common:backend_import_password_required',
  IMPORT_BAD_PASSWORD: 'common:backend_import_bad_password',
  IMPORT_INVALID_OPERATION: 'common:backend_import_invalid_operation',
  IMPORT_INVALID_CLOUD_OPTIONS: 'common:backend_import_invalid_cloud_options',
  IMPORT_OPERATION_MISMATCH: 'common:backend_import_operation_mismatch',
  IMPORT_OPERATION_NOT_FOUND: 'common:backend_import_operation_not_found',
  IMPORT_OPERATION_ABANDONED: 'common:backend_import_operation_abandoned',
  IMPORT_OPERATION_CONFLICT: 'common:backend_import_operation_conflict',
  IMPORT_READ_FAILED: 'common:backend_import_read_failed',
  IMPORT_FAILED: 'common:backend_import_failed',
  SYNC_NOT_ENABLED: 'common:backend_sync_not_enabled',
  SYNC_NOT_RUNNING: 'common:backend_sync_not_running',
  SYNC_INVALID_ADDRESS: 'common:backend_sync_invalid_address',
  SYNC_PEER_NOT_FOUND: 'common:backend_sync_peer_not_found',
  SYNC_CONNECT_FAILED: 'common:backend_sync_connect_failed',
  SYNC_CONNECT_TIMEOUT: 'common:backend_sync_connect_timeout',
  SYNC_CONNECT_REFUSED: 'common:backend_sync_connect_refused',
  SYNC_HANDSHAKE_FAILED: 'common:backend_sync_handshake_failed',
  SYNC_PAIRING_PENDING: 'common:backend_sync_pairing_pending',
  SYNC_PAIRING_INVALID: 'common:backend_sync_pairing_invalid',
  SYNC_SESSION_FAILED: 'common:backend_sync_session_failed',
  SYNC_TASK_UNCONFIRMED: 'common:backend_sync_task_unconfirmed',
  SYNC_ENABLE_FAILED: 'common:backend_sync_enable_failed',
  SYNC_ENABLE_TIMEOUT: 'common:backend_sync_enable_timeout',
  SYNC_DISCOVERY_FAILED: 'common:backend_sync_discovery_failed',
  SYNC_DISCOVERY_TIMEOUT: 'common:backend_sync_discovery_timeout',
  SYNC_READ_FAILED: 'common:backend_sync_read_failed',
  SYNC_WRITE_FAILED: 'common:backend_sync_write_failed',
  SYNC_CONFLICT_NOT_FOUND: 'common:backend_sync_conflict_not_found',
  SYNC_CONFLICT_INVALID: 'common:backend_sync_conflict_invalid',
  SYNC_CONFLICT_FAILED: 'common:backend_sync_conflict_failed',
  SYNC_RECOVERY_INVALID: 'common:backend_sync_recovery_invalid',
  SYNC_RECOVERY_FAILED: 'common:backend_sync_recovery_failed',
  SYNC_PERMISSION_DENIED: 'common:backend_sync_permission_denied',
  SYNC_UNSUPPORTED: 'common:backend_sync_unsupported',
  PLUGIN_CHECKSUM_MISMATCH: 'common:backend_plugin_checksum_mismatch',
  PLUGIN_CONSENT_DENIED: 'common:backend_plugin_consent_denied',
  PLUGIN_EXECUTION_FAILED: 'common:backend_plugin_execution_failed',
  PLUGIN_INVALID_ARGUMENT: 'common:backend_plugin_invalid_argument',
  PLUGIN_INVALID_FIELD: 'common:backend_plugin_invalid_field',
  PLUGIN_MANIFEST_INVALID: 'common:backend_plugin_manifest_invalid',
  PLUGIN_NETWORK_FAILED: 'common:backend_plugin_network_failed',
  PLUGIN_NOT_FOUND: 'common:backend_plugin_not_found',
  PLUGIN_RATE_LIMITED: 'common:backend_plugin_rate_limited',
  PLUGIN_REGISTRY_FAILED: 'common:backend_plugin_registry_failed',
  PLUGIN_SESSION_EXPIRED: 'common:backend_plugin_session_expired',
  PLUGIN_STORE_FAILED: 'common:backend_plugin_store_failed',
  PLUGIN_TASK_UNCONFIRMED: 'common:backend_plugin_task_unconfirmed',
  PLUGIN_VERSION_INCOMPATIBLE: 'common:backend_plugin_version_incompatible',
  PLUGIN_WASM_TOO_LARGE: 'common:backend_plugin_wasm_too_large',
  PLUGIN_INSTALL_CANCELLED: 'common:backend_plugin_install_cancelled',
  PLUGIN_INSTALL_ALREADY_STARTED: 'common:backend_plugin_install_already_started',
  PLUGIN_INVALID_OPERATION: 'common:backend_plugin_invalid_operation',
  PLUGIN_OUTPUT_INVALID: 'common:backend_plugin_output_invalid',
  PLUGIN_OUTPUT_DENIED: 'common:backend_plugin_output_denied',
  PLUGIN_OUTPUT_READ_FAILED: 'common:backend_plugin_output_read_failed',
  PLUGIN_OUTPUT_WRITE_FAILED: 'common:backend_plugin_output_write_failed',
  PLUGIN_OUTPUT_OPEN_FAILED: 'common:backend_plugin_output_open_failed',
  PLUGIN_UNSUPPORTED: 'common:backend_plugin_unsupported',
  PLUGIN_READ_FAILED: 'common:backend_plugin_read_failed',
  PLUGIN_INSTALL_FAILED: 'common:backend_plugin_install_failed',
} satisfies Record<BackendErrorCode, string>;

/** 显示层可使用当前 React 翻译器；机器码在传输与状态层保持原样。 */
export function getBackendErrorTranslationKey(code: BackendErrorCode): string {
  return BACKEND_ERROR_KEYS[code];
}

/** Rust 静态错误串 → i18n key 精确映射表。 */
const RUST_ERROR_MAP: Record<string, string> = {
  // Auth / Vault
  'Invalid password': 'common:invalid_password',
  'Verify failed': 'common:verify_failed',
  'Too many failed attempts; try again later': 'common:password_locked',
  'Account name is required': 'common:account_name_required',
  'Account name already taken': 'common:account_name_taken',
  'Account ID already exists': 'common:account_id_exists',
  'Account not found': 'common:account_not_found',
  // P029-R1: 原映射 common:password_too_short 在双语 common.json 均不存在，
  // 渲染裸键名；settings.json 已有同义键，改指之。
  'Password must be at least 8 characters': 'settings:password_too_short',
  'No account is currently unlocked': 'common:no_account_unlocked',

  // Backup
  'Backup name cannot be empty': 'common:backup_name_empty',

  // Attachments
  attachment_cleanup_pending: 'common:attachment_cleanup_pending',
  'No file path available': 'common:no_file_path',
  'Source path must not be inside vault storage': 'common:path_inside_vault',
  "Destination path must not contain '..'": 'common:path_traversal',
  'Source path must be within vault storage': 'common:path_outside_vault',
  'Attachment path is outside vault storage': 'common:path_outside_vault',
  'Destination parent directory does not exist': 'common:dest_parent_missing',
  'Invalid destination path': 'common:invalid_dest_path',

  // File system
  'Path traversal is not allowed': 'common:path_traversal',
  'Path is outside the allowed directory': 'common:path_outside_allowed',
  'Backup file too large (> 500 MB)': 'common:file_too_large',
  'Not a directory': 'common:not_a_directory',

  // Sync
  'Not connected': 'common:not_connected',
  'Cannot advertise sync service: no unlocked account': 'common:need_unlock_sync',
  'Invalid magic prefix': 'common:sync_invalid_prefix',

  // LLM
  'No active provider configured': 'common:no_active_provider',
  'Active provider not found': 'common:active_provider_not_found',
  'No text in Anthropic response': 'common:empty_ai_response',
  'No content in OpenAI response': 'common:empty_ai_response',

  // Embedding / Model
  'Model ID cannot be empty': 'common:model_id_empty',

  // Crypto
  'Master key must be 32 bytes': 'common:crypto_invalid_key',
  'Key derivation failed': 'common:crypto_derivation_failed',

  // Generic
  'Parse error': 'common:parse_error',
  'Config parse error': 'common:parse_error',
};

/** Rust 动态错误前缀 → i18n key 映射表（带 ID/路径等动态后缀）。 */
const RUST_PREFIX_MAP: Record<string, string> = {
  'Invalid attachment id: ': 'common:invalid_attachment_id',
  'Invalid addr: ': 'common:invalid_addr',
  'File too large': 'common:file_too_large',
};

/**
 * Translate a Rust error message to its i18n key (P029 并入本模块)。
 * Returns `null` when no mapping exists (caller should use the raw message).
 */
export function translateRustError(msg: string): string | null {
  const machineKey = Object.hasOwn(BACKEND_ERROR_KEYS, msg)
    ? BACKEND_ERROR_KEYS[msg as BackendErrorCode]
    : null;
  if (machineKey) return machineKey;
  const legacy = readLegacyObjectError(msg) ?? readLegacyLlmError(msg);
  if (legacy) return BACKEND_ERROR_KEYS[legacy.code];
  const key = Object.hasOwn(RUST_ERROR_MAP, msg) ? RUST_ERROR_MAP[msg] : null;
  if (key) return key;
  for (const [prefix, mappedKey] of Object.entries(RUST_PREFIX_MAP)) {
    if (msg.startsWith(prefix)) return mappedKey;
  }
  return null;
}

/**
 * 同步连接类错误（`__SYNC_ERR__:connect_failed:<os 错误>`）的 detail 翻译。
 * detail 是 Rust std::io::Error 的英文 Display（如 `Connection timed out (os error 110)`），
 * 这里把常见模式映射为本地化文案；未识别模式返回 null（保留原文透传）。
 */
function translateSyncConnectDetail(detail: string): string | null {
  const d = detail.toLowerCase();
  if (d.includes('timed out') || d.includes('timedout')) {
    return i18n.t('settings:sync_err_connect_timeout');
  }
  if (d.includes('refused')) {
    return i18n.t('settings:sync_err_connect_refused');
  }
  if (d.includes('unreachable') || d.includes('network is down')) {
    return i18n.t('settings:sync_err_connect_unreachable');
  }
  if (d.includes('no route to host') || d.includes('not known')) {
    return i18n.t('settings:sync_err_connect_no_route');
  }
  return null;
}

/**
 * 同步握手类错误（`__SYNC_ERR__:handshake_failed:<detail>`）的 detail 翻译。
 *
 * 后端 `wrap_session_error` 会把 vault 锁定等内部英文错误包进 detail
 * （如 `Vault is locked` / `Vault is not unlocked`），此前原样透传导致
 * 用户看到「与设备握手失败：vault is locked」的英文。这里把常见模式
 * 映射为本地化文案；未识别模式返回 null（保留原文透传）。
 */
function translateSyncHandshakeDetail(detail: string): string | null {
  const d = detail.toLowerCase();
  // 保险库已锁定：同步读写需要已解锁的 VaultStore，解锁后重试即可。
  if (d.includes('vault') && (d.includes('locked') || d.includes('not unlocked'))) {
    return i18n.t('settings:sync_err_handshake_vault_locked');
  }
  return null;
}

/**
 * Resolve a backend error into a user-facing localized message.
 *
 * Backend commands return strings like `__EXPORT_ERR__:PASSWORD_REQUIRE_LETTER_DIGIT`
 * so the frontend can translate them without embedding English in Rust.
 */
export function resolveBackendErrorMessage(err: unknown): string {
  const structured = readBackendError(err);
  if (structured) return i18n.t(BACKEND_ERROR_KEYS[structured.code]);
  if (typeof err === 'object' && err !== null && 'code' in err)
    return i18n.t('common:backend_operation_failed');
  const raw = err instanceof Error ? err.message : String(err);
  const parsed = resolveI18nPrefix(raw);
  if (!parsed) {
    // P029: 未命中前缀 token 时回退 Rust 静态错误映射（原 rustErrors.ts 职责）
    const rustKey = translateRustError(raw);
    if (rustKey) return i18n.t(rustKey);
    return raw;
  }

  const key = `${parsed.kind}_err_${parsed.code.toLowerCase()}`;
  const ns = 'settings';

  if (!i18n.exists(key, { ns })) {
    return raw;
  }

  let detail = parsed.payload;
  // 同步连接类错误：把 OS 层英文 detail 翻译为本地化文案（未识别模式保留原文）。
  if (parsed.code.toLowerCase() === 'connect_failed' && detail) {
    detail = translateSyncConnectDetail(detail) ?? detail;
  }
  // 同步握手类错误：vault 锁定等内部英文 detail 同样本地化（如「vault is locked」）。
  if (parsed.code.toLowerCase() === 'handshake_failed' && detail) {
    detail = translateSyncHandshakeDetail(detail) ?? detail;
  }

  return i18n.t(key, {
    ns,
    ...(detail ? { detail } : {}),
  });
}

/** 永久删除已提交，但实体文件仍待重试；不能与提交前失败混为一类。 */
export function isAttachmentCleanupPending(error: unknown): boolean {
  return (error instanceof Error ? error.message : String(error)) === 'attachment_cleanup_pending';
}

/** 插件错误只显示固定翻译，不展示旧 Host 的字段、路径或正文。 */
export function resolvePluginErrorMessage(error: unknown): string {
  return resolveBackendErrorMessage(normalizePluginError(error));
}
