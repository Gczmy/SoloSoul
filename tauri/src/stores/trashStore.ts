import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export type TrashRetentionPeriod = '30d' | '60d' | 'half_year' | 'one_year' | 'never';

// §23.9 — TrashItemSummary from backend
export interface TrashItemSummary {
  id: string;
  itemType: string;
  name: string;
  iconId?: string;
  deletedAt: number;
  expiresAt?: number;
  originalParentName?: string;
  originalSectionType?: string;
}

export type TrashTimeFilter = 'all' | '1d' | '3d' | '7d' | '30d' | 'half_year';
export type TrashTypeFilter = 'all' | 'page' | 'object';

interface TrashState {
  items: TrashItemSummary[];
  timeFilter: TrashTimeFilter;
  typeFilter: TrashTypeFilter;
  searchQuery: string;
  isLoading: boolean;
  error: string | null;

  loadItems: (accountId: string) => Promise<void>;
  setTimeFilter: (f: TrashTimeFilter) => void;
  setTypeFilter: (f: TrashTypeFilter) => void;
  setSearchQuery: (q: string) => void;
  restoreItem: (trashId: string) => Promise<void>;
  permanentDelete: (trashIds: string[]) => Promise<void>;
  clearOnVaultLock: () => void;
}

const TIME_SINCE: Record<TrashTimeFilter, number | null> = {
  all: null,
  '1d': 24 * 3600 * 1000,
  '3d': 3 * 24 * 3600 * 1000,
  '7d': 7 * 24 * 3600 * 1000,
  '30d': 30 * 24 * 3600 * 1000,
  half_year: 180 * 24 * 3600 * 1000,
};

export const useTrashStore = create<TrashState>((set, get) => ({
  items: [],
  timeFilter: 'all',
  typeFilter: 'all',
  searchQuery: '',
  isLoading: false,
  error: null,

  loadItems: async (_accountId) => {
    set({ isLoading: true, error: null });
    try {
      const since = TIME_SINCE[get().timeFilter];
      const items = await invoke<TrashItemSummary[]>('object_trash_list', {
        accountId: _accountId,
        ...(since && { since }),
      });
      set({ items, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  setTimeFilter: (f) => set({ timeFilter: f }),
  setTypeFilter: (f) => set({ typeFilter: f }),
  setSearchQuery: (q) => set({ searchQuery: q }),

  restoreItem: async (trashId) => {
    await invoke('trash_restore', { trashId });
    set((s) => ({ items: s.items.filter((i) => i.id !== trashId) }));
  },

  permanentDelete: async (trashIds) => {
    for (const id of trashIds) {
      await invoke('trash_permanent_delete', { trashId: id });
    }
    set((s) => ({ items: s.items.filter((i) => !trashIds.includes(i.id)) }));
  },

  clearOnVaultLock: () => set({ items: [], timeFilter: 'all', typeFilter: 'all', searchQuery: '' }),
}));
