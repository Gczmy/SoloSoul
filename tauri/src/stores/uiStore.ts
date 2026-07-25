import { create } from 'zustand';

interface Toast {
  id: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  timeoutId?: ReturnType<typeof setTimeout>;
  /** 可选的操作按钮，点击后执行回调并关闭 toast */
  action?: {
    label: string;
    onClick: () => void;
  };
}

export type SafSyncPhase = 'idle' | 'syncing' | 'complete' | 'error';

export interface SafSyncProgress {
  current: number;
  total: number;
}

interface UiState {
  sidebarCollapsed: boolean;
  toasts: Toast[];
  globalLoading: boolean;
  /** SAF 自动同步当前状态（仅 Android SAF 目录模式下有意义）。 */
  safSyncState: SafSyncPhase;
  /** SAF 自动同步进度（current/total）。 */
  safSyncProgress: SafSyncProgress;
  /** SAF 自动同步最近一次错误信息。 */
  safSyncError: string | null;
  /** SAF 授权是否已被撤销。 */
  safAuthRevoked: boolean;

  toggleSidebar: () => void;
  showToast: (toast: Omit<Toast, 'id'>) => void;
  dismissToast: (id: string) => void;
  setGlobalLoading: (loading: boolean) => void;
  setSafSyncState: (state: SafSyncPhase) => void;
  setSafSyncProgress: (progress: SafSyncProgress) => void;
  setSafSyncError: (error: string | null) => void;
  setSafAuthRevoked: (revoked: boolean) => void;
}

let toastCounter = 0;

export const useUiStore = create<UiState>((set) => ({
  sidebarCollapsed: false,
  toasts: [],
  globalLoading: false,
  safSyncState: 'idle',
  safSyncProgress: { current: 0, total: 0 },
  safSyncError: null,
  safAuthRevoked: false,

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),

  showToast: (toast) => {
    const id = `toast-${++toastCounter}`;
    const duration = toast.duration ?? 3000;
    const timeoutId = setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, duration);
    set((s) => ({ toasts: [...s.toasts, { ...toast, id, timeoutId }] }));
  },

  dismissToast: (id) =>
    set((s) => {
      const toast = s.toasts.find((t) => t.id === id);
      if (toast?.timeoutId) {
        clearTimeout(toast.timeoutId);
      }
      return { toasts: s.toasts.filter((t) => t.id !== id) };
    }),

  setGlobalLoading: (loading) => set({ globalLoading: loading }),
  setSafSyncState: (state) => set({ safSyncState: state }),
  setSafSyncProgress: (progress) => set({ safSyncProgress: progress }),
  setSafSyncError: (error) => set({ safSyncError: error }),
  setSafAuthRevoked: (revoked) => set({ safAuthRevoked: revoked }),
}));
