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
      style={{
        height: 36,
        padding: '0 10px',
        borderRadius: 6,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-elevated)',
        color: 'var(--text-primary)',
        fontSize: 13,
        cursor: 'pointer',
        boxSizing: 'border-box',
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
