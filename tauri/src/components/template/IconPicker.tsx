import React from 'react';
import type { LucideIcon } from 'lucide-react';
import {
  CUSTOM_ICON_MAP,
  DEFAULT_CUSTOM_ICON,
  type CustomIconId,
} from '@/lib/pageIcons';

interface IconPickerProps {
  value: string;
  onChange: (iconId: string) => void;
}

export function IconPicker({ value, onChange }: IconPickerProps) {
  const currentId = (value && value in CUSTOM_ICON_MAP ? value : DEFAULT_CUSTOM_ICON) as CustomIconId;

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(8, 1fr)',
        gap: 4,
        padding: '8px 0',
      }}
    >
      {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(([id, IconComp]) => (
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
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.borderColor = 'var(--text-tertiary)';
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
  );
}
