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
  bootstrap: (name: string, password: string) => Promise<void>;
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
      set({ accounts, hasAccount: accounts.length > 0, backendError: false });
    } catch {
      // silent — vault may be locked
    }
  },

  bootstrap: async (name, password) => {
    set({ isLoading: true, error: null });
    try {
      const account = await commands.bootstrap(name, password);
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
      const accounts = get().accounts;
      const account = accounts.find((a) => a.id === accountId) || null;
      set({
        isAuthenticated: true,
        currentAccount: account,
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  logout: async () => {
    await commands.logout();
    set({ isAuthenticated: false, currentAccount: null });
  },

  clearError: () => set({ error: null }),
}));
