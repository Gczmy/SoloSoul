import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog } from './Dialog';
import { Button } from './Button';
import styles from './Dialog.module.css';

interface PromptDialogProps {
  isOpen: boolean;
  title: string;
  defaultValue?: string;
  placeholder?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

export function PromptDialog({
  isOpen,
  title,
  defaultValue = '',
  placeholder,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
}: PromptDialogProps) {
  const { t } = useTranslation('common');
  const [value, setValue] = useState(defaultValue);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setValue(defaultValue);
      // Focus and select the default text after the dialog opens
      setTimeout(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      }, 0);
    }
  }, [isOpen, defaultValue]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onConfirm(value.trim() || defaultValue);
  };

  return (
    <Dialog isOpen={isOpen} onClose={onCancel} title={title}>
      <form onSubmit={handleSubmit}>
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder}
          style={{
            width: '100%',
            padding: '10px 12px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body)',
            fontFamily: 'inherit',
            marginBottom: 20,
            outline: 'none',
          }}
        />
        <div className={styles.actions}>
          <Button variant="secondary" type="button" onClick={onCancel} data-testid="prompt-dialog-cancel">
            {cancelLabel ?? t('cancel', { defaultValue: 'Cancel' })}
          </Button>
          <Button type="submit" data-testid="prompt-dialog-confirm">
            {confirmLabel ?? t('confirm', { defaultValue: 'Confirm' })}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
