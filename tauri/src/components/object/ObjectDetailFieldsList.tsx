import { useTranslation } from 'react-i18next';
import { Lock, Eye, Check, Copy } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { PluginBadge } from '@/components/template/PluginBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { PropertyType, TemplateProperty } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';
import styles from './ObjectDetailModal.module.css';

export interface FlattenedField {
  key: string;
  label?: string;
  value: string;
  fieldId?: string;
}

interface ObjectDetailFieldsListProps {
  fields: FlattenedField[];
  typeId: string;
  contractTypeId?: string;
  objFieldDefs?: Record<string, { name: string; type: string }> | undefined;
  getFieldProperty: (key: string) => TemplateProperty | undefined;
  getFieldSensitivity: (key: string) => SensitivityLevel;
  isFieldDeprecated: (key: string) => boolean;
  getFieldName: (key: string, label?: string) => string;
  isRevealed: (id: string) => boolean;
  maskValue: (value: string, id: string, level: SensitivityLevel) => string;
  handleRevealField: (fieldId: string, sens: SensitivityLevel, fieldName: string) => void;
  handleCopy: (value: string, key: string) => void;
  copiedField: string | null;
}

/**
 * 对象详情字段列表：字段行渲染（敏感度/复制/显示）。
 * 从 ObjectDetailModal 抽出。
 */
export function ObjectDetailFieldsList({
  fields,
  typeId,
  contractTypeId,
  objFieldDefs,
  getFieldProperty,
  getFieldSensitivity,
  isFieldDeprecated,
  getFieldName,
  isRevealed,
  maskValue,
  handleRevealField,
  handleCopy,
  copiedField,
}: ObjectDetailFieldsListProps) {
  const { t } = useTranslation(['common', 'navigation', 'editor']);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {fields.map((f) => {
        const sens = getFieldSensitivity(f.key);
        const deprecated = isFieldDeprecated(f.key);
        const fieldId = f.fieldId || `${typeId}.${f.key}`;
        const revealed = isRevealed(fieldId);
        const needsReveal = sens === 'sensitive' || sens === 'critical';
        // 字段类型图标：模板定义优先，回退到对象内嵌 __fields，最后按 text 处理
        const fieldType = (getFieldProperty(f.key)?.type ||
          objFieldDefs?.[f.key]?.type ||
          'text') as PropertyType;
        return (
          <div
            key={f.key}
            className={styles.fieldRow}
            style={{ opacity: deprecated ? 0.7 : 1 }}
          >
            <div className={styles.fieldRowTop}>
              <div className={styles.fieldLabel}>
                <FieldTypeIcon type={fieldType} />
                <span
                  style={{
                    fontSize: 'var(--text-caption)',
                    fontWeight: 600,
                    color: 'var(--text-secondary)',
                    textDecoration: deprecated ? 'line-through' : 'none',
                  }}
                >
                  {getFieldName(f.key, f.label)}
                </span>
                <SensitivityBadge level={sens} />
                {contractTypeId && (
                  <PluginBadge contractTypeId={contractTypeId} size="sm" variant="full" />
                )}
                {deprecated && <DeprecatedBadge />}
              </div>
              <div className={styles.fieldActions}>
                {needsReveal && !revealed && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      const revealName = f.label
                        ? `${t('editor:field_types.dynamic_group')}: ${f.label}`
                        : getFieldName(f.key);
                      handleRevealField(fieldId, sens, revealName);
                    }}
                    className={`${styles.revealBtn} ${sens === 'critical' ? styles.revealBtnCritical : ''}`}
                  >
                    {sens === 'critical' ? (
                      <Lock size={ICON_SIZE.xs} />
                    ) : (
                      <Eye size={ICON_SIZE.xs} />
                    )}
                    <span className={styles.btnLabel}>
                      {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
                    </span>
                  </button>
                )}
                <button
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() =>
                    handleCopy(revealed ? f.value : maskValue(f.value, fieldId, sens), f.key)
                  }
                  className={`${styles.copyBtn} ${copiedField === f.key ? styles.copyBtnCopied : ''}`}
                >
                  {copiedField === f.key ? (
                    <Check size={ICON_SIZE.xs} />
                  ) : (
                    <Copy size={ICON_SIZE.xs} />
                  )}
                  <span className={styles.btnLabel}>
                    {copiedField === f.key ? t('common:copied') : t('common:copy')}
                  </span>
                </button>
              </div>
            </div>
            <div
              className={styles.fieldValue}
              style={{
                color:
                  needsReveal && !revealed
                    ? 'var(--text-tertiary)'
                    : 'var(--text-primary)',
              }}
            >
              {revealed ? f.value : maskValue(f.value, fieldId, sens)}
            </div>
          </div>
        );
      })}
    </div>
  );
}
