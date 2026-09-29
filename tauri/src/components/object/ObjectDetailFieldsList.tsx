import { useTranslation } from 'react-i18next';
import { Lock, Eye, Check, Copy } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { PluginBadge } from '@/components/template/PluginBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import {
  ProtectedFieldValue,
  type ProtectedFieldControl,
} from '@/components/ui/ProtectedFieldValue';
import {
  fieldPresentationPolicy,
  strongestSensitivity,
  protectedDisplayValue,
} from '@/lib/fieldPresentationPolicy';
import type { ObjectDetailFieldEntry } from './objectDetailUtils';
import type { PropertyType, TemplateProperty } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';
import rowStyles from '@/components/ui/FieldRowLayout.module.css';
import styles from './ObjectDetailModal.module.css';

export interface FlattenedField {
  key: string;
  label?: string;
  value: string;
  fieldId?: string;
}
interface Props {
  accountId?: string;
  objectId: string;
  fields: ObjectDetailFieldEntry[];
  typeId: string;
  contractTypeId?: string;
  objFieldDefs?: Record<string, { name: string; type: string }>;
  getFieldProperty: (key: string) => TemplateProperty | undefined;
  getFieldSensitivity: (key: string) => SensitivityLevel;
  isFieldDeprecated: (key: string) => boolean;
  getFieldName: (key: string, label?: string) => string;
  handleRevealField: (id: string, sens: SensitivityLevel, name: string) => Promise<boolean>;
  handleCopy: (value: string, key: string) => void | Promise<void>;
  copiedField: string | null;
}

