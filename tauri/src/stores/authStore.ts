import { create } from 'zustand';
import { commands, AccountInfo } from '@/lib/ipc';

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
  backendError: false,

  checkHasAccount: async () => {
    try {
      const result = await commands.checkHasAccount();
      set({ hasAccount: result, backendError: false });
    } catch {
      // Backend unavailable — don't jump to bootstrap
      // hasAccount stays null (unknown), BootstrapGuard will wait
      set({ backendError: true });
    }
  },

  listAccounts: async () => {
    try {
      const accounts = await commands.vaultListAccounts();
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
      console.warn('[authStore.listAccounts] failed:', err);
      set({
        error: String(err),
        backendError: true,
      });
    }
  },

  refreshCurrentAccount: async () => {
    try {
      const accounts = await commands.vaultListAccounts();
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
      const account = await commands.bootstrap(name, password, locale, passwordHint);
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
      await commands.login(accountId, password);
      // Try to refresh account list, but do not fail authentication if the
      // refresh request errors (e.g. transient backend lock contention).
      let accounts: AccountInfo[] = [];
      try {
        accounts = (await commands.vaultListAccounts()) || [];
      } catch {
        // Keep authentication state even if the account-list refresh fails.
      }
      const account = accounts.find((a) => a.id === accountId) || {
        id: accountId,
        name: accountId,
      };
      set({
        isAuthenticated: true,
        currentAccount: account,
        accounts,
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  logout: async () => {
    await commands.logout();
    set({
      isAuthenticated: false,
      currentAccount: null,
      accounts: [],
      hasAccount: false,
    });
  },

  clearError: () => set({ error: null }),
}));
