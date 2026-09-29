import { ConfirmDialog } from '@/components/ui/ConfirmDialog';

interface ConfirmDeleteDialogProps {
  isOpen: boolean;
  /** Title shown in the dialog heading. */
  title: string;
  /** Fully translated and interpolated body text (caller handles i18n + truncation). */
  body: string;
  onConfirm: () => void;
  onCancel: () => void;
  /** Optional override for the confirm button label. */
  confirmLabel?: string;
  /** Optional override for the cancel button label. */
  cancelLabel?: string;
  submitting?: boolean;
}

/** P040: 收敛到 ui/ConfirmDialog 的薄封装——保留 workspace 侧 prop API 与默认文案。 */
export function ConfirmDeleteDialog({
  isOpen,
  title,
  body,
  onConfirm,
  onCancel,
  confirmLabel,
  cancelLabel,
  submitting,
}: ConfirmDeleteDialogProps) {
  return (
    <ConfirmDialog
      isOpen={isOpen}
      title={title}
      message={body}
      confirmLabel={confirmLabel ?? 'Delete'}
      cancelLabel={cancelLabel ?? 'Cancel'}
      submitting={submitting}
      priority="important"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
