/** IPC 纯传输层：验证并投影安全机器信息；没有 i18n/logger/store 依赖。 */
import type {
  BackendError,
  BackendErrorCode,
  BackendErrorStage,
  IpcCommandErrors,
} from './generated/ipcContracts';
export type { BackendError, BackendErrorCode } from './generated/ipcContracts';

const CODES = {
  VAULT_LOCKED: true,
  VAULT_BUSY: true,
  SESSION_EXPIRED: true,
  INTERNAL_ERROR: true,
  OBJECT_NAME_REQUIRED: true,
  OBJECT_NAME_TOO_LONG: true,
  OBJECT_PAYLOAD_TOO_LARGE: true,
  OBJECT_NOT_FOUND: true,
  OBJECT_ID_EXISTS: true,
  OBJECT_VALIDATION_FAILED: true,
  OBJECT_READ_FAILED: true,
  OBJECT_WRITE_FAILED: true,
  OBJECT_TEMPLATE_MISSING: true,
  OBJECT_TEMPLATE_NOT_FOUND: true,
  OBJECT_TEMPLATE_READ_FAILED: true,
  SNAPSHOT_READ_FAILED: true,
  SNAPSHOT_NOT_FOUND: true,
  SNAPSHOT_INVALID: true,
  SNAPSHOT_OWNERSHIP_MISMATCH: true,
  SNAPSHOT_ROLLBACK_FAILED: true,
  LLM_INVALID_REQUEST: true,
  LLM_PROVIDER_NOT_CONFIGURED: true,
  LLM_PROVIDER_DISABLED: true,
  LLM_PROVIDER_NOT_REGISTERED: true,
  LLM_CONFIRMATION_CANCELLED: true,
  LLM_CONFIRMATION_TIMEOUT: true,
  LLM_PROVIDER_READ_FAILED: true,
  LLM_PROVIDER_WRITE_FAILED: true,
  LLM_NETWORK_FAILED: true,
  LLM_TIMEOUT: true,
  LLM_PROVIDER_REJECTED: true,
  LLM_PROVIDER_UNAVAILABLE: true,
  LLM_RATE_LIMITED: true,
  LLM_RESPONSE_INVALID: true,
  LLM_CONVERSATION_NOT_FOUND: true,
  LLM_CONVERSATION_READ_FAILED: true,
  LLM_CONVERSATION_WRITE_FAILED: true,
  LLM_REPLY_SAVE_FAILED: true,
  LLM_CONTEXT_READ_FAILED: true,
  LLM_USAGE_FAILED: true,
  LLM_GUIDE_FAILED: true,
  LLM_EMBEDDING_FAILED: true,
  BACKUP_INVALID_NAME: true,
  BACKUP_NOT_FOUND: true,
  BACKUP_READ_FAILED: true,
  BACKUP_WRITE_FAILED: true,
  BACKUP_INVALID_PACKAGE: true,
  BACKUP_UNSUPPORTED_VERSION: true,
  BACKUP_RESTORE_FAILED: true,
  BACKUP_RESTORE_PARTIAL: true,
  BACKUP_METADATA_FAILED: true,
  TRANSFER_INVALID_PATH: true,
  TRANSFER_TASK_UNCONFIRMED: true,
  EXPORT_PASSWORD_REQUIRED: true,
  EXPORT_PASSWORD_MATCHES_MASTER: true,
  EXPORT_PASSWORD_CHECK_FAILED: true,
  EXPORT_SCOPE_EMPTY: true,
  EXPORT_OBJECT_NOT_FOUND: true,
  EXPORT_ATTACHMENT_TOO_LARGE: true,
  EXPORT_TOO_LARGE: true,
  EXPORT_FORMAT_UNSUPPORTED: true,
  EXPORT_READ_FAILED: true,
  EXPORT_WRITE_FAILED: true,
  EXPORT_RENDER_FAILED: true,
  EXPORT_FAILED: true,
  IMPORT_FILE_MISSING: true,
  IMPORT_INVALID_PACKAGE: true,
  IMPORT_MANIFEST_MISSING: true,
  IMPORT_SALT_MISSING: true,
  IMPORT_DECRYPT_FAILED: true,
  IMPORT_PASSWORD_REQUIRED: true,
  IMPORT_BAD_PASSWORD: true,
  IMPORT_INVALID_OPERATION: true,
  IMPORT_INVALID_CLOUD_OPTIONS: true,
  IMPORT_OPERATION_MISMATCH: true,
  IMPORT_OPERATION_NOT_FOUND: true,
  IMPORT_OPERATION_ABANDONED: true,
  IMPORT_OPERATION_CONFLICT: true,
  IMPORT_READ_FAILED: true,
  IMPORT_FAILED: true,
  SYNC_NOT_ENABLED: true,
  SYNC_NOT_RUNNING: true,
  SYNC_INVALID_ADDRESS: true,
  SYNC_PEER_NOT_FOUND: true,
  SYNC_CONNECT_FAILED: true,
  SYNC_CONNECT_TIMEOUT: true,
  SYNC_CONNECT_REFUSED: true,
  SYNC_HANDSHAKE_FAILED: true,
  SYNC_PAIRING_PENDING: true,
  SYNC_PAIRING_INVALID: true,
  SYNC_SESSION_FAILED: true,
  SYNC_TASK_UNCONFIRMED: true,
  SYNC_ENABLE_FAILED: true,
  SYNC_ENABLE_TIMEOUT: true,
  SYNC_DISCOVERY_FAILED: true,
  SYNC_DISCOVERY_TIMEOUT: true,
  SYNC_READ_FAILED: true,
  SYNC_WRITE_FAILED: true,
  SYNC_CONFLICT_NOT_FOUND: true,
  SYNC_CONFLICT_INVALID: true,
  SYNC_CONFLICT_FAILED: true,
  SYNC_RECOVERY_INVALID: true,
  SYNC_RECOVERY_FAILED: true,
  SYNC_PERMISSION_DENIED: true,
  SYNC_UNSUPPORTED: true,
  PLUGIN_CHECKSUM_MISMATCH: true,
  PLUGIN_CONSENT_DENIED: true,
  PLUGIN_EXECUTION_FAILED: true,
  PLUGIN_INVALID_ARGUMENT: true,
  PLUGIN_INVALID_FIELD: true,
  PLUGIN_MANIFEST_INVALID: true,
  PLUGIN_NETWORK_FAILED: true,
  PLUGIN_NOT_FOUND: true,
  PLUGIN_RATE_LIMITED: true,
  PLUGIN_REGISTRY_FAILED: true,
  PLUGIN_SESSION_EXPIRED: true,
  PLUGIN_STORE_FAILED: true,
  PLUGIN_TASK_UNCONFIRMED: true,
  PLUGIN_VERSION_INCOMPATIBLE: true,
  PLUGIN_WASM_TOO_LARGE: true,
  PLUGIN_INSTALL_CANCELLED: true,
  PLUGIN_INSTALL_ALREADY_STARTED: true,
  PLUGIN_INVALID_OPERATION: true,
  PLUGIN_OUTPUT_INVALID: true,
  PLUGIN_OUTPUT_DENIED: true,
  PLUGIN_OUTPUT_READ_FAILED: true,
  PLUGIN_OUTPUT_WRITE_FAILED: true,
  PLUGIN_OUTPUT_OPEN_FAILED: true,
  PLUGIN_UNSUPPORTED: true,
  PLUGIN_READ_FAILED: true,
  PLUGIN_INSTALL_FAILED: true,
} satisfies Record<BackendErrorCode, true>;
const STAGES = {
  validate: true,
  read: true,
  write: true,
  task: true,
  template: true,
  snapshotOwner: true,
  snapshotRead: true,
  snapshotParse: true,
  objectRead: true,
  labels: true,
  version: true,
  serialize: true,
  objectSave: true,
  snapshotSave: true,
  audit: true,
  connect: true,
  handshake: true,
  pairing: true,
  discovery: true,
  conflict: true,
  execute: true,
  install: true,
} satisfies Record<BackendErrorStage, true>;
const OBJECT_COMMANDS = [
  'object_list',
  'object_get',
  'object_field_suggestions',
  'object_create',
  'object_update',
  'object_delete',
  'object_sync_with_template',
  'object_ignore_template_sync',
  'object_list_deprecated_fields',
  'object_trash_list',
  'snapshot_count_batch',
  'snapshot_get_data',
  'snapshot_list',
  'snapshot_rollback',
] as const satisfies ReadonlyArray<keyof IpcCommandErrors>;
export function isObjectErrorCommand(command: string): boolean {
  return (OBJECT_COMMANDS as readonly string[]).includes(command);
}
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function own(value: Record<string, unknown>, key: string): unknown {
  return Object.hasOwn(value, key) ? value[key] : undefined;
}
/** 只保留声明的字段，禁止把后端任意 details/message/cause 放进 Error 或日志。 */
export function readBackendError(value: unknown): BackendError | null {
  if (value instanceof BackendCommandError) return value.backend;
  if (!record(value)) return null;
  const code = own(value, 'code');
  const retryable = own(value, 'retryable');
  if (typeof code !== 'string' || !Object.hasOwn(CODES, code) || typeof retryable !== 'boolean')
    return null;
  const details = own(value, 'safeDetails');
  if (details !== null && !record(details)) return null;
  const safe: BackendError = { code: code as BackendErrorCode, safeDetails: null, retryable };
  if (record(details)) {
    const stage = own(details, 'stage');
    if (typeof stage !== 'string' || !Object.hasOwn(STAGES, stage)) return null;
    safe.safeDetails = { stage: stage as BackendErrorStage };
    if (code === 'SYNC_PAIRING_PENDING') {
      const peer = own(details, 'syncPeerId');
      const sas = own(details, 'sasCode');
      if (stage !== 'pairing' || typeof peer !== 'string' || !/^[A-Za-z0-9_.-]{1,128}$/.test(peer))
        return null;
      if (sas !== undefined && sas !== null && (typeof sas !== 'string' || !/^\d{6}$/.test(sas)))
        return null;
      safe.safeDetails.syncPeerId = peer;
      if (typeof sas === 'string') safe.safeDetails.sasCode = sas;
    }
    const limit = own(details, 'limit');
    // 只接纳服务端两项固定上限；任意数字也可能编码用户数据。
    if (code === 'OBJECT_NAME_TOO_LONG' && limit === 200) safe.safeDetails.limit = 200;
    if (code === 'OBJECT_PAYLOAD_TOO_LARGE' && limit === 10485760)
      safe.safeDetails.limit = 10485760;
    const completedCount = own(details, 'completedCount');
    if (
      typeof completedCount === 'number' &&
      Number.isSafeInteger(completedCount) &&
      completedCount >= 0 &&
      ((code === 'BACKUP_RESTORE_FAILED' && stage === 'write' && completedCount === 0) ||
        (code === 'BACKUP_RESTORE_PARTIAL' && stage === 'write' && completedCount > 0) ||
        (code === 'BACKUP_METADATA_FAILED' && stage === 'read'))
    )
      safe.safeDetails.completedCount = completedCount;
  }
  return safe;
}
const LEGACY_OBJECT_CODES: Record<string, BackendErrorCode> = {
  'No account is currently unlocked': 'VAULT_LOCKED',
  'No account unlocked': 'VAULT_LOCKED',
  'No account is unlocked': 'VAULT_LOCKED',
  'Vault is not unlocked': 'VAULT_LOCKED',
  'Vault not unlocked': 'VAULT_LOCKED',
  'Vault is locked': 'VAULT_LOCKED',
  'Request belongs to an expired session': 'SESSION_EXPIRED',
  'Vault session is no longer current': 'SESSION_EXPIRED',
  IMPORT_DIRECTORY_BUSY: 'VAULT_BUSY',
  IMPORT_OPERATIONS_ACTIVE: 'VAULT_BUSY',
  对象名称不能为空: 'OBJECT_NAME_REQUIRED',
  '对象名称不能超过 200 字符': 'OBJECT_NAME_TOO_LONG',
  '对象属性载荷过大（超过 10 MiB）': 'OBJECT_PAYLOAD_TOO_LARGE',
  'Object not found': 'OBJECT_NOT_FOUND',
  'Object has no associated template': 'OBJECT_TEMPLATE_MISSING',
  'Template not found': 'OBJECT_TEMPLATE_NOT_FOUND',
  'Snapshot not found': 'SNAPSHOT_NOT_FOUND',
  'Snapshot does not belong to object': 'SNAPSHOT_OWNERSHIP_MISMATCH',
  'Snapshot field labels must be an object or null': 'SNAPSHOT_INVALID',
  'Object version overflow': 'SNAPSHOT_ROLLBACK_FAILED',
};
export function makeBackendError(code: BackendErrorCode): BackendError {
  return {
    code,
    safeDetails: null,
    retryable:
      code === 'VAULT_LOCKED' ||
      code === 'VAULT_BUSY' ||
      code === 'OBJECT_READ_FAILED' ||
      code === 'OBJECT_TEMPLATE_READ_FAILED' ||
      code === 'SNAPSHOT_READ_FAILED' ||
      code === 'LLM_NETWORK_FAILED' ||
      code === 'LLM_TIMEOUT' ||
      code === 'LLM_PROVIDER_UNAVAILABLE' ||
      code === 'LLM_RATE_LIMITED' ||
      code === 'LLM_PROVIDER_READ_FAILED' ||
      code === 'LLM_CONVERSATION_READ_FAILED' ||
      code === 'BACKUP_READ_FAILED' ||
      code === 'EXPORT_READ_FAILED' ||
      code === 'IMPORT_READ_FAILED' ||
      code === 'IMPORT_FILE_MISSING' ||
      code === 'IMPORT_PASSWORD_REQUIRED' ||
      code === 'IMPORT_BAD_PASSWORD' ||
      code === 'IMPORT_DECRYPT_FAILED' ||
      code === 'SYNC_NOT_ENABLED' ||
      code === 'SYNC_NOT_RUNNING' ||
      code === 'SYNC_CONNECT_FAILED' ||
      code === 'SYNC_CONNECT_TIMEOUT' ||
      code === 'SYNC_CONNECT_REFUSED' ||
      code === 'SYNC_DISCOVERY_FAILED' ||
      code === 'SYNC_DISCOVERY_TIMEOUT' ||
      code === 'SYNC_READ_FAILED' ||
      code === 'SYNC_ENABLE_TIMEOUT' ||
      code === 'PLUGIN_READ_FAILED' ||
      code === 'PLUGIN_REGISTRY_FAILED' ||
      code === 'PLUGIN_NETWORK_FAILED' ||
      code === 'PLUGIN_RATE_LIMITED',
  };
}
/** 仅供旧对象 Host 兼容；新 Host 已知失败由业务阶段直接编码。 */
export function readLegacyObjectError(value: unknown): BackendError | null {
  const raw = value instanceof Error ? value.message : typeof value === 'string' ? value : null;
  if (raw === null) return null;
  const code = Object.hasOwn(LEGACY_OBJECT_CODES, raw) ? LEGACY_OBJECT_CODES[raw] : null;
  if (code) return makeBackendError(code);
  if (/^Object with ID '[\s\S]*' already exists$/.test(raw))
    return makeBackendError('OBJECT_ID_EXISTS');
  if (/^字段 '[\s\S]*' (?:是动态字段组|最多允许|的第 )/.test(raw))
    return makeBackendError('OBJECT_VALIDATION_FAILED');
  return null;
}
export function normalizeObjectError(value: unknown): BackendError {
  return (
    readBackendError(value) ?? readLegacyObjectError(value) ?? makeBackendError('INTERNAL_ERROR')
  );
}
export class BackendCommandError extends Error {
  readonly backend: BackendError;
  constructor(error: BackendError) {
    const safe = readBackendError(error) ?? makeBackendError('INTERNAL_ERROR');
    super(safe.code);
    this.name = 'BackendCommandError';
    this.backend = Object.freeze({
      ...safe,
      safeDetails: safe.safeDetails ? Object.freeze(safe.safeDetails) : null,
    });
  }
}
/** 错误日志不保留自由文本，旧域也不能把密码/路径从 IPC catch 写到控制台。 */
export function backendErrorLogDetails(value: unknown): BackendError | { code: 'LEGACY_ERROR' } {
  const error = readBackendError(value) ?? readLegacyObjectError(value);
  if (!error) return { code: 'LEGACY_ERROR' };
  if (error.code === 'SYNC_PAIRING_PENDING' && error.safeDetails) {
    return { ...error, safeDetails: { stage: error.safeDetails.stage } };
  }
  return error;
}

