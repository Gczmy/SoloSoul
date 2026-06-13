import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { X, LayoutTemplate } from 'lucide-react';
import { SAMPLE_TEMPLATES, type SampleTemplate } from '@/lib/sampleTemplates';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { Input } from '@/components/ui/Input';
import type { SensitivityLevel } from '@/types/template';

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];
const SAMPLE_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

interface SampleTemplateGalleryProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (tpl: SampleTemplate) => void;
}

export function SampleTemplateGallery({ isOpen, onClose, onSelect }: SampleTemplateGalleryProps) {
  const { t } = useTranslation(['settings', 'editor', 'navigation', 'common']);
  const [pageFilter, setPageFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  const pageOptions = useMemo(
    () => [
      { id: 'all', label: t('settings:filter_all') },
      ...SAMPLE_PAGES.map((id) => ({ id, label: t(`navigation:${id}`) })),
    ],
    [t],
  );

  const filteredSamples = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return SAMPLE_TEMPLATES.filter((tpl) => {
      const matchesPage = pageFilter === 'all' || tpl.category === pageFilter;
      if (!matchesPage) return false;
      if (!q) return true;
      const displayName = t(tpl.nameI18nKey, tpl.nameFallback).toLowerCase();
      return (
        displayName.includes(q) ||
        tpl.nameFallback.toLowerCase().includes(q) ||
        tpl.nameI18nKey.toLowerCase().includes(q)
      );
    });
  }, [pageFilter, searchQuery, t]);

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
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 8,
          }}
        >
          <h2 style={{ fontSize: 17, fontWeight: 700, margin: 0 }}>
            {t('settings:sample_templates_title')}
          </h2>
          <button
            onClick={onClose}
            data-testid="sample-gallery-close"
            aria-label={t('common:close')}
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
        <p style={{ fontSize: 12, color: 'var(--text-secondary)', margin: '0 0 16px' }}>
          {t('settings:sample_templates_desc')}
        </p>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 16 }}>
          <Input
            placeholder={t('settings:search_sample_templates') || '搜索示例模板...'}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {pageOptions.map((opt) => (
              <button
                key={opt.id}
                type="button"
                data-testid={`page-filter-${opt.id}`}
                onClick={() => setPageFilter(opt.id)}
                aria-pressed={pageFilter === opt.id}
                style={{
                  padding: '5px 12px',
                  borderRadius: 6,
                  border: '1px solid var(--border-subtle)',
                  background: pageFilter === opt.id ? 'var(--accent-primary)' : 'transparent',
                  color: pageFilter === opt.id ? 'white' : 'var(--text-secondary)',
                  fontSize: 12,
                  cursor: 'pointer',
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
            gap: 12,
          }}
        >
          {filteredSamples.map((tpl) => {
            const present = new Set(tpl.properties.map((p) => p.sensitivityLevel));
            const ordered = SENSITIVITY_ORDER.filter((l) => present.has(l));
            return (
              <button
                key={tpl.key}
                data-testid="sample-template-card"
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
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <LayoutTemplate size={22} style={{ color: 'var(--accent-primary)' }} />
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>
                      {t(tpl.nameI18nKey, tpl.nameFallback)}
                    </div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2 }}>
                      {t(`navigation:${tpl.category}`, tpl.category)} · {tpl.properties.length}{' '}
                      {t('settings:template_fields')}
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
