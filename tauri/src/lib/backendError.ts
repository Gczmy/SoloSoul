import i18n from './i18n';
import { resolveI18nPrefix } from './utils';

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
  if (!parsed) return raw;

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
