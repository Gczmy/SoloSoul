import { create } from 'zustand';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import i18next from '@/lib/i18n';

export type TrashRetentionPeriod = '30d' | '60d' | 'half_year' | 'one_year' | 'never';

/** Convert retention period string to number of days (-1 for never). */
export function retentionPeriodDays(period: TrashRetentionPeriod): number {
  switch (period) {
    case '60d':
      return 60;
    case 'half_year':
      return 180;
    case 'one_year':
      return 365;
    case 'never':
      return -1;
    default:
      return 30;
  }
}

// §23.9 — TrashItemSummary from backend
export interface TrashItemSummary {
  id: string;
  itemType: string;
  originalId: string;
  name: string;
  iconId?: string;
  deletedAt: number;
  expiresAt?: number;
  originalParentId?: string;
  originalSectionType?: string;
  contractTypeId?: string;
}

export type TrashTimeFilter = 'all' | '1d' | '3d' | '7d' | '30d' | 'half_year';
export type TrashTypeFilter = 'all' | 'page' | 'object' | 'template';

export interface RestoreOutcome {
  restoredId: string;
  name: string;
  cascadedPageName?: string;
  cascadedCount?: number;
  rebuiltPageName?: string;
  consumedTrashIds?: string[];
}

interface TrashState {
  items: TrashItemSummary[];
  timeFilter: TrashTimeFilter;
  typeFilter: TrashTypeFilter;
  searchQuery: string;
  isLoading: boolean;
  error: string | null;
  selectedIds: Set<string>;

  loadItems: (accountId: string) => Promise<void>;
  setTimeFilter: (f: TrashTimeFilter) => void;
  setTypeFilter: (f: TrashTypeFilter) => void;
  setSearchQuery: (q: string) => void;
  restoreItem: (trashId: string) => Promise<RestoreOutcome>;
  permanentDelete: (trashIds: string[]) => Promise<void>;
  toggleSelection: (id: string) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
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
  selectedIds: new Set(),

  loadItems: async (_accountId) => {
    set({ isLoading: true, error: null });
    try {
      const since = TIME_SINCE[get().timeFilter];
      const items = await invoke<TrashItemSummary[]>('object_trash_list', {
        accountId: _accountId,
        ...(since && { since }),
      });
      set({ items, isLoading: false, selectedIds: new Set() });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  setTimeFilter: (f) => set({ timeFilter: f }),
  setTypeFilter: (f) => set({ typeFilter: f }),
  setSearchQuery: (q) => set({ searchQuery: q }),

  restoreItem: async (trashId) => {
    const item = get().items.find((i) => i.id === trashId);
    if (item?.itemType === 'template') {
      await invoke('template_restore', { trashId: trashId });
      set((s) => ({ items: s.items.filter((i) => i.id !== trashId) }));
      return { restoredId: item.originalId, name: item.name, cascadedCount: 0 };
    }
    try {
      const outcome = await invoke<RestoreOutcome>('trash_restore', {
        trashId: trashId,
        lang: i18next.language,
      });
      const consumed = outcome.consumedTrashIds ?? [trashId];
      set((s) => ({ items: s.items.filter((i) => !consumed.includes(i.id)) }));
      return outcome;
    } catch (err) {
      // If the item was cascade-restored by a sibling/page restore, its trash row is already gone.
      // Treat that as a success so batch restores don't fail halfway through.
      const message = typeof err === 'string' ? err : String(err);
      if (message.includes('Trash item not found')) {
        set((s) => ({ items: s.items.filter((i) => i.id !== trashId) }));
        return {
          restoredId: item?.originalId ?? trashId,
          name: item?.name ?? trashId,
          cascadedCount: 0,
        };
      }
      throw err;
    }
  },

  permanentDelete: async (trashIds) => {
    // P052: 并发化减少总等待时间；R2-20: 加并发上限，清空数百条时不瞬间发起数百 invoke。
    // 任一失败时整体 reject（与串行首错中止语义一致），未完成的其余删除在服务端仍会执行。
    const CONCURRENCY_LIMIT = 8;
    let cursor = 0;
    const worker = async (): Promise<void> => {
      while (cursor < trashIds.length) {
        const id = trashIds[cursor];
        cursor += 1;
        await invoke('trash_permanent_delete', { trashId: id });
      }
    };
    await Promise.all(
      Array.from(
        { length: Math.min(CONCURRENCY_LIMIT, trashIds.length) },
        () => worker(),
      ),
    );
    set((s) => ({ items: s.items.filter((i) => !trashIds.includes(i.id)) }));
  },

  toggleSelection: (id) => {
    set((s) => {
      const next = new Set(s.selectedIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { selectedIds: next };
    });
  },

  selectAll: (ids) => {
    set({ selectedIds: new Set(ids) });
  },

  clearSelection: () => {
    set({ selectedIds: new Set() });
  },

  clearOnVaultLock: () =>
    set({
      items: [],
      timeFilter: 'all',
      typeFilter: 'all',
      searchQuery: '',
      selectedIds: new Set(),
    }),
}));
