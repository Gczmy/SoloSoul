import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Lock, Eye, Check, Copy } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { PluginBadge } from '@/components/template/PluginBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { ObjectDetailFieldEntry } from './objectDetailUtils';
import type { PropertyType, TemplateProperty } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';
import styles from './ObjectDetailModal.module.css';

/** 兼容别名：旧消费方（测试等）仍引用此名。 */
export interface FlattenedField {
  key: string;
  label?: string;
  value: string;
  fieldId?: string;
}

interface ObjectDetailFieldsListProps {
  /** 分组保留条目：普通字段 + 动态字段组（树状渲染，与历史快照同构）。 */
  fields: ObjectDetailFieldEntry[];
  typeId: string;
  contractTypeId?: string;
  objFieldDefs?: Record<string, { name: string; type: string }> | undefined;
  getFieldProperty: (key: string) => TemplateProperty | undefined;
  getFieldSensitivity: (key: string) => SensitivityLevel;
  isFieldDeprecated: (key: string) => boolean;
  getFieldName: (key: string, label?: string) => string;
  isRevealed: (id: string) => boolean;
  /** 剩余揭示时长（ms），供揭示态倒计时展示（与 useRevealState 的 1 分钟 TTL 一致）。 */
  revealRemainingMs: (id: string) => number;
  maskValue: (value: string, id: string, level: SensitivityLevel) => string;
  handleRevealField: (fieldId: string, sens: SensitivityLevel, fieldName: string) => void;
  handleCopy: (value: string, key: string) => void;
  copiedField: string | null;
}

