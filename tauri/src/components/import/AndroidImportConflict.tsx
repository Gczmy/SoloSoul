import { useTranslation } from 'react-i18next';
import { AndroidChoiceField } from '@/components/android/AndroidChoiceField';
import type { ImportStrategy } from '@/types/exportImport';
import styles from './AndroidImportConflict.module.css';

export function AndroidImportConflict({
  importedName,
  existingName,
  kindLabel,
  value,
  onChange,
}: {
  importedName: string;
  existingName: string;
  kindLabel: string;
  value: ImportStrategy;
  onChange: (strategy: ImportStrategy) => void;
}) {
  const { t } = useTranslation('settings');
  return (
    <div className={styles.conflict}>
      <p>
        <span>{t('conflict_imported_name')}</span>
        {importedName}
      </p>
      <p>
        <span>{t('conflict_local_name')}</span>
        {existingName}
      </p>
      <small>{kindLabel}</small>
      <AndroidChoiceField
        title={t('import_strategy_title')}
        value={value}
        choices={(['skipExisting', 'overwrite', 'keepBoth'] as const).map((strategy) => ({
          value: strategy,
          label: t(`strategy_${strategy}`),
          description: t(`strategy_${strategy}_desc`),
        }))}
        onChange={(strategy) => onChange(strategy as ImportStrategy)}
      />
    </div>
  );
}
