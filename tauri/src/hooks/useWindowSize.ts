import { useEffect, useRef } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { PhysicalSize } from '@tauri-apps/api/dpi';
import { invoke } from '@tauri-apps/api/core';

const WINDOW_SIZE_KEY = 'windowSize';
const DEBOUNCE_MS = 500;

/**
 * Restore window size from Vault-encrypted preferences after login.
 */
export async function restoreWindowSize(accountId: string) {
  try {
    const prefs = await invoke<Record<string, unknown>>('user_data_get_preferences', { accountId });
    const raw = prefs[WINDOW_SIZE_KEY] as { width?: number; height?: number } | undefined;
    if (raw?.width && raw?.height) {
      const window = getCurrentWebviewWindow();
      await window.setSize(new PhysicalSize({ width: raw.width, height: raw.height }));
    }
  } catch {
    // Silently ignore — window size is not critical
  }
}

/**
 * Listen to window resize events and save size to Vault-encrypted preferences.
 * Should only be called when the user is authenticated (Vault unlocked).
 */
export function useWindowSize(accountId: string | undefined) {
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!accountId) return;

    const window = getCurrentWebviewWindow();

    const unlistenPromise = window.onResized(({ payload: size }) => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => {
        invoke('user_data_update_preference', {
          payload: {
            accountId,
            preferences: {
              [WINDOW_SIZE_KEY]: { width: size.width, height: size.height },
            },
          },
        }).catch(() => {});
      }, DEBOUNCE_MS);
    });

    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [accountId]);
}