/** RF-317：新 Host 仅使用结构化包；文本判定仅供旧版本兼容。 */
export const LLM_ERROR_COMMANDS = [
  'llm_get_config',
  'llm_get_providers',
  'llm_save_provider',
  'llm_set_active_provider',
  'llm_set_ai_features',
  'llm_set_system_prompt_switch',
  'llm_set_local_embedding',
  'llm_accept_risk',
  'llm_get_api_key',
  'llm_delete_provider',
  'llm_check_connection',
  'llm_test_provider',
  'llm_list_conversations',
  'llm_list_trash',
  'llm_get_conversation',
  'llm_save_conversation',
  'llm_soft_delete_conversation',
  'llm_restore_conversation',
  'llm_permanent_delete',
  'llm_rename_conversation',
  'llm_get_stats',
  'llm_reset_stats',
  'llm_send_message_stream',
  'llm_search_guide_chunks',
  'llm_rebuild_guide_embeddings',
  'llm_check_embedding_available',
] as const satisfies ReadonlyArray<keyof IpcCommandErrors>;
export function isLlmErrorCommand(command: string): boolean {
  return (LLM_ERROR_COMMANDS as readonly string[]).includes(command);
}
const LEGACY_LLM_CODES: Record<string, BackendErrorCode> = {
  'No active provider configured': 'LLM_PROVIDER_NOT_CONFIGURED',
  'No active provider': 'LLM_PROVIDER_NOT_CONFIGURED',
  'Active provider not found': 'LLM_PROVIDER_NOT_CONFIGURED',
  'Chat provider is not saved': 'LLM_PROVIDER_NOT_CONFIGURED',
  'Chat provider configuration is missing': 'LLM_PROVIDER_NOT_CONFIGURED',
  'Chat provider is disabled': 'LLM_PROVIDER_DISABLED',
  'Provider is disabled': 'LLM_PROVIDER_DISABLED',
  'Failed to load chat provider configuration': 'LLM_PROVIDER_READ_FAILED',
  'Invalid chat provider configuration': 'LLM_PROVIDER_READ_FAILED',
  'Invalid chat provider credentials': 'LLM_PROVIDER_READ_FAILED',
  'Not found': 'LLM_CONVERSATION_NOT_FOUND',
  'No text in Anthropic response': 'LLM_RESPONSE_INVALID',
  'No content in OpenAI response': 'LLM_RESPONSE_INVALID',
  登记确认等待超时: 'LLM_CONFIRMATION_TIMEOUT',
  登记确认对话框未响应: 'LLM_CONFIRMATION_TIMEOUT',
  '已取消 AI Provider 登记': 'LLM_CONFIRMATION_CANCELLED',
};
export function readLegacyLlmError(value: unknown): BackendError | null {
  const raw = value instanceof Error ? value.message : typeof value === 'string' ? value : null;
  if (raw === null) return null;
  if (Object.hasOwn(CODES, raw)) return makeBackendError(raw as BackendErrorCode);
  const code = Object.hasOwn(LEGACY_LLM_CODES, raw) ? LEGACY_LLM_CODES[raw] : null;
  if (code) return makeBackendError(code);
  if (raw.startsWith('__LLM_PERSIST_FAILED__')) return makeBackendError('LLM_REPLY_SAVE_FAILED');
  const status = /^HTTP (\d{3})(?:\b|:)/.exec(raw)?.[1];
  if (status)
    return makeBackendError(
      status === '429'
        ? 'LLM_RATE_LIMITED'
        : status.startsWith('5')
          ? 'LLM_PROVIDER_UNAVAILABLE'
          : 'LLM_PROVIDER_REJECTED',
    );
  if (/^(?:Request to |Stream error: |Client: )/.test(raw))
    return makeBackendError('LLM_NETWORK_FAILED');
  if (/^(?:Parse response from |Parse: )/.test(raw))
    return makeBackendError('LLM_RESPONSE_INVALID');
  if (raw.startsWith('base_url 未在当前账户登记'))
    return makeBackendError('LLM_PROVIDER_NOT_REGISTERED');
  return null;
}
export function normalizeLlmError(value: unknown): BackendError {
  return (
    readBackendError(value) ??
    readLegacyObjectError(value) ??
    readLegacyLlmError(value) ??
    makeBackendError('INTERNAL_ERROR')
  );
}

