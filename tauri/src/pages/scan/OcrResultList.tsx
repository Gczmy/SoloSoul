import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Loader2, Upload } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { MrzResultCard } from '@/components/ocr/MrzResultCard';
import { PromptDialog } from '@/components/ui/PromptDialog';
import type { MrzResult, OcrResult } from '@/lib/ipc';

interface OcrResultListProps {
  result: OcrResult | null;
  mrzResult: MrzResult | null;
  isScanning: boolean;
  isImporting: boolean;
  isNameDialogOpen: boolean;
  importNameDefault: string;
  onImportAsObject: (source: 'ocr' | 'mrz') => void;
  onConfirmImport: (name: string) => void;
  onCancelImport: () => void;
}

/**
 * 扫描结果面板：扫描中指示 + 通用 OCR/MRZ 结果卡片 + 导入命名对话框。
 * 数据与回调经 OcrPage 透传（P224-⑤ 拆分）。
 */
export function OcrResultList({
  result,
  mrzResult,
  isScanning,
  isImporting,
  isNameDialogOpen,
  importNameDefault,
  onImportAsObject,
  onConfirmImport,
  onCancelImport,
}: OcrResultListProps) {
  const { t } = useTranslation(['ocr', 'common']);
  return (
    <>
      {isScanning && (
        <Card>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 10,
              padding: 24,
              color: 'var(--text-secondary)',
            }}
          >
            <Loader2 size={ICON_SIZE.lg} className="spin" />
            <span>{t('ocr:scanning')}</span>
          </div>
        </Card>
      )}

      {/* General OCR result */}
      {result && (
        <Card>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 12,
            }}
          >
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{t('ocr:result_title')}</h3>
            <Button size="sm" onClick={() => onImportAsObject('ocr')} loading={isImporting}>
              <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('ocr:import_as_object')}
            </Button>
          </div>

          {result.boxes.length > 1 && (
            <div style={{ marginBottom: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {result.boxes.map((box, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    gap: 12,
                    padding: '8px 10px',
                    borderRadius: 6,
                    background: 'var(--bg-toolbar)',
                    fontSize: 'var(--text-body-sm)',
                  }}
                >
                  <span style={{ flex: 1, wordBreak: 'break-word' }}>{box.text}</span>
                  <span
                    style={{
                      fontSize: 'var(--text-badge)',
                      color: 'var(--text-tertiary)',
                      flexShrink: 0,
                    }}
                  >
                    {(box.confidence * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
            </div>
          )}

          {result.text && (
            <div
              style={{
                padding: 12,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                fontSize: 'var(--text-body-sm)',
                lineHeight: 1.6,
                whiteSpace: 'pre-wrap',
                maxHeight: 300,
                overflowY: 'auto',
              }}
            >
              {result.text}
            </div>
          )}

          {result.boxes.length === 0 && !result.text && (
            <p
              style={{
                textAlign: 'center',
                color: 'var(--text-tertiary)',
                padding: 24,
                fontSize: 'var(--text-body-sm)',
              }}
            >
              {t('ocr:no_text')}
            </p>
          )}
        </Card>
      )}

      {/* MRZ result */}
      {mrzResult && (
        <Card>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 12,
            }}
          >
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
              {t('ocr:mrz_result_title')}
            </h3>
            <Button size="sm" onClick={() => onImportAsObject('mrz')} loading={isImporting}>
              <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('ocr:import_as_object')}
            </Button>
          </div>
          <MrzResultCard result={mrzResult} />
        </Card>
      )}

      <PromptDialog
        isOpen={isNameDialogOpen}
        title={t('ocr:import_name_dialog_title')}
        defaultValue={importNameDefault}
        placeholder={t('ocr:import_name_placeholder')}
        confirmLabel={t('common:confirm')}
        cancelLabel={t('common:cancel')}
        onConfirm={onConfirmImport}
        onCancel={onCancelImport}
      />
    </>
  );
}
