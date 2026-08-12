import { create } from 'zustand';

export interface Toast {
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
  toasts: Toast[];
  /** SAF 自动同步当前状态（仅 Android SAF 目录模式下有意义）。 */
  safSyncState: SafSyncPhase;
  /** SAF 自动同步进度（current/total）。 */
  safSyncProgress: SafSyncProgress;
  /** SAF 自动同步最近一次错误信息。 */
  safSyncError: string | null;
  /** SAF 授权是否已被撤销。 */
  safAuthRevoked: boolean;
  /**
   * 「SAF 授权已失效」toast 是否已在本会话弹出过（仅 AppRoutes 监听器读写，
   * 用于去重：auto-sync 周期性重试会反复发射 saf-auth-revoked 事件）。
   * 独立于 safAuthRevoked——后者也被 GlobalSyncIndicator 置位，
   * 若共用会因监听器注册顺序（GlobalSyncIndicator 先注册）吞掉首次 toast。
   */
  safAuthToastShown: boolean;
  /** 从「创建新账户」页返回时重新打开 onboarding 账户来源决策卡片的标志。 */
  reopenAccountSource: boolean;

  showToast: (toast: Omit<Toast, 'id'>) => void;
  dismissToast: (id: string) => void;
  setSafSyncState: (state: SafSyncPhase) => void;
  setSafSyncProgress: (progress: SafSyncProgress) => void;
  setSafSyncError: (error: string | null) => void;
  setSafAuthRevoked: (revoked: boolean) => void;
  setSafAuthToastShown: (shown: boolean) => void;
  setReopenAccountSource: (reopen: boolean) => void;
}

let toastCounter = 0;

export const useUiStore = create<UiState>((set) => ({
  toasts: [],
  safSyncState: 'idle',
  safSyncProgress: { current: 0, total: 0 },
  safSyncError: null,
  safAuthRevoked: false,
  safAuthToastShown: false,
  reopenAccountSource: false,

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

  setSafSyncState: (state) => set({ safSyncState: state }),
  setSafSyncProgress: (progress) => set({ safSyncProgress: progress }),
  setSafSyncError: (error) => set({ safSyncError: error }),
  setSafAuthRevoked: (revoked) => set({ safAuthRevoked: revoked }),
  setSafAuthToastShown: (shown) => set({ safAuthToastShown: shown }),
  setReopenAccountSource: (reopen) => set({ reopenAccountSource: reopen }),
}));