/**
 * 对象详情字段列表：字段行渲染（敏感度/复制/显示）。
 * 从 ObjectDetailModal 抽出。
 *
 * 动态字段组以树状结构渲染（组头 + 缩进子行），与历史快照（HistoryViewer）
 * 同构：组头展示组图标与名称并**只标一次**敏感度徽章；子行展示各自类型的
 * 字段图标 + 名称 + 值，不再重复敏感度等级。
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
  revealRemainingMs,
  maskValue,
  handleRevealField,
  handleCopy,
  copiedField,
}: ObjectDetailFieldsListProps) {
  const { t } = useTranslation(['common', 'navigation', 'editor']);

  // 组条目的揭示/复制统一使用 `${typeId}.${key}` 作为 fieldId
  const entryFieldId = (f: ObjectDetailFieldEntry): string =>
    'fieldId' in f && f.fieldId ? f.fieldId : `${typeId}.${f.key}`;

  // 揭示中的字段展示自动隐藏倒计时（每秒跳动，驱动重渲染；具体秒数渲染时实时计算）
  const [, setTick] = useState(0);
  const anyRevealed = fields.some((f) => {
    const sens = getFieldSensitivity(f.key);
    if (sens !== 'sensitive' && sens !== 'critical') return false;
    return isRevealed(entryFieldId(f));
  });
  useEffect(() => {
    if (!anyRevealed) return;
    const timer = window.setInterval(() => setTick((t) => t + 1), 1000);
    return () => window.clearInterval(timer);
  }, [anyRevealed]);

  /** 动态字段组树状渲染（组头 + 子行），样式与 HistoryViewer 快照卡一致。 */
  const renderDynamicGroup = (f: Extract<ObjectDetailFieldEntry, { kind: 'dynamicGroup' }>) => {
    const sens = getFieldSensitivity(f.key);
    const deprecated = isFieldDeprecated(f.key);
    const fieldId = `${typeId}.${f.key}`;
    const revealed = isRevealed(fieldId);
    const needsReveal = sens === 'sensitive' || sens === 'critical';
    const displayMasked = needsReveal && !revealed;
    const revealSeconds = revealed
      ? Math.max(0, Math.ceil(revealRemainingMs(fieldId) / 1000))
      : 0;

    // 组名：模板名优先；`__dynamic_group__` 元键回退为本地化「动态字段组」
    const rawName = f.label || getFieldName(f.key);
    const groupName =
      f.key === '__dynamic_group__' || rawName === '__dynamic_group__'
        ? t('editor:field_types.dynamic_group', { defaultValue: '动态字段组' })
        : rawName;

    // 整组复制：逐子行 "label: value"，掩码态复制占位符（不泄露明文）
    const groupCopyValue = f.children
      .map((child) => {
        const v = displayMasked ? maskValue(child.value, fieldId, sens) : child.value;
        return `${child.label}: ${v}`;
      })
      .join('\n');

    return (
      <div key={f.key} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {/* 组头行：组图标 + 名称 + 单次敏感度徽章 */}
        <div className={styles.fieldRow} style={{ opacity: deprecated ? 0.7 : 1 }}>
          <div className={styles.fieldRowTop}>
            <div className={styles.fieldLabel}>
              <FieldTypeIcon type={(f.type as PropertyType) || 'text'} />
              <span
                style={{
                  fontSize: 'var(--text-caption)',
                  fontWeight: 600,
                  color: 'var(--text-secondary)',
                  textDecoration: deprecated ? 'line-through' : 'none',
                }}
              >
                {groupName}
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
                    handleRevealField(fieldId, sens, groupName);
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
              {needsReveal && revealed && (
                <span
                  title={t('common:reveal_countdown_title', {
                    seconds: revealSeconds,
                    defaultValue: `Auto-hides in ${revealSeconds}s`,
                  })}
                  data-testid="detail-reveal-countdown"
                  style={{
                    flexShrink: 0,
                    display: 'inline-flex',
                    alignItems: 'center',
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    fontVariantNumeric: 'tabular-nums',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {t('common:reveal_countdown', {
                    seconds: revealSeconds,
                    defaultValue: `${revealSeconds}s`,
                  })}
                </span>
              )}
              <button
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => handleCopy(groupCopyValue, fieldId)}
                className={`${styles.copyBtn} ${copiedField === fieldId ? styles.copyBtnCopied : ''}`}
              >
                {copiedField === fieldId ? (
                  <Check size={ICON_SIZE.xs} />
                ) : (
                  <Copy size={ICON_SIZE.xs} />
                )}
                <span className={styles.btnLabel}>
                  {copiedField === fieldId ? t('common:copied') : t('common:copy')}
                </span>
              </button>
            </div>
          </div>
        </div>

        {/* 子行：各自类型图标 + 名称 + 值（不重复敏感度徽章） */}
        {f.children.map((child, idx) => (
          <div
            key={`${fieldId}-child-${idx}`}
            className={styles.fieldRow}
            style={{ marginLeft: 16, opacity: deprecated ? 0.7 : 1 }}
          >
            <div className={styles.fieldRowTop}>
              <div className={styles.fieldLabel}>
                <FieldTypeIcon type={(child.type as PropertyType) || 'text'} />
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
              <div className={styles.fieldActions}>
                <button
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() =>
                    handleCopy(
                      displayMasked
                        ? maskValue(child.value, `${fieldId}.${idx}`, sens)
                        : child.value,
                      `${fieldId}.${idx}`,
                    )
                  }
                  className={`${styles.copyBtn} ${
                    copiedField === `${fieldId}.${idx}` ? styles.copyBtnCopied : ''
                  }`}
                >
                  {copiedField === `${fieldId}.${idx}` ? (
                    <Check size={ICON_SIZE.xs} />
                  ) : (
                    <Copy size={ICON_SIZE.xs} />
                  )}
                  <span className={styles.btnLabel}>
                    {copiedField === `${fieldId}.${idx}` ? t('common:copied') : t('common:copy')}
                  </span>
                </button>
              </div>
            </div>
            <div
              className={styles.fieldValue}
              style={{
                color: displayMasked ? 'var(--text-tertiary)' : 'var(--text-primary)',
              }}
            >
              {displayMasked ? maskValue(child.value, `${fieldId}.${idx}`, sens) : child.value}
            </div>
          </div>
        ))}
      </div>
    );
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {fields.map((f) => {
        // 动态字段组：树状渲染（组头单次徽章 + 缩进子行），与历史快照同构
        if (f.kind === 'dynamicGroup') {
          return renderDynamicGroup(f);
        }
        const sens = getFieldSensitivity(f.key);
        const deprecated = isFieldDeprecated(f.key);
        const fieldId = f.fieldId || `${typeId}.${f.key}`;
        const revealed = isRevealed(fieldId);
        // 详情卡片：internal/public 直接明文；仅 sensitive/critical 掩码（点击揭示）。
        // workspace 卡片仍按 masking.shouldMaskSensitivity 对 internal 模糊（模糊层不同）。
        const needsReveal = sens === 'sensitive' || sens === 'critical';
        const displayMasked = needsReveal && !revealed;
        const revealSeconds = revealed
          ? Math.max(0, Math.ceil(revealRemainingMs(fieldId) / 1000))
          : 0;
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
                {needsReveal && revealed && (
                  <span
                    title={t('common:reveal_countdown_title', {
                      seconds: revealSeconds,
                      defaultValue: `Auto-hides in ${revealSeconds}s`,
                    })}
                    data-testid="detail-reveal-countdown"
                    style={{
                      flexShrink: 0,
                      display: 'inline-flex',
                      alignItems: 'center',
                      fontSize: 'var(--text-badge)',
                      color: 'var(--text-tertiary)',
                      fontVariantNumeric: 'tabular-nums',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {t('common:reveal_countdown', {
                      seconds: revealSeconds,
                      defaultValue: `${revealSeconds}s`,
                    })}
                  </span>
                )}
                <button
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() =>
                    handleCopy(
                      displayMasked ? maskValue(f.value, fieldId, sens) : f.value,
                      f.key,
                    )
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
                color: displayMasked ? 'var(--text-tertiary)' : 'var(--text-primary)',
              }}
            >
              {displayMasked ? maskValue(f.value, fieldId, sens) : f.value}
            </div>
          </div>
        );
      })}
    </div>
  );
}
