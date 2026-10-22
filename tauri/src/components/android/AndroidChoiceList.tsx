import type { ReactNode } from 'react';
import { Check, Square } from 'lucide-react';
import styles from './AndroidChoice.module.css';

export interface AndroidChoice {
  value: string;
  label: string;
  description?: string;
  icon?: ReactNode;
  disabled?: boolean;
  group?: string;
}

export function AndroidChoiceList({
  label,
  value,
  choices,
  onChoose,
  multiple = false,
}: {
  label: string;
  value?: string | string[];
  choices: AndroidChoice[];
  onChoose: (value: string) => void;
  multiple?: boolean;
}) {
  const selected = Array.isArray(value) ? value : [value];
  return (
    <div className={styles.list} role={multiple ? 'group' : 'radiogroup'} aria-label={label}>
      {choices.map((choice, index) => (
        <div key={choice.value}>
          {choice.group && choice.group !== choices[index - 1]?.group && (
            <p className={styles.group}>{choice.group}</p>
          )}
          <button
            type="button"
            className={styles.option}
            role={multiple ? 'checkbox' : 'radio'}
            aria-checked={selected.includes(choice.value)}
            disabled={choice.disabled}
            onClick={() => onChoose(choice.value)}
          >
            {choice.icon}
            <span>
              {choice.label}
              {choice.description && <small>{choice.description}</small>}
            </span>
            {selected.includes(choice.value) ? (
              <Check size={20} aria-hidden="true" />
            ) : multiple ? (
              <Square size={20} aria-hidden="true" />
            ) : null}
          </button>
        </div>
      ))}
    </div>
  );
}
