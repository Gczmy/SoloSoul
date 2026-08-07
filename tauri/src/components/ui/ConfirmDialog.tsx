import { useTranslation } from 'react-i18next';
import { Dialog } from './Dialog';
import { Button } from './Button';
import styles from './Dialog.module.css';

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  /** 提示正文：字符串或任意节点（P040 收敛后各对话框的 ReactNode body 均走此通道）。 */
  message: React.ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  priority?: 'default' | 'important' | 'auth';
  /** 确认按钮视觉变体（默认 danger-outline，正向流程可用 primary 等）。 */
  confirmVariant?: 'primary' | 'secondary' | 'tertiary' | 'glass' | 'danger' | 'danger-outline';
  /** 提交中态：禁用两按钮并拦截背景/Escape 关闭（由调用方在 onCancel 中守卫）。 */
  submitting?: boolean;
  /** 提交中确认按钮的文案（如「加载中」）。 */
  submittingLabel?: string;
  /** 可选插槽：渲染在 message 与按钮之间（如「忘记设备」确认框中的设备信息块）。 */
  children?: React.ReactNode;
}

export function ConfirmDialog({
  isOpen,
  title,
  message,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
  priority = 'default',
  confirmVariant = 'danger-outline',
  submitting = false,
  submittingLabel,
  children,
}: ConfirmDialogProps) {
  const { t } = useTranslation('common');

  return (
    <Dialog isOpen={isOpen} onClose={onCancel} title={title} priority={priority}>
      <p className={styles.message}>{message}</p>
      {children}
      <div className={styles.actions}>
        <Button variant="secondary" onClick={onCancel} disabled={submitting}>
          {cancelLabel ?? t('cancel', { defaultValue: 'Cancel' })}
        </Button>
        <Button variant={confirmVariant} onClick={onConfirm} disabled={submitting}>
          {submitting && submittingLabel
            ? submittingLabel
            : confirmLabel ?? t('confirm', { defaultValue: 'Confirm' })}
        </Button>
      </div>
    </Dialog>
  );
}
