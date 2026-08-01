import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { ICON_SIZE } from '@/lib/constants';
import type { DeprecatedField } from '@/lib/templateSync';

interface DeprecatedFieldsViewerProps {
  isOpen: boolean;
  objectName: string;
  fields: DeprecatedField[];
  onClose: () => void;
}

function renderValue(value: unknown): string {
  if (value === null || value === undefined) return '-';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

export function DeprecatedFieldsViewer({
  isOpen,
  objectName,
  fields,
  onClose,
}: DeprecatedFieldsViewerProps) {
  const { t } = useTranslation(['editor', 'common']);

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('editor:deprecated_fields_title', { name: objectName })}
      dialogStyle={{ maxWidth: 520, width: '90%' }}
    >
      <div style={{ position: 'relative' }}>
        <button
          onClick={onClose}
          className="interactive-icon"
          style={{
            position: 'absolute',
            top: -40,
            right: 0,
            padding: 6,
            borderRadius: 8,
            border: 'none',
            cursor: 'pointer',
          }}
          aria-label={t('common:close')}
        >
          <X size={ICON_SIZE.xl} />
        </button>

        <p
          style={{
            margin: '0 0 12px',
            color: 'var(--text-secondary)',
            fontSize: 'var(--text-body)',
          }}
        >
          {t('editor:deprecated_fields_body')}
        </p>

        {fields.length === 0 ? (
          <p style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-body-sm)' }}>
            {t('editor:deprecated_fields_empty')}
          </p>
        ) : (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 10,
              maxHeight: '50vh',
              overflowY: 'auto',
              paddingRight: 4,
            }}
          >
            {fields.map((f) => (
              <div
                key={f.id}
                style={{
                  padding: '12px 14px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    marginBottom: 6,
                  }}
                >
                  <span style={{ fontWeight: 600, fontSize: 'var(--text-body)' }}>{f.name}</span>
                  <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                    {t(`editor:field_types.${f.fieldType}` as const, { defaultValue: f.fieldType })}
                  </span>
                </div>
                <div
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    fontFamily: 'var(--font-mono, monospace)',
                    wordBreak: 'break-word',
                    marginBottom: 6,
                  }}
                >
                  {renderValue(f.value)}
                </div>
                <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {t('editor:deprecated_fields_archived_at', { date: f.deprecatedAt.slice(0, 10) })}
                  {f.reason && ` · ${f.reason}`}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Dialog>
  );
}
