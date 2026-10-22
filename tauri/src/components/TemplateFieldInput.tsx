import React from 'react';
import { useTranslation } from 'react-i18next';
import { isAndroidSync } from '@/lib/platform';
import { AndroidChoiceField } from '@/components/android/AndroidChoiceField';
import type { PropertyType } from '@/types/template';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DatePicker } from '@/components/forms/DatePicker';
import styles from './TemplateFieldInput.module.css';

interface TemplateFieldInputProps {
  propertyId: string;
  label: string;
  type: PropertyType;
  value: unknown;
  options?: string[];
  onChange: (value: unknown) => void;
  disabled?: boolean;
  readOnly?: boolean;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  hint?: string;
}

export function TemplateFieldInput({
  propertyId,
  label,
  type,
  value,
  options,
  onChange: changeValue,
  disabled,
  readOnly,
  icon,
  badge,
  hint,
}: TemplateFieldInputProps) {
  const { t } = useTranslation('common');
  const onChange = (next: unknown) => {
    if (!disabled && !readOnly) changeValue(next);
  };
  const labelRow = (
    <span className={styles.labelRow}>
      {icon}
      <span className={styles.labelText}>{label}</span>
      {badge}
    </span>
  );
  const inputTypeMap: Record<string, string> = {
    text: 'text',
    number: 'number',
    url: 'url',
    email: 'email',
    phone: 'tel',
    file: 'text', // file references stored as text (attachment id or path)
  };
  const uniqueOptions = [...new Set(options ?? [])];

  if (isAndroidSync() && (type === 'select' || type === 'multiselect')) {
    const title = label || t('select_option', { defaultValue: 'Select an option' });
    const choices = uniqueOptions.map((option) => ({ value: option, label: option }));
    return (
      <div className={styles.field}>
        <label htmlFor={propertyId} className={styles.label}>
          {labelRow}
        </label>
        {hint && <div className={styles.hint}>{hint}</div>}
        {type === 'multiselect' ? (
          <AndroidChoiceField
            id={propertyId}
            title={title}
            choices={choices}
            multiple
            value={uniqueOptions.filter((option) => Array.isArray(value) && value.includes(option))}
            onChange={onChange}
            disabled={disabled || readOnly}
          />
        ) : (
          <AndroidChoiceField
            id={propertyId}
            title={title}
            choices={[{ value: '', label: t('clear', { defaultValue: 'Clear' }) }, ...choices]}
            value={String(value || '')}
            onChange={onChange}
            disabled={disabled || readOnly}
            summary={value ? undefined : t('select_option', { defaultValue: 'Select an option' })}
          />
        )}
      </div>
    );
  }

  switch (type) {
    case 'date':
      return (
        <div className={styles.field}>
          <label className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <DatePicker
            value={String(value ?? '')}
            onChange={(v) => onChange(v)}
            disabled={disabled || readOnly}
          />
        </div>
      );

    case 'datetime':
      return (
        <div className={styles.field}>
          <label className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <DatePicker
            value={String(value ?? '')}
            onChange={(v) => onChange(v)}
            includeTime
            disabled={disabled || readOnly}
          />
        </div>
      );

    case 'multiline':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>
            {labelRow}
          </label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <textarea
            id={propertyId}
            className={styles.textarea}
            value={String(value || '')}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            rows={6}
            readOnly={readOnly}
          />
        </div>
      );

    case 'boolean':
      return (
        <div className={styles.field}>
          <label className={styles.checkboxLabel}>
            <SelectCheckbox
              checked={Boolean(value)}
              onChange={(v) => onChange(v)}
              disabled={disabled || readOnly}
            />
            {labelRow}
          </label>
        </div>
      );

    case 'select':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>
            {labelRow}
          </label>
          <select
            id={propertyId}
            className={styles.select}
            value={String(value || '')}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled || readOnly}
          >
            <option value="">-- 请选择 --</option>
            {uniqueOptions.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        </div>
      );

    case 'multiselect': {
      const rawSelected = Array.isArray(value) ? value : [];
      // Always display in template-defined order, regardless of selection order
      const selected = uniqueOptions.filter((o) => rawSelected.includes(o));
      return (
        <div className={styles.field}>
          <label className={styles.label}>{labelRow}</label>
          <div className={styles.multiSelect}>
            {uniqueOptions.map((opt) => (
              <label key={opt} className={styles.checkboxLabel}>
                <SelectCheckbox
                  checked={selected.includes(opt)}
                  onChange={(v) => {
                    const next = v
                      ? uniqueOptions.filter((o) => selected.includes(o) || o === opt)
                      : selected.filter((v) => v !== opt);
                    onChange(next);
                  }}
                  disabled={disabled || readOnly}
                />
                <span>{opt}</span>
              </label>
            ))}
          </div>
        </div>
      );
    }

    default: {
      const inputType = inputTypeMap[type] || 'text';
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>
            {labelRow}
          </label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <input
            id={propertyId}
            type={inputType}
            readOnly={readOnly}
            className={styles.input}
            value={String(value ?? '')}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          />
        </div>
      );
    }
  }
}
