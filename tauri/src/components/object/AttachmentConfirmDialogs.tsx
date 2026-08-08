import { useTranslation } from 'react-i18next';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { truncateFileName, type AttachmentItem } from '@/lib/attachmentUtils';

interface AttachmentConfirmDialogsProps {
  deleteItem: AttachmentItem | null;
  permDeleteItem: AttachmentItem | null;
  batchDeleteConfirm: boolean;
  batchRestoreConfirm: boolean;
  batchPermanentDeleteConfirm: boolean;
  selectedCount: number;
  onConfirmDelete: () => void;
  onCancelDelete: () => void;
  onConfirmPermanentDelete: () => void;
  onCancelPermanentDelete: () => void;
  onConfirmBatchDelete: () => void;
  onCancelBatchDelete: () => void;
  onConfirmBatchRestore: () => void;
  onCancelBatchRestore: () => void;
  onConfirmBatchPermanentDelete: () => void;
  onCancelBatchPermanentDelete: () => void;
}

/** AttachmentViewer 全部确认对话框（软删/永久删/批量软删/批量恢复/批量永久删）（P013 拆分）。 */
export function AttachmentConfirmDialogs({
  deleteItem,
  permDeleteItem,
  batchDeleteConfirm,
  batchRestoreConfirm,
  batchPermanentDeleteConfirm,
  selectedCount,
  onConfirmDelete,
  onCancelDelete,
  onConfirmPermanentDelete,
  onCancelPermanentDelete,
  onConfirmBatchDelete,
  onCancelBatchDelete,
  onConfirmBatchRestore,
  onCancelBatchRestore,
  onConfirmBatchPermanentDelete,
  onCancelBatchPermanentDelete,
}: AttachmentConfirmDialogsProps) {
  const { t } = useTranslation('common');
  return (
    <>
      <ConfirmDialog
        open={!!deleteItem}
        title={t('confirm_delete_title', 'Delete attachment')}
        body={t('confirm_delete_body', {
          name: deleteItem ? truncateFileName(deleteItem.fileName) : '',
          defaultValue: `Delete "${deleteItem ? truncateFileName(deleteItem.fileName) : ''}"? It will be moved to trash.`,
        })}
        confirmLabel={t('delete')}
        cancelLabel={t('cancel')}
        confirmStyle="danger"
        onConfirm={onConfirmDelete}
        onCancel={onCancelDelete}
      />
      <ConfirmDialog
        open={batchDeleteConfirm}
        title={t('batch_delete_title')}
        body={t('batch_delete_body', { n: selectedCount })}
        confirmLabel={t('delete')}
        cancelLabel={t('cancel')}
        confirmStyle="danger"
        onConfirm={onConfirmBatchDelete}
        onCancel={onCancelBatchDelete}
      />
      <ConfirmDialog
        open={batchRestoreConfirm}
        title={t('batch_restore_title')}
        body={t('batch_restore_body', { n: selectedCount })}
        confirmLabel={t('restore')}
        cancelLabel={t('cancel')}
        confirmStyle="primary"
        onConfirm={onConfirmBatchRestore}
        onCancel={onCancelBatchRestore}
      />
      <ConfirmDialog
        open={batchPermanentDeleteConfirm}
        title={t('batch_perm_delete_title')}
        body={t('batch_perm_delete_body', { n: selectedCount })}
        confirmLabel={t('delete_permanently')}
        cancelLabel={t('cancel')}
        confirmStyle="danger"
        onConfirm={onConfirmBatchPermanentDelete}
        onCancel={onCancelBatchPermanentDelete}
      />
      <ConfirmDialog
        open={!!permDeleteItem}
        title={t('perm_delete_title')}
        body={t('perm_delete_body', {
          name: permDeleteItem ? truncateFileName(permDeleteItem.fileName) : '',
        })}
        confirmLabel={t('delete_permanently')}
        cancelLabel={t('cancel')}
        confirmStyle="danger"
        onConfirm={onConfirmPermanentDelete}
        onCancel={onCancelPermanentDelete}
      />
    </>
  );
}