/** RF318：旧包错误前缀只在兼容读取时识别；detail 永不进入新的 Error/日志。 */
export const TRANSFER_ERROR_COMMANDS = [
  'backup_create',
  'backup_delete',
  'backup_list',
  'backup_restore',
  'export_document_preflight',
  'export_estimate_size',
  'export_execute',
  'export_get_attachments_batch',
  'export_get_scope_tree',
  'export_objects_document',
  'import_decrypt_preview',
  'import_execute_advanced',
  'import_operation_get',
  'import_operation_resume',
  'import_operations_list',
  'import_parse_package',
] as const satisfies ReadonlyArray<keyof IpcCommandErrors>;
export function isTransferErrorCommand(command: string): boolean {
  return (TRANSFER_ERROR_COMMANDS as readonly string[]).includes(command);
}
const LEGACY_TRANSFER_CODES: Record<string, BackendErrorCode> = {
  EXPORT_PASSWORD_EMPTY: 'EXPORT_PASSWORD_REQUIRED',
  EXPORT_SAME_AS_MASTER_PASSWORD: 'EXPORT_PASSWORD_MATCHES_MASTER',
  EXPORT_MASTER_VERIFY_FAILED: 'EXPORT_PASSWORD_CHECK_FAILED',
  EXPORT_NO_OBJECTS_SELECTED: 'EXPORT_SCOPE_EMPTY',
  EXPORT_ATTACHMENT_TOO_LARGE: 'EXPORT_ATTACHMENT_TOO_LARGE',
  EXPORT_TOTAL_SIZE_EXCEEDED: 'EXPORT_TOO_LARGE',
  EXPORT_FORMAT_NOT_SUPPORTED: 'EXPORT_FORMAT_UNSUPPORTED',
  IMPORT_FILE_NOT_FOUND: 'IMPORT_FILE_MISSING',
  IMPORT_INVALID_PACKAGE: 'IMPORT_INVALID_PACKAGE',
  IMPORT_MISSING_MANIFEST: 'IMPORT_MANIFEST_MISSING',
  IMPORT_MISSING_SALT: 'IMPORT_SALT_MISSING',
  IMPORT_DECRYPT_FAILED: 'IMPORT_DECRYPT_FAILED',
  IMPORT_PASSWORD_REQUIRED: 'IMPORT_PASSWORD_REQUIRED',
  IMPORT_BAD_PASSWORD: 'IMPORT_BAD_PASSWORD',
  IMPORT_INVALID_OPERATION_ID: 'IMPORT_INVALID_OPERATION',
  IMPORT_INVALID_CLOUD_IMPORT_OPTIONS: 'IMPORT_INVALID_CLOUD_OPTIONS',
  IMPORT_OPERATION_MISMATCH: 'IMPORT_OPERATION_MISMATCH',
  IMPORT_OPERATION_NOT_FOUND: 'IMPORT_OPERATION_NOT_FOUND',
  IMPORT_OPERATION_ABANDONED: 'IMPORT_OPERATION_ABANDONED',
  IMPORT_OPERATION_CONFLICT: 'IMPORT_OPERATION_CONFLICT',
};
export function readLegacyTransferError(value: unknown): BackendError | null {
  const raw = value instanceof Error ? value.message : typeof value === 'string' ? value : null;
  if (raw === null) return null;
  if (Object.hasOwn(CODES, raw)) return makeBackendError(raw as BackendErrorCode);
  const token = /^__(EXPORT|IMPORT)_ERR__:([A-Z_]+)(?::|$)/.exec(raw);
  if (token) {
    const key = token[1] + '_' + token[2];
    return makeBackendError(
      Object.hasOwn(LEGACY_TRANSFER_CODES, key)
        ? LEGACY_TRANSFER_CODES[key]
        : token[1] === 'IMPORT'
          ? 'IMPORT_FAILED'
          : 'EXPORT_FAILED',
    );
  }
  if (/^Backup '[\s\S]*' not found$/.test(raw)) return makeBackendError('BACKUP_NOT_FOUND');
  if (raw === 'Backup name cannot be empty') return makeBackendError('BACKUP_INVALID_NAME');
  return null;
}
export function normalizeTransferError(value: unknown): BackendError {
  return (
    readBackendError(value) ??
    readLegacyObjectError(value) ??
    readLegacyTransferError(value) ??
    makeBackendError('INTERNAL_ERROR')
  );
}

