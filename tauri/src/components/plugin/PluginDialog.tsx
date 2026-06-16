import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MessageSquareText, CircleAlert, CircleHelp } from 'lucide-react';
import styles from './PluginDialog.module.css';
import type { DialogConfig, DialogRequestEvent } from '@/lib/plugin';

interface PluginDialogProps {
  pluginName: string;
  request: DialogRequestEvent;
  onSubmit: (value?: string) => void;
  onCancel: () => void;
}

const ICONS: Record<string, typeof MessageSquareText> = {
  alert: CircleAlert,
  confirm: CircleHelp,
  radio_list: MessageSquareText,
  checkbox_list: MessageSquareText,
  input: MessageSquareText,
};

const DIALOG_TYPES = ['alert', 'confirm', 'radio_list', 'checkbox_list', 'input'] as const;

function isDialogConfig(value: unknown): value is DialogConfig {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return DIALOG_TYPES.includes(v.type as (typeof DIALOG_TYPES)[number]);
}

function parseConfig(json: string): DialogConfig | null {
  try {
    const parsed = JSON.parse(json);
    if (isDialogConfig(parsed)) return parsed;
    return null;
  } catch {
    return null;
  }
}

export function PluginDialog({ pluginName, request, onSubmit, onCancel }: PluginDialogProps) {
  const { t } = useTranslation('plugin');
  const config = useMemo(() => parseConfig(request.jsonData), [request.jsonData]);
  const [inputValue, setInputValue] = useState(config?.defaultValue ?? '');
  const [selected, setSelected] = useState<Set<string>>(() => {
    if (config?.type === 'radio_list' && config.defaultValue) {
      return new Set([config.defaultValue]);
    }
    if (config?.type === 'checkbox_list' && Array.isArray(config.defaultValue)) {
      return new Set(config.defaultValue as string[]);
    }
    return new Set<string>();
  });

  const Icon = ICONS[config?.type ?? ''] ?? MessageSquareText;
  const title = config?.title ?? t('dialog_title', { defaultValue: 'Plugin Dialog' });
  const message = config?.message ?? '';
  const items = config?.items ?? [];

  const handleSubmit = () => {
    switch (config?.type) {
      case 'alert':
        onSubmit('');
        break;
      case 'confirm':
        onSubmit('true');
        break;
      case 'input':
        onSubmit(inputValue);
        break;
      case 'radio_list': {
        const first = Array.from(selected)[0];
        onSubmit(first ?? '');
        break;
      }
      case 'checkbox_list':
        onSubmit(JSON.stringify(Array.from(selected)));
        break;
      default:
        onSubmit('');
    }
  };

  const toggleOption = (id: string, multiple: boolean) => {
    setSelected((prev) => {
      const next = multiple ? new Set(prev) : new Set<string>();
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  return (
    <div className={styles.overlay}>
      <div className={styles.dialog}>
        <div className={styles.header}>
          <Icon size={22} className={styles.icon} />
          <h3 className={styles.title}>{title}</h3>
        </div>

        <p className={styles.subtitle}>
          {message ||
            t('dialog_subtitle', { pluginName, defaultValue: `${pluginName} is asking:` })}
        </p>

        {config?.type === 'input' && (
          <input
            type="text"
            className={styles.input}
            value={inputValue}
            placeholder={config.placeholder ?? ''}
            onChange={(e) => setInputValue(e.target.value)}
            autoFocus
          />
        )}

        {(config?.type === 'radio_list' || config?.type === 'checkbox_list') && (
          <div className={styles.list}>
            {items.map((item) => {
              const isSelected = selected.has(item.id);
              const multiple = config.type === 'checkbox_list';
              return (
                <label
                  key={item.id}
                  className={`${styles.option} ${isSelected ? styles.optionSelected : ''}`}
                >
                  <input
                    type={multiple ? 'checkbox' : 'radio'}
                    name={`dialog-${request.requestId}`}
                    checked={isSelected}
                    onChange={() => toggleOption(item.id, multiple)}
                  />
                  <span className={styles.optionLabel}>{item.label}</span>
                </label>
              );
            })}
          </div>
        )}

        <div className={styles.actions}>
          {config?.type !== 'alert' && (
            <button className={styles.cancelBtn} onClick={onCancel}>
              {t('dialog_cancel', { defaultValue: 'Cancel' })}
            </button>
          )}
          <button className={styles.confirmBtn} onClick={handleSubmit}>
            {config?.type === 'alert'
              ? t('dialog_ok', { defaultValue: 'OK' })
              : t('dialog_confirm', { defaultValue: 'Confirm' })}
          </button>
        </div>
      </div>
    </div>
  );
}
