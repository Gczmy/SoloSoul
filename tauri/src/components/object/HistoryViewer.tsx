import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Clock, ChevronLeft, ChevronRight, X } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { useRevealState } from '@/hooks/useRevealState';

export interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[]
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

const pgBtn: React.CSSProperties = {
  width: 30,
  height: 30,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  color: 'var(--text-secondary)',
  fontSize: 14,
};

function SnapshotCard({
  snap,
  index,
  total,
  t,
  getFieldSensitivity,
  isFieldDeprecated,
  getFieldName,
  fieldOrder,
  verifyPassword,
}: {
  snap: SnapshotEntry;
  index: number;
  total: number;
  t: (k: string) => string;
  getFieldSensitivity: (fieldKey: string) => SensitivityLevel;
  isFieldDeprecated: (fieldKey: string) => boolean;
  getFieldName: (fieldKey: string) => string;
  fieldOrder?: string[];
  verifyPassword: () => Promise<boolean>;
}) {
  const [snapData, setSnapData] = useState<Record<string, unknown> | null>(null);
  const { isRevealed, reveal } = useRevealState();

  useEffect(() => {
    invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId: snap.id }).then(
      setSnapData
    );
  }, [snap.id]);

  const rawProps =
    snapData && typeof snapData === 'object' && 'properties' in snapData
      ? (snapData.properties as Record<string, unknown> | undefined)
      : undefined;
  const fields = flattenProperties(rawProps, fieldOrder);
  const snapName =
    snapData && typeof snapData === 'object' && 'name' in snapData
      ? String(snapData.name)
      : '';
  const tags: string[] =
    snapData && typeof snapData === 'object' && 'tags' in snapData && Array.isArray(snapData.tags)
      ? (snapData.tags as string[])
      : [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {/* Version badge */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <SnapshotVersionBadge index={index} total={total} />
        <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{snapName}</div>
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
                  fontSize: 12,
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
                                const ok = await verifyPassword();
                                if (ok) reveal(fieldId);
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
    </div>
  );
}

export interface HistoryViewerProps {
  objectId: string;
  onClose: () => void;
  passwordVerify: () => Promise<boolean>;
  getFieldSensitivity: (fieldKey: string) => SensitivityLevel;
  isFieldDeprecated: (fieldKey: string) => boolean;
  getFieldName: (fieldKey: string) => string;
  fieldOrder?: string[];
  zIndex?: number;
}

export function HistoryViewer({
  objectId,
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
  const { t } = useTranslation(['common', 'editor']);

  useEffect(() => {
    invoke<SnapshotEntry[]>('snapshot_get', { objectId })
      .then(setSnapshots)
      .finally(() => setLoading(false));
  }, [objectId]);

  const goPrev = () => {
    if (currentIdx < snapshots.length - 1) {
      setAnimDir('right');
      setTimeout(() => {
        setCurrentIdx((i) => i + 1);
        setAnimDir(null);
      }, 150);
    }
  };
  const goNext = () => {
    if (currentIdx > 0) {
      setAnimDir('left');
      setTimeout(() => {
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
      <div
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
            animDir === 'left'
              ? 'left center'
              : animDir === 'right'
                ? 'right center'
                : 'center',
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
          <div style={{ fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
            <Clock size={14} /> {t('common:history')}
            <span style={{ fontSize: 11, color: 'var(--text-tertiary)', fontWeight: 400 }}>
              {loading ? '' : `${currentIdx + 1} / ${total}`}
            </span>
          </div>
          <div style={{ display: 'flex', gap: 6 }}>
            <button
              onClick={goPrev}
              disabled={isOldest || loading}
              style={{ ...pgBtn, opacity: isOldest || loading ? 0.3 : 1 }}
            >
              <ChevronLeft size={16} />
            </button>
            <button
              onClick={goNext}
              disabled={isLatest || loading}
              style={{ ...pgBtn, opacity: isLatest || loading ? 0.3 : 1 }}
            >
              <ChevronRight size={16} />
            </button>
            <button onClick={onClose} style={{ ...pgBtn, marginLeft: 4 }}>
              <X size={16} />
            </button>
          </div>
        </div>
        {/* Content */}
        <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
          {loading ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-tertiary)' }}>
              {t('common:loading')}
            </div>
          ) : !snap ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)', fontSize: 14 }}>
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
            />
          )}
        </div>
        {/* Footer */}
        <div
          style={{
            padding: '10px 18px',
            borderTop: '1px solid var(--border-subtle)',
            fontSize: 11,
            color: 'var(--text-tertiary)',
            textAlign: 'center',
          }}
        >
          {snap &&
            `${t('common:version')} #${total - currentIdx} · ${new Date(snap.timestamp).toLocaleString()} · ${t(`common:trigger_${snap.triggeredBy}`)}`}
        </div>
      </div>
    </div>
  );
}
