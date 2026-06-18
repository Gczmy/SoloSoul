import { useTranslation } from 'react-i18next';
import { FileText, Loader2 } from 'lucide-react';
import { type OcrTierInfo, type OcrModelStatus } from '@/lib/ipc';
import { OCR_MODEL_SERIES } from '@/lib/constants';
import { getTierLabel } from '@/lib/ocr';

interface OcrScanControlsProps {
  activeTier: string;
  scanMode: 'general' | 'mrz';
  isScanning: boolean;
  loadingStatus: boolean;
  tiers: OcrTierInfo[];
  statusMap: Record<string, OcrModelStatus>;
  onTierChange: (tier: string) => void;
  onScanModeChange: (mode: 'general' | 'mrz') => void;
  onSelectFile: () => void;
}

export function OcrScanControls({
  activeTier,
  scanMode,
  isScanning,
  loadingStatus,
  tiers,
  statusMap,
  onTierChange,
  onScanModeChange,
  onSelectFile,
}: OcrScanControlsProps) {
  const { t } = useTranslation(['ocr', 'common']);

  return (
    <>
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
          {t('ocr:active_model_series', { model: OCR_MODEL_SERIES })}
        </label>
        <select
          value={activeTier}
          onChange={(e) => onTierChange(e.target.value)}
          disabled={loadingStatus || isScanning}
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
          {tiers.map((tier) => {
            const label = getTierLabel(t, tier);
            return (
              <option key={tier.tier} value={tier.tier}>
                {label.name} — {label.description}
                {!statusMap[tier.tier]?.installed ? ` (${t('ocr:status_not_installed')})` : ''}
              </option>
            );
          })}
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
          onClick={() => onScanModeChange('general')}
          disabled={isScanning}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            fontSize: 13,
            cursor: 'pointer',
            background: scanMode === 'general' ? 'var(--bg-elevated)' : 'transparent',
            color: 'var(--text-primary)',
            fontWeight: scanMode === 'general' ? 600 : 400,
            opacity: isScanning ? 0.6 : 1,
          }}
        >
          {t('ocr:scan_mode_general')}
        </button>
        <button
          onClick={() => onScanModeChange('mrz')}
          disabled={isScanning}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            fontSize: 13,
            cursor: 'pointer',
            background: scanMode === 'mrz' ? 'var(--bg-elevated)' : 'transparent',
            color: 'var(--text-primary)',
            fontWeight: scanMode === 'mrz' ? 600 : 400,
            opacity: isScanning ? 0.6 : 1,
          }}
        >
          {t('ocr:scan_mode_mrz')}
        </button>
      </div>

      {/* Scan action area */}
      <div style={{ textAlign: 'center', padding: '8px 0' }}>
        <button
          onClick={onSelectFile}
          disabled={isScanning}
          style={{
            padding: '10px 20px',
            borderRadius: 10,
            border: 'none',
            background: 'var(--accent-primary)',
            color: 'white',
            fontSize: 14,
            fontWeight: 500,
            cursor: isScanning ? 'not-allowed' : 'pointer',
            display: 'inline-flex',
            alignItems: 'center',
            gap: 8,
            opacity: isScanning ? 0.7 : 1,
          }}
        >
          <FileText size={16} />
          {scanMode === 'mrz'
            ? t('ocr:select_image')
            : t('ocr:select_image_or_pdf')}
        </button>
      </div>

      {/* Scanning state */}
      {isScanning && (
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
    </>
  );
}
