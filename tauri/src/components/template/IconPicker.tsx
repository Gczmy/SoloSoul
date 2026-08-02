import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { LucideIcon } from 'lucide-react';
import {
  CUSTOM_ICON_MAP,
  DEFAULT_CUSTOM_ICON,
  ICON_CATEGORIES,
  CATEGORY_LABELS,
  type CustomIconId,
} from '@/lib/pageIcons';
import { ICON_SIZE } from '@/lib/constants';

interface IconPickerProps {
  value: string;
  onChange: (iconId: string) => void;
}

const CATEGORY_ORDER = [
  'all',
  'general',
  'security',
  'identity',
  'finance',
  'travel',
  'work',
  'communication',
  'health',
  'education',
  'life',
  'nature',
  'special',
] as const;

export function IconPicker({ value, onChange }: IconPickerProps) {
  const { t } = useTranslation('navigation');
  const currentId = (
    value && value in CUSTOM_ICON_MAP ? value : DEFAULT_CUSTOM_ICON
  ) as CustomIconId;
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
              className={isActive ? 'interactive-toolbar selected-accent' : 'interactive-toolbar'}
              style={{
                padding: '4px 10px',
                fontSize: 'var(--text-caption)',
                fontWeight: isActive ? 600 : 400,
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
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
            className={
              currentId === id ? 'interactive-tile selected-accent' : 'interactive-tile'
            }
            style={{
              width: 32,
              height: 32,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 6,
              borderWidth: currentId === id ? 2 : 1,
              borderStyle: 'solid',
              cursor: 'pointer',
            }}
          >
            <IconComp
              size={ICON_SIZE.md}
              style={{
                color: currentId === id ? 'var(--accent-primary)' : 'var(--text-secondary)',
              }}
            />
          </button>
        ))}
      </div>
    </div>
  );
}
