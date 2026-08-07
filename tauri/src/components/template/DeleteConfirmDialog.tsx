import { useTranslation } from 'react-i18next';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';

interface DeleteConfirmDialogProps {
  name: string;
  title: string;
  body: string;
  onCancel: () => void;
  onConfirm: () => void;
}

/** P040: 收敛到 ui/ConfirmDialog 的薄封装——保留 {name} 插值语义与 i18n 按钮文案。 */
export function DeleteConfirmDialog({
  name,
  title,
  body,
  onCancel,
  onConfirm,
}: DeleteConfirmDialogProps) {
  const { t } = useTranslation(['common']);

  return (
    <ConfirmDialog
      isOpen
      title={title}
      message={body.replace('{name}', name)}
      confirmLabel={t('common:delete', { defaultValue: '删除' })}
      cancelLabel={t('common:cancel', { defaultValue: '取消' })}
      priority="important"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
