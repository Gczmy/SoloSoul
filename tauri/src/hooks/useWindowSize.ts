import { useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { PhysicalSize } from '@tauri-apps/api/dpi';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';

const WINDOW_SIZE_KEY = 'windowSize';
const WINDOW_SIZE_CACHE_KEY = 'solosoul_window_size';
const DEBOUNCE_MS = 200;

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
  } catch {
    /* ignore */
  }
  return null;
}

function writeCachedSize(size: WindowSize) {
  try {
    localStorage.setItem(WINDOW_SIZE_CACHE_KEY, JSON.stringify(size));
  } catch {
    /* ignore */
  }
}

async function persistWindowSize(payload: WindowSize) {
  writeCachedSize(payload);
  await invoke('ui_update_preference', {
    key: WINDOW_SIZE_KEY,
    value: JSON.stringify(payload),
  }).catch((err) => console.warn('[useWindowSize] Update UI pref failed:', err));
  // Mirror to encrypted account preferences so each account can retain its own
  // window geometry while keeping a plaintext fallback for the login window.
  const accountId = useAuthStore.getState().currentAccount?.id;
  if (accountId) {
    await invoke('user_data_update_preference', {
      payload: { accountId, preferences: { windowSize: payload } },
    }).catch((err) => console.warn('[useWindowSize] Update account pref failed:', err));
  }
}

/**
 * Restore window size from plaintext UI preferences before login.
 */
export async function restoreWindowSize() {
  try {
    // The localStorage cache is updated synchronously on every resize, so it is
    // the freshest source of truth for the last known window geometry.
    const cached = readCachedSize();
    if (cached) {
      const window = getCurrentWindow();
      await window.setSize(new PhysicalSize({ width: cached.width, height: cached.height }));
      return;
    }

    // Fallback to the persisted UI preference only when the cache is missing
    // (e.g. first launch or after clearing app data).
    const prefs = await invoke<{ windowSize?: WindowSize }>('ui_get_preferences');
    const raw = prefs.windowSize;
    if (raw?.width && raw?.height) {
      writeCachedSize(raw);
      const window = getCurrentWindow();
      await window.setSize(new PhysicalSize({ width: raw.width, height: raw.height }));
    }
  } catch {
    // Silently ignore — window size is not critical
  }
}

/**
 * Listen to window resize events and save size.
 * Can be called unconditionally; it does not depend on Vault unlock state.
 *
 * Strategy:
 * - localStorage is updated synchronously on every resize so the next cold
 *   launch can restore immediately even if the app closes before IPC finishes.
 * - Plaintext UI prefs are written with a short debounce for disk I/O so the
 *   login window can be sized before the Vault is unlocked.
 * - When an account is logged in, the size is also mirrored to the encrypted
 *   account preferences so each account can retain its own window geometry.
 */
export function useWindowSize() {
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const pendingRef = useRef<WindowSize | null>(null);
  const lastSizeRef = useRef<WindowSize | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlistenCleanup: (() => void) | undefined;
    let onVisibility: (() => void) | undefined;

    const flush = () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = undefined;
      }
      if (pendingRef.current) {
        persistWindowSize(pendingRef.current);
        pendingRef.current = null;
      }
    };

    const setup = async () => {
      // Restore the saved size *before* registering the resize listener.
      // Otherwise the listener may fire with the default window size and
      // overwrite the saved geometry before we have a chance to apply it.
      await restoreWindowSize();
      if (!mounted) return;

      const window = getCurrentWindow();

      // Seed the last known size from the cache so a startup resize event that
      // matches the saved size is treated as a no-op.
      lastSizeRef.current = readCachedSize();

      const unlisten = await window.onResized(({ payload: size }) => {
        const payload = { width: size.width, height: size.height };
        const last = lastSizeRef.current;
        if (last && last.width === payload.width && last.height === payload.height) return;
        lastSizeRef.current = payload;
        pendingRef.current = payload;

        // Always cache locally first so recovery is reliable on abrupt exit.
        writeCachedSize(payload);

        if (timeoutRef.current) clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => {
          if (pendingRef.current) {
            persistWindowSize(pendingRef.current);
            pendingRef.current = null;
          }
        }, DEBOUNCE_MS);
      });

      if (!mounted) {
        unlisten();
        return;
      }
      unlistenCleanup = unlisten;

      // Flush pending size before the page hides or unloads. We intentionally do
      // NOT use Tauri's onCloseRequested listener because registering it changes
      // the window's close behavior and can break the traffic-light close button
      // on macOS. localStorage is already written synchronously on every resize,
      // so the next cold launch is covered even if the debounced disk write misses.
      globalThis.window?.addEventListener('beforeunload', flush);
      onVisibility = () => {
        if (document.hidden) flush();
      };
      document.addEventListener('visibilitychange', onVisibility);
    };

    setup();

    return () => {
      mounted = false;
      unlistenCleanup?.();
      globalThis.window?.removeEventListener('beforeunload', flush);
      if (onVisibility) document.removeEventListener('visibilitychange', onVisibility);
      flush();
    };
  }, []);
}
