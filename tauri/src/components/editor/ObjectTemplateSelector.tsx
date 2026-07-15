import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { PluginBadge } from '@/components/template/PluginBadge';
import { resolveCollectionLabel } from '@/lib/utils';
import { LayoutTemplate } from 'lucide-react';
import type { UserTemplate } from '@/types/template';
import type { ObjectData } from '@/stores/objectStore';
import type { CustomPage } from '@/stores/settingsStore';
import { ICON_SIZE } from '@/lib/constants';

interface ObjectTemplateSelectorProps {
  isNew: boolean;
  visibleTemplates: string[];
  selectedType: string;
  onSelect: (type: string) => void;
  templateMeta: Record<string, { category: string; label: string }>;
  userTemplates: UserTemplate[];
  collectionType?: string;
  currentObject?: ObjectData | null;
  contractTypeId?: string;
  customPages: CustomPage[];
  sectionParam?: string;
}

export function ObjectTemplateSelector({
  isNew,
  visibleTemplates,
  selectedType,
  onSelect,
  templateMeta,
  userTemplates,
  collectionType,
  currentObject,
  contractTypeId,
  customPages,
  sectionParam,
}: ObjectTemplateSelectorProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['common', 'editor', 'navigation']);

  if (isNew) {
    return (
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
          {t('common:object_type')}
          {sectionParam && (
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginLeft: 8,
                fontWeight: 400,
              }}
            >
              {t('editor:in_section', {
                section: resolveCollectionLabel(sectionParam, customPages, t),
              })}
            </span>
          )}
        </h3>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {visibleTemplates.map((type) => {
            const label = templateMeta[type]?.label || type;
            const tpl = userTemplates.find((t) => t.id === type);
            return (
              <button
                key={type}
                onClick={() => onSelect(type)}
                onMouseEnter={(e) => {
                  if (selectedType !== type) {
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 6%, transparent)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (selectedType !== type) {
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.background = 'var(--bg-elevated)';
                  }
                }}
                style={{
                  padding: '10px 16px',
                  borderRadius: 8,
                  border:
                    selectedType === type
                      ? '1px solid var(--accent-primary)'
                      : '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: selectedType === type ? 'var(--accent-primary)' : 'var(--text-primary)',
                  fontSize: 'var(--text-body-sm)',
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
              >
                {label}
                {tpl?.contractTypeId && (
                  <span style={{ marginLeft: 6 }}>
                    <PluginBadge contractTypeId={tpl.contractTypeId} size="sm" variant="full" />
                  </span>
                )}
              </button>
            );
          })}
          <button
            onClick={() =>
              navigate('/settings/templates', {
                state: { from: location.pathname + location.search },
              })
            }
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.borderStyle = 'solid';
              e.currentTarget.style.color = 'var(--accent-primary)';
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 6%, transparent)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = 'var(--border-strong)';
              e.currentTarget.style.borderStyle = 'dashed';
              e.currentTarget.style.color = 'var(--text-secondary)';
              e.currentTarget.style.background = 'transparent';
            }}
            style={{
              marginLeft: 'auto',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '8px 12px',
              borderRadius: 8,
              border: '1px dashed var(--border-strong)',
              background: 'transparent',
              color: 'var(--text-secondary)',
              fontSize: 'var(--text-caption)',
              cursor: 'pointer',
              transition: 'all 0.15s ease',
            }}
            title={t('editor:manage_templates')}
          >
            <LayoutTemplate size={ICON_SIZE.sm} /> {t('editor:manage_templates')}
          </button>
          {visibleTemplates.length === 0 && (
            <div
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                padding: '8px 0',
              }}
            >
              {t('editor:no_template_for_section') || '此页面暂无模板，'}
              <span
                onClick={() =>
                  navigate('/settings/templates', {
                    state: { from: location.pathname + location.search },
                  })
                }
                style={{
                  color: 'var(--accent-primary)',
                  cursor: 'pointer',
                  textDecoration: 'underline',
                }}
              >
                {t('editor:go_create_template') || '前往模板管理新建'}
              </span>
            </div>
          )}
        </div>
      </Card>
    );
  }

  // Edit mode: show collection type badge and template info
  if (!collectionType) return null;

  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-secondary)' }}>
          {t('common:object_type')}:
        </span>
        <span
          style={{
            fontSize: 'var(--text-caption)',
            fontWeight: 500,
            padding: '2px 8px',
            borderRadius: 4,
            background: 'rgba(91,124,153,0.08)',
            color: 'var(--accent-primary)',
          }}
        >
          {resolveCollectionLabel(collectionType, customPages, t)}
        </span>
        {(selectedType || currentObject?.templateId) && (
          <span
            style={{
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              textDecoration: selectedType ? 'none' : 'line-through',
              wordBreak: 'break-word',
              minWidth: 0,
            }}
          >
            ·{' '}
            {selectedType
              ? templateMeta[selectedType]?.label || selectedType
              : (() => {
                  const tplName = (currentObject?.properties as Record<string, unknown>)
                    ?.__templateName as string | undefined;
                  const tplId = currentObject?.templateId || '';
                  return tplName ? `${tplName} (${tplId.slice(0, 8)}…)` : tplId;
                })()}
          </span>
        )}
        {contractTypeId && <PluginBadge contractTypeId={contractTypeId} size="sm" variant="full" />}
      </div>
    </Card>
  );
}