/** 显示与复制共用 ProtectedFieldValue；组级操作按最高子项等级授权。 */
export function ObjectDetailFieldsList(props: Props) {
  const {
    accountId,
    objectId,
    fields,
    typeId,
    contractTypeId,
    objFieldDefs,
    getFieldProperty,
    getFieldSensitivity,
    isFieldDeprecated,
    getFieldName,
    handleRevealField,
    handleCopy,
    copiedField,
  } = props;
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const copyButton = (control: ProtectedFieldControl, value: string, key: string) => (
    <button
      type="button"
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => void control.copy(value, key)}
      className={`${styles.copyBtn} ${copiedField === key ? styles.copyBtnCopied : ''}`}
    >
      {copiedField === key ? <Check size={ICON_SIZE.xs} /> : <Copy size={ICON_SIZE.xs} />}
      <span className={styles.btnLabel}>
        {copiedField === key ? t('common:copied') : t('common:copy')}
      </span>
    </button>
  );
  const revealControl = (control: ProtectedFieldControl, sens: SensitivityLevel) => {
    if (sens === 'public') return null;
    if (!control.revealed)
      return (
        <button
          type="button"
          onClick={() => void control.reveal()}
          className={`${styles.revealBtn} ${sens === 'critical' ? styles.revealBtnCritical : ''}`}
        >
          {sens === 'critical' ? <Lock size={ICON_SIZE.xs} /> : <Eye size={ICON_SIZE.xs} />}
          <span className={styles.btnLabel}>
            {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
          </span>
        </button>
      );
    const seconds = Math.max(0, Math.ceil(control.remainingMs / 1000));
    return (
      <span
        data-testid="detail-reveal-countdown"
        title={t('common:reveal_countdown_title', {
          seconds,
          defaultValue: `Auto-hides in ${seconds}s`,
        })}
        style={{
          flexShrink: 0,
          fontSize: 'var(--text-badge)',
          color: 'var(--text-tertiary)',
          fontVariantNumeric: 'tabular-nums',
          whiteSpace: 'nowrap',
        }}
      >
        {t('common:reveal_countdown', { seconds, defaultValue: `${seconds}s` })}
      </span>
    );
  };
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {fields.map((f) => {
        const parent = fieldPresentationPolicy({
          fieldId: f.key,
          definition: { sensitivityLevel: getFieldSensitivity(f.key) },
        }).sensitivity;
        const children =
          f.kind === 'dynamicGroup'
            ? f.children.map((child) => ({
                ...child,
                level: fieldPresentationPolicy({
                  fieldId: child.id ?? child.label,
                  definition: { sensitivityLevel: child.sensitivityLevel },
                  parent,
                }).sensitivity,
              }))
            : [];
        const sens = strongestSensitivity([parent, ...children.map((child) => child.level)]);
        const policy = fieldPresentationPolicy({
          fieldId: f.key,
          definition: { sensitivityLevel: sens },
        });
        const fieldId = 'fieldId' in f && f.fieldId ? f.fieldId : `${typeId}.${f.key}`;
        const rawName = getFieldName(f.key, f.label);
        const name =
          f.key === '__dynamic_group__' || rawName === '__dynamic_group__'
            ? t('editor:field_types.dynamic_group', { defaultValue: '动态字段组' })
            : rawName;
        const deprecated = isFieldDeprecated(f.key);
        const value =
          f.kind === 'field'
            ? f.value
            : children.map((child) => `${child.label}: ${child.value}`).join('\n');
        const copyKey = f.kind === 'field' ? f.key : fieldId;
        const type = (getFieldProperty(f.key)?.type ||
          objFieldDefs?.[f.key]?.type ||
          ('type' in f ? f.type : undefined) ||
          'text') as PropertyType;
        return (
          <ProtectedFieldValue
            key={f.key}
            accountId={accountId}
            objectId={objectId}
            fieldId={fieldId}
            contentVersion={f}
            value={value}
            policy={policy}
            authorize={() => handleRevealField(fieldId, sens, name)}
            onCopy={handleCopy}
          >
            {(control) => (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                <div className={styles.fieldRow} style={{ opacity: deprecated ? 0.7 : 1 }}>
                  <div
                    data-field-presentation-row
                    className={`${styles.fieldRowTop} ${rowStyles.row}`}
                  >
                    <div
                      data-field-label-slot
                      className={`${styles.fieldLabel} ${rowStyles.label}`}
                    >
                      <FieldTypeIcon type={type} />
                      <span
                        style={{
                          fontSize: 'var(--text-caption)',
                          fontWeight: 600,
                          color: 'var(--text-secondary)',
                          textDecoration: deprecated ? 'line-through' : 'none',
                        }}
                      >
                        {name}
                      </span>
                      <SensitivityBadge level={sens} />
                      {contractTypeId && (
                        <PluginBadge contractTypeId={contractTypeId} size="sm" variant="full" />
                      )}
                      {deprecated && <DeprecatedBadge />}
                    </div>
                    <div
                      data-field-actions-slot
                      className={`${styles.fieldActions} ${rowStyles.actions}`}
                    >
                      {revealControl(control, sens)}
                      {copyButton(control, value, copyKey)}
                    </div>
                  </div>
                  {f.kind === 'field' && (
                    <div
                      className={styles.fieldValue}
                      style={{
                        color:
                          policy.concealed && !control.revealed
                            ? 'var(--text-tertiary)'
                            : 'var(--text-primary)',
                      }}
                    >
                      {control.displayValue}
                    </div>
                  )}
                </div>
                {children.map((child, index) => (
                  <div
                    key={child.id ?? index}
                    className={styles.fieldRow}
                    style={{ marginLeft: 16, opacity: deprecated ? 0.7 : 1 }}
                  >
                    <div
                      data-field-presentation-row
                      className={`${styles.fieldRowTop} ${rowStyles.row}`}
                    >
                      <div
                        data-field-label-slot
                        className={`${styles.fieldLabel} ${rowStyles.label}`}
                      >
                        <FieldTypeIcon type={(child.type || 'text') as PropertyType} />
                        <span
                          style={{
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            color: 'var(--text-secondary)',
                          }}
                        >
                          {child.label}
                        </span>
                      </div>
                      <div
                        data-field-actions-slot
                        className={`${styles.fieldActions} ${rowStyles.actions}`}
                      >
                        {copyButton(control, child.value, `${fieldId}.${index}`)}
                      </div>
                    </div>
                    <div
                      className={styles.fieldValue}
                      style={{
                        color:
                          child.level !== 'public' && !control.revealed
                            ? 'var(--text-tertiary)'
                            : 'var(--text-primary)',
                      }}
                    >
                      {protectedDisplayValue(child.value, child.level, control.revealed)}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </ProtectedFieldValue>
        );
      })}
    </div>
  );
}
