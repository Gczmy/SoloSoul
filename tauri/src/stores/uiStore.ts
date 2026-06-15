import { create } from 'zustand';

interface Toast {
  id: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  timeoutId?: ReturnType<typeof setTimeout>;
}

interface UiState {
  sidebarCollapsed: boolean;
  toasts: Toast[];
  globalLoading: boolean;

  toggleSidebar: () => void;
  showToast: (toast: Omit<Toast, 'id'>) => void;
  dismissToast: (id: string) => void;
  setGlobalLoading: (loading: boolean) => void;
}

let toastCounter = 0;

export const useUiStore = create<UiState>((set) => ({
  sidebarCollapsed: false,
  toasts: [],
  globalLoading: false,

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
}));
