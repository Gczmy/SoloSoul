import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface ProfileState {
  accountId: string | null;
  isLoading: boolean;
  error: string | null;

  loadProfile: (accountId: string) => Promise<void>;
  clear: () => void;
}

export const useProfileStore = create<ProfileState>((set) => ({
  accountId: null,
  isLoading: false,
  error: null,

  loadProfile: async (accountId) => {
    set({ isLoading: true, error: null });
    try {
      // TODO: implement profile_get command in Rust
      set({ accountId, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  clear: () => set({ accountId: null, error: null }),
}));
