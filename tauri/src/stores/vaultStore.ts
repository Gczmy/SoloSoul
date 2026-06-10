import { create } from 'zustand';
import { commands, VaultStateStr } from '@/lib/ipc';

interface AccountSummary {
  id: string;
  name: string;
}

interface VaultStoreState {
  vaultState: VaultStateStr;
  accounts: AccountSummary[];
  isLoading: boolean;
  error: string | null;

  loadAccounts: () => Promise<void>;
  unlock: (accountId: string, password: string) => Promise<void>;
  lock: () => Promise<void>;
}

export const useVaultStore = create<VaultStoreState>((set, _get) => ({
  vaultState: 'locked',
  accounts: [],
  isLoading: false,
  error: null,

  loadAccounts: async () => {
    set({ isLoading: true });
    try {
      const state = await commands.vaultGetState();
      set({ vaultState: state, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  unlock: async (accountId, password) => {
    set({ isLoading: true, error: null });
    try {
      await commands.vaultUnlock(accountId, password);
      set({ vaultState: 'unlocked', isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  lock: async () => {
    await commands.vaultLock();
    set({ vaultState: 'locked' });
  },
}));
