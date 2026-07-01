import { useState, useCallback, useMemo } from 'react';

export interface UseBatchSelectOptions {
  /** Initial selected IDs (default: empty Set). */
  initialSelected?: Set<string>;
}

export interface UseBatchSelectReturn {
  /** Set of composite keys (e.g. `${objectId}::${attachmentId}`) currently selected. */
  selectedIds: Set<string>;
  /** True when the batch soft-delete confirmation dialog should be shown. */
  batchDeleteConfirm: boolean;
  /** True when the batch restore confirmation dialog should be shown. */
  batchRestoreConfirm: boolean;
  /** True when the batch permanent-delete confirmation dialog should be shown. */
  batchPermanentDeleteConfirm: boolean;
  /** Whether all visible items (from `allVisibleKeys`) are currently selected. */
  allSelected: boolean;

  /** Toggle a single composite key — add if absent, remove if present. */
  toggleSelect: (compositeKey: string) => void;
  /** Select all visible keys if not all selected, otherwise deselect all. */
  handleSelectAll: (allVisibleKeys: string[]) => void;
  /** Clear all selections. */
  clearSelection: () => void;

  setBatchDeleteConfirm: (v: boolean) => void;
  setBatchRestoreConfirm: (v: boolean) => void;
  setBatchPermanentDeleteConfirm: (v: boolean) => void;
}

/**
 * useBatchSelect — manages checkbox selection state for batch operations.
 *
 * @param allVisibleKeys — the list of all visible composite keys used to derive `allSelected`.
 * @param options — optional initial state.
 */
export function useBatchSelect(
  allVisibleKeys: string[],
  options?: UseBatchSelectOptions,
): UseBatchSelectReturn {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(
    options?.initialSelected ?? new Set(),
  );

  const [batchDeleteConfirm, setBatchDeleteConfirm] = useState(false);
  const [batchRestoreConfirm, setBatchRestoreConfirm] = useState(false);
  const [batchPermanentDeleteConfirm, setBatchPermanentDeleteConfirm] = useState(false);

  const allSelected = useMemo(
    () => allVisibleKeys.length > 0 && allVisibleKeys.every((k) => selectedIds.has(k)),
    [allVisibleKeys, selectedIds],
  );

  const toggleSelect = useCallback((compositeKey: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(compositeKey)) {
        next.delete(compositeKey);
      } else {
        next.add(compositeKey);
      }
      return next;
    });
  }, []);

  const handleSelectAll = useCallback((visibleKeys: string[]) => {
    setSelectedIds((prev) => {
      if (visibleKeys.length > 0 && visibleKeys.every((k) => prev.has(k))) {
        return new Set();
      }
      return new Set(visibleKeys);
    });
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedIds(new Set());
  }, []);

  return {
    selectedIds,
    batchDeleteConfirm,
    batchRestoreConfirm,
    batchPermanentDeleteConfirm,
    allSelected,

    toggleSelect,
    handleSelectAll,
    clearSelection,

    setBatchDeleteConfirm,
    setBatchRestoreConfirm,
    setBatchPermanentDeleteConfirm,
  };
}
