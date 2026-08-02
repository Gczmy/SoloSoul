import React, { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { openWithPause } from '@/lib/dialog';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { OcrTierInfo, OcrModelStatus } from '@/lib/ipc';
import { useToastError } from '@/hooks/useToastError';
import { isMobilePlatformSync } from '@/lib/platform';
import { OcrPopoverHeader } from '@/components/ocr/OcrPopoverHeader';
import styles from './OcrQuickScanPopover.module.css';
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
  // P215: 字段级选择器订阅数据（扫描进度/历史/错误），动作走 getState()——
  // 避免整店订阅让本浮层在 store 任意字段变化时都重渲染。
  const scanMode = useOcrScanStore((s) => s.scanMode);
  const currentScanId = useOcrScanStore((s) => s.currentScanId);
  const isScanning = useOcrScanStore((s) => s.isScanning);
  const activeTier = useOcrScanStore((s) => s.activeTier);
  const lastScanError = useOcrScanStore((s) => s.lastScanError);
  const scanHistory = useOcrScanStore((s) => s.scanHistory);

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
        const list = await invoke<OcrTierInfo[]>('ocr_list_available_tiers');
        if (cancelled) return;
        setTiers(list);
        const statuses: Record<string, OcrModelStatus> = {};
        await Promise.all(
          list.map(async (tier) => {
            const st = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: tier.tier });
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
    const activeHistory = useOcrScanStore.getState().scanHistory.filter((h) => !h.isDeleted);
    if (!currentScanId && activeHistory.length > 0) {
      useOcrScanStore.setState({ currentScanId: activeHistory[0].id });
    }
    // P215: mount-only — getState/currentScanId are stable refs.
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

  const isMobilePlatform = isMobilePlatformSync();

  const handleSelectFile = async () => {
    try {
      const filters =
        scanMode === 'mrz' || isMobilePlatform
          ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }]
          : [
              {
                name: 'Images & PDFs',
                extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'],
              },
            ];
      const path = await openWithPause({
        filters,
        multiple: false,
        title: scanMode === 'mrz' ? t('ocr:select_image_title') : t('ocr:select_file_title'),
      });
      if (path && typeof path === 'string') {
        await useOcrScanStore.getState().performScan(path);
      }
    } catch (e) {
      onError(e, t('ocr:select_image_failed'));
    }
  };

  const handleTierChange = async (tier: string) => {
    try {
      await invoke<void>('ocr_set_active_tier', { tier });
      useOcrScanStore.getState().setActiveTier(tier);
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleLoadHistoryEntry = (entry: OcrScanEntry) => {
    useOcrScanStore.getState().setCardOpen(true);
    useOcrScanStore.setState((_s) => ({ currentScanId: entry.id }));
    setShowHistory(false);
    requestAnimationFrame(() => {
      const resultContainer = document.querySelector('.ocr-result-container');
      resultContainer?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  };

  const activeHistory = scanHistory.filter((h) => !h.isDeleted);
  const trash = scanHistory.filter((h) => h.isDeleted);
  const currentEntry = scanHistory.find((h) => h.id === currentScanId) || null;

  const isFloating = placement === 'bottom' || placement === 'top';
  const isRight = placement === 'right';

  return (
    <div
      ref={cardRef}
      data-ocr-quick-scan="open"
      className={styles.card}
      style={{
        ...(isFloating
          ? { right: 12, left: 'auto' }
          : isRight
            ? { right: 52, left: 'auto' }
            : { left: 52, right: 'auto' }),
        top: position?.top ?? 100,
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
          activeTier={activeTier}
          scanMode={scanMode}
          isScanning={isScanning}
          loadingStatus={loadingStatus}
          tiers={tiers}
          statusMap={statusMap}
          onTierChange={handleTierChange}
          onScanModeChange={(mode) => useOcrScanStore.getState().setScanMode(mode)}
          onSelectFile={handleSelectFile}
          isMobile={isMobilePlatform}
        />

        <OcrResultPanel
          currentEntry={currentEntry}
          isScanning={isScanning}
          lastScanError={lastScanError}
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
        @media (hover: hover) and (pointer: fine) {
          .ocr-history-item:hover {
            background-color: rgba(91, 124, 153, 0.10);
          }
          .ocr-history-item--selected:hover {
            background-color: rgba(91, 124, 153, 0.16);
          }
          .ocr-history-item__btn:hover {
            color: var(--accent-primary) !important;
          }
        }
        .ocr-history-item--selected {
          background-color: rgba(91, 124, 153, 0.08);
        }
        .ocr-history-item:active {
          background-color: rgba(91, 124, 153, 0.22);
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
