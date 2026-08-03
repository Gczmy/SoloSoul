// ============================================================
// TrashSnapshotView.tsx — 回收站详情的历史快照展示域
// P224-① 自 TrashDetailPanel.tsx 整体平移（逐字搬运，零行为变更）。
// SnapshotContent / SnapshotDataView 与动态字段组渲染。
// ============================================================

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { ICON_SIZE } from '@/lib/constants';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';
import type { SnapshotEntry } from './types';

interface SnapshotContentProps {
  _detailId: string;
  snapshots: SnapshotEntry[];
  currentSnapIdx: number;
  data: Record<string, unknown> | null | undefined;
  loading: boolean | undefined;
  detailTemplate: UserTemplate | null;
  currentPropertyLabels?: Record<string, SensitivityLevel>;
  onChangeSnapshot: (newIdx: number) => void;
}
export function SnapshotContent({
  _detailId,
  snapshots,
  currentSnapIdx,
  data,
  loading,
  detailTemplate,
  currentPropertyLabels,
  onChangeSnapshot,
}: SnapshotContentProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  // Clamp index to prevent out-of-bounds when snapshots array changes after mount
  const clampedIdx = Math.min(currentSnapIdx, Math.max(0, snapshots.length - 1));
  const currentSnap = snapshots[clampedIdx];

  return (
    <div style={{ marginTop: 8, fontSize: 'var(--text-caption)' }}>
      {snapshots.length > 1 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          <button
            disabled={clampedIdx >= snapshots.length - 1}
            onClick={() => onChangeSnapshot(clampedIdx + 1)}
            className="interactive-nav"
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderWidth: 1,
              borderStyle: 'solid',
              borderRadius: 6,
              cursor: clampedIdx >= snapshots.length - 1 ? 'default' : 'pointer',
              fontSize: 'var(--text-badge)',
              color: clampedIdx >= snapshots.length - 1 ? 'var(--text-tertiary)' : undefined,
              opacity: clampedIdx >= snapshots.length - 1 ? 0.35 : 1,
            }}
          >
            <ChevronLeft size={ICON_SIZE.sm} />
          </button>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 'var(--text-badge)',
              fontWeight: 500,
              color: 'var(--text-secondary)',
            }}
          >
            <span
              style={{
                color: 'var(--accent-primary)',
                fontWeight: 600,
                minWidth: 14,
                textAlign: 'center',
              }}
            >
              {clampedIdx + 1}
            </span>
            <span style={{ color: 'var(--text-tertiary)' }}>/</span>
            <span style={{ color: 'var(--text-tertiary)' }}>{snapshots.length}</span>
          </div>
          <button
            disabled={clampedIdx <= 0}
            onClick={() => onChangeSnapshot(Math.max(0, clampedIdx - 1))}
            className="interactive-nav"
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderWidth: 1,
              borderStyle: 'solid',
              borderRadius: 6,
              cursor: clampedIdx <= 0 ? 'default' : 'pointer',
              fontSize: 'var(--text-badge)',
              color: clampedIdx <= 0 ? 'var(--text-tertiary)' : undefined,
              opacity: clampedIdx <= 0 ? 0.35 : 1,
            }}
          >
            <ChevronRight size={ICON_SIZE.sm} />
          </button>
        </div>
      )}
      {currentSnap && (
        <div>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              padding: '6px 8px',
              background: 'var(--bg-elevated-hover)',
              borderRadius: 6,
              marginBottom: 6,
              minHeight: 32,
            }}
          >
            <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
              <SnapshotVersionBadge index={currentSnapIdx} total={snapshots.length} />
              <span
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  fontSize: 'var(--text-badge)',
                  fontWeight: 500,
                  background: 'rgba(91,124,153,0.08)',
                  color: 'var(--accent-primary)',
                }}
              >
                {t(`common:trigger_${currentSnap.triggeredBy}` as const, {
                  defaultValue: currentSnap.triggeredBy,
                })}
              </span>
            </div>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginLeft: 'auto',
              }}
            >
              {new Date(currentSnap.timestamp).toLocaleString()}
            </span>
          </div>
          <div style={{ minHeight: 60 }}>
            {loading && !data && <LoadingPlaceholder variant="base" minHeight={60} />}
            {data && (
              <SnapshotDataView
                data={data}
                detailTemplate={detailTemplate}
                currentPropertyLabels={currentPropertyLabels}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}
export interface SnapshotDataViewProps {
  data: Record<string, unknown>;
  detailTemplate: UserTemplate | null;
  currentPropertyLabels?: Record<string, SensitivityLevel>;
}
function isMetaPropertyKey(key: string): boolean {
  return [
    '__fields',
    '__attachments',
    '__templateName',
    '__templateHash',
    '__deprecatedFields',
  ].includes(key);
}
export function SnapshotDataView({
  data,
  detailTemplate,
  currentPropertyLabels: _currentPropertyLabels,
}: SnapshotDataViewProps) {
  const { t } = useTranslation(['editor', 'common']);
  const rawProps = data.properties as Record<string, unknown> | undefined;
  const tags: string[] = Array.isArray(data.tags) ? (data.tags as string[]) : [];
  const snapName = typeof data.name === 'string' ? data.name : '';

  // 字段级敏感度的真实来源是快照自身的 propertyLabels；对象当前标签（currentPropertyLabels）
  // 只用于主内容预览，不能混入历史快照，否则旧版本会显示成对象被删除时的最新敏感度。
  const sensitivityMap = useMemo(() => {
    const map = new Map<string, SensitivityLevel>();
    const labels = data.propertyLabels as Record<string, SensitivityLevel> | undefined;
    if (labels && typeof labels === 'object') {
      for (const [id, level] of Object.entries(labels)) {
        if (level) map.set(id, level);
      }
    }
    return map;
  }, [data.propertyLabels]);

  // 优先使用对象自带的 __fields 字段定义获取名称/类型；模板存在时用于排序和补充。
  const fieldDefs = useMemo(() => {
    const defs = new Map<string, { name: string; type?: PropertyType }>();
    const rawFields = rawProps?.__fields as
      | Record<string, { name?: string; type?: PropertyType }>
      | undefined;
    if (rawFields && typeof rawFields === 'object') {
      for (const [id, def] of Object.entries(rawFields)) {
        defs.set(id, {
          name: def?.name || id,
          type: def?.type,
        });
      }
    }
    if (detailTemplate) {
      for (const p of detailTemplate.properties) {
        if (defs.has(p.id)) continue;
        defs.set(p.id, {
          name: p.name,
          type: p.type,
        });
      }
    }
    return defs;
  }, [rawProps, detailTemplate]);

  const orderedFields = useMemo(() => {
    type FieldChild = {
      key: string;
      value: string;
      type?: PropertyType;
    };
    type FieldEntry =
      | {
          kind: 'field';
          key: string;
          value: string;
          type?: PropertyType;
          sensitivityLevel?: SensitivityLevel;
        }
      | {
          kind: 'dynamicGroup';
          key: string;
          type?: PropertyType;
          sensitivityLevel?: SensitivityLevel;
          children: FieldChild[];
        };

    const result: FieldEntry[] = [];
    if (!rawProps || typeof rawProps !== 'object') return result;

    const seen = new Set<string>();

    // 快照 __fields 中的敏感度是历史版本的直接证据，模板顺序与 __fields 顺序都需要它。
    const rawFields = rawProps.__fields as
      | Record<string, { name?: string; type?: PropertyType; sensitivityLevel?: SensitivityLevel }>
      | undefined;

    // 1. 模板顺序
    if (detailTemplate) {
      for (const p of detailTemplate.properties) {
        const v = rawProps[p.id];
        if (v !== null && v !== undefined && v !== '' && !isMetaPropertyKey(String(p.id))) {
          seen.add(p.id);
          const def = fieldDefs.get(p.id);
          // 快照敏感度优先顺序：快照 propertyLabels -> 快照 __fields -> 当前模板 -> internal
          const snapshotLevel = rawFields?.[p.id]?.sensitivityLevel;
          const sensitivityLevel =
            sensitivityMap.get(p.id) ||
            snapshotLevel ||
            ((p.sensitivityLevel || 'internal') as SensitivityLevel);
          if ((def?.type || p.type) === 'dynamic_group') {
            result.push({
              kind: 'dynamicGroup',
              key: def?.name || p.name,
              type: 'dynamic_group',
              sensitivityLevel,
              children: parseDynamicGroupValue(v),
            });
          } else {
            result.push({
              kind: 'field',
              key: def?.name || p.name,
              value: typeof v === 'string' ? v : JSON.stringify(v),
              type: def?.type || p.type,
              sensitivityLevel,
            });
          }
        }
      }
    }

    // 2. __fields 顺序（模板不存在时尤为重要）
    if (rawFields && typeof rawFields === 'object') {
      for (const id of Object.keys(rawFields)) {
        if (seen.has(id) || isMetaPropertyKey(String(id))) continue;
        const v = rawProps[id];
        if (v === null || v === undefined || v === '') continue;
        seen.add(id);
        const def = fieldDefs.get(id);
        const snapshotLevel = rawFields[id]?.sensitivityLevel;
        const sensitivityLevel = sensitivityMap.get(id) || snapshotLevel;
        if ((def?.type || rawFields[id]?.type) === 'dynamic_group') {
          result.push({
            kind: 'dynamicGroup',
            key: def?.name || id,
            type: 'dynamic_group',
            sensitivityLevel,
            children: parseDynamicGroupValue(v),
          });
        } else {
          result.push({
            kind: 'field',
            key: def?.name || id,
            value: typeof v === 'string' ? v : JSON.stringify(v),
            type: def?.type,
            sensitivityLevel,
          });
        }
      }
    }

    // 3. 其余未定义字段
    for (const [k, v] of Object.entries(rawProps)) {
      if (!isMetaPropertyKey(k) && !seen.has(k) && v !== null && v !== undefined && v !== '') {
        result.push({
          kind: 'field',
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
          sensitivityLevel: sensitivityMap.get(k),
        });
      }
    }

    return result;
  }, [rawProps, detailTemplate, fieldDefs, sensitivityMap]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      {snapName && (
        <div
          style={{
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            textAlign: 'right',
            overflowWrap: 'break-word',
            wordBreak: 'break-word',
          }}
        >
          {snapName}
        </div>
      )}
      {orderedFields.slice(0, 8).map((f) => {
        if (f.kind === 'dynamicGroup') {
          return (
            <DynamicGroupSnapshotRow
              key={f.key}
              groupKey={f.key}
              sensitivityLevel={f.sensitivityLevel}
              children={f.children}
            />
          );
        }
        const displayKey =
          f.key === '__dynamic_group__'
            ? t('editor:field_types.dynamic_group', f.key)
            : f.key === 'description'
              ? t('common:description')
              : f.key;
        return (
          <div
            key={f.key}
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 4,
              fontSize: 'var(--text-caption)',
              padding: '4px 0',
              borderBottom: '1px solid var(--border-subtle)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {f.type && <FieldTypeIcon type={f.type} size={ICON_SIZE.sm} />}
              <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{displayKey}</span>
              {f.sensitivityLevel && <SensitivityBadge level={f.sensitivityLevel} />}
            </div>
            <span
              style={{
                color: 'var(--text-primary)',
                textAlign: 'right',
                overflowWrap: 'break-word',
                wordBreak: 'break-word',
                whiteSpace: 'pre-wrap',
                width: '100%',
              }}
            >
              {f.value}
            </span>
          </div>
        );
      })}
      {tags.length > 0 && (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
          {tags.map((tag) => (
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
    </div>
  );
}
function parseDynamicGroupValue(v: unknown): {
  key: string;
  value: string;
  type?: PropertyType;
}[] {
  let arr: unknown[] | undefined;
  if (Array.isArray(v)) {
    arr = v;
  } else if (typeof v === 'string') {
    try {
      const parsed = JSON.parse(v);
      if (Array.isArray(parsed)) arr = parsed;
    } catch {
      arr = undefined;
    }
  }
  if (!arr) return [];
  return arr
    .filter((item): item is Record<string, unknown> => item !== null && typeof item === 'object')
    .map((item) => ({
      key: typeof item.name === 'string' ? item.name : String(item.id || ''),
      value:
        typeof item.value === 'string'
          ? item.value
          : item.value !== undefined && item.value !== null
            ? JSON.stringify(item.value)
            : '',
      type: typeof item.type === 'string' ? (item.type as PropertyType) : undefined,
    }));
}
function DynamicGroupSnapshotRow({
  groupKey,
  sensitivityLevel,
  children,
}: {
  groupKey: string;
  sensitivityLevel?: SensitivityLevel;
  children: { key: string; value: string; type?: PropertyType }[];
}) {
  const { t } = useTranslation(['editor', 'common']);
  const displayKey =
    groupKey === '__dynamic_group__'
      ? t('editor:field_types.dynamic_group', groupKey)
      : groupKey === 'description'
        ? t('common:description')
        : groupKey;
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          fontSize: 'var(--text-caption)',
          padding: '3px 0',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <FieldTypeIcon type="dynamic_group" size={ICON_SIZE.sm} />
          <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{displayKey}</span>
          {sensitivityLevel && <SensitivityBadge level={sensitivityLevel} />}
        </div>
      </div>
      {children.map((child) => (
        <div
          key={child.key}
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 4,
            fontSize: 'var(--text-caption)',
            padding: '4px 0 4px 20px',
            borderBottom: '1px solid var(--border-subtle)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            {child.type && <FieldTypeIcon type={child.type} size={ICON_SIZE.sm} />}
            <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{child.key}</span>
          </div>
          <span
            style={{
              color: 'var(--text-primary)',
              textAlign: 'right',
              overflowWrap: 'break-word',
              wordBreak: 'break-word',
              whiteSpace: 'pre-wrap',
              width: '100%',
            }}
          >
            {child.value}
          </span>
        </div>
      ))}
    </div>
  );
}
