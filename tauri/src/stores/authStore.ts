import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { AccountInfo } from '@/lib/ipc';
import { logger } from '@/lib/logger';

interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  currentAccount: AccountInfo | null;
  accounts: AccountInfo[];
  error: string | null;
  hasAccount: boolean | null;
  backendError: boolean;

  checkHasAccount: () => Promise<void>;
  listAccounts: () => Promise<void>;
  refreshCurrentAccount: () => Promise<void>;
  bootstrap: (
    name: string,
    password: string,
    locale: string,
    passwordHint?: string,
  ) => Promise<void>;
  login: (accountId: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  isAuthenticated: false,
  isLoading: false,
  currentAccount: null,
  accounts: [],
  error: null,
  hasAccount: null,
  backendError: false,    checkHasAccount: async () => {
    try {
      const result = await invoke<boolean>('check_has_account');
      set({ hasAccount: result, backendError: false });
    } catch (err) {
      // Backend unavailable — don't jump to bootstrap
      // hasAccount stays null (unknown), BootstrapGuard will wait
      logger.error('[authStore] check_has_account failed:', err);
      set({ backendError: true });
    }
  },

  listAccounts: async () => {
    try {
      const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
      const currentId = get().currentAccount?.id;
      const refreshed = currentId ? accounts.find((a) => a.id === currentId) : null;
      set({
        accounts,
        currentAccount: refreshed || get().currentAccount,
        hasAccount: accounts.length > 0,
        backendError: false,
      });
    } catch (err) {
      // Surface the error so the user can report it; vault may be locked.
      set({
        error: String(err),
        backendError: true,
      });
    }
  },

  refreshCurrentAccount: async () => {
    try {
      const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
      const currentId = get().currentAccount?.id;
      const refreshed = currentId ? accounts.find((a) => a.id === currentId) : null;
      if (refreshed) {
        set({ currentAccount: refreshed, accounts });
      }
    } catch {
      // silent
    }
  },

  bootstrap: async (name, password, locale, passwordHint) => {
    set({ isLoading: true, error: null });
    try {
      const account = await invoke<AccountInfo>('bootstrap', {
        accountName: name,
        password,
        locale,
        passwordHint,
      });
      set({
        isAuthenticated: true,
        currentAccount: account,
        accounts: [account],
        hasAccount: true,
        isLoading: false,
        backendError: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  login: async (accountId, password) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('login', { accountId, password });
      // Try to refresh account list, but do not fail authentication if the
      // refresh request errors (e.g. transient backend lock contention).
      let accounts: AccountInfo[] = [];
      try {
        accounts = (await invoke<AccountInfo[]>('vault_list_accounts')) || [];
      } catch {
        // Keep authentication state even if the account-list refresh fails.
      }
      const account = accounts.find((a) => a.id === accountId) || {
        id: accountId,
        name: accountId,
      };
      // 记录解锁完成时刻供 T2 性能基线使用
      (window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }).__SOLOSOUL_UNLOCK_TIME =
        performance.now();

      set({
        isAuthenticated: true,
        currentAccount: account,
        accounts,
        isLoading: false,
      });

      // 解锁后延迟检查备份提醒，避免启动时立即弹权限/通知
      setTimeout(() => {
        import('@/lib/notification')
          .then((m) => m.checkBackupReminder())
          .catch((err) => logger.warn('[authStore] backup reminder check failed:', err));
      }, 2000);
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  logout: async () => {
    await invoke<void>('logout');
    // 登出仅重置认证状态，hasAccount 保持为 null（未知），
    // 由 BootstrapGuard 重新调用 checkHasAccount 确认后端账户状态。
    set({
      isAuthenticated: false,
      currentAccount: null,
      accounts: [],
      hasAccount: null,
    });
  },

  clearError: () => set({ error: null }),
}));
