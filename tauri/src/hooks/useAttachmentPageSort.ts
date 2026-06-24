import { useMemo } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { SYSTEM_PAGE_KEYS } from '@/components/layout/useNavigationItems';

/** Minimum page shape required for sorting — compatible with AttachmentTreePage. */
export interface SortableAttachmentPage {
  pageId?: string | null;
  pageName: string;
}

/**
 * Sorts an array of attachment pages to match the sidebar page order:
 * 1. Built-in system pages: identity → travel → financial → professional
 * 2. Custom pages: ordered by their `sortOrder` from the settings store
 * 3. Unknown pages fall to the end
 */
export function useAttachmentPageSort<T extends SortableAttachmentPage>(pages: T[]): T[] {
  const customPages = useSettingsStore((s) => s.settings.customPages);

  const customPageOrder = useMemo(() => {
    const map = new Map<string, number>();
    for (const p of customPages) {
      map.set(p.id, p.sortOrder);
    }
    return map;
  }, [customPages]);

  return useMemo(() => {
    return [...pages].sort((a, b) => {
      const aIsSystem = !a.pageId;
      const bIsSystem = !b.pageId;

      if (aIsSystem && bIsSystem) {
        const aIdx = SYSTEM_PAGE_KEYS.indexOf(a.pageName as (typeof SYSTEM_PAGE_KEYS)[number]);
        const bIdx = SYSTEM_PAGE_KEYS.indexOf(b.pageName as (typeof SYSTEM_PAGE_KEYS)[number]);
        return (aIdx === -1 ? 999 : aIdx) - (bIdx === -1 ? 999 : bIdx);
      }
      if (aIsSystem) return -1;
      if (bIsSystem) return 1;

      // Both are custom pages — sort by sortOrder from settings store
      const aOrder = a.pageId ? customPageOrder.get(a.pageId) ?? 999 : 999;
      const bOrder = b.pageId ? customPageOrder.get(b.pageId) ?? 999 : 999;
      return aOrder - bOrder;
    });
  }, [pages, customPageOrder]);
}
