import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings2 } from 'lucide-react';
import styles from './PluginRunParamsDialog.module.css';
import type { PluginParam } from '@/lib/plugin';

interface PluginRunParamsDialogProps {
  pluginName: string;
  params: PluginParam[];
  onSubmit: (values: Record<string, string>) => void;
  onCancel: () => void;
}

function initialValues(params: PluginParam[]): Record<string, string> {
  const values: Record<string, string> = {};
  for (const p of params) {
    if (p.defaultValue !== undefined) {
      values[p.id] = p.defaultValue;
    } else if (p.type === 'boolean') {
      values[p.id] = 'false';
    } else {
      values[p.id] = '';
    }
  }
  return values;
}

export function PluginRunParamsDialog({
  pluginName,
  params,
  onSubmit,
  onCancel,
}: PluginRunParamsDialogProps) {
  const { t } = useTranslation('plugin');
  const [values, setValues] = useState<Record<string, string>>(() => initialValues(params));
  const [errors, setErrors] = useState<Record<string, string>>({});

  const handleChange = (id: string, value: string) => {
    setValues((prev) => ({ ...prev, [id]: value }));
    if (errors[id]) {
      setErrors((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const handleSubmit = () => {
    const nextErrors: Record<string, string> = {};
    for (const p of params) {
      if (p.required && !values[p.id]?.trim()) {
        nextErrors[p.id] = t('param_required', { defaultValue: 'Required' });
      }
    }
    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }
    onSubmit(values);
  };

  return (
    <div className={styles.overlay}>
      <div className={styles.dialog}>
        <div className={styles.header}>
          <Settings2 size={20} className={styles.icon} />
          <h3 className={styles.title}>{t('run_params_title', { defaultValue: 'Run Plugin' })}</h3>
        </div>
        <p className={styles.subtitle}>
          {t('run_params_subtitle', {
            pluginName,
            defaultValue: `${pluginName} needs the following parameters:`,
          })}
        </p>

        <div className={styles.form}>
          {params.map((param) => {
            const error = errors[param.id];
            return (
              <div key={param.id} className={styles.field}>
                <label className={styles.label}>
                  {param.label}
                  {param.required && <span className={styles.required}>*</span>}
                </label>
                {param.description && (
                  <span className={styles.description}>{param.description}</span>
                )}

                {param.type === 'boolean' && (
                  <label className={styles.checkboxRow}>
                    <input
                      type="checkbox"
                      checked={values[param.id] === 'true'}
                      onChange={(e) => handleChange(param.id, e.target.checked ? 'true' : 'false')}
                    />
                    <span>{param.label}</span>
                  </label>
                )}

                {param.type === 'select' && (
                  <select
                    className={styles.input}
                    value={values[param.id]}
                    onChange={(e) => handleChange(param.id, e.target.value)}
                  >
                    <option value="">
                      {t('param_select_placeholder', { defaultValue: 'Select...' })}
                    </option>
                    {param.options?.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                )}

                {param.type !== 'boolean' && param.type !== 'select' && (
                  <input
                    type={param.type === 'number' ? 'number' : 'text'}
                    className={styles.input}
                    value={values[param.id]}
                    placeholder={param.label}
                    onChange={(e) => handleChange(param.id, e.target.value)}
                  />
                )}

                {error && <span className={styles.error}>{error}</span>}
              </div>
            );
          })}
        </div>

        <div className={styles.actions}>
          <button className={styles.cancelBtn} onClick={onCancel}>
            {t('dialog_cancel', { defaultValue: 'Cancel' })}
          </button>
          <button className={styles.confirmBtn} onClick={handleSubmit}>
            {t('run', { defaultValue: 'Run' })}
          </button>
        </div>
      </div>
    </div>
  );
}
