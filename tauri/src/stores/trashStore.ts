import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { create } from 'zustand';
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
  restoreBatch: (trashIds: string[]) => Promise<RestoreOutcome[]>;
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

const requests = createSessionRequests();

export const useTrashStore = create<TrashState>((set, get) => ({
  items: [],
  timeFilter: 'all',
  typeFilter: 'all',
  searchQuery: '',
  isLoading: false,
  error: null,
  selectedIds: new Set(),

  loadItems: async (_accountId) => {
    const request = requests.begin('items', _accountId);
    const setCurrent = request.guardSet<TrashState>(set);
    setCurrent({ isLoading: true, error: null });
    try {
      // 后端 since 语义为「绝对毫秒时间戳」（SQL: deleted_at >= since），
      // TIME_SINCE 表存的是相对偏移量，必须换算为 Date.now() - offset 再传——
      // 否则 1d=86400000 会被当成 1970 年的时间戳比较，任何真实 deleted_at
      // （≈1.78e12）都恒 ≥ 它，筛选等于不过滤（修复：回收站时间筛选失效）。
      const offset = TIME_SINCE[get().timeFilter];
      const since = offset === null ? undefined : Date.now() - offset;
      const items = await request.invoke<TrashItemSummary[]>('object_trash_list', {
        accountId: _accountId,
        ...(since !== undefined && { since }),
      });
      request.assertCurrent();
      setCurrent({ items, isLoading: false, selectedIds: new Set() });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoading: false });
    }
  },

  setTimeFilter: (f) => set({ timeFilter: f }),
  setTypeFilter: (f) => set({ typeFilter: f }),
  setSearchQuery: (q) => set({ searchQuery: q }),

  restoreItem: async (trashId) => {
    const request = requests.begin();
    const setCurrent = request.guardSet<TrashState>(set);
    const item = get().items.find((i) => i.id === trashId);
    if (item?.itemType === 'template') {
      await request.invoke('template_restore', { trashId: trashId });
      request.assertCurrent();
      setCurrent((s) => ({ items: s.items.filter((i) => i.id !== trashId) }));
      return { restoredId: item.originalId, name: item.name, cascadedCount: 0 };
    }
    try {
      const outcome = await request.invoke<RestoreOutcome>('trash_restore', {
        trashId: trashId,
        lang: i18next.language,
      });
      request.assertCurrent();
      const consumed = outcome.consumedTrashIds ?? [trashId];
      setCurrent((s) => ({ items: s.items.filter((i) => !consumed.includes(i.id)) }));
      return outcome;
    } catch (err) {
      request.assertCurrent();
      // If the item was cascade-restored by a sibling/page restore, its trash row is already gone.
      // Treat that as a success so batch restores don't fail halfway through.
      const message = typeof err === 'string' ? err : String(err);
      if (message.includes('Trash item not found')) {
        setCurrent((s) => ({ items: s.items.filter((i) => i.id !== trashId) }));
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
    const request = requests.begin();
    const setCurrent = request.guardSet<TrashState>(set);
    // P024: 服务端批量端点——N 次 IPC → 1 次；任一失败整体 reject（已删项保持已删）。
    await request.invoke('trash_permanent_delete_batch', { trashIds });
    request.assertCurrent();
    setCurrent((s) => ({ items: s.items.filter((i) => !trashIds.includes(i.id)) }));
  },

  restoreBatch: async (trashIds: string[]) => {
    const request = requests.begin();
    const setCurrent = request.guardSet<TrashState>(set);
    // P014: 服务端批量端点（对齐 permanentDelete）——N 次串行 IPC → 1 次。
    // 模板/对象统一由后端分派；级联恢复/已删除项幂等跳过。
    const outcomes = await request.invoke<RestoreOutcome[]>('trash_restore_batch', {
      trashIds,
      lang: i18next.language,
    });
    request.assertCurrent();
    // 按 consumedTrashIds（含级联消费的子对象/页面）过滤本地列表
    const consumed = new Set<string>();
    for (const o of outcomes) {
      for (const id of o.consumedTrashIds ?? []) consumed.add(id);
    }
    setCurrent((s) => ({ items: s.items.filter((i) => !consumed.has(i.id)) }));
    return outcomes;
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

  clearOnVaultLock: () => {
    requests.invalidate();
    return set({
      items: [],
      isLoading: false,
      error: null,
      timeFilter: 'all',
      typeFilter: 'all',
      searchQuery: '',
      selectedIds: new Set(),
    });
  },
}));

onRequestSessionChange(() => useTrashStore.getState().clearOnVaultLock());
