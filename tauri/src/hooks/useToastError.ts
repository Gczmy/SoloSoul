import { useCallback } from 'react';
import { useUiStore } from '@/stores/uiStore';

/**
 * Returns a helper that shows an error toast for a given operation.
 * Use in try/catch blocks:
 *
 *   const { onError } = useToastError();
 *   try { await riskyOp(); } catch (e) { onError(e, 'Failed to save'); }
 */
export function useToastError() {
  const showToast = useUiStore((s) => s.showToast);

  const onError = useCallback(
    (err: unknown, context: string) => {
      const message = err instanceof Error ? err.message : String(err);
      showToast({ type: 'error', message: `${context}: ${message}`, duration: 5000 });
    },
    [showToast],
  );

  const onSuccess = useCallback(
    (message: string) => {
      showToast({ type: 'success', message, duration: 3000 });
    },
    [showToast],
  );

  return { onError, onSuccess };
}
