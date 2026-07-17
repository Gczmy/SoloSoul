import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import type { PropertyType, SensitivityLevel } from '@/types/template';
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

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

interface DynamicGroupConfigProps {
  allowedTypes?: PropertyType[];
  maxItems?: number;
  sensitivity?: SensitivityLevel;
  onAllowedTypesChange: (types: PropertyType[]) => void;
  onMaxItemsChange: (maxItems: number | undefined) => void;
  onSensitivityChange: (level: SensitivityLevel) => void;
}

export function DynamicGroupConfig({
  allowedTypes,
  maxItems,
  sensitivity,
  onAllowedTypesChange,
  onMaxItemsChange,
  onSensitivityChange,
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
  const sensitivityLabel = t('editor:dynamic_group_sensitivity');
  const typesSummary = isAllSelected
    ? t('editor:dynamic_group_no_limit')
    : `${effectiveAllowed.length}/${ALL_PROPERTY_TYPES.length}`;
  const maxSummary = maxItems === undefined ? t('editor:dynamic_group_no_limit') : `${maxItems}`;
  const sensitivitySummary = sensitivity ?? 'internal';

  return (
    <div className={styles.wrapper}>
      <button type="button" className={styles.triggerBtn} onClick={() => setExpanded((v) => !v)}>
        <span
          className={styles.chevron}
          style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
        >
          <ChevronRight size={ICON_SIZE.sm} />
        </span>
        <span className={styles.triggerText}>
          {typesLabel}: {typesSummary}
          <span className={styles.separator}>·</span>
          {maxLabel}: {maxSummary}
          <span className={styles.separator}>·</span>
          {sensitivityLabel}:{' '}
          {t(`editor:sensitivity_levels.${sensitivitySummary}`, sensitivitySummary)}
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

          {/* 敏感度等级 */}
          <div className={styles.configSection}>
            <span className={styles.configSectionLabel}>
              {t('editor:dynamic_group_sensitivity')}
            </span>
            <select
              value={sensitivity ?? 'internal'}
              onChange={(e) => onSensitivityChange(e.target.value as SensitivityLevel)}
              style={{
                height: 34,
                padding: '0 10px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)',
                color: 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                cursor: 'pointer',
                outline: 'none',
                fontFamily: 'inherit',
                alignSelf: 'flex-start',
                minWidth: 120,
              }}
            >
              {SENSITIVITY_LEVELS.map((sl) => (
                <option key={sl} value={sl}>
                  {t(`editor:sensitivity_levels.${sl}`, sl)}
                </option>
              ))}
            </select>
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
