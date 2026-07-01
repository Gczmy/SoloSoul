import React, { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { commands, type OcrTierInfo, type OcrModelStatus } from '@/lib/ipc';
import { useToastError } from '@/hooks/useToastError';
import { OcrPopoverHeader } from '@/components/ocr/OcrPopoverHeader';
import { OcrHistoryTrashDropdown } from '@/components/ocr/OcrHistoryTrashDropdown';
import { OcrScanControls } from '@/components/ocr/OcrScanControls';
import { OcrResultPanel } from '@/components/ocr/OcrResultPanel';

// =============================================================================
// OcrQuickScanPopover — floating OCR card beside sidebar
// =============================================================================

export function OcrQuickScanPopover({
  position,
  onClose,
  placement = 'left',
}: {
  position: { top: number } | null;
  onClose: () => void;
  placement?: 'left' | 'right' | 'bottom' | 'top';
}) {
  const { t } = useTranslation(['ocr', 'common']);
  const { onError } = useToastError();
  const store = useOcrScanStore();

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [showHistory, setShowHistory] = useState(false);
  const [showTrash, setShowTrash] = useState(false);

  const cardRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const restoredCurrentScanRef = useRef(false);

  // Load model tiers on mount
  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        setLoadingStatus(true);
        const list = await commands.ocrListAvailableTiers();
        if (cancelled) return;
        setTiers(list);
        const statuses: Record<string, OcrModelStatus> = {};
        await Promise.all(
          list.map(async (tier) => {
            const st = await commands.ocrGetModelStatus(tier.tier);
            statuses[tier.tier] = st;
          }),
        );
        if (cancelled) return;
        setStatusMap(statuses);
      } catch (e) {
        if (!cancelled) onError(e, t('ocr:load_status_failed'));
      } finally {
        if (!cancelled) setLoadingStatus(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [onError, t]);

  // On first open, restore the most recent history entry as current result
  useEffect(() => {
    if (restoredCurrentScanRef.current) return;
    restoredCurrentScanRef.current = true;
    const activeHistory = store.getActiveHistory();
    if (!store.currentScanId && activeHistory.length > 0) {
      useOcrScanStore.setState({ currentScanId: activeHistory[0].id });
    }
    // P212: mount-only — store.getActiveHistory/useOcrScanStore.setState are stable refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Close history dropdown on outside click within card
  useEffect(() => {
    if (!showHistory) return;
    const handler = (e: MouseEvent) => {
      if (historyRef.current && !historyRef.current.contains(e.target as Node)) {
        setShowHistory(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [showHistory]);

  // Close on outside click (ignore OCR sidebar button clicks)
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        if ((e.target as HTMLElement).closest('[data-ocr-button]')) return;
        onClose();
      }
    };
    outsideClickTimeoutRef.current = setTimeout(
      () => document.addEventListener('mousedown', handler),
      0,
    );
    return () => {
      if (outsideClickTimeoutRef.current) {
        clearTimeout(outsideClickTimeoutRef.current);
      }
      document.removeEventListener('mousedown', handler);
    };
  }, [onClose]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  const handleSelectFile = async () => {
    try {
      const filters =
        store.scanMode === 'mrz'
          ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }]
          : [
              {
                name: 'Images & PDFs',
                extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'],
              },
            ];
      const path = await open({
        filters,
        multiple: false,
        title: store.scanMode === 'mrz' ? t('ocr:select_image_title') : t('ocr:select_file_title'),
      });
      if (path && typeof path === 'string') {
        await store.performScan(path);
      }
    } catch (e) {
      onError(e, t('ocr:select_image_failed'));
    }
  };

  const handleTierChange = async (tier: string) => {
    try {
      await commands.ocrSetActiveTier(tier);
      store.setActiveTier(tier);
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleLoadHistoryEntry = (entry: OcrScanEntry) => {
    store.setCardOpen(true);
    useOcrScanStore.setState((_s) => ({ currentScanId: entry.id }));
    setShowHistory(false);
    requestAnimationFrame(() => {
      const resultContainer = document.querySelector('.ocr-result-container');
      resultContainer?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  };

  const activeHistory = store.getActiveHistory();
  const trash = store.getTrash();
  const currentEntry = store.getCurrentEntry();

  const isFloating = placement === 'bottom' || placement === 'top';
  const isRight = placement === 'right';

  return (
    <div
      ref={cardRef}
      data-ocr-quick-scan="open"
      style={{
        position: 'fixed',
        ...(isFloating
          ? { right: 12, left: 'auto' }
          : isRight
            ? { right: 52, left: 'auto' }
            : { left: 52, right: 'auto' }),
        top: position?.top ?? 100,
        width: 400,
        height: 560,
        zIndex: 200,
        background: 'var(--bg-elevated)',
        borderRadius: 14,
        boxShadow: 'var(--shadow-lg), 0 0 0 1px var(--border-subtle)',
        border: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        animation: 'ocrQuickScanSlideIn 0.18s cubic-bezier(0.34, 1.56, 0.64, 1) both',
      }}
    >
      <OcrPopoverHeader
        showHistory={showHistory}
        onToggleHistory={() => {
          setShowHistory((prev) => !prev);
          setShowTrash(false);
        }}
        onClose={onClose}
      />

      {showHistory && (
        <div ref={historyRef}>
          <OcrHistoryTrashDropdown
            showTrash={showTrash}
            onShowTrashChange={setShowTrash}
            activeHistory={activeHistory}
            trash={trash}
            currentEntryId={currentEntry?.id ?? null}
            onSelectEntry={handleLoadHistoryEntry}
          />
        </div>
      )}

      {/* Scrollable content */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '12px 14px',
          display: 'flex',
          flexDirection: 'column',
          gap: 14,
        }}
      >
        <OcrScanControls
          activeTier={store.activeTier}
          scanMode={store.scanMode}
          isScanning={store.isScanning}
          loadingStatus={loadingStatus}
          tiers={tiers}
          statusMap={statusMap}
          onTierChange={handleTierChange}
          onScanModeChange={(mode) => store.setScanMode(mode)}
          onSelectFile={handleSelectFile}
        />

        <OcrResultPanel
          currentEntry={currentEntry}
          isScanning={store.isScanning}
          lastScanError={store.lastScanError}
        />
      </div>

      <style>{`
        @keyframes ocrQuickScanSlideIn {
          from { opacity: 0; transform: translateX(-8px) scale(0.97); }
          to { opacity: 1; transform: translateX(0) scale(1); }
        }
        .spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @keyframes ocrResultFadeIn {
          from { opacity: 0; transform: translateY(2px); }
          to { opacity: 1; transform: translateY(0); }
        }
        .ocr-history-item {
          transition: background-color 120ms ease;
        }
        .ocr-history-item:hover {
          background-color: rgba(91, 124, 153, 0.10);
        }
        .ocr-history-item--selected {
          background-color: rgba(91, 124, 153, 0.08);
        }
        .ocr-history-item--selected:hover {
          background-color: rgba(91, 124, 153, 0.16);
        }
        .ocr-history-item:active {
          background-color: rgba(91, 124, 153, 0.22);
        }
        .ocr-history-item__btn:hover {
          color: var(--accent-primary) !important;
        }
        .ocr-result-container {
          display: flex;
          flex-direction: column;
          gap: 10px;
          animation: ocrResultFadeIn 180ms ease-out both;
        }
      `}</style>
    </div>
  );
}
