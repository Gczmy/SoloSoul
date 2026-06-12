import React from 'react';
import type { PropertyType } from '@/types/template';
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
  onChange,
  disabled,
  icon,
  badge,
  hint,
}: TemplateFieldInputProps) {
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

  switch (type) {
    case 'date':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <DatePicker
            value={String(value ?? '')}
            onChange={(v) => onChange(v ?? '')}
            disabled={disabled}
          />
        </div>
      );

    case 'datetime':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <DatePicker
            value={String(value ?? '')}
            onChange={(v) => onChange(v ?? '')}
            includeTime
            disabled={disabled}
          />
        </div>
      );

    case 'multiline':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <textarea
            id={propertyId}
            className={styles.textarea}
            value={String(value || '')}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            rows={4}
          />
        </div>
      );

    case 'boolean':
      return (
        <div className={styles.field}>
          <label className={styles.checkboxLabel}>
            <input
              type="checkbox"
              className={styles.checkbox}
              checked={Boolean(value)}
              onChange={(e) => onChange(e.target.checked)}
              disabled={disabled}
            />
            {labelRow}
          </label>
        </div>
      );

    case 'select':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{labelRow}</label>
          <select
            id={propertyId}
            className={styles.select}
            value={String(value || '')}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
          >
            <option value="">-- 请选择 --</option>
            {(options || []).map((opt) => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
        </div>
      );

    case 'multiselect': {
      const rawSelected = Array.isArray(value) ? value : [];
      // Always display in template-defined order, regardless of selection order
      const selected = (options || []).filter((o) => rawSelected.includes(o));
      return (
        <div className={styles.field}>
          <label className={styles.label}>{labelRow}</label>
          <div className={styles.multiSelect}>
            {(options || []).map((opt) => (
              <label key={opt} className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  className={styles.checkbox}
                  checked={selected.includes(opt)}
                  onChange={(e) => {
                    const next = e.target.checked
                      ? (options || []).filter((o) => selected.includes(o) || o === opt)
                      : selected.filter((v) => v !== opt);
                    onChange(next);
                  }}
                  disabled={disabled}
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
          <label htmlFor={propertyId} className={styles.label}>{labelRow}</label>
          {hint && <div className={styles.hint}>{hint}</div>}
          <input
            id={propertyId}
            type={inputType}
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
