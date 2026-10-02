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
  'Vault not unlocked': 'VAULT_LOCKED',
  'Vault is locked': 'VAULT_LOCKED',
  'Request belongs to an expired session': 'SESSION_EXPIRED',
  'Vault session is no longer current': 'SESSION_EXPIRED',
  IMPORT_DIRECTORY_BUSY: 'VAULT_BUSY',
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
      code === 'IMPORT_DECRYPT_FAILED',
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
  return readBackendError(value) ?? readLegacyObjectError(value) ?? { code: 'LEGACY_ERROR' };
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
