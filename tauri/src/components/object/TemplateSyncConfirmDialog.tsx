import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import type {
  TemplateSyncResult,
  SyncFieldInfo,
  SyncFieldChange,
  SyncFieldChangeItem,
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
  getFieldNameLabel,
  getFieldTypeLabel,
}: {
  fields: SyncFieldInfo[];
  getFieldNameLabel: (name: string) => string;
  getFieldTypeLabel: (type: string) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{getFieldNameLabel(f.name)}</span>
          <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
            {' '}
            ({getFieldTypeLabel(f.fieldType)})
          </span>
        </li>
      ))}
    </ul>
  );
}

function formatChangeItem(
  item: SyncFieldChangeItem,
  t: (key: string, options?: Record<string, string | number>) => string,
  getFieldNameLabel: (name: string) => string,
  getFieldTypeLabel: (type: string) => string,
  getSensitivityLabel: (level: string) => string,
): string {
  switch (item.kind) {
    case 'type': {
      const p = item.payload as { oldType: string; newType: string };
      return t('editor:template_sync_change_type', {
        old: getFieldTypeLabel(p.oldType),
        new: getFieldTypeLabel(p.newType),
      });
    }
    case 'name': {
      const p = item.payload as { oldName: string; newName: string };
      return t('editor:template_sync_change_name', {
        old: getFieldNameLabel(p.oldName),
        new: getFieldNameLabel(p.newName),
      });
    }
    case 'sensitivity': {
      const p = item.payload as { oldLevel: string; newLevel: string };
      return t('editor:template_sync_change_sensitivity', {
        old: getSensitivityLabel(p.oldLevel),
        new: getSensitivityLabel(p.newLevel),
      });
    }
    case 'options':
      return t('editor:template_sync_change_options');
    case 'metadata': {
      const p = item.payload as { metadataKeys: string[] } | undefined;
      return t('editor:template_sync_change_metadata', {
        keys: p?.metadataKeys?.join(', ') ?? '',
      });
    }
    default:
      return '';
  }
}

function UpdatedFieldList({
  fields,
  getFieldNameLabel,
  getFieldTypeLabel,
  getSensitivityLabel,
  t,
}: {
  fields: SyncFieldChange[];
  getFieldNameLabel: (name: string) => string;
  getFieldTypeLabel: (type: string) => string;
  getSensitivityLabel: (level: string) => string;
  t: (key: string, options?: Record<string, string | number>) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{getFieldNameLabel(f.name)}</span>
          <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
            {' '}
            ({getFieldTypeLabel(f.fieldType)})
          </span>
          <span style={{ display: 'block', fontSize: 'var(--text-caption)' }}>
            {f.changes
              .map((c) =>
                formatChangeItem(c, t, getFieldNameLabel, getFieldTypeLabel, getSensitivityLabel),
              )
              .join(' · ')}
          </span>
        </li>
      ))}
    </ul>
  );
}

function IncompatibleFieldList({
  fields,
  getFieldNameLabel,
  getFieldTypeLabel,
}: {
  fields: SyncFieldIncompatible[];
  getFieldNameLabel: (name: string) => string;
  getFieldTypeLabel: (type: string) => string;
}) {
  if (fields.length === 0) return null;
  return (
    <ul style={{ margin: 0, paddingLeft: 18, color: 'var(--text-secondary)' }}>
      {fields.map((f) => (
        <li key={f.id} style={{ marginBottom: 4 }}>
          <span style={{ fontWeight: 500 }}>{getFieldNameLabel(f.name)}</span>
          <span style={{ display: 'block', fontSize: 'var(--text-caption)' }}>
            {getFieldTypeLabel(f.oldType)} → {getFieldTypeLabel(f.newType)}
          </span>
          {f.oldValuePreview && (
            <span
              style={{
                display: 'block',
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
              }}
            >
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

  const getSensitivityLabel = (level: string) =>
    t(`editor:sensitivity_levels.${level}` as const, { defaultValue: level });

  const getFieldNameLabel = (name: string) =>
    name === '__dynamic_group__'
      ? t('editor:field_types.dynamic_group', { defaultValue: '动态字段组' })
      : name;

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
      <div
        style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body)', lineHeight: 1.5 }}
      >
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
                <FieldList
                  fields={result.fieldsAdded}
                  getFieldNameLabel={getFieldNameLabel}
                  getFieldTypeLabel={getFieldTypeLabel}
                />
              </>
            )}
            {result.fieldsDeprecated.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_deprecated')}</p>
                <FieldList
                  fields={result.fieldsDeprecated}
                  getFieldNameLabel={getFieldNameLabel}
                  getFieldTypeLabel={getFieldTypeLabel}
                />
              </>
            )}
            {result.fieldsUpdated.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_updated')}</p>
                <UpdatedFieldList
                  fields={result.fieldsUpdated}
                  getFieldNameLabel={getFieldNameLabel}
                  getFieldTypeLabel={getFieldTypeLabel}
                  getSensitivityLabel={getSensitivityLabel}
                  t={t}
                />
              </>
            )}
            {result.fieldsIncompatible.length > 0 && (
              <>
                <p style={sectionTitleStyle}>{t('editor:template_sync_incompatible')}</p>
                <IncompatibleFieldList
                  fields={result.fieldsIncompatible}
                  getFieldNameLabel={getFieldNameLabel}
                  getFieldTypeLabel={getFieldTypeLabel}
                />
                <p
                  style={{
                    margin: '8px 0 0',
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-tertiary)',
                  }}
                >
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
        <Button
          variant="primary"
          onClick={onConfirm}
          loading={loading}
          disabled={!result || noChanges}
        >
          {t('editor:template_sync_apply')}
        </Button>
      </div>
    </Dialog>
  );
}
