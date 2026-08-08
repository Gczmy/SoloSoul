import { useTranslation } from 'react-i18next';
import { Download, RotateCw } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentBatchToolbarProps {
  showTrash: boolean;
  allSelected: boolean;
  selectedCount: number;
  onToggleSelectAll: () => void;
  onBatchDownload: () => void;
  onBatchDelete: () => void;
  onBatchRestore: () => void;
  onBatchPermanentDelete: () => void;
}

/** AttachmentViewer 批量操作工具栏（常驻显示，选中项出现时展开操作按钮）（P013 拆分）。 */
export function AttachmentBatchToolbar({
  showTrash,
  allSelected,
  selectedCount,
  onToggleSelectAll,
  onBatchDownload,
  onBatchDelete,
  onBatchRestore,
  onBatchPermanentDelete,
}: AttachmentBatchToolbarProps) {
  const { t } = useTranslation('common');
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '8px 12px',
        borderBottom: '1px solid var(--border-subtle)',
        background:
          selectedCount > 0
            ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
            : 'var(--bg-toolbar)',
        fontSize: 'var(--text-body-sm)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <SelectCheckbox
          checked={allSelected}
          onClick={onToggleSelectAll}
          indeterminate={selectedCount > 0 && !allSelected}
        />
        <span
          style={{ color: 'var(--text-secondary)', cursor: 'pointer', userSelect: 'none' }}
          onClick={onToggleSelectAll}
        >
          {allSelected ? t('deselect_all') : t('select_all')}
        </span>
        {selectedCount > 0 && (
          <span style={{ color: 'var(--text-tertiary)' }}>
            {t('selected_count', { n: selectedCount })}
          </span>
        )}
      </div>
      {selectedCount > 0 && !showTrash ? (
        <div style={{ display: 'flex', gap: 6 }}>
          <Button variant="secondary" size="sm" onClick={onBatchDownload}>
            <Download size={ICON_SIZE.sm} /> {t('download')}
          </Button>
          <DeleteButton onClick={onBatchDelete} title={t('delete')}>
            {t('delete')}
          </DeleteButton>
        </div>
      ) : selectedCount > 0 && showTrash ? (
        <div style={{ display: 'flex', gap: 6 }}>
          <Button variant="secondary" size="sm" onClick={onBatchRestore}>
            <RotateCw size={ICON_SIZE.sm} /> {t('restore')}
          </Button>
          <DeleteButton onClick={onBatchPermanentDelete} title={t('delete_permanently')}>
            {t('delete_permanently')}
          </DeleteButton>
        </div>
      ) : null}
    </div>
  );
}
