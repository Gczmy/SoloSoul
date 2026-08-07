import { ConfirmDialog as UiConfirmDialog } from '@/components/ui/ConfirmDialog';
import type { ReactNode } from 'react';

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  body: ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  /** 'danger' = red confirm button (delete), 'primary' = accent (restore) */
  confirmStyle?: 'danger' | 'primary';
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * P040: 收敛到 ui/ConfirmDialog 的薄封装——保留 attachment 侧 prop API
 * （open/body/confirmStyle），渲染行为统一走共享 Dialog（portal/滚动锁定/Escape/动画）。
 */
export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel,
  cancelLabel = 'Cancel',
  confirmStyle = 'danger',
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <UiConfirmDialog
      isOpen={open}
      title={title}
      message={body}
      confirmLabel={confirmLabel}
      cancelLabel={cancelLabel}
      confirmVariant={confirmStyle === 'danger' ? 'danger-outline' : 'primary'}
      priority="auth"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
