import type { IpcEvents } from '@/lib/generated/ipcContracts';
import { useEffect } from 'react';
import type { NavigateFunction } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { trackAsyncListener } from '@/lib/asyncListener';
import { observeNativeWindowLayout, refreshNativeAppearance } from '@/lib/nativeWindow';
import { logger } from '@/lib/logger';
import { useSafSyncStore } from '@/stores/safSyncStore';
import { useUiStore } from '@/stores/uiStore';

type ShortcutWindow = typeof window & {
  __SOLOSOUL_HANDLE_SHORTCUT__?: (action: string) => void;
};

export function useNativeAppEvents({
  navigate,
  isAuthenticated,
}: {
  navigate: NavigateFunction;
  isAuthenticated: boolean;
}) {
  const { t } = useTranslation(['settings']);

  useEffect(observeNativeWindowLayout, []);

  useEffect(
    () =>
      trackAsyncListener(
        listen('native-appearance-changed', () => {
          void refreshNativeAppearance();
        }),
      ),
    [],
  );

  useEffect(() => {
    if (!isAuthenticated) return;
    useSafSyncStore.getState().startListening();
    return () => useSafSyncStore.getState().stopListening();
  }, [isAuthenticated]);

  useEffect(() => {
    if (!isAuthenticated) return;
    let active = true;
    const dispose = trackAsyncListener(
      listen<IpcEvents['saf-auth-revoked']>('saf-auth-revoked', () => {
        if (!active) return;
        // auto-sync 周期重试会重复发出此事件；专用标志保证一会话只弹一次。
        logger.warn('[AppRoutes] SAF auth revoked event received');
        const ui = useUiStore.getState();
        if (ui.safAuthToastShown) return;
        ui.setSafAuthToastShown(true);
        ui.setSafAuthRevoked(true);
        ui.showToast({
          type: 'warning',
          message: t(
            'settings:vault_directory_invalid_toast',
            'SAF directory access revoked. Go to Settings > Data Management to re-select.',
          ),
          duration: 10000,
        });
      }),
    );
    return () => {
      active = false;
      dispose();
    };
  }, [isAuthenticated, t]);

  useEffect(() => {
    const handleShortcut = () => {
      if (isAuthenticated) navigate('/editor?new=1');
      else sessionStorage.setItem('solosoul_pending_shortcut', 'new_object');
    };

    const pending = sessionStorage.getItem('solosoul_pending_shortcut');
    if (pending === 'new_object' && isAuthenticated) {
      sessionStorage.removeItem('solosoul_pending_shortcut');
      navigate('/editor?new=1');
    }

    (window as ShortcutWindow).__SOLOSOUL_HANDLE_SHORTCUT__ = (action: string) => {
      if (action === 'new_object') handleShortcut();
    };

    return () => {
      delete (window as ShortcutWindow).__SOLOSOUL_HANDLE_SHORTCUT__;
    };
  }, [navigate, isAuthenticated]);
}
