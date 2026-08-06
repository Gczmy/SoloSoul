import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import type { TrashConfirmAction } from './types';

interface TrashConfirmDialogProps {
  action: TrashConfirmAction;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

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
    <>
      <div
        style={{
          position: 'fixed',
          inset: 0,
          background: 'var(--bg-overlay)',
          zIndex: 'var(--z-modal-important)',
        }}
        onClick={submitting ? undefined : onClose}
      />
      <div
        style={{
          position: 'fixed',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: '90%',
          maxWidth: 360,
          zIndex: 'var(--z-modal-important)',
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: '24px 28px',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <h3 style={{ fontSize: 'var(--text-section-title)', fontWeight: 600, margin: '0 0 8px' }}>
          {action.type === 'delete'
            ? t('settings:confirm_delete_title')
            : t('settings:confirm_restore_title')}
        </h3>
        <p
          style={{
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            marginBottom: 20,
            lineHeight: 1.5,
          }}
        >
          {action.type === 'delete'
            ? action.pageChildCount !== undefined
              ? t('settings:trash_delete_page_with_children_desc', {
                  count: action.pageChildCount,
                })
              : t('settings:confirm_delete_desc', { count: action.count })
            : t('settings:confirm_restore_desc', { count: action.count })}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onClose} disabled={submitting}>
            {t('common:cancel')}
          </Button>
          <Button
            variant={action.type === 'delete' ? 'danger-outline' : 'primary'}
            onClick={handleConfirm}
            disabled={submitting}
          >
            {submitting ? t('common:loading') : action.type === 'delete' ? t('common:delete_permanently') : t('common:restore')}
          </Button>
        </div>
      </div>
    </>
  );
}
