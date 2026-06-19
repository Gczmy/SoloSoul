import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { X, RotateCcw, ChevronLeft, ChevronRight } from 'lucide-react';
import { formatBytes } from '@/lib/format';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';
import type { TrashDetail, SnapshotEntry, TrashAttachment } from './types';

interface TrashDetailPanelProps {
  detailItem: TrashDetail;
  detailTemplate: UserTemplate | null;
  onClose: () => void;
  onRequestRestore: (id: string) => void;
  onRequestDelete: (id: string) => void;
}

export function TrashDetailPanel({
  detailItem,
  detailTemplate,
  onClose,
  onRequestRestore,
  onRequestDelete,
}: TrashDetailPanelProps) {
  const { t } = useTranslation(['settings', 'common', 'editor', 'navigation']);
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({});
  const [showTrashAttachments, setShowTrashAttachments] = useState(false);
  const [historySnapIndex, setHistorySnapIndex] = useState<Record<string, number>>({});
  const [historySnapData, setHistorySnapData] = useState<Record<string, Record<string, unknown> | null>>({});
  const [historySnapLoading, setHistorySnapLoading] = useState<Record<string, boolean>>({});

  const toggleSection = (key: string) => {
    setExpandedSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const loadSnapshotData = useCallback(async (detailId: string, snapshotId: string) => {
    setHistorySnapLoading((prev) => ({ ...prev, [detailId]: true }));
    try {
      const data = await invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId });
      setHistorySnapData((prev) => ({ ...prev, [detailId]: data }));
    } catch {
      setHistorySnapData((prev) => ({ ...prev, [detailId]: null }));
    } finally {
      setHistorySnapLoading((prev) => ({ ...prev, [detailId]: false }));
    }
  }, []);

  const changeSnapshot = (detailId: string, snapshots: SnapshotEntry[], newIdx: number) => {
    setHistorySnapIndex((prev) => ({ ...prev, [detailId]: newIdx }));
    if (snapshots[newIdx]) {
      loadSnapshotData(detailId, snapshots[newIdx].id);
    }
  };

  const currentSnapIdx = historySnapIndex[detailItem.id] ?? 5;

  // Load first snapshot when panel opens; skip if already cached for this item
  useEffect(() => {
    if (detailItem.snapshots.length > 0 && !historySnapData[detailItem.id]) {
      loadSnapshotData(detailItem.id, detailItem.snapshots[0].id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detailItem.id, detailItem.snapshots, loadSnapshotData]);

  const activeAttachments: TrashAttachment[] = detailItem.attachments;
  const deletedAttachments: TrashAttachment[] = detailItem.deletedAttachments;

  return (
    <>
      <div
        style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)', zIndex: 99 }}
        onClick={onClose}
      />
      <div
        style={{
          position: 'fixed',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: 380,
          maxHeight: '80vh',
          overflowY: 'auto',
          zIndex: 100,
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: 24,
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
        }}
      >
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-start',
            marginBottom: 16,
          }}
        >
          <div>
            <h3 style={{ fontSize: 16, fontWeight: 600, margin: 0, overflowWrap: 'break-word', wordBreak: 'break-word' }}>{detailItem.name}</h3>
            <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
              {t(`settings:trash_type_${detailItem.itemType}`)}
            </span>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 4,
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={18} />
          </button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, fontSize: 13 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:delete_time')}</span>
            <span>{new Date(detailItem.deletedAt).toLocaleString()}</span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--text-tertiary)' }}>
              {t('settings:original_location')}
            </span>
            <span>
              {t(`navigation:${detailItem.sectionType}`, detailItem.originalLocation)}
            </span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--text-tertiary)' }}>
              {t('settings:remaining_retention')}
            </span>
            <span>
              {detailItem.remainingDays != null
                ? t('settings:trash_expires_in', { days: detailItem.remainingDays })
                : t('settings:never_delete')}
            </span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:deleted_by')}</span>
            <span>
              {detailItem.deletedBy === 'user'
                ? t('settings:deleted_by_user')
                : t('settings:deleted_by_system')}
            </span>
          </div>
        </div>

        {detailItem.previewProperties.length > 0 && (
          <div
            style={{
              marginTop: 16,
              borderTop: '1px solid var(--border-subtle)',
              paddingTop: 12,
            }}
          >
            <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
              {t('settings:content_preview')}
            </h4>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 6,
                fontSize: 12,
                color: 'var(--text-secondary)',
              }}
            >
              {detailItem.previewProperties.map((p, i) => {
                const propType = (p as Record<string, unknown>).type as PropertyType | undefined;
                const sensitivity = (p as Record<string, unknown>).sensitivityLevel as
                  | SensitivityLevel
                  | undefined;
                const typeLabel = propType
                  ? t(`editor:field_types.${propType}`, propType)
                  : String(p.value);
                return (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    {propType && <FieldTypeIcon type={propType} size={14} />}
                    <span style={{ fontWeight: 500, flexShrink: 0 }}>{p.key}</span>
                    {sensitivity && <SensitivityBadge level={sensitivity} />}
                    <span
                      style={{
                        color: 'var(--text-tertiary)',
                        marginLeft: 'auto',
                        flexShrink: 0,
                      }}
                    >
                      {typeLabel}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {detailItem.itemType !== 'template' && (
          <>
            {/* Attachments section */}
            <div
              style={{
                marginTop: 12,
                borderTop: '1px solid var(--border-subtle)',
                paddingTop: 10,
              }}
            >
              <div
                onClick={() => toggleSection('attachments')}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: 600,
                  userSelect: 'none',
                }}
              >
                <span
                  style={{
                    transform: expandedSections.attachments ? 'rotate(90deg)' : 'none',
                    transition: 'transform 0.15s',
                    fontSize: 10,
                  }}
                >
                  ▶
                </span>
                {t('common:attachments')} ({activeAttachments.length + deletedAttachments.length})
              </div>
              {expandedSections.attachments && (
                <div style={{ marginTop: 8 }}>
                  {deletedAttachments.length > 0 && (
                    <div style={{ display: 'flex', gap: 6, marginBottom: 8 }}>
                      <button
                        onClick={() => setShowTrashAttachments(false)}
                        style={{
                          padding: '3px 10px',
                          borderRadius: 4,
                          border: '1px solid var(--border-subtle)',
                          cursor: 'pointer',
                          fontSize: 11,
                          background: !showTrashAttachments
                            ? 'var(--accent-primary)'
                            : 'transparent',
                          color: !showTrashAttachments ? 'white' : 'var(--text-secondary)',
                        }}
                      >
                        {t('common:active')} ({activeAttachments.length})
                      </button>
                      <button
                        onClick={() => setShowTrashAttachments(true)}
                        style={{
                          padding: '3px 10px',
                          borderRadius: 4,
                          border: '1px solid var(--border-subtle)',
                          cursor: 'pointer',
                          fontSize: 11,
                          background: showTrashAttachments
                            ? 'var(--accent-primary)'
                            : 'transparent',
                          color: showTrashAttachments ? 'white' : 'var(--text-secondary)',
                        }}
                      >
                        {t('common:trash')} ({deletedAttachments.length})
                      </button>
                    </div>
                  )}
                  {(showTrashAttachments ? deletedAttachments : activeAttachments).length === 0 ? (
                    <p
                      style={{
                        fontSize: 12,
                        color: 'var(--text-tertiary)',
                        padding: '8px 0',
                      }}
                    >
                      {t('common:no_data')}
                    </p>
                  ) : (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                      {(showTrashAttachments ? deletedAttachments : activeAttachments).map((a) => (
                        <div
                          key={a.id}
                          style={{
                            fontSize: 12,
                            padding: '6px 8px',
                            background: 'var(--bg-elevated-hover)',
                            borderRadius: 6,
                          }}
                        >
                          <div
                            style={{
                              fontWeight: 500,
                              marginBottom: 2,
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                            }}
                          >
                            {a.fileName}
                          </div>
                          <div style={{ color: 'var(--text-tertiary)', fontSize: 11 }}>
                            {formatBytes(a.sizeBytes)} ·{' '}
                            {new Date(a.createdAt).toLocaleDateString()}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Snapshots section */}
            <div
              style={{
                marginTop: 12,
                borderTop: '1px solid var(--border-subtle)',
                paddingTop: 10,
              }}
            >
              <div
                onClick={() => toggleSection('snapshots')}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: 600,
                  userSelect: 'none',
                }}
              >
                <span
                  style={{
                    transform: expandedSections.snapshots ? 'rotate(90deg)' : 'none',
                    transition: 'transform 0.15s',
                    fontSize: 10,
                  }}
                >
                  ▶
                </span>
                {t('settings:data_snapshots')} ({detailItem.snapshots.length})
              </div>
              {expandedSections.snapshots && (
                <SnapshotContent
                  _detailId={detailItem.id}
                  snapshots={detailItem.snapshots}
                  currentSnapIdx={currentSnapIdx}
                  data={historySnapData[detailItem.id]}
                  loading={historySnapLoading[detailItem.id]}
                  detailTemplate={detailTemplate}
                  onChangeSnapshot={(newIdx) =>
                    changeSnapshot(detailItem.id, detailItem.snapshots, newIdx)
                  }
                />
              )}
            </div>
          </>
        )}

        <div style={{ marginTop: 16, display: 'flex', gap: 8 }}>
          <Button
            size="sm"
            onClick={() => {
              onRequestRestore(detailItem.id);
              onClose();
            }}
          >
            <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              onRequestDelete(detailItem.id);
              onClose();
            }}
          >
            {t('common:delete_permanently')}
          </Button>
        </div>
      </div>
    </>
  );
}

interface SnapshotContentProps {
  _detailId: string;
  snapshots: SnapshotEntry[];
  currentSnapIdx: number;
  data: Record<string, unknown> | null | undefined;
  loading: boolean | undefined;
  detailTemplate: UserTemplate | null;
  onChangeSnapshot: (newIdx: number) => void;
}

function SnapshotContent({
  _detailId,
  snapshots,
  currentSnapIdx,
  data,
  loading,
  detailTemplate,
  onChangeSnapshot,
}: SnapshotContentProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const currentSnap = snapshots[currentSnapIdx];

  return (
    <div style={{ marginTop: 8, fontSize: 12 }}>
      {snapshots.length > 1 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          <button
            disabled={currentSnapIdx >= snapshots.length - 1}
            onClick={() => onChangeSnapshot(currentSnapIdx + 1)}
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              border: '1px solid var(--border-subtle)',
              borderRadius: 6,
              cursor: currentSnapIdx >= snapshots.length - 1 ? 'default' : 'pointer',
              fontSize: 11,
              background: 'transparent',
              color: currentSnapIdx >= snapshots.length - 1 ? 'var(--text-tertiary)' : 'var(--text-secondary)',
              opacity: currentSnapIdx >= snapshots.length - 1 ? 0.35 : 1,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              if (currentSnapIdx < snapshots.length - 1) {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color = currentSnapIdx >= snapshots.length - 1 ? 'var(--text-tertiary)' : 'var(--text-secondary)';
            }}
          >
            <ChevronLeft size={14} />
          </button>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 11,
              fontWeight: 500,
              color: 'var(--text-secondary)',
            }}
          >
            <span style={{ color: 'var(--accent-primary)', fontWeight: 600, minWidth: 14, textAlign: 'center' }}>
              {currentSnapIdx + 1}
            </span>
            <span style={{ color: 'var(--text-tertiary)' }}>/</span>
            <span style={{ color: 'var(--text-tertiary)' }}>{snapshots.length}</span>
          </div>
          <button
            disabled={currentSnapIdx <= 0}
            onClick={() => onChangeSnapshot(Math.max(0, currentSnapIdx - 1))}
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              border: '1px solid var(--border-subtle)',
              borderRadius: 6,
              cursor: currentSnapIdx <= 0 ? 'default' : 'pointer',
              fontSize: 11,
              background: 'transparent',
              color: currentSnapIdx <= 0 ? 'var(--text-tertiary)' : 'var(--text-secondary)',
              opacity: currentSnapIdx <= 0 ? 0.35 : 1,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              if (currentSnapIdx > 0) {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color = currentSnapIdx <= 0 ? 'var(--text-tertiary)' : 'var(--text-secondary)';
            }}
          >
            <ChevronRight size={14} />
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
              {currentSnap.diffSummary &&
                !(
                  snapshots.length > 2 &&
                  currentSnapIdx === snapshots.length - 1 &&
                  currentSnap.diffSummary === 'Created'
                ) && (
                  <span
                    style={{
                      padding: '2px 6px',
                      borderRadius: 4,
                      fontSize: 10,
                      fontWeight: 500,
                      background: 'rgba(91,124,153,0.08)',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    {t(`common:diff_${currentSnap.diffSummary}`, currentSnap.diffSummary)}
                  </span>
                )}
              <SnapshotVersionBadge index={currentSnapIdx} total={snapshots.length} />
              <span
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  fontSize: 10,
                  fontWeight: 500,
                  background: 'rgba(91,124,153,0.08)',
                  color: 'var(--accent-primary)',
                }}
              >
                {t(`common:trigger_${currentSnap.triggeredBy}`, currentSnap.triggeredBy)}
              </span>
            </div>
            <span
              style={{
                fontSize: 11,
                color: 'var(--text-tertiary)',
                marginLeft: 'auto',
              }}
            >
              {new Date(currentSnap.timestamp).toLocaleString()}
            </span>
          </div>
          <div style={{ minHeight: 60 }}>
            {loading && !data && <LoadingPlaceholder variant="base" minHeight={60} />}
            {data && <SnapshotDataView data={data} detailTemplate={detailTemplate} />}
          </div>
        </div>
      )}
    </div>
  );
}

