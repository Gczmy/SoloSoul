import { Button } from '@/components/ui/Button';

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
}

export function ConfirmDeleteDialog({
  isOpen,
  title,
  body,
  onConfirm,
  onCancel,
  confirmLabel,
  cancelLabel,
}: ConfirmDeleteDialogProps) {
  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal-important)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.4)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={onCancel}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: '24px 28px',
          maxWidth: 360,
          width: '90%',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <h3 style={{ margin: '0 0 8px', fontSize: 'var(--text-section-title)', fontWeight: 600 }}>
          {title}
        </h3>
        <p
          style={{
            margin: '0 0 20px',
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
          }}
        >
          {body}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onCancel}>
            {cancelLabel || 'Cancel'}
          </Button>
          <Button variant="danger-outline" onClick={onConfirm}>
            {confirmLabel || 'Delete'}
          </Button>
        </div>
      </div>
    </div>
  );
}
