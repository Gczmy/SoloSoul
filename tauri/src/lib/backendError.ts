import i18n from './i18n';
import { tryParsePrefixedError } from './utils';

const EXPORT_PREFIX = '__EXPORT_ERR__:';
const IMPORT_PREFIX = '__IMPORT_ERR__:';

type ErrorKind = 'export' | 'import';

interface ParsedBackendError {
  kind: ErrorKind;
  code: string;
  payload: string | null;
}

function tryParse(message: string): ParsedBackendError | null {
  for (const [prefix, kind] of [
    [EXPORT_PREFIX, 'export'],
    [IMPORT_PREFIX, 'import'],
  ] as const) {
    const rest = tryParsePrefixedError(message, prefix);
    if (rest !== null) {
      const sep = rest.indexOf(':');
      const code = sep >= 0 ? rest.slice(0, sep) : rest;
      const payload = sep >= 0 ? rest.slice(sep + 1) : null;
      return { kind, code, payload };
    }
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
  const parsed = tryParse(raw);
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
