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

  return (
    <>
      <div
        style={{ position: 'fixed', inset: 0, background: 'var(--bg-overlay)', zIndex: 99 }}
        onClick={onClose}
      />
      <div
        style={{
          position: 'fixed',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: 340,
          zIndex: 100,
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: 24,
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
        }}
      >
        <h3 style={{ fontSize: 15, fontWeight: 600, margin: '0 0 8px' }}>
          {action.type === 'delete'
            ? t('settings:confirm_delete_title')
            : t('settings:confirm_restore_title')}
        </h3>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
          {action.type === 'delete'
            ? t('settings:confirm_delete_desc', { count: action.count })
            : t('settings:confirm_restore_desc', { count: action.count })}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button size="sm" variant="secondary" onClick={onClose}>
            {t('common:cancel')}
          </Button>
          <Button
            size="sm"
            variant={action.type === 'delete' ? 'danger' : 'primary'}
            onClick={async () => {
              await onConfirm();
            }}
          >
            {action.type === 'delete' ? t('common:delete_permanently') : t('common:restore')}
          </Button>
        </div>
      </div>
    </>
  );
}
