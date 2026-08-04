import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';

/** P041: 删除确认对话框——从 ObjectDetailModal 提取。 */
export function ObjectDetailDeleteDialog({
  open,
  objectName,
  deleting,
  t,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  objectName: string;
  deleting: boolean;
  t: ReturnType<typeof useTranslation>['t'];
  onCancel: () => void;
  onConfirm: () => void;
}) {
  if (!open) return null;
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
      onClick={onCancel}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: '24px 28px',
          maxWidth: 360,
          width: '90%',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <h3 style={{ margin: '0 0 8px', fontSize: 'var(--text-section-title)', fontWeight: 600 }}>
          {t('common:object_delete_confirm_title')}
        </h3>
        <p
          style={{
            margin: '0 0 20px',
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
          }}
        >
          {t('common:object_delete_confirm_body', {
            name: objectName.length > 28 ? objectName.slice(0, 27) + '…' : objectName,
          })}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onCancel}>
            {t('common:cancel')}
          </Button>
          <Button variant="danger-outline" onClick={onConfirm} disabled={deleting}>
            {t('common:delete')}
          </Button>
        </div>
      </div>
    </div>
  );
}
