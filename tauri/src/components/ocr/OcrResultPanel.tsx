import { useTranslation } from 'react-i18next';
import { FileText, AlertCircle, Scan } from 'lucide-react';
import type { OcrScanEntry } from '@/stores/ocrScanStore';
import { MrzResultCard } from '@/components/ocr/MrzResultCard';
import { ICON_SIZE } from '@/lib/iconSizes';

interface OcrResultPanelProps {
  currentEntry: OcrScanEntry | null;
  isScanning: boolean;
  lastScanError: string | null;
}

export function OcrResultPanel({ currentEntry, isScanning, lastScanError }: OcrResultPanelProps) {
  const { t } = useTranslation(['ocr', 'common']);

  if (isScanning) return null;

  if (!currentEntry) {
    return (
      <div
        style={{
          textAlign: 'center',
          padding: '24px 8px',
          color: 'var(--text-tertiary)',
          fontSize: 'var(--text-body-sm)',
        }}
      >
        <Scan size={ICON_SIZE['3xl']} style={{ marginBottom: 8, opacity: 1.3 }} />
        <p style={{ margin: 1 }}>{t('ocr:quick_scan_hint')}</p>
      </div>
    );
  }

  return (
    <div className="ocr-result-container">
      {/* Result header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '6px 8px',
          borderRadius: 6,
          background: 'rgba(91,124,153,0.06)',
          fontSize: 'var(--text-badge)',
          color: 'var(--text-tertiary)',
          lineHeight: 1.4,
        }}
      >
        <FileText size={ICON_SIZE['2xs']} style={{ flexShrink: 0 }} />
        <span
          style={{
            flex: 1,
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
          title={currentEntry.fileName}
        >
          {currentEntry.fileName}
        </span>
        <span style={{ flexShrink: 0 }}>{currentEntry.mode === 'mrz' ? 'MRZ' : 'OCR'}</span>
      </div>

      {/* Last transient error (if different from current entry error) */}
      {lastScanError && lastScanError !== currentEntry.error && (
        <div
          style={{
            padding: 12,
            borderRadius: 10,
            background: 'rgba(231,76,60,0.08)',
            border: '1px solid rgba(231,76,60,0.2)',
            color: '#e74c3c',
            fontSize: 'var(--text-body-sm)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <AlertCircle size={ICON_SIZE.md} />
          {lastScanError}
        </div>
      )}

      {/* Current entry error */}
      {currentEntry.error && (
        <div
          style={{
            padding: 12,
            borderRadius: 10,
            background: 'rgba(231,76,60,0.08)',
            border: '1px solid rgba(231,76,60,0.2)',
            color: '#e74c3c',
            fontSize: 'var(--text-body-sm)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <AlertCircle size={ICON_SIZE.md} />
          {currentEntry.error}
        </div>
      )}

      {/* General OCR result */}
      {!currentEntry.error && currentEntry.mode === 'general' && currentEntry.result && (
        <div
          style={{
            padding: 12,
            borderRadius: 10,
            background: 'var(--bg-toolbar)',
            fontSize: 'var(--text-body-sm)',
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
      {!currentEntry.error && currentEntry.mode === 'mrz' && currentEntry.mrzResult && (
        <MrzResultCard result={currentEntry.mrzResult} />
      )}

      {/* MRZ fallback: general OCR text when no MRZ detected */}
      {!currentEntry.error && currentEntry.mode === 'mrz' && !currentEntry.mrzResult && (
        <>
          {currentEntry.result ? (
            <>
              <div
                style={{
                  padding: '6px 10px',
                  borderRadius: 6,
                  background: 'rgba(41,128,185,0.08)',
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  textAlign: 'center',
                }}
              >
                {t('ocr:mrz_no_detected')}
              </div>
              <div
                style={{
                  padding: 12,
                  borderRadius: 10,
                  background: 'var(--bg-toolbar)',
                  fontSize: 'var(--text-body-sm)',
                  lineHeight: 1.6,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 200,
                  overflowY: 'auto',
                  color: 'var(--text-primary)',
                }}
              >
                {currentEntry.result.text || t('ocr:no_text')}
              </div>
            </>
          ) : (
            <div
              style={{
                padding: 12,
                borderRadius: 10,
                background: 'var(--bg-toolbar)',
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                textAlign: 'center',
              }}
            >
              {t('ocr:mrz_no_detected')}
            </div>
          )}
        </>
      )}
    </div>
  );
}
