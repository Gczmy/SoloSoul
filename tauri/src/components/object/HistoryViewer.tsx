import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Clock, ChevronLeft, ChevronRight, X } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import type { PropertyType } from '@/types/template';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { useRevealState } from '@/hooks/useRevealState';
import { resolveCollectionLabel } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { ICON_SIZE } from '@/lib/constants';
import { ValueContainer } from '@/components/ui/ValueContainer';

export interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

export type FlattenedField =
  | {
      kind: 'field';
      key: string;
      value: string;
      label?: string;
      sensitivity?: SensitivityLevel;
      type?: PropertyType;
    }
  | {
      kind: 'dynamicGroup';
      key: string;
      label?: string;
      sensitivity?: SensitivityLevel;
      type?: PropertyType;
      children: { label: string; value: string; type?: string }[];
    };

export function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
): FlattenedField[] {
  if (!props) return [];
  const fieldDefs = props.__fields as Record<string, { type?: string; name?: string }> | undefined;
  const entries: FlattenedField[] = [];
  for (const [k, v] of Object.entries(props)) {
    // 跳过对象元数据字段，但保留字段定义中存在的 key（如 __dynamic_group__）
    if (k.startsWith('__') && !fieldDefs?.[k]) continue;
    if (v === null || v === undefined || v === '') continue;

    if (fieldDefs?.[k]?.type === 'dynamic_group' && Array.isArray(v)) {
      if (v.length === 0) continue;
      const children: { label: string; value: string; type?: string }[] = [];
      for (const item of v) {
        if (!item || typeof item !== 'object') continue;
        const { name, value: itemVal, type: itemType } = item as Record<string, unknown>;
        if (name === undefined || name === null || name === '') continue;
        let displayVal = '';
        if (Array.isArray(itemVal)) {
          displayVal = itemVal.join(', ');
        } else if (itemVal !== null && itemVal !== undefined) {
          displayVal = String(itemVal);
        }
        children.push({
          label: String(name),
          value: displayVal,
          type: typeof itemType === 'string' ? itemType : undefined,
        });
      }
      entries.push({
        kind: 'dynamicGroup',
        key: k,
        label: fieldDefs?.[k]?.name,
        type: 'dynamic_group',
        children,
      });
      continue;
    }

    if (typeof v === 'string') {
      entries.push({
        kind: 'field',
        key: k,
        value: v,
        label: fieldDefs?.[k]?.name,
        type: fieldDefs?.[k]?.type as PropertyType | undefined,
      });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      entries.push({
        kind: 'field',
        key: k,
        value: String(v),
        label: fieldDefs?.[k]?.name,
        type: fieldDefs?.[k]?.type as PropertyType | undefined,
      });
    } else if (Array.isArray(v) && v.length > 0) {
      entries.push({
        kind: 'field',
        key: k,
        value: v.join(', '),
        label: fieldDefs?.[k]?.name,
        type: fieldDefs?.[k]?.type as PropertyType | undefined,
      });
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

function SnapshotCard({
  snap,
  index,
  total,
  t: _t,
  getFieldSensitivity,
  isFieldDeprecated,
  getFieldName,
  fieldOrder,
  verifyPassword,
  onCriticalAccess,
}: {
  snap: SnapshotEntry;
  index: number;
  total: number;
  t: (k: string) => string;
  getFieldSensitivity: (fieldKey: string) => SensitivityLevel;
  isFieldDeprecated: (fieldKey: string) => boolean;
  getFieldName: (fieldKey: string) => string;
  fieldOrder?: string[];
  verifyPassword: () => Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
  }>;
  onCriticalAccess?: (
    fieldName: string,
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin',
  ) => void;
}) {
  const [snapData, setSnapData] = useState<Record<string, unknown> | null>(null);
  const { isRevealed, reveal } = useRevealState();
  const { t } = useTranslation(['common', 'editor']);

  useEffect(() => {
    invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshot_id: snap.id }).then(
      setSnapData,
    );
  }, [snap.id]);

  const rawProps =
    snapData && typeof snapData === 'object' && 'properties' in snapData
      ? (snapData.properties as Record<string, unknown> | undefined)
      : undefined;
  const snapPropertyLabels =
    snapData && typeof snapData === 'object' && 'propertyLabels' in snapData
      ? (snapData.propertyLabels as Record<string, string> | undefined)
      : undefined;
  const fieldDefs = rawProps?.__fields as
    | Record<string, { type?: string; sensitivityLevel?: string }>
    | undefined;
  const fields = flattenProperties(rawProps, fieldOrder);
  const snapName =
    snapData && typeof snapData === 'object' && 'name' in snapData ? String(snapData.name) : '';
  const tags: string[] =
    snapData && typeof snapData === 'object' && 'tags' in snapData && Array.isArray(snapData.tags)
      ? (snapData.tags as string[])
      : [];

  const resolveFieldSensitivity = (field: FlattenedField): SensitivityLevel => {
    return (
      field.sensitivity ||
      (snapPropertyLabels?.[field.key] as SensitivityLevel | undefined) ||
      (fieldDefs?.[field.key]?.sensitivityLevel as SensitivityLevel | undefined) ||
      getFieldSensitivity(field.key) ||
      'internal'
    );
  };

  const renderValueSpan = (opts: {
    value: string;
    fieldId: string;
    sens: SensitivityLevel;
    fieldLabel?: string;
  }) => {
    const { value, fieldId, sens, fieldLabel } = opts;
    const revealed = isRevealed(fieldId);
    const needsReveal = sens === 'sensitive' || sens === 'critical';
    return (
      <span
        onClick={
          needsReveal && !revealed
            ? async () => {
                try {
                  if (sens === 'critical') {
                    const result = await verifyPassword();
                    if (result.ok) {
                      reveal(fieldId);
                      const criticalFieldName = fieldLabel
                        ? `${t('editor:field_types.dynamic_group')}: ${fieldLabel}`
                        : getFieldName(fieldId);
                      onCriticalAccess?.(criticalFieldName, result.method);
                    }
                  } else {
                    reveal(fieldId);
                  }
                } catch {
                  /* ignore */
                }
              }
            : undefined
        }
        style={{
          cursor: needsReveal && !revealed ? 'pointer' : 'default',
          filter: needsReveal && !revealed ? 'blur(5px)' : 'blur(0px)',
          userSelect: needsReveal && !revealed ? 'none' : 'auto',
          background:
            needsReveal && !revealed ? 'var(--bg-subtle, rgba(128,128,128,0.15))' : 'transparent',
          borderRadius: 2,
          padding: '0 2px',
          color: 'var(--text-primary)',
          transition: 'filter 0.15s ease, background 0.15s ease',
          willChange: needsReveal && !revealed ? 'filter' : 'auto',
        }}
        title={needsReveal && !revealed ? t('common:click_to_reveal') || 'Click to reveal' : ''}
      >
        {value}
      </span>
    );
  };

  const getFieldNameLabel = (field: FlattenedField): string => {
    const rawLabel = field.label || getFieldName(field.key);
    if (field.key === '__dynamic_group__' || rawLabel === '__dynamic_group__') {
      return t('editor:field_types.dynamic_group', { defaultValue: '动态字段组' });
    }
    return rawLabel;
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {/* Version badge */}
      <div style={{ display: 'flex', alignItems: 'flex-start' }}>
        <SnapshotVersionBadge index={index} total={total} />
        <div
          style={{
            marginLeft: 'auto',
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            overflowWrap: 'break-word',
            wordBreak: 'break-word',
            textAlign: 'right',
            maxWidth: '70%',
          }}
        >
          {snapName}
        </div>
      </div>
      {/* Properties — tree-structured dynamic groups */}
      {fields.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginTop: 4 }}>
          {fields.map((f) => {
            if (f.kind === 'dynamicGroup') {
              const sens = resolveFieldSensitivity(f);
              const deprecated = isFieldDeprecated(f.key);
              const fieldId = f.key;
              return (
                <div key={fieldId} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  {/* Parent dynamic group row */}
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      fontSize: 'var(--text-caption)',
                      padding: '6px 8px',
                      borderRadius: 6,
                      background: 'var(--bg-toolbar)',
                      border: '1px solid var(--border-subtle)',
                      opacity: deprecated ? 0.7 : 1,
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4, minWidth: 90 }}>
                      <FieldTypeIcon type={f.type || 'text'} />
                      <span
                        style={{
                          fontWeight: 500,
                          color: 'var(--text-secondary)',
                          textDecoration: deprecated ? 'line-through' : 'none',
                        }}
                      >
                        {getFieldNameLabel(f)}
                      </span>
                      <SensitivityBadge level={sens} />
                      {deprecated && <DeprecatedBadge />}
                    </div>
                  </div>
                  {/* Child fields */}
                  {f.children.map((child, idx) => (
                    <div
                      key={`${fieldId}-child-${idx}`}
                      style={{
                        display: 'flex',
                        flexWrap: 'wrap',
                        alignItems: 'flex-start',
                        gap: 8,
                        marginLeft: 16,
                        fontSize: 'var(--text-caption)',
                        padding: '6px 8px',
                        borderRadius: 6,
                        background: 'var(--bg-toolbar)',
                        border: '1px solid var(--border-subtle)',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: 4, minWidth: 74, flex: '0 0 auto' }}>
                        <FieldTypeIcon type={(child.type as PropertyType) || 'text'} />
                        <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>
                          {child.label}
                        </span>
                      </div>
                      <ValueContainer value={child.value}>
                        {renderValueSpan({
                          value: child.value,
                          fieldId,
                          sens,
                          fieldLabel: child.label,
                        })}
                      </ValueContainer>
                    </div>
                  ))}
                </div>
              );
            }

            const sens = resolveFieldSensitivity(f);
            const deprecated = isFieldDeprecated(f.key);
            const fieldId = f.key;
            return (
              <div
                key={`${f.key}-${f.label}`}
                style={{
                  display: 'flex',
                  flexWrap: 'wrap',
                  alignItems: 'flex-start',
                  gap: 8,
                  fontSize: 'var(--text-caption)',
                  padding: '6px 8px',
                  borderRadius: 6,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                  opacity: deprecated ? 0.7 : 1,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, minWidth: 90, flex: '0 0 auto' }}>
                  <FieldTypeIcon type={f.type || 'text'} />
                  <span
                    style={{
                      fontWeight: 500,
                      color: 'var(--text-secondary)',
                      textDecoration: deprecated ? 'line-through' : 'none',
                    }}
                  >
                    {getFieldNameLabel(f)}
                  </span>
                  <SensitivityBadge level={sens} />
                  {deprecated && <DeprecatedBadge />}
                </div>
                <ValueContainer value={f.value}>
                  {renderValueSpan({ value: f.value, fieldId, sens, fieldLabel: f.label })}
                </ValueContainer>
              </div>
            );
          })}
        </div>
      )}
      {/* Tags */}
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

