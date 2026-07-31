import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ST_OCR_FIRST_INSTALL } from '@/lib/constants';
import { logger } from '@/lib/logger';

export interface OcrInstallProgressPayload {
  tier: string;
  progress: number;
  done: boolean;
  error?: string;
}

interface OcrInstallState {
  isInstalling: boolean;
  progress: number;
  error: string | null;
  unlisten: UnlistenFn | null;
  unlistenPromise: Promise<UnlistenFn> | null;

  startListening: () => void;
  stopListening: () => void;
  onProgress: (payload: OcrInstallProgressPayload) => void;
  reset: () => void;
}

export function isOcrFirstInstallDone(): boolean {
  try {
    return localStorage.getItem(ST_OCR_FIRST_INSTALL) === 'true';
  } catch {
    return false;
  }
}

export function markOcrFirstInstallDone() {
  try {
    localStorage.setItem(ST_OCR_FIRST_INSTALL, 'true');
  } catch {
    /* ignore */
  }
}

export const useOcrInstallStore = create<OcrInstallState>((set, get) => ({
  isInstalling: false,
  progress: 0,
  error: null,
  unlisten: null,
  unlistenPromise: null,

  startListening: () => {
    const state = get();
    state.stopListening();

    set({
      isInstalling: true,
      progress: 0,
      error: null,
      unlisten: null,
      unlistenPromise: null,
    });

    const pending = listen<OcrInstallProgressPayload>('ocr-install-progress', (event) => {
      get().onProgress(event.payload);
    });
    set({ unlistenPromise: pending });
    pending
      .then((unlistenFn) => {
        set({ unlisten: unlistenFn, unlistenPromise: null });
      })
      .catch((err) => {
        set({ error: String(err), isInstalling: false, unlistenPromise: null });
      });
  },

  stopListening: () => {
    const state = get();
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise
        .then((fn) => fn())
        .catch((err) => logger.warn('[ocrInstallStore] Failed to clean up listener:', err));
    }
  },

  onProgress: (payload) => {
    if (payload.error) {
      set({ error: payload.error, isInstalling: false });
      get().stopListening();
      return;
    }

    set({ progress: payload.progress });

    if (payload.done) {
      set({ isInstalling: false });
      markOcrFirstInstallDone();
      get().stopListening();
    }
  },

  reset: () => {
    get().stopListening();
    set({ isInstalling: false, progress: 0, error: null, unlisten: null, unlistenPromise: null });
  },
}));
