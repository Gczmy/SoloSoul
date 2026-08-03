import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Scan } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

/** 扫描模式类型：本组件为规范出处（OcrPage 经 ScanDropZone 导入使用）。 */
export type ScanMode = 'general' | 'mrz';

interface ScanDropZoneProps {
  scanMode: ScanMode;
  onScanModeChange: (mode: ScanMode) => void;
  isScanning: boolean;
  isMobilePlatform: boolean;
  activeTier: string;
  onSelectFile: () => void;
  onTakePhoto: () => void;
}

/**
 * 扫描入口面板：模式切换（通用/MRZ）+ 选择文件/拍照。
 * 数据与回调经 OcrPage 透传（P224-⑤ 拆分）。
 */
export function ScanDropZone({
  scanMode,
  onScanModeChange,
  isScanning,
  isMobilePlatform,
  activeTier,
  onSelectFile,
  onTakePhoto,
}: ScanDropZoneProps) {
  const { t } = useTranslation(['ocr', 'common']);
  // Scan mode + action
  return (
    <Card>
      <div style={{ textAlign: 'center', padding: 24 }}>
        <Scan
          size={ICON_SIZE['5xl']}
          style={{ marginBottom: 12, opacity: 0.3, color: 'var(--text-tertiary)' }}
        />
        <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 600, marginBottom: 4 }}>
          {t('ocr:title')}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 16,
          }}
        >
          {t('ocr:description')}
        </p>

        {/* Mode toggle */}
        <div
          style={{
            display: 'inline-flex',
            gap: 4,
            padding: 4,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            marginBottom: 16,
          }}
        >
          <button
            onClick={() => onScanModeChange('general')}
            style={{
              padding: '6px 14px',
              borderRadius: 6,
              border:
                scanMode === 'general'
                  ? '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)'
                  : '1px solid transparent',
              fontSize: 'var(--text-body-sm)',
              cursor: 'pointer',
              background:
                scanMode === 'general'
                  ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                  : 'transparent',
              color: scanMode === 'general' ? 'var(--accent-primary)' : 'var(--text-secondary)',
              fontWeight: scanMode === 'general' ? 600 : 400,
            }}
          >
            {t('ocr:scan_mode_general')}
          </button>
          <button
            onClick={() => onScanModeChange('mrz')}
            style={{
              padding: '6px 14px',
              borderRadius: 6,
              border:
                scanMode === 'mrz'
                  ? '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)'
                  : '1px solid transparent',
              fontSize: 'var(--text-body-sm)',
              cursor: 'pointer',
              background:
                scanMode === 'mrz'
                  ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                  : 'transparent',
              color: scanMode === 'mrz' ? 'var(--accent-primary)' : 'var(--text-secondary)',
              fontWeight: scanMode === 'mrz' ? 600 : 400,
            }}
          >
            {t('ocr:scan_mode_mrz')}
          </button>
        </div>

        <br />

        <div style={{ display: 'flex', gap: 8, justifyContent: 'center', alignItems: 'center' }}>
          <Button onClick={onSelectFile} loading={isScanning}>
            {scanMode === 'mrz' || isMobilePlatform || activeTier === 'vision'
              ? t('ocr:select_image')
              : t('ocr:select_image_or_pdf')}
          </Button>
          {isMobilePlatform && (
            <Button onClick={onTakePhoto} loading={isScanning}>
              {t('ocr:take_photo')}
            </Button>
          )}
        </div>
      </div>
    </Card>
  );
}
