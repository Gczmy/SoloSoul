import type { useTranslation } from 'react-i18next';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';

/** P041: 删除确认对话框——从 ObjectDetailModal 提取（P040 收敛到 ui/ConfirmDialog）。 */
export function ObjectDetailDeleteDialog({
  open,
  objectName,
  deleting,
  t,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  objectName: string;
  deleting: boolean;
  t: ReturnType<typeof useTranslation>['t'];
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <ConfirmDialog
      isOpen={open}
      title={t('common:object_delete_confirm_title')}
      message={t('common:object_delete_confirm_body', {
        name: objectName.length > 28 ? objectName.slice(0, 27) + '…' : objectName,
      })}
      confirmLabel={t('common:delete')}
      cancelLabel={t('common:cancel')}
      priority="important"
      submitting={deleting}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
