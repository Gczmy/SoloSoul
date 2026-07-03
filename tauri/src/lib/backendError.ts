import i18n from './i18n';
import { resolveI18nPrefix } from './utils';

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

  return i18n.t(key, {
    ns,
    ...(parsed.payload ? { detail: parsed.payload } : {}),
  });
}
