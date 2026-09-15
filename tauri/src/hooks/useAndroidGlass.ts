import { useCallback, useLayoutEffect, useSyncExternalStore } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { isAndroidSync } from '@/lib/platform';

export function useMediaPreference(query: string) {
  const subscribe = useCallback(
    (notify: () => void) => {
      const media = window.matchMedia(query);
      media.addEventListener('change', notify);
      return () => media.removeEventListener('change', notify);
    },
    [query],
  );
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(query).matches,
    () => false,
  );
}

export function useAndroidGlassMode() {
  const mode = useSettingsStore((s) => s.settings.androidGlass);
  const forcedColors = useMediaPreference('(forced-colors: active)');
  const reduceTransparency = useMediaPreference('(prefers-reduced-transparency: reduce)');
  return !isAndroidSync() || forcedColors || reduceTransparency ? 'off' : mode;
}

export function useAndroidGlassSurface() {
  const mode = useAndroidGlassMode();
  useLayoutEffect(() => {
    if (!isAndroidSync()) return;
    const root = document.documentElement;
    root.dataset.androidGlass = mode;
    root.dataset.androidBackdrop = String(
      CSS.supports('backdrop-filter', 'blur(1px)') ||
        CSS.supports('-webkit-backdrop-filter', 'blur(1px)'),
    );
  }, [mode]);
}
