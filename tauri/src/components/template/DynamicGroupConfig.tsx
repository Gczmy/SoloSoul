import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import type { PropertyType } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';
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
  const [expanded, setExpanded] = useState(false);

  const effectiveAllowed = allowedTypes?.length ? allowedTypes : ALL_PROPERTY_TYPES;
  const isAllSelected = effectiveAllowed.length === ALL_PROPERTY_TYPES.length;

  const toggleType = (type: PropertyType) => {
    if (effectiveAllowed.includes(type)) {
      onAllowedTypesChange(effectiveAllowed.filter((t) => t !== type));
    } else {
      onAllowedTypesChange([...effectiveAllowed, type]);
    }
  };

  // 摘要文本：类型 + 最大数量
  const typesLabel = t('editor:dynamic_group_allowed_types');
  const maxLabel = t('editor:dynamic_group_max_items_label');
  const typesSummary = isAllSelected
    ? t('editor:dynamic_group_no_limit')
    : `${effectiveAllowed.length}/${ALL_PROPERTY_TYPES.length}`;
  const maxSummary =
    maxItems === undefined
      ? t('editor:dynamic_group_no_limit')
      : `${maxItems}`;

  return (
    <div className={styles.wrapper}>
      <button
        type="button"
        className={styles.triggerBtn}
        onClick={() => setExpanded((v) => !v)}
      >
        <span
          className={styles.chevron}
          style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
        >
          <ChevronRight size={ICON_SIZE.sm} />
        </span>
        <span className={styles.triggerText}>
          {typesLabel}: {typesSummary}<span className={styles.separator}>·</span>{maxLabel}: {maxSummary}
        </span>
      </button>

      {expanded && (
        <div className={styles.configArea}>
          {/* 允许类型 */}
          <div className={styles.configSection}>
            <span className={styles.configSectionLabel}>
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

          <div className={styles.divider} />

          {/* 最大数量 */}
          <div className={styles.configSection}>
            <span className={styles.configSectionLabel}>
              {t('editor:dynamic_group_max_items_label')}
            </span>
            <div className={styles.stepperRow}>
              <div className={styles.stepperGroup}>
                <button
                  type="button"
                  className={styles.stepperBtn}
                  disabled={maxItems === undefined || maxItems <= 1}
                  onClick={() => {
                    if (maxItems !== undefined && maxItems > 1) {
                      onMaxItemsChange(maxItems - 1);
                    }
                  }}
                >
                  −
                </button>
                <input
                  type="number"
                  min={1}
                  className={styles.stepperInput}
                  value={maxItems ?? ''}
                  placeholder={t('editor:dynamic_group_no_limit')}
                  onChange={(e) => {
                    const raw = e.target.value;
                    if (raw === '' || raw === '0') {
                      onMaxItemsChange(undefined);
                    } else {
                      const num = parseInt(raw, 10);
                      if (!isNaN(num) && num >= 1) {
                        onMaxItemsChange(num);
                      }
                    }
                  }}
                />
                <button
                  type="button"
                  className={styles.stepperBtn}
                  onClick={() => onMaxItemsChange((maxItems ?? 0) + 1)}
                >
                  +
                </button>
              </div>
              <button
                type="button"
                className={styles.unlimitedBtn}
                disabled={maxItems === undefined}
                onClick={() => onMaxItemsChange(undefined)}
              >
                {t('editor:dynamic_group_no_limit')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
