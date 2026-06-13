import { useCallback, useEffect, useRef } from 'react';

/**
 * Returns a stable function that tells whether the current component/effect
 * has been cancelled (e.g. because it unmounted or a newer request started).
 *
 * Typical usage inside an async effect:
 *
 *   useEffect(() => {
 *     const { isCancelled, cancel } = makeCancellable();
 *     invoke('load').then((data) => {
 *       if (!isCancelled()) setState(data);
 *     });
 *     return cancel;
 *   }, [deps]);
 */
export function useCancellable() {
  const cancelRef = useRef<(() => void) | null>(null);

  const makeCancellable = useCallback(() => {
    // Cancel any previous run created by this hook instance.
    cancelRef.current?.();

    let cancelled = false;
    const isCancelled = () => cancelled;
    const cancel = () => {
      cancelled = true;
    };
    cancelRef.current = cancel;
    return { isCancelled, cancel };
  }, []);

  useEffect(() => {
    return () => {
      cancelRef.current?.();
      cancelRef.current = null;
    };
  }, []);

  return makeCancellable;
}
