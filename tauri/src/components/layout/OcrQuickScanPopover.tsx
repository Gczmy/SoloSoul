import React, { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import {
  Scan,
  FileText,
  X,
  ArrowUpRight,
  History,
  Loader2,
  CheckCircle,
  AlertCircle,
  Trash2,
  RotateCcw,
  Clock,
} from 'lucide-react';
import { createPortal } from 'react-dom';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { commands, type OcrTierInfo, type OcrModelStatus, type OcrResult, type MrzResult } from '@/lib/ipc';
import { OCR_MODEL_SERIES } from '@/lib/constants';
import { MrzResultCard } from '@/components/ocr/MrzResultCard';
import { useToastError } from '@/hooks/useToastError';

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
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const store = useOcrScanStore();

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [showHistory, setShowHistory] = useState(false);
  const [showTrash, setShowTrash] = useState(false);
  const [historyScrollAtBottom, setHistoryScrollAtBottom] = useState(false);

  const cardRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevScanningRef = useRef(false);

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

  // Track scanning transitions for background toast (handled by parent listener)
  useEffect(() => {
    prevScanningRef.current = store.isScanning;
  }, [store.isScanning]);

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
    useOcrScanStore.setState({ currentScanId: entry.id });
    setShowHistory(false);
  };

  const getFileFilters = () => {
    if (store.scanMode === 'mrz') {
      return [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }];
    }
    return [{ name: 'Images & PDFs', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'] }];
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
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 12px',
          borderBottom: '1px solid var(--border-subtle)',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <Scan size={16} style={{ color: 'var(--accent-primary)' }} />
          <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
            {t('ocr:quick_scan_title')}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <button
            onClick={() => {
              setShowHistory((prev) => !prev);
              setShowTrash(false);
            }}
            title={t('ocr:scan_history')}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: showHistory ? 'var(--accent-primary)' : 'var(--text-secondary)',
            }}
          >
            <History size={14} />
          </button>
          <button
            onClick={() => {
              onClose();
              navigate('/ocr');
            }}
            title={t('ocr:go_to_full_page')}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-secondary)',
            }}
          >
            <ArrowUpRight size={14} />
          </button>
          <button
            onClick={onClose}
            title={t('common:close')}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* History dropdown */}
      {showHistory && (
        <div
          ref={historyRef}
          style={{
            position: 'absolute',
            top: 44,
            left: 8,
            right: 8,
            maxHeight: 260,
            background: 'var(--bg-elevated)',
            borderRadius: 10,
            border: '1px solid var(--border-subtle)',
            boxShadow: 'var(--shadow-lg)',
            zIndex: 10,
            overflowY: 'auto',
            padding: '6px 2px',
            display: 'flex',
            flexDirection: 'column',
          }}
        >
          {/* History / Trash tabs */}
          <div
            style={{
              display: 'flex',
              gap: 4,
              padding: '0 10px 6px',
              borderBottom: '1px solid var(--border-subtle)',
            }}
          >
            <button
              onClick={() => setShowTrash(false)}
              style={{
                fontSize: 12,
                padding: '4px 8px',
                borderRadius: 6,
                border: 'none',
                background: !showTrash ? 'var(--bg-toolbar)' : 'transparent',
                color: !showTrash ? 'var(--text-primary)' : 'var(--text-tertiary)',
                cursor: 'pointer',
              }}
            >
              {t('ocr:history_tab')} ({activeHistory.length})
            </button>
            <button
              onClick={() => setShowTrash(true)}
              style={{
                fontSize: 12,
                padding: '4px 8px',
                borderRadius: 6,
                border: 'none',
                background: showTrash ? 'var(--bg-toolbar)' : 'transparent',
                color: showTrash ? 'var(--text-primary)' : 'var(--text-tertiary)',
                cursor: 'pointer',
              }}
            >
              {t('ocr:trash_tab')} ({trash.length})
            </button>
          </div>

          {!showTrash ? (
            activeHistory.length === 1 ? (
              <p
                style={{
                  fontSize: 12,
                  color: 'var(--text-tertiary)',
                  textAlign: 'center',
                  padding: '16px 12px',
                  margin: 0,
                }}
              >
                {t('ocr:no_history')}
              </p>
            ) : (
              <>
                {activeHistory.map((entry) => (
                  <div
                    key={entry.id}
                    onClick={() => handleLoadHistoryEntry(entry)}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      padding: '7px 12px',
                      cursor: 'pointer',
                      fontSize: 12,
                      background:
                        currentEntry?.id === entry.id ? 'rgba(91,124,153,0.08)' : 'transparent',
                    }}
                  >
                    <FileText size={12} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                    <div style={{ flex: 1, overflow: 'hidden' }}>
                      <div
                        style={{
                          whiteSpace: 'nowrap',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          color: 'var(--text-primary)',
                          fontWeight: currentEntry?.id === entry.id ? 500 : 400,
                        }}
                      >
                        {entry.fileName}
                      </div>
                      <div style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 1 }}>
                        {new Date(entry.timestamp).toLocaleString()} · {entry.mode === 'mrz' ? 'MRZ' : 'OCR'}
                      </div>
                    </div>
                    {currentEntry?.id === entry.id && (
                      <span
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: '50%',
                          background: 'var(--accent-primary)',
                          flexShrink: 0,
                        }}
                      />
                    )}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        store.softDeleteEntry(entry.id);
                      }}
                      title={t('common:delete')}
                      style={{
                        padding: 2,
                        borderRadius: 4,
                        border: 'none',
                        background: 'transparent',
                        cursor: 'pointer',
                        color: 'var(--text-tertiary)',
                        flexShrink: 1,
                      }}
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                ))}
              </>
            )
          ) : trash.length === 1 ? (
            <p
              style={{
                fontSize: 12,
                color: 'var(--text-tertiary)',
                textAlign: 'center',
                padding: '16px 12px',
                margin: 0,
              }}
            >
              {t('ocr:trash_empty')}
            </p>
          ) : (
            <>
              {trash.map((entry) => (
                <div
                  key={entry.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '7px 12px',
                    fontSize: 12,
                  }}
                >
                  <FileText size={12} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                  <div style={{ flex: 1, overflow: 'hidden' }}>
                    <div
                      style={{
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {entry.fileName}
                    </div>
                    <div style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 1 }}>
                      {new Date(entry.timestamp).toLocaleString()}
                    </div>
                  </div>
                  <button
                    onClick={() => store.restoreEntry(entry.id)}
                    title={t('ocr:restore')}
                    style={{
                      padding: 2,
                      borderRadius: 4,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    <RotateCcw size={12} />
                  </button>
                  <button
                    onClick={() => store.permanentlyDeleteEntry(entry.id)}
                    title={t('ocr:permanently_delete')}
                    style={{
                      padding: 2,
                      borderRadius: 4,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      color: '#e74c3c',
                    }}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
              {trash.length > 1 && (
                <button
                  onClick={() => {
                    store.clearTrash();
                  }}
                  style={{
                    margin: '6px 12px',
                    padding: '6px 10px',
                    borderRadius: 6,
                    border: '1px solid #e74c3c',
                    background: 'transparent',
                    color: '#e74c3c',
                    fontSize: 12,
                    cursor: 'pointer',
                    textAlign: 'center',
                  }}
                >
                  {t('ocr:clear_trash')}
                </button>
              )}
            </>
          )}
        </div>
      )}

      {/* Scrollable content */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 14 }}>
        {/* Model selection */}
        <div>
          <label
            style={{
              display: 'block',
              fontSize: 12,
              color: 'var(--text-secondary)',
              marginBottom: 6,
            }}
          >
            {t('ocr:active_model')} — {OCR_MODEL_SERIES}系列
          </label>
          <select
            value={store.activeTier}
            onChange={(e) => handleTierChange(e.target.value)}
            disabled={loadingStatus || store.isScanning}
            style={{
              width: '100%',
              padding: '8px 10px',
              fontSize: 13,
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
            }}
          >
            {tiers.map((tier) => (
              <option key={tier.tier} value={tier.tier}>
                {tier.name} — {tier.description}
                {!statusMap[tier.tier]?.installed ? ` (${t('ocr:status_not_installed')})` : ''}
              </option>
            ))}
          </select>
        </div>

        {/* Mode toggle */}
        <div
          style={{
            display: 'inline-flex',
            gap: 4,
            padding: 4,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            alignSelf: 'center',
          }}
        >
          <button
            onClick={() => store.setScanMode('general')}
            disabled={store.isScanning}
            style={{
              padding: '6px 14px',
              borderRadius: 6,
              border: 'none',
              fontSize: 13,
              cursor: 'pointer',
              background: store.scanMode === 'general' ? 'var(--bg-elevated)' : 'transparent',
              color: 'var(--text-primary)',
              fontWeight: store.scanMode === 'general' ? 600 : 400,
              opacity: store.isScanning ? 0.6 : 1,
            }}
          >
            {t('ocr:scan_mode_general')}
          </button>
          <button
            onClick={() => store.setScanMode('mrz')}
            disabled={store.isScanning}
            style={{
              padding: '6px 14px',
              borderRadius: 6,
              border: 'none',
              fontSize: 13,
              cursor: 'pointer',
              background: store.scanMode === 'mrz' ? 'var(--bg-elevated)' : 'transparent',
              color: 'var(--text-primary)',
              fontWeight: store.scanMode === 'mrz' ? 600 : 400,
              opacity: store.isScanning ? 0.6 : 1,
            }}
          >
            {t('ocr:scan_mode_mrz')}
          </button>
        </div>

        {/* Scan action area */}
        <div style={{ textAlign: 'center', padding: '8px 0' }}>
          <button
            onClick={handleSelectFile}
            disabled={store.isScanning}
            style={{
              padding: '10px 20px',
              borderRadius: 10,
              border: 'none',
              background: 'var(--accent-primary)',
              color: 'white',
              fontSize: 14,
              fontWeight: 500,
              cursor: store.isScanning ? 'not-allowed' : 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
              gap: 8,
              opacity: store.isScanning ? 0.7 : 1,
            }}
          >
            <FileText size={16} />
            {store.scanMode === 'mrz'
              ? t('ocr:select_image')
              : t('ocr:select_image_or_pdf')}
          </button>
        </div>

        {/* Scanning state */}
        {store.isScanning && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 10,
              padding: 16,
              color: 'var(--text-secondary)',
              background: 'var(--bg-toolbar)',
              borderRadius: 10,
            }}
          >
            <Loader2 size={18} className="spin" />
            <span style={{ fontSize: 13 }}>{t('ocr:scanning')}</span>
          </div>
        )}

        {/* Error state */}
        {store.lastScanError && !store.isScanning && (
          <div
            style={{
              padding: 12,
              borderRadius: 10,
              background: 'rgba(231,76,60,0.08)',
              border: '1px solid rgba(231,76,60,0.2)',
              color: '#e74c3c',
              fontSize: 13,
              display: 'flex',
              alignItems: 'center',
              gap: 8,
            }}
          >
            <AlertCircle size={16} />
            {store.lastScanError}
          </div>
        )}

        {/* Current result */}
        {currentEntry && !store.isScanning && (
          <>
            {/* General OCR result */}
            {currentEntry.mode === 'general' && currentEntry.result && (
              <div
                style={{
                  padding: 12,
                  borderRadius: 10,
                  background: 'var(--bg-toolbar)',
                  fontSize: 13,
                  lineHeight: 1.6,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 200,
                  overflowY: 'auto',
                  color: 'var(--text-primary)',
                }}
              >
                {currentEntry.result.text || t('ocr:no_text')}
              </div>
            )}

            {/* MRZ result */}
            {currentEntry.mode === 'mrz' && currentEntry.mrzResult && (
              <MrzResultCard result={currentEntry.mrzResult} />
            )}

            {/* MRZ not detected fallback */}
            {currentEntry.mode === 'mrz' && !currentEntry.mrzResult && !currentEntry.error && (
              <div
                style={{
                  padding: 12,
                  borderRadius: 10,
                  background: 'var(--bg-toolbar)',
                  fontSize: 13,
                  color: 'var(--text-secondary)',
                  textAlign: 'center',
                }}
              >
                {t('ocr:mrz_no_detected')}
              </div>
            )}
          </>
        )}

        {/* Empty state hint */}
        {!store.isScanning && !currentEntry && (
          <div
            style={{
              textAlign: 'center',
              padding: '24px 8px',
              color: 'var(--text-tertiary)',
              fontSize: 13,
            }}
          >
            <Scan size={32} style={{ marginBottom: 8, opacity: 0.3 }} />
            <p style={{ margin: 1 }}>{t('ocr:quick_scan_hint')}</p>
          </div>
        )}
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
      `}</style>
    </div>
  );
}