/** RF319：机器错误在 Store 保留代码；配对元数据只为当前确认流程提供。 */
export const SYNC_ERROR_COMMANDS = [
  'mdns_discover',
  'recovery_discover_hosts',
  'recovery_host_cancel',
  'recovery_host_start',
  'recovery_restore_existing_from_host',
  'recovery_restore_from_host',
  'sync_enable',
  'sync_forget_peer',
  'sync_generate_qr_payload',
  'sync_get_auto_status',
  'sync_get_conflict_detail',
  'sync_get_status',
  'sync_get_ui_prefs_sync',
  'sync_list_conflicts',
  'sync_listen_addr',
  'sync_rename_peer',
  'sync_resolve_conflict',
  'sync_set_auto_enabled',
  'sync_set_ui_prefs_sync',
  'sync_trigger_foreground',
  'sync_trust_peer',
  'sync_with_device',
  'vault_sync_background',
  'vault_sync_from_remote',
  'vault_sync_to_remote',
] as const satisfies ReadonlyArray<keyof IpcCommandErrors>;
export function isSyncErrorCommand(command: string): boolean {
  return (SYNC_ERROR_COMMANDS as readonly string[]).includes(command);
}
const LEGACY_SYNC_CODES: Record<string, BackendErrorCode> = {
  not_enabled: 'SYNC_NOT_ENABLED',
  not_running: 'SYNC_NOT_RUNNING',
  invalid_address: 'SYNC_INVALID_ADDRESS',
  peer_not_found: 'SYNC_PEER_NOT_FOUND',
  connect_failed: 'SYNC_CONNECT_FAILED',
  handshake_failed: 'SYNC_HANDSHAKE_FAILED',
  session_failed: 'SYNC_SESSION_FAILED',
  enable_timeout: 'SYNC_ENABLE_TIMEOUT',
  discovery_timeout: 'SYNC_DISCOVERY_TIMEOUT',
  nsd_failed: 'SYNC_DISCOVERY_FAILED',
  recovery_invalid: 'SYNC_RECOVERY_INVALID',
  permission_denied: 'SYNC_PERMISSION_DENIED',
  unsupported: 'SYNC_UNSUPPORTED',
};
export function readLegacySyncError(value: unknown): BackendError | null {
  const raw = value instanceof Error ? value.message : typeof value === 'string' ? value : null;
  if (raw === null) return null;
  if (Object.hasOwn(CODES, raw)) return makeBackendError(raw as BackendErrorCode);
  const token = /^__SYNC_ERR__:([a-z_]+)(?::([\s\S]*))?$/.exec(raw);
  if (!token) return null;
  if (token[1] === 'pairing_pending') {
    const pair = /^([A-Za-z0-9_.-]{1,128})(?::(\d{6}))?$/.exec(token[2] ?? '');
    if (!pair) return makeBackendError('SYNC_PAIRING_INVALID');
    return {
      code: 'SYNC_PAIRING_PENDING',
      safeDetails: {
        stage: 'pairing',
        syncPeerId: pair[1],
        ...(pair[2] ? { sasCode: pair[2] } : {}),
      },
      retryable: false,
    };
  }
  return makeBackendError(
    Object.hasOwn(LEGACY_SYNC_CODES, token[1])
      ? LEGACY_SYNC_CODES[token[1]]
      : 'SYNC_SESSION_FAILED',
  );
}
export function normalizeSyncError(
  value: unknown,
  fallback: BackendErrorCode = 'SYNC_SESSION_FAILED',
): BackendError {
  return (
    readBackendError(value) ??
    readLegacyObjectError(value) ??
    readLegacySyncError(value) ??
    makeBackendError(fallback)
  );
}

