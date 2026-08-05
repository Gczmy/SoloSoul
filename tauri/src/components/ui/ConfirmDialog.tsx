import { useTranslation } from 'react-i18next';
import { Dialog } from './Dialog';
import { Button } from './Button';
import styles from './Dialog.module.css';

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  priority?: 'default' | 'important' | 'auth';
  /** 确认按钮视觉变体（默认 danger-outline，正向流程可用 primary 等）。 */
  confirmVariant?: 'primary' | 'secondary' | 'tertiary' | 'glass' | 'danger' | 'danger-outline';
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
  children,
}: ConfirmDialogProps) {
  const { t } = useTranslation('common');

  return (
    <Dialog isOpen={isOpen} onClose={onCancel} title={title} priority={priority}>
      <p className={styles.message}>{message}</p>
      {children}
      <div className={styles.actions}>
        <Button variant="secondary" onClick={onCancel}>
          {cancelLabel ?? t('cancel', { defaultValue: 'Cancel' })}
        </Button>
        <Button variant={confirmVariant} onClick={onConfirm}>
          {confirmLabel ?? t('confirm', { defaultValue: 'Confirm' })}
        </Button>
      </div>
    </Dialog>
  );
}
