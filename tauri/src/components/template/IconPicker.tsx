import React, { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { LucideIcon } from 'lucide-react';
import {
  CUSTOM_ICON_MAP,
  DEFAULT_CUSTOM_ICON,
  ICON_CATEGORIES,
  CATEGORY_LABELS,
  type CustomIconId,
} from '@/lib/pageIcons';

interface IconPickerProps {
  value: string;
  onChange: (iconId: string) => void;
}

const CATEGORY_ORDER = ['all', 'general', 'security', 'identity', 'finance', 'travel', 'work', 'communication', 'health', 'education', 'life', 'nature', 'special'] as const;

export function IconPicker({ value, onChange }: IconPickerProps) {
  const { t } = useTranslation('navigation');
  const currentId = (value && value in CUSTOM_ICON_MAP ? value : DEFAULT_CUSTOM_ICON) as CustomIconId;
  const [categoryFilter, setCategoryFilter] = useState('all');

  const filteredEntries = useMemo(() => {
    const entries = Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][];
    if (categoryFilter === 'all') return entries;
    return entries.filter(([id]) => ICON_CATEGORIES[id] === categoryFilter);
  }, [categoryFilter]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {/* Category tabs */}
      <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
        {CATEGORY_ORDER.map((cat) => {
          const isActive = categoryFilter === cat;
          return (
            <button
              key={cat}
              type="button"
              onClick={() => setCategoryFilter(cat)}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'transparent';
                }
              }}
              style={{
                padding: '4px 10px',
                fontSize: 12,
                fontWeight: isActive ? 600 : 400,
                borderRadius: 6,
                border: isActive
                  ? '1px solid color-mix(in srgb, var(--accent-primary) 40%, transparent)'
                  : '1px solid transparent',
                background: isActive
                  ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                  : 'var(--bg-toolbar)',
                color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
                cursor: 'pointer',
                transition: 'all 0.12s ease',
                whiteSpace: 'nowrap',
              }}
            >
              {t(`icon_category_${cat}`, CATEGORY_LABELS[cat])}
            </button>
          );
        })}
      </div>

      {/* Icon grid */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(8, 1fr)',
          gap: 4,
          padding: '8px 0',
        }}
      >
        {filteredEntries.map(([id, IconComp]) => (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            title={id}
            style={{
              width: 32,
              height: 32,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 6,
              border:
                currentId === id
                  ? '2px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
              background:
                currentId === id ? 'var(--accent-primary-soft, rgba(91,124,153,0.08))' : 'transparent',
              cursor: 'pointer',
              transition: 'all 0.1s ease',
            }}
            onMouseEnter={(e) => {
              if (currentId !== id) {
                e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              if (currentId !== id) {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
              }
            }}
          >
            <IconComp
              size={16}
              style={{
                color:
                  currentId === id
                    ? 'var(--accent-primary)'
                    : 'var(--text-secondary)',
              }}
            />
          </button>
        ))}
      </div>
    </div>
  );
}
