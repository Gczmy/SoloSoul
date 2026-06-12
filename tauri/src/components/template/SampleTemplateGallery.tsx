import { useTranslation } from 'react-i18next';
import { X, LayoutTemplate } from 'lucide-react';
import { SAMPLE_TEMPLATES, type SampleTemplate } from '@/lib/sampleTemplates';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import type { SensitivityLevel } from '@/types/template';

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

interface SampleTemplateGalleryProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (tpl: SampleTemplate) => void;
}

export function SampleTemplateGallery({ isOpen, onClose, onSelect }: SampleTemplateGalleryProps) {
  const { t } = useTranslation(['settings', 'editor', 'navigation']);

  if (!isOpen) return null;

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
          padding: '24px 28px',
          maxWidth: 720,
          width: '90%',
          maxHeight: '85vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
          <h2 style={{ fontSize: 17, fontWeight: 700, margin: 0 }}>{t('settings:sample_templates_title')}</h2>
          <button
            onClick={onClose}
            style={{ padding: 6, borderRadius: 8, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}
          >
            <X size={20} />
          </button>
        </div>
        <p style={{ fontSize: 12, color: 'var(--text-secondary)', margin: '0 0 16px' }}>
          {t('settings:sample_templates_desc')}
        </p>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 12 }}>
          {SAMPLE_TEMPLATES.map((tpl) => {
            const present = new Set(tpl.properties.map((p) => p.sensitivityLevel));
            const ordered = SENSITIVITY_ORDER.filter((l) => present.has(l));
            return (
              <button
                key={tpl.key}
                onClick={() => onSelect(tpl)}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 10,
                  padding: 16,
                  borderRadius: 12,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  cursor: 'pointer',
                  textAlign: 'left',
                  transition: 'border-color 0.15s, transform 0.1s',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'var(--accent-primary)'; }}
                onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'var(--border-subtle)'; }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <LayoutTemplate size={22} style={{ color: 'var(--accent-primary)' }} />
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>
                      {t(tpl.nameI18nKey, tpl.nameFallback)}
                    </div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2 }}>
                      {t(`navigation:${tpl.category}`, tpl.category)} · {tpl.properties.length} {t('settings:template_fields')}
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
                  {ordered.map((level) => (
                    <SensitivityBadge key={level} level={level} />
                  ))}
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
