import type { ReactNode } from 'react';
import { Check } from 'lucide-react';
import styles from './AndroidTemplateEditor.module.css';

export interface TemplateChoice {
  value: string;
  label: string;
  description?: string;
  icon?: ReactNode;
  disabled?: boolean;
  group?: string;
}

export function AndroidTemplateChoiceList({
  label,
  value,
  choices,
  onChoose,
}: {
  label: string;
  value?: string;
  choices: TemplateChoice[];
  onChoose: (value: string) => void;
}) {
  return (
    <div className={styles.choiceList} role="radiogroup" aria-label={label}>
      {choices.map((choice, index) => (
        <div key={choice.value}>
          {choice.group && choice.group !== choices[index - 1]?.group && (
            <p className={styles.subheading}>{choice.group}</p>
          )}
          <button
            type="button"
            className={styles.option}
            role="radio"
            aria-checked={choice.value === value}
            disabled={choice.disabled}
            onClick={() => onChoose(choice.value)}
          >
            {choice.icon}
            <span>
              {choice.label}
              {choice.description && <small>{choice.description}</small>}
            </span>
            {choice.value === value && <Check size={20} aria-hidden="true" />}
          </button>
        </div>
      ))}
    </div>
  );
}
