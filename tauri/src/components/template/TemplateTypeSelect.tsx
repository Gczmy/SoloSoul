import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import type { PropertyType } from '@/types/template';

const PROPERTY_TYPES: PropertyType[] = [
  'text',
  'multiline',
  'number',
  'date',
  'datetime',
  'boolean',
  'select',
  'multiselect',
  'url',
  'email',
  'phone',
  'file',
];

interface TemplateTypeSelectProps {
  value: PropertyType;
  onChange: (v: PropertyType) => void;
}

export const TemplateTypeSelect = memo(function TemplateTypeSelect({
  value,
  onChange,
}: TemplateTypeSelectProps) {
  const { t } = useTranslation('editor');

  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as PropertyType)}
      onMouseEnter={(e) => {
        e.currentTarget.style.borderColor = 'var(--accent-primary)';
        e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)';
      }}
      onMouseLeave={(e) => {
        if (document.activeElement !== e.currentTarget) {
          e.currentTarget.style.borderColor = 'var(--border-subtle)';
          e.currentTarget.style.boxShadow = 'none';
        }
      }}
      onFocus={(e) => {
        e.currentTarget.style.borderColor = 'var(--accent-primary)';
        e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, var(--accent-primary) 15%, transparent)';
      }}
      onBlur={(e) => {
        e.currentTarget.style.borderColor = 'var(--border-subtle)';
        e.currentTarget.style.boxShadow = 'none';
      }}
      style={{
        height: 36,
        padding: '0 10px',
        borderRadius: 6,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-elevated)',
        color: 'var(--text-primary)',
        fontSize: 'var(--text-body-sm)',
        cursor: 'pointer',
        boxSizing: 'border-box',
        transition: 'border-color 0.2s, box-shadow 0.2s',
      }}
    >
      {PROPERTY_TYPES.map((pt) => (
        <option key={pt} value={pt}>
          {t(`field_types.${pt}`, pt)}
        </option>
      ))}
    </select>
  );
});