interface SnapshotDataViewProps {
  data: Record<string, unknown>;
  detailTemplate: UserTemplate | null;
}

function SnapshotDataView({ data, detailTemplate }: SnapshotDataViewProps) {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { t } = useTranslation(['editor']);
  const rawProps = data.properties as Record<string, unknown> | undefined;
  const tags: string[] = Array.isArray(data.tags) ? (data.tags as string[]) : [];
  const snapName = typeof data.name === 'string' ? data.name : '';

  const orderedFields: {
    key: string;
    value: string;
    type?: PropertyType;
    sensitivityLevel?: SensitivityLevel;
  }[] = [];

  if (rawProps && typeof rawProps === 'object' && detailTemplate) {
    for (const p of detailTemplate.properties) {
      const v = rawProps[p.id];
      if (v !== null && v !== undefined && v !== '' && !String(p.id).startsWith('__')) {
        orderedFields.push({
          key: p.name,
          value: typeof v === 'string' ? v : JSON.stringify(v),
          type: p.type,
          sensitivityLevel: (p.sensitivityLevel || 'internal') as SensitivityLevel,
        });
      }
    }
    const known = new Set(detailTemplate.properties.map((p) => p.id));
    for (const [k, v] of Object.entries(rawProps)) {
      if (!k.startsWith('__') && !known.has(k) && v !== null && v !== undefined && v !== '') {
        orderedFields.push({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        });
      }
    }
  } else if (rawProps && typeof rawProps === 'object') {
    for (const [k, v] of Object.entries(rawProps)) {
      if (!k.startsWith('__') && v !== null && v !== undefined && v !== '') {
        orderedFields.push({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        });
      }
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      {snapName && (
        <div style={{ fontSize: 11, color: 'var(--text-tertiary)', textAlign: 'right', overflowWrap: 'break-word', wordBreak: 'break-word' }}>
          {snapName}
        </div>
      )}
      {orderedFields.slice(0, 8).map((f) => (
        <div
          key={f.key}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            fontSize: 12,
            padding: '3px 0',
            borderBottom: '1px solid var(--border-subtle)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            {f.type && <FieldTypeIcon type={f.type} size={14} />}
            <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{f.key}</span>
            {f.sensitivityLevel && <SensitivityBadge level={f.sensitivityLevel} />}
          </div>
          <span style={{ color: 'var(--text-primary)', marginLeft: 'auto', textAlign: 'right' }}>
            {f.value}
          </span>
        </div>
      ))}
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
