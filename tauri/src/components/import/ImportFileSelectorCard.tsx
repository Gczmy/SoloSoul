import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import type { DecryptedImportPreview, ImportPreview } from '@/types/exportImport';

/**
 * ImportSection 的「选择文件」卡片（P046 拆分：展示子组件）。
 */
export function ImportFileSelectorCard({
  importPath,
  importPreview,
  isPreviewing,
  onSetImportPath,
  onSetImportPreview,
  onSetDecryptedPreview,
  onSetImportPw,
  onSetShowStrategySelector,
  onPreview,
  t,
}: {
  importPath: string;
  importPreview: ImportPreview | null;
  isPreviewing: boolean;
  onSetImportPath: (v: string) => void;
  onSetImportPreview: (v: ImportPreview | null) => void;
  onSetDecryptedPreview: (v: DecryptedImportPreview | null) => void;
  onSetImportPw: (v: string) => void;
  onSetShowStrategySelector: (v: boolean) => void;
  onPreview: () => void;
  t: TFunction;
}) {
  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
        {t('settings:select_file')}
      </h3>
      <div
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          marginBottom: 8,
          // Android content:// URI 很长，折行防止溢出卡片
          wordBreak: 'break-all',
        }}
      >
        {importPath || t('settings:no_file_selected')}
      </div>
      <TransferButton
        onClick={async () => {
          const { openWithPause } = await import('@/lib/dialog');
          const selected = await openWithPause({
            filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
            multiple: false,
          });
          if (selected) {
            onSetImportPath(selected as string);
            onSetImportPreview(null);
            onSetDecryptedPreview(null);
            onSetImportPw('');
            onSetShowStrategySelector(false);
          }
        }}
      >
        {t('settings:select_file')}
      </TransferButton>
      {importPath && !importPreview && (
        <div style={{ marginTop: 8 }}>
          <TransferButton onClick={onPreview} disabled={isPreviewing} busy={isPreviewing}>
            {isPreviewing ? t('common:loading', { defaultValue: '...' }) : t('settings:preview')}
          </TransferButton>
        </div>
      )}
    </Card>
  );
}
