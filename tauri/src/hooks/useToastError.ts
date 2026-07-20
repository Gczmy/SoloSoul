import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useUiStore } from '@/stores/uiStore';
import { translateRustError } from '@/lib/rustErrors';

/**
 * Returns helpers for showing error / success toasts.
 *
 * `onError(err, context)` shows an error toast. The `err` from Rust is
 * automatically translated via {@link translateRustError} when a mapping
 * exists, so users see localized text instead of raw English.
 *
 *   const { onError, onSuccess } = useToastError();
 *   try { await riskyOp(); } catch (e) { onError(e, t('common:failed')); }
 */
export function useToastError() {
  const showToast = useUiStore((s) => s.showToast);
  const { t } = useTranslation('common');

  const onError = useCallback(
    (err: unknown, context: string) => {
      const raw = err instanceof Error ? err.message : String(err);
      const translated = translateRustError(raw);
      const message = translated ? t(translated) : raw;
      showToast({ type: 'error', message: `${context}: ${message}`, duration: 5000 });
    },
    [showToast, t],
  );

  const onSuccess = useCallback(
    (message: string) => {
      showToast({ type: 'success', message, duration: 3000 });
    },
    [showToast],
  );

  return { onError, onSuccess };
}