/** 旧 Host 的非机器正文按命令阶段降级，正文不参与分类。 */
export function normalizeSyncCommandError(command: string, value: unknown): BackendError {
  let fallback: BackendErrorCode = 'SYNC_SESSION_FAILED';
  if (command === 'sync_enable') fallback = 'SYNC_ENABLE_FAILED';
  else if (command === 'mdns_discover' || command === 'recovery_discover_hosts')
    fallback = 'SYNC_DISCOVERY_FAILED';
  else if (
    command === 'sync_trigger_foreground' ||
    command === 'recovery_host_cancel' ||
    command === 'vault_sync_background'
  )
    fallback = 'SYNC_TASK_UNCONFIRMED';
  else if (command.startsWith('recovery_')) fallback = 'SYNC_RECOVERY_FAILED';
  else if (command === 'sync_resolve_conflict') fallback = 'SYNC_CONFLICT_FAILED';
  else if (command === 'sync_get_conflict_detail') fallback = 'SYNC_CONFLICT_INVALID';
  else if (
    command.startsWith('sync_get_') ||
    command === 'sync_list_conflicts' ||
    command === 'sync_listen_addr' ||
    command === 'sync_generate_qr_payload'
  )
    fallback = 'SYNC_READ_FAILED';
  else if (command !== 'sync_with_device') fallback = 'SYNC_WRITE_FAILED';
  return normalizeSyncError(value, fallback);
}

