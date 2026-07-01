import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Clock, ChevronLeft, ChevronRight, X } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { useRevealState } from '@/hooks/useRevealState';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { useSettingsStore } from '@/stores/settingsStore';
import { ICON_SIZE } from '@/lib/iconSizes';

export interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

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

  useEffect(() => {
    invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId: snap.id }).then(
      setSnapData,
    );
  }, [snap.id]);

  const rawProps =
    snapData && typeof snapData === 'object' && 'properties' in snapData
      ? (snapData.properties as Record<string, unknown> | undefined)
      : undefined;
  const fields = flattenProperties(rawProps, fieldOrder);
  const snapName =
    snapData && typeof snapData === 'object' && 'name' in snapData ? String(snapData.name) : '';
  const tags: string[] =
    snapData && typeof snapData === 'object' && 'tags' in snapData && Array.isArray(snapData.tags)
      ? (snapData.tags as string[])
      : [];

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
      {/* Properties — like detail panel, with sensitivity */}
      {fields.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginTop: 4 }}>
          {fields.map((f) => {
            const sens = getFieldSensitivity(f.key);
            const deprecated = isFieldDeprecated(f.key);
            const fieldId = f.key;
            const revealed = isRevealed(fieldId);
            const needsReveal = sens === 'sensitive' || sens === 'critical';
            return (
              <div
                key={f.key}
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
                  <span
                    style={{
                      fontWeight: 500,
                      color: 'var(--text-secondary)',
                      textDecoration: deprecated ? 'line-through' : 'none',
                    }}
                  >
                    {getFieldName(f.key)}
                  </span>
                  <SensitivityBadge level={sens} />
                  {deprecated && <DeprecatedBadge />}
                </div>
                <div style={{ flex: 1 }}>
                  <span
                    onClick={
                      needsReveal && !revealed
                        ? async () => {
                            try {
                              if (sens === 'critical') {
                                const result = await verifyPassword();
                                if (result.ok) {
                                  reveal(fieldId);
                                  onCriticalAccess?.(getFieldName(f.key), result.method);
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
                        needsReveal && !revealed
                          ? 'var(--bg-subtle, rgba(128,128,128,0.15))'
                          : 'transparent',
                      borderRadius: 2,
                      padding: '0 2px',
                      color: 'var(--text-primary)',
                      transition: 'filter 0.15s ease, background 0.15s ease',
                      willChange: needsReveal && !revealed ? 'filter' : 'auto',
                    }}
                    title={needsReveal && !revealed ? 'Click to reveal' : ''}
                  >
                    {f.value}
                  </span>
                </div>
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
    invoke<SnapshotEntry[]>('snapshot_get', { objectId })
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
              `${t('common:version')} #${total - currentIdx} · ${new Date(snap.timestamp).toLocaleString()} · ${t(`common:trigger_${snap.triggeredBy}`)}`}
          </div>
        </motion.div>
      )}
    </div>
  );
}
