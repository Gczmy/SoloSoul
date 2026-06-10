import type { PropertyType } from '@/types/template';
import styles from './TemplateFieldInput.module.css';

interface TemplateFieldInputProps {
  propertyId: string;
  label: string;
  type: PropertyType;
  value: unknown;
  options?: string[];
  onChange: (value: unknown) => void;
  disabled?: boolean;
}

export function TemplateFieldInput({
  propertyId,
  label,
  type,
  value,
  options,
  onChange,
  disabled,
}: TemplateFieldInputProps) {
  const inputTypeMap: Record<string, string> = {
    text: 'text',
    number: 'number',
    date: 'date',
    datetime: 'datetime-local',
    url: 'url',
    email: 'email',
    phone: 'tel',
    file: 'text', // file references stored as text (attachment id or path)
  };

  switch (type) {
    case 'multiline':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{label}</label>
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
            <span>{label}</span>
          </label>
        </div>
      );

    case 'select':
      return (
        <div className={styles.field}>
          <label htmlFor={propertyId} className={styles.label}>{label}</label>
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
      const selected = Array.isArray(value) ? value : [];
      return (
        <div className={styles.field}>
          <label className={styles.label}>{label}</label>
          <div className={styles.multiSelect}>
            {(options || []).map((opt) => (
              <label key={opt} className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  className={styles.checkbox}
                  checked={selected.includes(opt)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      onChange([...selected, opt]);
                    } else {
                      onChange(selected.filter((v) => v !== opt));
                    }
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
          <label htmlFor={propertyId} className={styles.label}>{label}</label>
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
