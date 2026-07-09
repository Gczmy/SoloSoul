import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import type {
  TemplateSyncResult,
  SyncFieldInfo,
  SyncFieldChange,
  SyncFieldIncompatible,
} from '@/lib/templateSync';

interface TemplateSyncConfirmDialogProps {
  isOpen: boolean;
  result: TemplateSyncResult | null;
  loading?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function FieldList({
  fields,
  getFieldTypeLabel,
}: {
  fields: SyncFieldInfo[];
  getFieldTypeLabel: (type: string) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{f.name}</span>
          <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
            {' '}
            ({getFieldTypeLabel(f.fieldType)})
          </span>
        </li>
      ))}
    </ul>
  );
}

function UpdatedFieldList({
  fields,
  getFieldTypeLabel,
}: {
  fields: SyncFieldChange[];
  getFieldTypeLabel: (type: string) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{f.name}</span>
          <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
            {' '}
            ({getFieldTypeLabel(f.fieldType)})
          </span>
          <span style={{ display: 'block', fontSize: 'var(--text-caption)' }}>
            {f.changes.join(' · ')}
          </span>
        </li>
      ))}
    </ul>
  );
}

function IncompatibleFieldList({
  fields,
  getFieldTypeLabel,
}: {
  fields: SyncFieldIncompatible[];
  getFieldTypeLabel: (type: string) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{f.name}</span>
          <span style={{ display: 'block', fontSize: 'var(--text-caption)' }}>
            {getFieldTypeLabel(f.oldType)} → {getFieldTypeLabel(f.newType)}
          </span>
          {f.oldValuePreview && (
            <span style={{ display: 'block', fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {f.oldValuePreview}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

export function TemplateSyncConfirmDialog({
  isOpen,
  result,
  loading,
  onConfirm,
  onCancel,
}: TemplateSyncConfirmDialogProps) {
  const { t } = useTranslation(['editor', 'common']);

  const getFieldTypeLabel = (type: string) =>
    t(`editor:field_types.${type}` as const, { defaultValue: type });

  const sectionTitleStyle: React.CSSProperties = {
    fontSize: 'var(--text-body-sm)',
    fontWeight: 600,
    color: 'var(--text-primary)',
    margin: '12px 0 6px',
  };

  const noChanges = result ? !result.hasChanges : false;

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onCancel}
      title={t('editor:template_sync_title')}
      dialogStyle={{ maxWidth: 480, width: '90%' }}
    >
      <div style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body)', lineHeight: 1.5 }}>
        <p style={{ margin: '0 0 12px' }}>{t('editor:template_sync_body')}</p>

        {!result && (
          <p style={{ margin: '8px 0', color: 'var(--text-tertiary)' }}>
            {t('editor:template_sync_loading')}
          </p>
        )}

        {noChanges && (
          <p style={{ margin: '8px 0', color: 'var(--text-tertiary)' }}>
            {t('editor:template_sync_no_changes')}
          </p>
        )}

        {result && !noChanges && (
          <div
            style={{
              maxHeight: '50vh',
              overflowY: 'auto',
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              padding: '12px 16px',
              background: 'var(--bg-toolbar)',
            }}
          >
            {result.fieldsAdded.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_added')}</p>
                <FieldList fields={result.fieldsAdded} getFieldTypeLabel={getFieldTypeLabel} />
              </>
            )}
            {result.fieldsDeprecated.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_deprecated')}</p>
                <FieldList fields={result.fieldsDeprecated} getFieldTypeLabel={getFieldTypeLabel} />
              </>
            )}
            {result.fieldsUpdated.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_updated')}</p>
                <UpdatedFieldList
                  fields={result.fieldsUpdated}
                  getFieldTypeLabel={getFieldTypeLabel}
                />
              </>
            )}
            {result.fieldsIncompatible.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_incompatible')}</p>
                <IncompatibleFieldList
                  fields={result.fieldsIncompatible}
                  getFieldTypeLabel={getFieldTypeLabel}
                />
                <p style={{ margin: '8px 0 0', fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {t('editor:template_sync_incompatible_hint')}
                </p>
              </>
            )}
          </div>
        )}
      </div>

      <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 20 }}>
        <Button variant="secondary" onClick={onCancel} disabled={loading}>
          {t('common:cancel')}
        </Button>
        <Button variant="primary" onClick={onConfirm} loading={loading} disabled={!result || noChanges}>
          {t('editor:template_sync_apply')}
        </Button>
      </div>
    </Dialog>
  );
}
