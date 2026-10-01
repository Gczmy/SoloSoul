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
      code === 'SNAPSHOT_READ_FAILED',
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
