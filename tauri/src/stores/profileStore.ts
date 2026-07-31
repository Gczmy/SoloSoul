import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface ProfileSectionData {
  sectionType: string;
  fields: Array<{
    key: string;
    label: string;
    value: unknown;
    sensitivityLevel?: string;
  }>;
}

interface RawProfileSection {
  type?: string;
  fields?: Array<{
    key?: string;
    label?: string;
    value?: unknown;
    sensitivityLevel?: string;
  }>;
}

interface ProfileState {
  accountId: string | null;
  sections: ProfileSectionData[];
  isLoading: boolean;
  error: string | null;

  loadProfile: (accountId: string) => Promise<void>;
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
      const profile = await invoke<{ accountId: string; data: number[] } | null>('profile_load', {
        accountId: accountId,
      });
      if (profile?.data) {
        const json = JSON.parse(new TextDecoder().decode(new Uint8Array(profile.data)));
        const loadedSections: ProfileSectionData[] = (json.sections || []).map(
          (sec: RawProfileSection) => ({
            sectionType: sec.type || '',
            fields: (sec.fields || []).map((f) => ({
              key: f.key || '',
              label: f.label || '',
              value: f.value,
              sensitivityLevel: f.sensitivityLevel,
            })),
          }),
        );
        set({ accountId: profile.accountId, sections: loadedSections, isLoading: false });
      } else {
        set({ accountId, sections: [], isLoading: false });
      }
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  clear: () => set({ accountId: null, sections: [], error: null }),
}));
