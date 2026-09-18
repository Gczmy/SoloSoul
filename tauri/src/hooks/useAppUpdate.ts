import { useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useUpdateStore } from '@/stores/updateStore';
export type { AppUpdateState } from '@/stores/updateStore';

export function useAppUpdate() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const store = useUpdateStore();
  useEffect(() => {
    void useUpdateStore.getState().check();
  }, [isAuthenticated]);
  useEffect(() => {
    const retry = () => {
      const state = useUpdateStore.getState();
      if (!document.hidden && Date.now() - state.lastChecked > 60_000) void state.check();
    };
    window.addEventListener('online', retry);
    document.addEventListener('visibilitychange', retry);
    const timer = window.setInterval(retry, 5 * 60_000);
    return () => {
      window.removeEventListener('online', retry);
      document.removeEventListener('visibilitychange', retry);
      window.clearInterval(timer);
    };
  }, []);
  return {
    updateState: store.updateState,
    startDownload: store.startDownload,
    cancelDownload: store.cancelDownload,
    installUpdate: store.installUpdate,
    dismissUpdate: store.dismissUpdate,
  };
}
