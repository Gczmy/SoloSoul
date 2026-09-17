import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { create } from 'zustand';

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

const requests = createSessionRequests();

export const useProfileStore = create<ProfileState>((set) => ({
  accountId: null,
  sections: [],
  isLoading: false,
  error: null,

  loadProfile: async (accountId) => {
    const request = requests.begin('profile', accountId);
    const setCurrent = request.guardSet<ProfileState>(set);
    setCurrent({ isLoading: true, error: null });
    try {
      const profile = await request.invoke<{ accountId: string; data: number[] } | null>(
        'profile_load',
        {
          accountId: accountId,
        },
      );
      request.assertCurrent();
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
        setCurrent({ accountId: profile.accountId, sections: loadedSections, isLoading: false });
      } else {
        setCurrent({ accountId, sections: [], isLoading: false });
      }
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoading: false });
    }
  },

  clear: () => {
    requests.invalidate();
    return set({ accountId: null, sections: [], isLoading: false, error: null });
  },
}));

onRequestSessionChange(() => useProfileStore.getState().clear());
