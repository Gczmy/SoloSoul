import { useEffect, useRef } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { PhysicalSize } from '@tauri-apps/api/dpi';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';

const WINDOW_SIZE_KEY = 'windowSize';
const WINDOW_SIZE_CACHE_KEY = 'solosoul_window_size';
const DEBOUNCE_MS = 500;

interface WindowSize {
  width: number;
  height: number;
}

function readCachedSize(): WindowSize | null {
  try {
    const raw = localStorage.getItem(WINDOW_SIZE_CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as WindowSize;
    if (parsed.width && parsed.height) return parsed;
  } catch { /* ignore */ }
  return null;
}

function writeCachedSize(size: WindowSize) {
  try {
    localStorage.setItem(WINDOW_SIZE_CACHE_KEY, JSON.stringify(size));
  } catch { /* ignore */ }
}

/**
 * Restore window size from plaintext UI preferences before login.
 */
export async function restoreWindowSize() {
  try {
    // 1. Apply cached localStorage value instantly (no IPC delay on startup).
    const cached = readCachedSize();
    if (cached) {
      const window = getCurrentWebviewWindow();
      await window.setSize(new PhysicalSize({ width: cached.width, height: cached.height }));
    }

    // 2. Reconcile with the persisted UI preference.
    const prefs = await invoke<{ windowSize?: WindowSize }>('ui_get_preferences');
    const raw = prefs.windowSize;
    if (raw?.width && raw?.height) {
      writeCachedSize(raw);
      const window = getCurrentWebviewWindow();
      await window.setSize(new PhysicalSize({ width: raw.width, height: raw.height }));
    }
  } catch {
    // Silently ignore — window size is not critical
  }
}

/**
 * Listen to window resize events and save size to plaintext UI preferences.
 * Can be called unconditionally; it does not depend on Vault unlock state.
 */
export function useWindowSize() {
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const lastSizeRef = useRef<WindowSize | null>(null);

  useEffect(() => {
    const window = getCurrentWebviewWindow();

    const unlistenPromise = window.onResized(({ payload: size }) => {
      const payload = { width: size.width, height: size.height };
      const last = lastSizeRef.current;
      if (last && last.width === payload.width && last.height === payload.height) return;
      lastSizeRef.current = payload;

      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => {
        writeCachedSize(payload);
        invoke('ui_update_preference', {
          key: WINDOW_SIZE_KEY,
          value: JSON.stringify(payload),
        }).catch(() => {});
        const accountId = useAuthStore.getState().currentAccount?.id;
        if (accountId) {
          invoke('user_data_update_preference', {
            payload: { accountId, preferences: { windowSize: payload } },
          }).catch(() => {});
        }
      }, DEBOUNCE_MS);
    });

    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);
}
