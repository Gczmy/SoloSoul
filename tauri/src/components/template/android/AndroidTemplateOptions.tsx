import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import styles from './AndroidTemplateEditor.module.css';

export function AndroidTemplateOptions({
  options,
  onSave,
  onClose,
}: {
  options: string[];
  onSave: (options: string[]) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const [draft, setDraft] = useState(() => options.join('\n'));
  return (
    <form
      className={styles.config}
      onSubmit={(event) => {
        event.preventDefault();
        onSave([
          ...new Set(
            draft
              .split('\n')
              .map((line) => line.trim())
              .filter(Boolean),
          ),
        ]);
      }}
    >
      <label className={styles.config}>
        <span className={styles.hint}>{t('settings:template_mobile.options_hint')}</span>
        <textarea
          className={styles.textarea}
          rows={8}
          data-initial-focus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </label>
      <div className={styles.actions}>
        <Button type="button" variant="secondary" onClick={onClose}>
          {t('common:cancel')}
        </Button>
        <Button type="submit">{t('common:confirm')}</Button>
      </div>
    </form>
  );
}
