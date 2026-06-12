import { useTranslation } from 'react-i18next';
import { X, ArrowLeft, Type, AlignLeft, Hash, Calendar, Clock, CheckSquare, List, ListChecks, Link, Mail, Phone, File } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import type { SampleTemplate } from '@/lib/sampleTemplates';
import type { SensitivityLevel } from '@/types/template';

interface SampleTemplateDetailProps {
  template: SampleTemplate;
  onBack: () => void;
  onUse: () => void;
}

const iconMap: Record<string, React.ReactNode> = {
  text: <Type size={14} />,
  multiline: <AlignLeft size={14} />,
  number: <Hash size={14} />,
  date: <Calendar size={14} />,
  datetime: <Clock size={14} />,
  boolean: <CheckSquare size={14} />,
  select: <List size={14} />,
  multiselect: <ListChecks size={14} />,
  url: <Link size={14} />,
  email: <Mail size={14} />,
  phone: <Phone size={14} />,
  file: <File size={14} />,
};

export function SampleTemplateDetail({ template, onBack, onUse }: SampleTemplateDetailProps) {
  const { t } = useTranslation(['settings', 'editor', 'navigation']);

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
      onClick={onBack}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '24px 28px',
          maxWidth: 520,
          width: '90%',
          maxHeight: '80vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
          <button
            onClick={onBack}
            style={{
              display: 'flex', alignItems: 'center', gap: 6,
              padding: '6px 10px', borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'transparent',
              color: 'var(--text-secondary)',
              fontSize: 12, cursor: 'pointer',
            }}
          >
            <ArrowLeft size={14} /> {t('common:back', '返回')}
          </button>
          <button
            onClick={onBack}
            style={{ padding: 6, borderRadius: 8, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}
          >
            <X size={20} />
          </button>
        </div>

        <h2 style={{ fontSize: 18, fontWeight: 700, margin: '0 0 4px' }}>
          {t(template.nameI18nKey, template.nameFallback)}
        </h2>
        <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 20 }}>
          {t(`navigation:${template.category}`, template.category)} · {template.properties.length} {t('settings:template_fields')}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
          {template.properties.map((prop) => (
            <div
              key={prop.id}
              style={{
                display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                gap: 12, padding: '10px 14px',
                borderRadius: 8, background: 'var(--bg-toolbar)',
                border: '1px solid var(--border-subtle)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
                <span style={{ color: 'var(--text-tertiary)', display: 'flex', alignItems: 'center' }}>
                  {iconMap[prop.type] || iconMap.text}
                </span>
                <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>
                  {t(prop.nameI18nKey, prop.nameFallback)}
                </span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                  {t(`editor:field_types.${prop.type}`, prop.type)}
                </span>
                <SensitivityBadge level={prop.sensitivityLevel as SensitivityLevel} />
              </div>
            </div>
          ))}
        </div>

        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={onBack}>{t('common:close')}</Button>
          <Button onClick={onUse}>{t('settings:use_sample_template')}</Button>
        </div>
      </div>
    </div>
  );
}