export interface HistoryViewerProps {
  objectId: string;
  objectName?: string;
  collectionType?: string;
  onClose: () => void;
  passwordVerify: () => Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin';
  }>;
  getFieldSensitivity: (fieldKey: string) => SensitivityLevel;
  isFieldDeprecated: (fieldKey: string) => boolean;
  getFieldName: (fieldKey: string) => string;
  fieldOrder?: string[];
  zIndex?: number;
}

export function HistoryViewer({
  objectId,
  objectName,
  collectionType,
  onClose,
  passwordVerify,
  getFieldSensitivity,
  isFieldDeprecated,
  getFieldName,
  fieldOrder,
  zIndex = 2000,
}: HistoryViewerProps) {
  const [snapshots, setSnapshots] = useState<SnapshotEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentIdx, setCurrentIdx] = useState(0);
  const [animDir, setAnimDir] = useState<'left' | 'right' | null>(null);
  const navTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { t } = useTranslation(['common', 'editor', 'navigation']);
  const customPages = useSettingsStore((s) => s.settings.customPages);

  const resolveCollectionLabelLocal = (collectionType: string) =>
    resolveCollectionLabel(collectionType, customPages, t);

  const writeCriticalAccessLog = async (
    fieldName: string,
    method: 'password' | 'touchId' | 'faceId' | 'windowsHello' | 'pin',
  ) => {
    if (!objectName) return;
    const actionType =
      method === 'password'
        ? 'critical_field_login'
        : method === 'pin'
          ? 'critical_field_pin'
          : method === 'touchId'
            ? 'critical_field_touch_id'
            : method === 'windowsHello'
              ? 'critical_field_windows_hello'
              : 'critical_field_face_id';
    const entityType = method === 'password' || method === 'pin' ? 'auth' : 'biometric';
    const pageLabel = collectionType ? resolveCollectionLabelLocal(collectionType) : '';
    const details = `objectName=${objectName} page=${pageLabel} fieldName=${fieldName}`;
    try {
      await invoke('log_write', {
        request: {
          actionType,
          entityType,
          entityId: objectId,
          entityName: null,
          details,
        },
      });
    } catch {
      // best effort
    }
  };

  useEffect(() => {
    invoke<SnapshotEntry[]>('snapshot_list', { object_id: objectId })
      .then(setSnapshots)
      .finally(() => setLoading(false));
  }, [objectId]);

  useEffect(() => {
    return () => {
      if (navTimeoutRef.current) {
        clearTimeout(navTimeoutRef.current);
      }
    };
  }, []);

  const goPrev = () => {
    if (currentIdx < snapshots.length - 1) {
      setAnimDir('right');
      navTimeoutRef.current = setTimeout(() => {
        setCurrentIdx((i) => i + 1);
        setAnimDir(null);
      }, 150);
    }
  };
  const goNext = () => {
    if (currentIdx > 0) {
      setAnimDir('left');
      navTimeoutRef.current = setTimeout(() => {
        setCurrentIdx((i) => i - 1);
        setAnimDir(null);
      }, 150);
    }
  };

  const snap = snapshots[currentIdx];
  const total = snapshots.length;
  const isOldest = currentIdx >= total - 1;
  const isLatest = currentIdx <= 0;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(6px)',
      }}
      onClick={onClose}
    >
      {!loading && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          onClick={(e) => e.stopPropagation()}
          style={{
            position: 'relative',
            width: 460,
            maxHeight: '80vh',
            overflowY: 'auto',
            display: 'flex',
            flexDirection: 'column',
            background: 'var(--bg-elevated)',
            borderRadius: 16,
            boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
            border: '1px solid var(--border-subtle)',
            transform:
              animDir === 'left'
                ? 'perspective(1200px) rotateY(-8deg)'
                : animDir === 'right'
                  ? 'perspective(1200px) rotateY(8deg)'
                  : 'perspective(1200px) rotateY(0)',
            transition: 'transform 0.15s ease',
            transformOrigin:
              animDir === 'left' ? 'left center' : animDir === 'right' ? 'right center' : 'center',
          }}
        >
          {/* Header */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '14px 18px',
              borderBottom: '1px solid var(--border-subtle)',
            }}
          >
            <div
              style={{
                fontSize: 'var(--text-body-sm)',
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Clock size={ICON_SIZE.sm} /> {t('common:history')}
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  fontWeight: 400,
                }}
              >
                {loading ? '' : `${currentIdx + 1} / ${total}`}
              </span>
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <BadgeIconButton
                Icon={ChevronLeft}
                onClick={goPrev}
                title={t('common:previous') || 'Previous'}
                disabled={isOldest || loading}
                iconSize={ICON_SIZE.md}
              />
              <BadgeIconButton
                Icon={ChevronRight}
                onClick={goNext}
                title={t('common:next') || 'Next'}
                disabled={isLatest || loading}
                iconSize={ICON_SIZE.md}
              />
              <BadgeIconButton
                Icon={X}
                onClick={onClose}
                title={t('common:close') || 'Close'}
                iconSize={ICON_SIZE.md}
              />
            </div>
          </div>
          {/* Content */}
          <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
            {!snap ? (
              <div
                style={{
                  textAlign: 'center',
                  padding: 48,
                  color: 'var(--text-secondary)',
                  fontSize: 'var(--text-body)',
                }}
              >
                {t('common:no_history')}
              </div>
            ) : (
              <SnapshotCard
                snap={snap}
                index={currentIdx}
                total={total}
                t={t}
                getFieldSensitivity={getFieldSensitivity}
                isFieldDeprecated={isFieldDeprecated}
                getFieldName={getFieldName}
                fieldOrder={fieldOrder}
                verifyPassword={passwordVerify}
                onCriticalAccess={writeCriticalAccessLog}
              />
            )}
          </div>
          {/* Footer */}
          <div
            style={{
              padding: '10px 18px',
              borderTop: '1px solid var(--border-subtle)',
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
            }}
          >
            {snap &&
              (() => {
                const triggerLabel = t(`common:trigger_${snap.triggeredBy}` as const, {
                  defaultValue: snap.triggeredBy,
                });
                return `${t('common:version')} #${total - currentIdx} · ${new Date(snap.timestamp).toLocaleString()} · ${triggerLabel}`;
              })()}
          </div>
        </motion.div>
      )}
    </div>
  );
}
