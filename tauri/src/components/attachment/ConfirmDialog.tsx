import { Button } from '@/components/ui/Button';
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
 * Reusable confirmation dialog overlaid on the screen.
 * Used for batch delete/restore and single permanent delete confirmations.
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
  if (!open) return null;

  // Remove unused confirmBg/confirmHoverBg — now using Button variant="danger-outline" or "primary"

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal-important)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={(e) => {
        e.stopPropagation();
        onCancel();
      }}
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
        <div
          style={{
            margin: '0 0 20px',
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
          }}
        >
          {body}
        </div>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button
            variant={confirmStyle === 'danger' ? 'danger-outline' : 'primary'}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
