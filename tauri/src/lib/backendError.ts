import i18n from './i18n';
import { resolveI18nPrefix } from './utils';

// ─── P029: 两套后端错误库合并——Rust 静态错误映射（原 rustErrors.ts）与
// 前缀 token 解析（原 backendError.ts）统一在本模块。单一入口：
// ① resolveBackendErrorMessage：先查前缀 token，未命中回退 Rust 精确/前缀映射；
// ② translateRustError：直接返回 i18n key（供需 key 判断的调用方，如 BootstrapPage）。

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
  const key = RUST_ERROR_MAP[msg];
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
