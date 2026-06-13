import { create } from 'zustand';
import { commands, VaultStateStr } from '@/lib/ipc';

interface VaultStoreState {
  vaultState: VaultStateStr;
  isLoading: boolean;
  error: string | null;

  loadVaultState: () => Promise<void>;
  unlock: (accountId: string, password: string) => Promise<void>;
  lock: () => Promise<void>;
}

export const useVaultStore = create<VaultStoreState>((set, _get) => ({
  vaultState: 'locked',
  isLoading: false,
  error: null,

  loadVaultState: async () => {
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
