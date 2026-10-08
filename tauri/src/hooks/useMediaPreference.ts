import { useCallback, useSyncExternalStore } from 'react';

/** 共用系统媒体偏好；订阅变化，避免只在首次渲染读取。 */
export function useMediaPreference(query: string) {
  const subscribe = useCallback(
    (notify: () => void) => {
      const media = window.matchMedia?.(query);
      media?.addEventListener('change', notify);
      return () => media?.removeEventListener('change', notify);
    },
    [query],
  );
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia?.(query).matches ?? false,
    () => false,
  );
}
