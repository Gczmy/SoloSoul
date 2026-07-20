/**
 * Maps common Rust backend error strings to i18n keys so that
 * user-facing error toasts show localized text instead of raw English.
 *
 * Only exact‑match static strings are mapped. Dynamic errors
 * (e.g. "HTTP 404: …", "Invalid attachment id: xxx") fall back
 * to the original English.
 */

const RUST_ERROR_MAP: Record<string, string> = {
  // Auth / Vault
  'Invalid password': 'common:invalid_password',
  'Verify failed': 'common:verify_failed',
  'Account name is required': 'common:account_name_required',
  'Account name already taken': 'common:account_name_taken',
  'Account not found': 'common:account_not_found',
  'Password must be at least 8 characters': 'common:password_too_short',

  // Backup
  'Backup name cannot be empty': 'common:backup_name_empty',

  // Attachments
  'No file path available': 'common:no_file_path',
  'Source path must not be inside vault storage': 'common:path_inside_vault',
  'Destination path must not contain \'..\'': 'common:path_traversal',
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

/**
 * Translate a Rust error message to its localized form.
 * Returns `null` when no mapping exists (caller should use the raw message).
 */
export function translateRustError(msg: string): string | null {
  // Try exact match first
  const key = RUST_ERROR_MAP[msg];
  if (key) return key;

  // Prefix‑based matching for errors with dynamic suffixes
  for (const [prefix, mappedKey] of Object.entries(PREFIX_MAP)) {
    if (msg.startsWith(prefix)) return mappedKey;
  }

  return null;
}

// Prefix‑based matches for errors with dynamic content (IDs, paths, etc.)
const PREFIX_MAP: Record<string, string> = {
  'Invalid attachment id: ': 'common:invalid_attachment_id',
  'Invalid addr: ': 'common:invalid_addr',
  'File too large': 'common:file_too_large',
};
