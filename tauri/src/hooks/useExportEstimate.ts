import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import type { ExportEstimate } from '@/types/exportImport';

interface ScopeState {
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  selectedTags: Set<string>;
  includeAttachments: boolean;
  selectedAttachmentIds: Set<string>;
  includePreferences: boolean;
  includeBehavioral: boolean;
}

/**
 * Encapsulates the export size estimation logic.
 * Fixes P062 by keeping estimation logic in a dedicated hook with stable
 * dependency tracking via a computed scope key.
 */
export function useExportEstimate(accountId: string, scope: ScopeState, totalSelected: number) {
  const [estimate, setEstimate] = useState<ExportEstimate | null>(null);
  const [estimating, setEstimating] = useState(false);

  // Stable key from scope contents — changes only when actual selection changes.
  const scopeKey = useMemo(
    () =>
      JSON.stringify({
        pages: Array.from(scope.selectedPageIds).sort(),
        objects: Array.from(scope.selectedObjectIds).sort(),
        tags: Array.from(scope.selectedTags).sort(),
        attachments: Array.from(scope.selectedAttachmentIds).sort(),
        includeAttachments: scope.includeAttachments,
        includePreferences: scope.includePreferences,
        includeBehavioral: scope.includeBehavioral,
      }),
    [
      scope.selectedPageIds,
      scope.selectedObjectIds,
      scope.selectedTags,
      scope.selectedAttachmentIds,
      scope.includeAttachments,
      scope.includePreferences,
      scope.includeBehavioral,
    ],
  );

  useEffect(() => {
    if (totalSelected === 0) {
      setEstimate(null);
      return;
    }

    const debounce = setTimeout(() => {
      setEstimating(true);
      invoke<ExportEstimate>('export_estimate_size', {
        account_id: accountId,
        scope: {
          selectedPageIds: Array.from(scope.selectedPageIds),
          selectedObjectIds: Array.from(scope.selectedObjectIds),
          selectedTags: Array.from(scope.selectedTags),
          includeAttachments: scope.includeAttachments,
          selectedAttachmentIds: Array.from(scope.selectedAttachmentIds),
          includePreferences: scope.includePreferences,
          includeBehavioral: scope.includeBehavioral,
        },
      })
        .then(setEstimate)
        .catch(() => setEstimate(null))
        .finally(() => setEstimating(false));
    }, DEBOUNCE_DELAY_MS);

    return () => clearTimeout(debounce);
    // scopeKey 已覆盖 scope 所有字段，scope 本体不含在 deps 中是故意的，scope 加入 deps 是 ESLint 要求
  }, [totalSelected, accountId, scopeKey, scope]);

  return { estimate, estimating };
}
