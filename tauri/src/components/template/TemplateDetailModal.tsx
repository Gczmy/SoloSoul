import { useTranslation } from 'react-i18next';
import { X, Pencil, LayoutTemplate } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SensitivityBadges } from './SensitivityBadges';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';

interface DetailProperty {
  id: string;
  name: string;
  type: string;
  sensitivityLevel?: string;
  deprecatedAt?: string;
}

interface ListTemplate {
  id: string;
  name: string;
  category: string;
  properties: DetailProperty[];
}

interface TemplateDetailModalProps {
  detailTemplate: ListTemplate | null;
  templates: UserTemplate[];
  pageLabel: (category: string) => { name: string; deleted: boolean };
  onClose: () => void;
  onEdit: (id: string) => void;
}

export function TemplateDetailModal({
  detailTemplate,
  templates,
  pageLabel,
  onClose,
  onEdit,
}: TemplateDetailModalProps) {
  const { t } = useTranslation(['common', 'settings']);
  if (!detailTemplate) return null;

  const page = pageLabel(detailTemplate.category || 'identity');

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '28px 32px',
          maxWidth: 520,
          width: '90%',
          maxHeight: '80vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        {/* Title row */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 20,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <LayoutTemplate size={24} color="var(--accent-primary)" />
            <div>
              <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 700, margin: 0 }}>
                {detailTemplate.name}
              </h2>
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                <span
                  style={
                    page.deleted
                      ? { textDecoration: 'line-through', opacity: 0.6 }
                      : undefined
                  }
                >
                  {page.name}
                </span>
                <span>·</span>
                <span>
                  {detailTemplate.properties.length} {t('settings:template_fields') || '个字段'}
                </span>
                <SensitivityBadges properties={detailTemplate.properties} />
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              padding: 6,
              borderRadius: 8,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Divider */}
        <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 20 }} />

        {/* Fields */}
        {detailTemplate.properties.length === 0 ? (
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              padding: '16px 0',
            }}
          >
            {t('settings:empty_template_hint') || '此模板暂无字段'}
          </p>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {detailTemplate.properties.map((prop) => (
              <div
                key={prop.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 12,
                  padding: '10px 14px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                  opacity: prop.deprecatedAt ? 0.7 : 1,
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    flex: 1,
                    minWidth: 0,
                  }}
                >
                  <span
                    style={{
                      color: 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                    }}
                  >
                    <FieldTypeIcon type={prop.type as PropertyType} size={14} />
                  </span>
                  <span
                    style={{
                      fontSize: 'var(--text-body)',
                      fontWeight: 500,
                      color: 'var(--text-primary)',
                      textDecoration: prop.deprecatedAt ? 'line-through' : 'none',
                    }}
                  >
                    {prop.name}
                  </span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <SensitivityBadge
                    level={(prop.sensitivityLevel || 'internal') as SensitivityLevel}
                  />
                  {prop.deprecatedAt && <DeprecatedBadge />}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Actions */}
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 24 }}>
          <Button variant="secondary" onClick={onClose}>
            {t('common:close') || '关闭'}
          </Button>
          <Button
            variant="secondary"
            style={{ border: '1px solid var(--accent-primary)', color: 'var(--accent-primary)' }}
            onClick={() => {
              const ut = templates.find((u) => u.id === detailTemplate.id);
              if (ut) {
                onClose();
                onEdit(detailTemplate.id);
              }
            }}
          >
            <Pencil size={16} style={{ marginRight: 4 }} />
            {t('common:edit') || '编辑'}
          </Button>
        </div>
      </div>
    </div>
  );
}
