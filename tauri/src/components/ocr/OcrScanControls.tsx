import { useTranslation } from 'react-i18next';
import { FileText, Loader2 } from 'lucide-react';
import { type OcrTierInfo, type OcrModelStatus } from '@/lib/ipc';
import { OCR_MODEL_SERIES } from '@/lib/constants';
import { getTierLabel } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';

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
            fontSize: 'var(--text-caption)',
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
          onMouseEnter={(e) => {
            if (!e.currentTarget.disabled) {
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.boxShadow =
                '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            }
          }}
          onMouseLeave={(e) => {
            if (document.activeElement !== e.currentTarget) {
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.boxShadow = 'none';
            }
          }}
          onFocus={(e) => {
            e.currentTarget.style.borderColor = 'var(--accent-primary)';
            e.currentTarget.style.boxShadow =
              '0 0 0 2px color-mix(in srgb, var(--accent-primary) 15%, transparent)';
          }}
          onBlur={(e) => {
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
            e.currentTarget.style.boxShadow = 'none';
          }}
          style={{
            width: '100%',
            padding: '8px 10px',
            fontSize: 'var(--text-body-sm)',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            transition: 'border-color 0.2s, box-shadow 0.2s',
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
          onMouseEnter={(e) => {
            if (scanMode !== 'general' && !isScanning) {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background =
              scanMode === 'general'
                ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                : 'transparent';
            e.currentTarget.style.color =
              scanMode === 'general' ? 'var(--accent-primary)' : 'var(--text-primary)';
            e.currentTarget.style.fontWeight = scanMode === 'general' ? '600' : '400';
          }}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
            background:
              scanMode === 'general'
                ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                : 'transparent',
            color: scanMode === 'general' ? 'var(--accent-primary)' : 'var(--text-primary)',
            fontWeight: scanMode === 'general' ? 600 : 400,
            opacity: isScanning ? 0.6 : 1,
            transition: 'background 0.15s, color 0.15s',
          }}
        >
          {t('ocr:scan_mode_general')}
        </button>
        <button
          onClick={() => onScanModeChange('mrz')}
          disabled={isScanning}
          onMouseEnter={(e) => {
            if (scanMode !== 'mrz' && !isScanning) {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background =
              scanMode === 'mrz'
                ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                : 'transparent';
            e.currentTarget.style.color =
              scanMode === 'mrz' ? 'var(--accent-primary)' : 'var(--text-primary)';
            e.currentTarget.style.fontWeight = scanMode === 'mrz' ? '600' : '400';
          }}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
            background:
              scanMode === 'mrz'
                ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                : 'transparent',
            color: scanMode === 'mrz' ? 'var(--accent-primary)' : 'var(--text-primary)',
            fontWeight: scanMode === 'mrz' ? 600 : 400,
            opacity: isScanning ? 0.6 : 1,
            transition: 'background 0.15s, color 0.15s',
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
          onMouseEnter={(e) => {
            if (!isScanning) {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'var(--bg-toolbar)';
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
          }}
          style={{
            padding: '10px 20px',
            borderRadius: 10,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body)',
            fontWeight: 500,
            cursor: isScanning ? 'not-allowed' : 'pointer',
            display: 'inline-flex',
            alignItems: 'center',
            gap: 8,
            opacity: isScanning ? 0.7 : 1,
            transition: 'background 0.2s, border-color 0.2s, opacity 0.15s ease',
          }}
        >
          <FileText size={ICON_SIZE.md} />
          {scanMode === 'mrz' ? t('ocr:select_image') : t('ocr:select_image_or_pdf')}
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
          <Loader2 size={ICON_SIZE.lg} className="spin" />
          <span style={{ fontSize: 'var(--text-body-sm)' }}>{t('ocr:scanning')}</span>
        </div>
      )}
    </>
  );
}
