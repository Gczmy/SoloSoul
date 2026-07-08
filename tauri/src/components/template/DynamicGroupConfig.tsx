import { useTranslation } from 'react-i18next';
import type { PropertyType } from '@/types/template';
import styles from './DynamicGroupConfig.module.css';

const ALL_PROPERTY_TYPES: PropertyType[] = [
  'text',
  'multiline',
  'number',
  'date',
  'datetime',
  'boolean',
  'select',
  'multiselect',
  'url',
  'email',
  'phone',
  'file',
];

interface DynamicGroupConfigProps {
  allowedTypes?: PropertyType[];
  maxItems?: number;
  onAllowedTypesChange: (types: PropertyType[]) => void;
  onMaxItemsChange: (maxItems: number | undefined) => void;
}

export function DynamicGroupConfig({
  allowedTypes,
  maxItems,
  onAllowedTypesChange,
  onMaxItemsChange,
}: DynamicGroupConfigProps) {
  const { t } = useTranslation(['editor']);
  const effectiveAllowed = allowedTypes?.length ? allowedTypes : ALL_PROPERTY_TYPES;
  const isAllSelected = effectiveAllowed.length === ALL_PROPERTY_TYPES.length;

  const toggleType = (type: PropertyType) => {
    if (effectiveAllowed.includes(type)) {
      onAllowedTypesChange(effectiveAllowed.filter((t) => t !== type));
    } else {
      onAllowedTypesChange([...effectiveAllowed, type]);
    }
  };

  return (
    <div className={styles.container}>
      <div className={styles.section}>
        <span className={styles.label}>
          {t('editor:dynamic_group_allowed_types')}
        </span>
        <div className={styles.typeGrid}>
          {ALL_PROPERTY_TYPES.map((type) => (
            <label key={type} className={styles.typeChip}>
              <input
                type="checkbox"
                checked={effectiveAllowed.includes(type)}
                onChange={() => toggleType(type)}
              />
              <span>{t(`editor:field_types.${type}`, type)}</span>
            </label>
          ))}
        </div>
        <button
          type="button"
          className={styles.toggleAll}
          onClick={() => onAllowedTypesChange(isAllSelected ? [] : [...ALL_PROPERTY_TYPES])}
        >
          {isAllSelected
            ? t('editor:dynamic_group_no_limit')
            : t('editor:dynamic_group_select_all')}
        </button>
      </div>

      <div className={styles.section}>
        <label className={styles.label}>
          {t('editor:dynamic_group_max_items_label')}
        </label>
        <div className={styles.maxItemsRow}>
          <input
            type="number"
            min={0}
            value={maxItems ?? ''}
            onChange={(e) => {
              const v = e.target.value;
              onMaxItemsChange(v === '' ? undefined : Math.max(0, parseInt(v, 10)));
            }}
            placeholder={t('editor:dynamic_group_no_limit')}
            className={styles.maxInput}
          />
          {maxItems !== undefined && (
            <button
              type="button"
              className={styles.clearBtn}
              onClick={() => onMaxItemsChange(undefined)}
            >
              {t('common:clear')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
