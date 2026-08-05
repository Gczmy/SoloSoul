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

  return i18n.t(key, {
    ns,
    ...(detail ? { detail } : {}),
  });
}
