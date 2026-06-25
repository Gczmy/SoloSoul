import { Card } from '@/components/ui/Card';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { Clock, Paperclip, Pencil, Trash2 } from 'lucide-react';
import { getSensitivityStyle, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import type { ObjectSummary, ObjectData } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';
import { PluginBadge } from '@/components/template/PluginBadge';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';

/** Extract displayable key-value pairs from object properties (filters internal __ fields). */
function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
): { key: string; value: string }[] {
  if (!props) return [];
  const entries: { key: string; value: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue;
    if (v === null || v === undefined || v === '') continue;
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
  onClick: () => void;
  onHistory: () => void;
  onAttachments: () => void;
  onEdit: () => void;
  onDelete: () => void;
  /** 拖拽文件上传完成后的回调，用于刷新附件计数 */
  onUploadComplete?: () => void;
}

export function WorkspaceObjectCard({
  obj,
  collectionLabel,
  userTemplates,
  snapshotCount,
  attachmentCount,
  onClick,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
  onUploadComplete,
}: WorkspaceObjectCardProps) {
  const tpl = userTemplates.find((t) => t.id === obj.templateId);
  // 模板匹配需同时满足 ID 和页面归属（与编辑器 ObjectEditorPage 对齐）
  const tplMatch = tpl && (tpl.category || 'identity') === obj.collectionType;
  const fieldOrder = tpl?.properties.map((p) => p.id);
  const fields = flattenProperties(
    obj.properties as Record<string, unknown> | undefined,
    fieldOrder,
  );

  const TemplateIcon = tpl?.iconId ? resolveCustomIcon(tpl.iconId) : PAGE_ICON_MAP.custom;

  const getFieldProperty = (fieldKey: string) =>
    tpl?.properties.find((p) => p.id === fieldKey);
  const objLabels = obj.propertyLabels as Record<string, string> | undefined;
  // 从 properties 中提取 __fields（即使模板被删除，字段定义仍保留在对象上）
  const objFieldDefs = (obj.properties as Record<string, unknown>)?.__fields as
    | Record<string, { name: string; type: string; options?: string[]; contractField?: boolean }>
    | undefined;
  const getFieldDef = (fieldKey: string) =>
    objFieldDefs?.[fieldKey] || getFieldProperty(fieldKey);
  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
    if (objLabels?.[fieldKey]) {
      return objLabels[fieldKey] as SensitivityLevel;
    }
    // 2. 回退到模板定义
    return (getFieldProperty(fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public';
  };
  const isFieldDeprecated = (fieldKey: string): boolean =>
    !!getFieldProperty(fieldKey)?.deprecatedAt;
  const getFieldName = (fieldKey: string): string =>
    getFieldProperty(fieldKey)?.name || objFieldDefs?.[fieldKey]?.name || fieldKey;

  const iconButtonStyle: React.CSSProperties = {
    width: 32,
    height: 32,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    border: 'none',
    borderRadius: 8,
    background: 'transparent',
    cursor: 'pointer',
    color: 'var(--text-tertiary)',
    transition: 'all 0.15s ease',
  };

  const { ref: dragRef, dragState } = useDragToAttach(obj.id, {
    onComplete: onUploadComplete,
  });

  return (
    <div ref={dragRef} style={{ position: 'relative' }}>
    <Card interactive onClick={onClick}>
      {/* Header row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: fields.length > 0 ? 8 : 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, overflow: 'hidden', minWidth: 0 }}>
          <span style={{ flexShrink: 0, display: 'flex' }}>
            <TemplateIcon size={22} />
          </span>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, minWidth: 0 }}>
            <span style={{ fontSize: 14, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{obj.name}</span>
            <span
              style={{
                fontSize: 10,
                color: 'var(--text-tertiary)',
                marginLeft: 2,
                padding: '1px 5px',
                borderRadius: 4,
                background: 'var(--bg-elevated)',
                flexShrink: 0,
                whiteSpace: 'nowrap',
              }}
            >
              {collectionLabel}
            </span>
            {obj.contractTypeId && (
              <PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="full" />
            )}
            {/* 模板名 — 模板不匹配（已删除/更改页面）时显示删除线 */}
            {obj.templateId && (
              <span
                style={{
                  fontSize: 10,
                  color: 'var(--text-tertiary)',
                  textDecoration: tplMatch ? 'none' : 'line-through',
                  flexShrink: 0,
                  whiteSpace: 'nowrap',
                }}
              >
                {tplMatch
                  ? tpl!.name
                  : (() => {
                      const tplName = (obj.properties as Record<string, unknown>)?.__templateName as string | undefined;
                      const tid = obj.templateId || '';
                      return tplName ? `${tplName} (${tid.slice(0, 8)}…)` : tid;
                    })()
                }
              </span>
            )}
          </div>
        </div>
        {/* Edit + Delete + History actions */}
        <div style={{ display: 'flex', gap: 2 }} onClick={(e) => e.stopPropagation()}>
          <CountButton count={snapshotCount} onClick={onHistory} title="History" icon={Clock} />
          <CountButton
            count={attachmentCount}
            onClick={onAttachments}
            title="Attachments"
            icon={Paperclip}
          />
          <button
            onClick={onEdit}
            title="Edit"
            style={iconButtonStyle}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }}
          >
            <Pencil size={14} />
          </button>
          <button
            onClick={onDelete}
            title="Move to trash"
            style={iconButtonStyle}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
              e.currentTarget.style.color = '#e74c3c';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }}
          >
            <Trash2 size={14} />
          </button>
        </div>
      </div>
      {/* Property chips — label always visible, value blurred when masked */}
      {fields.length > 0 && (
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {fields.map((f) => {
            const sens = getFieldSensitivity(f.key);
            const deprecated = isFieldDeprecated(f.key);
            const isMasked = sens !== 'public';
            const fieldLabel = getFieldName(f.key);
            const fieldProp = getFieldProperty(f.key);
            const isContractField = fieldProp?.contractField === true || objFieldDefs?.[f.key]?.contractField === true;
            const s = getSensitivityStyle(sens);
            return (
              <span
                key={f.key}
                style={{
                  padding: '3px 8px',
                  borderRadius: 6,
                  fontSize: 11,
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
                  {isContractField && <span style={{ marginRight: 4 }}><PluginBadge contractTypeId={obj.contractTypeId} size="sm" variant="icon" /></span>}
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
                  {isMasked ? '••••' : f.value}
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
                fontSize: 10,
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
}

function CountButton({
  count,
  onClick,
  title,
  icon: Icon,
}: {
  count?: number;
  onClick: () => void;
  title: string;
  icon: React.ComponentType<{ size?: number }>;
}) {
  return (
    <div style={{ position: 'relative' }}>
      <button
        onClick={onClick}
        title={title}
        style={{
          width: 32,
          height: 32,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          border: 'none',
          borderRadius: 8,
          background: 'transparent',
          cursor: 'pointer',
          color: 'var(--text-tertiary)',
          transition: 'all 0.15s ease',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
          e.currentTarget.style.color = 'var(--accent-primary)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'transparent';
          e.currentTarget.style.color = 'var(--text-tertiary)';
        }}
      >
        <Icon size={14} />
      </button>
      {count !== undefined && count > 0 && (
        <span
          data-testid={`count-badge-${title.toLowerCase()}`}
          style={{
            position: 'absolute',
            top: -2,
            right: -2,
            minWidth: 14,
            height: 14,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'transparent',
            border: '1px solid var(--accent-primary)',
            color: 'var(--accent-primary)',
            fontSize: 9,
            fontWeight: 700,
            borderRadius: 7,
            padding: '0 3px',
            lineHeight: 1,
          }}
        >
          {count > 99 ? '99+' : count}
        </span>
      )}
    </div>
  );
}
