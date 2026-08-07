import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import type { TrashConfirmAction } from './types';

interface TrashConfirmDialogProps {
  action: TrashConfirmAction;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

/** P040: 收敛到 ui/ConfirmDialog 的薄封装——保留异步提交中态（防重复点击）与 i18n 文案。 */
export function TrashConfirmDialog({ action, onClose, onConfirm }: TrashConfirmDialogProps) {
  const { t } = useTranslation(['settings', 'common']);
  // 提交中态：防重复点击（破坏性操作关键路径，连续点击会并发重复删除）
  const [submitting, setSubmitting] = useState(false);

  const handleConfirm = async () => {
    if (submitting) return;
    setSubmitting(true);
    try {
      await onConfirm();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <ConfirmDialog
      isOpen
      title={
        action.type === 'delete'
          ? t('settings:confirm_delete_title')
          : t('settings:confirm_restore_title')
      }
      message={
        action.type === 'delete'
          ? action.pageChildCount !== undefined
            ? t('settings:trash_delete_page_with_children_desc', {
                count: action.pageChildCount,
              })
            : t('settings:confirm_delete_desc', { count: action.count })
          : t('settings:confirm_restore_desc', { count: action.count })
      }
      confirmLabel={
        action.type === 'delete' ? t('common:delete_permanently') : t('common:restore')
      }
      cancelLabel={t('common:cancel')}
      confirmVariant={action.type === 'delete' ? 'danger-outline' : 'primary'}
      priority="important"
      submitting={submitting}
      submittingLabel={t('common:loading')}
      onConfirm={handleConfirm}
      onCancel={submitting ? () => {} : onClose}
    />
  );
}