export const PLUGIN_ERROR_COMMANDS = [
  'plugin_list_all',
  'plugin_list_installed',
  'plugin_list_attachments',
  'create_plugin_install',
  'plugin_install',
  'plugin_update',
  'plugin_uninstall',
  'plugin_run',
  'plugin_consent_response',
  'plugin_dialog_response',
  'plugin_list_sessions',
  'plugin_audit_log',
  'plugin_update_registry',
  'plugin_open_output_file',
  'plugin_copy_output_file',
] as const satisfies ReadonlyArray<keyof IpcCommandErrors>;

export function isPluginErrorCommand(command: string): boolean {
  return (PLUGIN_ERROR_COMMANDS as readonly string[]).includes(command);
}
const LEGACY_PLUGIN_CODES: Record<string, BackendErrorCode> = {
  PLUGIN_INSTALL_CANCELLED: 'PLUGIN_INSTALL_CANCELLED',
  安装任务已启动: 'PLUGIN_INSTALL_ALREADY_STARTED',
  用户拒绝授权: 'PLUGIN_CONSENT_DENIED',
  当前平台暂不支持: 'PLUGIN_UNSUPPORTED',
  非法文件名: 'PLUGIN_OUTPUT_INVALID',
  目标目录不存在: 'PLUGIN_OUTPUT_INVALID',
  输出目录不存在: 'PLUGIN_OUTPUT_INVALID',
  输出文件不存在: 'PLUGIN_OUTPUT_INVALID',
  '输出文件位于插件输出目录之外，已拒绝访问': 'PLUGIN_OUTPUT_DENIED',
  '插件执行失败: 插件会话已失效：Vault 已锁定或授权已过期': 'PLUGIN_SESSION_EXPIRED',
  '插件执行失败: 插件与会话不匹配': 'PLUGIN_SESSION_EXPIRED',
  '插件执行失败: Vault 未解锁': 'VAULT_LOCKED',
  '插件执行失败: 未选择账户': 'VAULT_LOCKED',
};
export function normalizePluginError(
  value: unknown,
  fallback: BackendErrorCode = 'PLUGIN_EXECUTION_FAILED',
): BackendError {
  const packet = readBackendError(value) ?? readLegacyObjectError(value);
  if (packet) return packet;
  if (value instanceof DOMException && value.name === 'AbortError')
    return makeBackendError('PLUGIN_INSTALL_CANCELLED');
  const raw = value instanceof Error ? value.message : typeof value === 'string' ? value : null;
  if (raw !== null) {
    if (Object.hasOwn(CODES, raw)) return makeBackendError(raw as BackendErrorCode);
    if (Object.hasOwn(LEGACY_PLUGIN_CODES, raw)) return makeBackendError(LEGACY_PLUGIN_CODES[raw]);
    // 仅集中读取旧 Core 的固定变体前缀；detail 一律丢弃。
    const old: [string, BackendErrorCode][] = [
      ['插件未找到:', 'PLUGIN_NOT_FOUND'],
      ['无效的插件 manifest:', 'PLUGIN_MANIFEST_INVALID'],
      ['Wasm 文件过大:', 'PLUGIN_WASM_TOO_LARGE'],
      ['Wasm SHA-256 校验和不匹配', 'PLUGIN_CHECKSUM_MISMATCH'],
      ['插件版本不兼容:', 'PLUGIN_VERSION_INCOMPATIBLE'],
      ['非法字段:', 'PLUGIN_INVALID_FIELD'],
      ['非法参数:', 'PLUGIN_INVALID_ARGUMENT'],
      ['频率超限', 'PLUGIN_RATE_LIMITED'],
      ['插件存储错误:', 'PLUGIN_STORE_FAILED'],
      ['插件注册表错误:', 'PLUGIN_REGISTRY_FAILED'],
      ['网络错误:', 'PLUGIN_NETWORK_FAILED'],
    ];
    const mapped = old.find(([prefix]) => raw.startsWith(prefix));
    if (mapped) return makeBackendError(mapped[1]);
  }
  return makeBackendError(fallback);
}
export function normalizePluginCommandError(command: string, value: unknown): BackendError {
  let fallback: BackendErrorCode = 'PLUGIN_READ_FAILED';
  if (['plugin_install', 'plugin_update', 'create_plugin_install'].includes(command))
    fallback = 'PLUGIN_INSTALL_FAILED';
  else if (command === 'plugin_run') fallback = 'PLUGIN_EXECUTION_FAILED';
  else if (command === 'plugin_open_output_file') fallback = 'PLUGIN_OUTPUT_OPEN_FAILED';
  else if (command === 'plugin_copy_output_file') fallback = 'PLUGIN_OUTPUT_WRITE_FAILED';
  else if (command === 'plugin_uninstall') fallback = 'PLUGIN_STORE_FAILED';
  else if (command === 'plugin_update_registry') fallback = 'PLUGIN_REGISTRY_FAILED';
  else if (['plugin_consent_response', 'plugin_dialog_response'].includes(command))
    fallback = 'PLUGIN_INVALID_ARGUMENT';
  return normalizePluginError(value, fallback);
}
export function isPluginCancellation(value: unknown): boolean {
  return normalizePluginError(value).code === 'PLUGIN_INSTALL_CANCELLED';
}
export function readPluginEventError(jsonData: string): BackendError {
  try {
    const parsed: unknown = JSON.parse(jsonData);
    if (record(parsed)) {
      const code = own(parsed, 'code');
      if (
        typeof code === 'string' &&
        (code.startsWith('PLUGIN_') ||
          ['VAULT_LOCKED', 'VAULT_BUSY', 'SESSION_EXPIRED'].includes(code)) &&
        Object.hasOwn(CODES, code)
      )
        return makeBackendError(code as BackendErrorCode);
      return normalizePluginError(own(parsed, 'message'));
    }
  } catch {
    /* 兼容旧非 JSON 的错误通道。 */
  }
  return normalizePluginError(jsonData);
}
