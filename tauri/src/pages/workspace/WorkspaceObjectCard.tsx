import { memo, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { Clock, Paperclip, Pencil, Trash2 } from 'lucide-react';
import { getSensitivityStyle, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { MASK_PLACEHOLDER, shouldMaskSensitivity } from '@/lib/masking';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import type { ObjectSummary, ObjectData } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';
import { objectNeedsSync, resolveSemanticNeedsSync } from '@/lib/templateSync';
import { PluginBadge } from '@/components/template/PluginBadge';
import { useAuthStore } from '@/stores/authStore';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { ICON_SIZE } from '@/lib/constants';
import styles from './WorkspaceObjectCard.module.css';

/** Extract displayable key-value pairs from object properties (filters internal __ fields).
 * 对于 dynamic_group 类型，每个子字段作为独立条目返回，使用子字段名称作为 label。
 */
function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
): { key: string; value: string; label?: string }[] {
  if (!props) return [];
  // 从 properties 中提取 __fields 定义，用于识别 dynamic_group 字段
  const fieldDefs = props.__fields as Record<string, { type?: string }> | undefined;
  const entries: { key: string; value: string; label?: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue;
    if (v === null || v === undefined || v === '') continue;

    // dynamic_group 字段：每个子字段作为独立 chip 展示
    if (fieldDefs?.[k]?.type === 'dynamic_group' && Array.isArray(v)) {
      for (const item of v) {
        if (!item || typeof item !== 'object') continue;
        const { name, value: itemVal } = item as Record<string, unknown>;
        if (name === undefined || name === null || name === '') continue;
        let displayVal = '';
        if (Array.isArray(itemVal)) {
          displayVal = itemVal.join(', ');
        } else if (itemVal !== null && itemVal !== undefined) {
          displayVal = String(itemVal);
        }
        entries.push({ key: k, value: displayVal, label: String(name) });
      }
      continue;
    }

    if (typeof v === 'string') {
      entries.push({ key: k, value: v });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      entries.push({ key: k, value: String(v) });
    } else if (Array.isArray(v) && v.length > 0) {
      entries.push({ key: k, value: v.join(', ') });
    }
  }
  if (fieldOrder && fieldOrder.length > 0) {
    const orderMap = new Map(fieldOrder.map((id, i) => [id, i]));
    entries.sort((a, b) => {
      const ia = orderMap.get(a.key);
      const ib = orderMap.get(b.key);
      if (ia !== undefined && ib !== undefined) return ia - ib;
      if (ia !== undefined) return -1;
      if (ib !== undefined) return 1;
      return a.key.localeCompare(b.key);
    });
  }
  return entries;
}

interface WorkspaceObjectCardProps {
  obj: ObjectSummary | ObjectData;
  collectionLabel: string;
  userTemplates: UserTemplate[];
  snapshotCount?: number;
  attachmentCount?: number;
  /** 模板指纹映射；卡片据此懒计算是否需要同步。 */
  templateHashMap?: Map<string, string>;
  /** 当前对象是否正在打开模板同步确认弹窗；打开期间临时隐藏提示条。 */
  isSyncDialogOpen?: boolean;
  // P118: 回调统一接收 obj 参数，父级即可用 useCallback 提供稳定引用，
  // 避免搜索框每次击键重建全部卡片的内联闭包击穿 memo。
  onClick: (obj: ObjectSummary | ObjectData) => void;
  onHistory: (obj: ObjectSummary | ObjectData) => void;
  onAttachments: (obj: ObjectSummary | ObjectData) => void;
  onEdit: (obj: ObjectSummary | ObjectData) => void;
  onDelete: (obj: ObjectSummary | ObjectData) => void;
  /** 用户确认应用模板更新。 */
  onSync?: (obj: ObjectSummary | ObjectData) => void;
  /** 用户选择暂不应用模板更新。 */
  onDismissSync?: (obj: ObjectSummary | ObjectData) => void;
  /** 拖拽文件上传完成后的回调，用于刷新附件计数 */
  onUploadComplete?: () => void;
}

export const WorkspaceObjectCard = memo(function WorkspaceObjectCard({
  obj,
  collectionLabel,
  userTemplates,
  snapshotCount,
  attachmentCount,
  templateHashMap,
  isSyncDialogOpen,
  onClick,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
  onSync,
  onDismissSync,
  onUploadComplete,
}: WorkspaceObjectCardProps) {
  const { t } = useTranslation(['editor', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const tpl = userTemplates.find((t) => t.id === obj.templateId);

  // 懒加载：每张卡片根据模板指纹映射独立计算同步状态，避免父级批量计算导致切换页面闪烁。
  // 用户点击“否”后记录当时模板指纹；模板再次变更时提示条重新出现。
  const hashNeedsSync = useMemo(() => {
    if (!templateHashMap || isSyncDialogOpen) return false;
    return objectNeedsSync(obj, templateHashMap);
  }, [obj, templateHashMap, isSyncDialogOpen]);

  // 语义复核（异步）：复核期间不显示提示条，确认有真实变更才显示
  const [semanticNeedsSync, setSemanticNeedsSync] = useState(false);
  useEffect(() => {
    if (!hashNeedsSync || !accountId) {
      setSemanticNeedsSync(false);
      return;
    }
    let cancelled = false;
    resolveSemanticNeedsSync(accountId, obj.id).then((needed) => {
      if (!cancelled) setSemanticNeedsSync(needed);
    });
    return () => {
      cancelled = true;
    };
  }, [hashNeedsSync, accountId, obj.id]);

  const needsSync = hashNeedsSync && semanticNeedsSync;
  // 模板匹配需同时满足 ID 和页面归属（与编辑器 ObjectEditorPage 对齐）
  const tplMatch = tpl && (tpl.category || 'identity') === obj.typeId;
  const fieldOrder = tpl?.properties.map((p) => p.id);
  const fields = useMemo(
    () => flattenProperties(obj.properties as Record<string, unknown> | undefined, fieldOrder),
    [obj.properties, fieldOrder],
  );

  const TemplateIcon = tpl?.iconId ? resolveCustomIcon(tpl.iconId) : PAGE_ICON_MAP.custom;

  const getFieldProperty = (fieldKey: string) => tpl?.properties.find((p) => p.id === fieldKey);
  const objLabels = obj.propertyLabels as Record<string, string> | undefined;
  // 从 properties 中提取 __fields（即使模板被删除，字段定义仍保留在对象上）
  const objFieldDefs = (obj.properties as Record<string, unknown>)?.__fields as
    | Record<string, { name: string; type: string; options?: string[]; contractField?: boolean }>
    | undefined;
  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
    if (objLabels?.[fieldKey]) {
      return objLabels[fieldKey] as SensitivityLevel;
    }
    // 2. 回退到模板定义
    return (getFieldProperty(fieldKey)?.sensitivityLevel as SensitivityLevel) || 'internal';
  };
  const isFieldDeprecated = (fieldKey: string): boolean =>
    !!getFieldProperty(fieldKey)?.deprecatedAt;
  const getFieldName = (fieldKey: string): string =>
    getFieldProperty(fieldKey)?.name || objFieldDefs?.[fieldKey]?.name || fieldKey;

  const { ref: dragRef, dragState } = useDragToAttach(obj.id, {
    onComplete: onUploadComplete,
  });

  return (
    <div ref={dragRef} style={{ position: 'relative' }} data-testid="workspace-object-card">
      <Card interactive onClick={() => onClick(obj)}>
        {/* 模板更新提示条 */}
        {needsSync && onSync && (
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: 12,
              marginBottom: 12,
              padding: '10px 12px',
              borderRadius: 8,
              background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
              border: '1px solid color-mix(in srgb, var(--accent-primary) 25%, transparent)',
            }}
          >
            <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-primary)' }}>
              {t('editor:template_updated_hint')}
            </span>
            <div style={{ display: 'flex', gap: 8, flexShrink: 0 }}>
              <button
                onClick={() => onSync?.(obj)}
                style={{
                  padding: '4px 10px',
                  borderRadius: 6,
                  border: 'none',
                  background: 'var(--accent-primary)',
                  color: '#fff',
                  fontSize: 'var(--text-caption)',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                {t('common:yes')}
              </button>
              <button
                onClick={() => onDismissSync?.(obj)}
                style={{
                  padding: '4px 10px',
                  borderRadius: 6,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-secondary)',
                  fontSize: 'var(--text-caption)',
                  fontWeight: 500,
                  cursor: 'pointer',
                }}
              >
                {t('common:no')}
              </button>
            </div>
          </div>
        )}
        {/* Header row */}
        <div className={styles.cardHeader} style={{ marginBottom: fields.length > 0 ? 8 : 0 }}>
          <div className={styles.headerMain}>
            <span className={styles.headerIcon}>
              <TemplateIcon size={ICON_SIZE['2xl']} />
            </span>
            <div className={styles.headerContent}>
              {/* 第1行：对象名 */}
              <span className={styles.objectName}>{obj.name}</span>
              {/* 第2行：所属页面 */}
              <span className={styles.pageBadge}>{collectionLabel}</span>
              {/* 第3行：模板名 — 模板不匹配（已删除/更改页面）时显示删除线 */}
              {obj.templateId && (
                <span
                  className={styles.templateName}
                  style={{ textDecoration: tplMatch ? 'none' : 'line-through' }}
                >
                  {tplMatch
                    ? tpl!.name
                    : (() => {
                        const tplName = (obj.properties as Record<string, unknown>)
                          ?.__templateName as string | undefined;
                        const tid = obj.templateId || '';
                        return tplName ? `${tplName} (${tid.slice(0, 8)}…)` : tid;
                      })()}
                </span>
              )}
              {/* 第4行：插件徽章 */}
              {obj.contractTypeId && (
                <span className={styles.pluginRow}>
                  <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="full" />
                </span>
              )}
            </div>
          </div>
          {/* 移动端右列：操作按钮（仅移动端显示） */}
          <div className={styles.mobileActionCol} onClick={(e) => e.stopPropagation()}>
            <div className={styles.actionRow}>
              <BadgeIconButton
                Icon={Clock}
                count={snapshotCount}
                onClick={() => onHistory(obj)}
                title="History"
              />
              <BadgeIconButton
                Icon={Paperclip}
                count={attachmentCount}
                onClick={() => onAttachments(obj)}
                title="Attachments"
              />
            </div>
            <div className={styles.actionRow}>
              <BadgeIconButton Icon={Pencil} onClick={() => onEdit(obj)} title="Edit" />
              <BadgeIconButton
                Icon={Trash2}
                onClick={() => onDelete(obj)}
                title="Move to trash"
                dangerOutline
              />
            </div>
          </div>
          {/* 桌面端：所有操作按钮在一行（headerActions） */}
          <div className={styles.headerActions} onClick={(e) => e.stopPropagation()}>
            <BadgeIconButton
              Icon={Clock}
              count={snapshotCount}
              onClick={() => onHistory(obj)}
              title="History"
            />
            <BadgeIconButton
              Icon={Paperclip}
              count={attachmentCount}
              onClick={() => onAttachments(obj)}
              title="Attachments"
            />
            <div className={styles.actionsDivider} />
            <BadgeIconButton Icon={Pencil} onClick={() => onEdit(obj)} title="Edit" />
            <BadgeIconButton
              Icon={Trash2}
              onClick={() => onDelete(obj)}
              title="Move to trash"
              dangerOutline
            />
          </div>
        </div>
        {/* Property chips — label always visible, value blurred when masked */}
        {fields.length > 0 && (
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {fields.map((f) => {
              const sens = getFieldSensitivity(f.key);
              const deprecated = isFieldDeprecated(f.key);
              // P036: 规则统一——仅 public 不掩码（internal 同样自动掩码）
              const isMasked = shouldMaskSensitivity(sens);
              const fieldLabel = f.label || getFieldName(f.key);
              const fieldProp = getFieldProperty(f.key);
              const isContractField =
                fieldProp?.contractField === true || objFieldDefs?.[f.key]?.contractField === true;
              const s = getSensitivityStyle(sens);
              return (
                <span
                  key={f.key}
                  style={{
                    padding: '3px 8px',
                    borderRadius: 6,
                    fontSize: 'var(--text-badge)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-secondary)',
                    border: `1px solid ${isMasked ? s.fg : s.fg}`,
                    maxWidth: 220,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    opacity: deprecated ? 0.6 : 1,
                    ...(isMasked
                      ? { boxShadow: `0 0 3px ${s.fg}44` }
                      : { boxShadow: `0 0 2px ${s.fg}33` }),
                  }}
                >
                  <span
                    style={{
                      fontWeight: 600,
                      textDecoration: deprecated ? 'line-through' : 'none',
                    }}
                  >
                    {isContractField && (
                      <span style={{ marginRight: 4 }}>
                        <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="icon" />
                      </span>
                    )}
                    {fieldLabel}
                  </span>
                  <span style={{ margin: '0 3px' }}>:</span>
                  <span
                    style={{
                      ...(isMasked
                        ? {
                            filter: 'blur(5px)',
                            cursor: 'default',
                            userSelect: 'none',
                            background: 'var(--bg-subtle, rgba(128,128,128,0.12))',
                            borderRadius: 2,
                            padding: '0 2px',
                          }
                        : { color: 'var(--text-primary)' }),
                    }}
                  >
                    {isMasked ? MASK_PLACEHOLDER : f.value}
                  </span>
                </span>
              );
            })}
          </div>
        )}
        {/* Tag pills */}
        {obj.tags && obj.tags.length > 0 && (
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 6 }}>
            {obj.tags.map((tag) => (
              <span
                key={tag}
                style={{
                  padding: '1px 7px',
                  borderRadius: 10,
                  fontSize: 'var(--text-badge)',
                  background: 'rgba(91,124,153,0.08)',
                  color: 'var(--accent-primary)',
                  fontWeight: 500,
                }}
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </Card>
      <DragUploadOverlay dragState={dragState} borderRadius={12} />
    </div>
  );
});
