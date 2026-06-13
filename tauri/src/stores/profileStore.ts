import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface ProfileSection {
  sectionType: string;
  fields: Array<{
    key: string;
    label: string;
    value: unknown;
    sensitivityLevel?: string;
  }>;
}

interface ProfileState {
  accountId: string | null;
  sections: ProfileSection[];
  isLoading: boolean;
  error: string | null;

  loadProfile: (accountId: string) => Promise<void>;
  loadSection: (accountId: string, sectionType: string) => Promise<ProfileSection | null>;
  updateField: (
    accountId: string,
    sectionType: string,
    fieldKey: string,
    value: unknown,
  ) => Promise<void>;
  clear: () => void;
}

export const useProfileStore = create<ProfileState>((set) => ({
  accountId: null,
  sections: [],
  isLoading: false,
  error: null,

  loadProfile: async (accountId) => {
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<{ accountId: string } | null>('profile_load', { accountId });
      set({ accountId: profile?.accountId || accountId, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  loadSection: async (accountId, sectionType) => {
    try {
      const section = await invoke<ProfileSection | null>('profile_get_section', {
        accountId,
        sectionType,
      });
      return section;
    } catch {
      return null;
    }
  },

  updateField: async (accountId, sectionType, fieldKey, value) => {
    try {
      await invoke('profile_update_field', {
        accountId,
        sectionType,
        fieldKey,
        fieldValue: value,
      });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  clear: () => set({ accountId: null, sections: [], error: null }),
}));
