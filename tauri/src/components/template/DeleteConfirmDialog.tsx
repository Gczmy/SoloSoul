import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';

interface DeleteConfirmDialogProps {
  name: string;
  title: string;
  body: string;
  onCancel: () => void;
  onConfirm: () => void;
}

export function DeleteConfirmDialog({
  name,
  title,
  body,
  onCancel,
  onConfirm,
}: DeleteConfirmDialogProps) {
  const { t } = useTranslation(['common']);

  return (
    <div        style={{
          position: 'fixed',
          inset: 0,
          zIndex: 9999,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'var(--bg-overlay)',
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
        <h3 style={{ margin: '0 0 8px', fontSize: 'var(--text-section-title)', fontWeight: 600 }}>{title}</h3>
        <p
          style={{
            margin: '0 0 20px',
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
          }}
        >
          {body.replace('{name}', name)}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onCancel}>
            {t('common:cancel') || '取消'}
          </Button>
          <Button variant="danger-outline" onClick={onConfirm}>
            {t('common:delete') || '删除'}
          </Button>
        </div>
      </div>
    </div>
  );
}
