import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';

interface ExportEstimate {
  objectCount: number;
  attachmentCount: number;
  attachmentSelectedCount: number;
  estimatedBytes: number;
}

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
  const prevKeyRef = useRef<string>('');

  useEffect(() => {
    if (totalSelected === 0) {
      setEstimate(null);
      prevKeyRef.current = '';
      return;
    }

    // Build a stable key from scope contents to avoid re-triggering on Set identity changes.
    const scopeKey = JSON.stringify({
      pages: Array.from(scope.selectedPageIds).sort(),
      objects: Array.from(scope.selectedObjectIds).sort(),
      tags: Array.from(scope.selectedTags).sort(),
      attachments: Array.from(scope.selectedAttachmentIds).sort(),
      includeAttachments: scope.includeAttachments,
      includePreferences: scope.includePreferences,
      includeBehavioral: scope.includeBehavioral,
    });

    if (prevKeyRef.current === scopeKey) return;
    prevKeyRef.current = scopeKey;

    const debounce = setTimeout(() => {
      setEstimating(true);
      invoke<ExportEstimate>('export_estimate_size', {
        accountId,
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [totalSelected, accountId]);

  return { estimate, estimating };
}
