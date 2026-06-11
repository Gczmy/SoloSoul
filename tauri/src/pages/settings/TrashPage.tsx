import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { useTemplateStore } from '@/stores/templateStore';
import { invoke } from '@tauri-apps/api/core';
import { Trash2, RotateCcw, FileText, X, Info } from 'lucide-react';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';

// ── Detail panel types ──────────────────────────────────────────

interface TrashDetail {
  id: string;
  itemType: string;
  originalId: string;
  name: string;
  sectionType?: string;
  deletedAt: number;
  expiresAt?: number;
  deletedBy: string;
  remainingDays?: number;
  originalLocation: string;
  templateId?: string;
  previewProperties: { key: string; value: unknown; type?: PropertyType; sensitivityLevel?: SensitivityLevel }[];
  attachments: TrashAttachment[];
  deletedAttachments: TrashAttachment[];
  snapshots: SnapshotEntry[];
}

interface TrashAttachment {
  id: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
  deletedAt?: string | null;
}

interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

const TIME_OPTIONS: { value: TrashTimeFilter; labelKey: string }[] = [
  { value: 'all', labelKey: 'all' },
  { value: '1d', labelKey: '1d' },
  { value: '3d', labelKey: '3d' },
  { value: '7d', labelKey: '7d' },
  { value: '30d', labelKey: '30d' },
  { value: 'half_year', labelKey: 'half_year' },
];

const TYPE_OPTIONS: { value: TrashTypeFilter; i18nKey: string }[] = [
  { value: 'all', i18nKey: 'all' },
  { value: 'page', i18nKey: 'page' },
  { value: 'object', i18nKey: 'object' },
  { value: 'template', i18nKey: 'template' },
];

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function timeAgo(ms: number, t: (k: string) => string): string {
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return t('time_minutes_ago').replace('{n}', String(mins));
  const hours = Math.floor(mins / 60);
  if (hours < 24) return t('time_hours_ago').replace('{n}', String(hours));
  const days = Math.floor(hours / 24);
  if (days < 30) return t('time_days_ago').replace('{n}', String(days));
  return t('time_months_ago').replace('{n}', String(Math.floor(days / 30)));
}

export function TrashPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    items, timeFilter, typeFilter, searchQuery,
    loadItems, setTimeFilter, setTypeFilter, setSearchQuery,
    restoreItem, permanentDelete, isLoading, error,
    selectedIds, toggleSelection, selectAll, clearSelection,
  } = useTrashStore();

  const [detailItem, setDetailItem] = useState<TrashDetail | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<UserTemplate | null>(null);
  const [, setLoadingDetail] = useState(false);
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({});
  const [showTrashAttachments, setShowTrashAttachments] = useState(false);
  const [historySnapIndex, setHistorySnapIndex] = useState<Record<string, number>>({});
  const [historySnapData, setHistorySnapData] = useState<Record<string, Record<string, unknown> | null>>({});
  const [historySnapLoading, setHistorySnapLoading] = useState<Record<string, boolean>>({});
  const { getTemplate } = useTemplateStore();

  // ── Confirmation dialog state ──────────────────────────────────
  const [confirmAction, setConfirmAction] = useState<{
    type: 'restore' | 'delete';
    ids: string[];
    count: number;
    callback: () => Promise<void>;
  } | null>(null);

  // Reset type filter to 'all' on mount so users always see all trash items
  useEffect(() => {
    setTypeFilter('all');
  }, []);

  useEffect(() => {
    if (accountId) loadItems(accountId);
  }, [accountId, timeFilter]);

  const filtered = items
    .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
    .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase()));

  const allFilteredSelected = filtered.length > 0 && filtered.every((i) => selectedIds.has(i.id));
  const hasSelection = selectedIds.size > 0;

  // ── Confirmation wrappers ─────────────────────────────────────
  const doRestore = (ids: string[]) => {
    const count = ids.length;
    setConfirmAction({
      type: 'restore', ids, count,
      callback: async () => {
        for (const id of ids) await restoreItem(id);
        clearSelection();
      },
    });
  };

  const doDelete = (ids: string[]) => {
    const count = ids.length;
    setConfirmAction({
      type: 'delete', ids, count,
      callback: async () => {
        await permanentDelete(ids);
        clearSelection();
      },
    });
  };

  const openDetail = async (trashId: string) => {
    setLoadingDetail(true);
    try {
      const d = await invoke<TrashDetail>('trash_get_detail', { trashId });
      setDetailItem(d);
      // Load template for field name / type / sensitivity resolution
      if (d.templateId) {
        getTemplate(d.templateId).then((tpl) => setDetailTemplate(tpl));
      } else {
        setDetailTemplate(null);
      }
      // Load first snapshot data
      if (d.snapshots.length > 0) {
        loadSnapshotData(d.id, d.snapshots[0].id);
      }
    } catch {
      setDetailItem(null);
      setDetailTemplate(null);
    } finally {
      setLoadingDetail(false);
    }
  };

  const loadSnapshotData = async (detailId: string, snapshotId: string) => {
    setHistorySnapLoading(prev => ({ ...prev, [detailId]: true }));
    try {
      const data = await invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId });
      setHistorySnapData(prev => ({ ...prev, [detailId]: data }));
    } catch {
      setHistorySnapData(prev => ({ ...prev, [detailId]: null }));
    } finally {
      setHistorySnapLoading(prev => ({ ...prev, [detailId]: false }));
    }
  };

  const changeSnapshot = (detailId: string, snapshots: SnapshotEntry[], newIdx: number) => {
    setHistorySnapIndex(prev => ({ ...prev, [detailId]: newIdx }));
    if (snapshots[newIdx]) {
      loadSnapshotData(detailId, snapshots[newIdx].id);
    }
  };

  return (
    <AppShell title={t('settings:trash')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {/* Search */}
        <Input
          placeholder={t('settings:search_logs')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        {/* Row 1: Time filter */}
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {TIME_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => setTimeFilter(opt.value)}
              style={{
                padding: '5px 12px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                background: timeFilter === opt.value ? 'var(--accent-primary)' : 'transparent',
                color: timeFilter === opt.value ? 'white' : 'var(--text-secondary)',
                fontSize: 12, cursor: 'pointer',
              }}
            >
              {t(`settings:${opt.labelKey}`, opt.labelKey)}
            </button>
          ))}
        </div>

        {/* Row 2: Type filter */}
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {TYPE_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => setTypeFilter(opt.value)}
              style={{
                padding: '5px 12px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                background: typeFilter === opt.value ? 'var(--accent-primary)' : 'transparent',
                color: typeFilter === opt.value ? 'white' : 'var(--text-secondary)',
                fontSize: 12, cursor: 'pointer',
              }}
            >
              {t(`settings:trash_type_${opt.value}`)}
            </button>
          ))}
        </div>

        {/* Select all bar */}
        {filtered.length > 0 && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: 'var(--text-secondary)', padding: '4px 0' }}>
            <input
              type="checkbox"
              checked={allFilteredSelected}
              ref={(el) => { if (el) el.indeterminate = !allFilteredSelected && hasSelection; }}
              onChange={() => allFilteredSelected ? clearSelection() : selectAll(filtered.map(i => i.id))}
              style={{ accentColor: 'var(--accent-primary)' }}
            />
            <span>{t('settings:select_all')} ({filtered.length})</span>
            {hasSelection && (
              <span style={{ marginLeft: 'auto' }}>{selectedIds.size} {t('settings:selected')}</span>
            )}
          </div>
        )}

        {/* Error */}
        {error && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--error)', padding: 16, fontSize: 13 }}>{error}</p>
          </Card>
        )}

        {/* List */}
        {isLoading ? (
          <Card><p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 24 }}>{t('common:loading')}</p></Card>
        ) : filtered.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: '48px 24px' }}>
              <Trash2 size={48} style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>
                {items.length > 0
                  ? t('settings:trash_empty_filtered')
                  : t('settings:trash_empty')}
              </p>
            </div>
          </Card>
        ) : (
          filtered.map((item) => (
            <Card key={item.id}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <input
                  type="checkbox"
                  checked={selectedIds.has(item.id)}
                  onChange={() => toggleSelection(item.id)}
                  style={{ accentColor: 'var(--accent-primary)', flexShrink: 0 }}
                />
                <div
                  style={{ display: 'flex', alignItems: 'center', gap: 10, flex: 1, cursor: 'pointer', minWidth: 0 }}
                  onClick={() => openDetail(item.id)}
                >
                  <FileText size={18} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: 13, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{item.name}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {t(`settings:trash_type_${item.itemType}`)} · {timeAgo(item.deletedAt, t)}
                      {item.expiresAt && ` · ${t('settings:trash_expires_in', { days: Math.max(0, Math.floor((item.expiresAt - Date.now()) / 86400000)) })}`}
                    </div>
                  </div>
                </div>
                <Button size="sm" onClick={() => doRestore([item.id])} title={t('common:restore')}>
                  <RotateCcw size={13} />
                </Button>
                <Button size="sm" variant="secondary" onClick={() => doDelete([item.id])} title={t('common:delete_permanently')}>
                  <Trash2 size={13} />
                </Button>
                <button
                  onClick={() => openDetail(item.id)}
                  style={{ background: 'none', border: 'none', cursor: 'pointer', padding: 4, color: 'var(--text-tertiary)' }}
                  title={t('common:details')}
                >
                  <Info size={16} />
                </button>
              </div>
            </Card>
          ))
        )}

        {/* ── Bottom action bar (batch) ─────────────────────────── */}
        {hasSelection && (
          <div
            style={{
              position: 'sticky', bottom: 0, padding: '12px 16px',
              background: 'var(--bg-elevated)', borderRadius: 10,
              border: '1px solid var(--border-subtle)',
              display: 'flex', alignItems: 'center', gap: 8,
              boxShadow: '0 -2px 12px rgba(0,0,0,0.08)',
            }}
          >
            <span style={{ fontSize: 13, color: 'var(--text-secondary)', marginRight: 'auto' }}>
              {selectedIds.size} {t('settings:selected')}
            </span>
            <Button size="sm" onClick={() => doRestore(Array.from(selectedIds))}>
              <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
            </Button>
            <Button size="sm" variant="secondary" onClick={() => doDelete(Array.from(selectedIds))}>
              {t('common:delete_permanently')}
            </Button>
          </div>
        )}

        {/* ── Detail panel (overlay) ───────────────────────────── */}
        {detailItem && (
          <>
            <div style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)', zIndex: 99 }} onClick={() => setDetailItem(null)} />
            <div style={{
              position: 'fixed', top: '50%', left: '50%', transform: 'translate(-50%, -50%)',
              width: 380, maxHeight: '80vh', overflowY: 'auto', zIndex: 100,
              background: 'var(--bg-elevated)', borderRadius: 12, padding: 24,
              boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
                <div>
                  <h3 style={{ fontSize: 16, fontWeight: 600, margin: 0 }}>{detailItem.name}</h3>
                  <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                    {t(`settings:trash_type_${detailItem.itemType}`)}
                  </span>
                </div>
                <button
                  onClick={() => setDetailItem(null)}
                  style={{ background: 'none', border: 'none', cursor: 'pointer', padding: 4, color: 'var(--text-tertiary)' }}
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
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:original_location')}</span>
                  <span>{t(`navigation:${detailItem.sectionType}`, detailItem.originalLocation)}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:remaining_retention')}</span>
                  <span>{detailItem.remainingDays != null ? t('settings:trash_expires_in', { days: detailItem.remainingDays }) : t('settings:never_delete')}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:deleted_by')}</span>
                  <span>{detailItem.deletedBy === 'user' ? t('settings:deleted_by_user') : t('settings:deleted_by_system')}</span>
                </div>
              </div>

              {detailItem.previewProperties.length > 0 && (
                <div style={{ marginTop: 16, borderTop: '1px solid var(--border-subtle)', paddingTop: 12 }}>
                  <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>{t('settings:content_preview')}</h4>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 6, fontSize: 12, color: 'var(--text-secondary)' }}>
                    {detailItem.previewProperties.map((p, i) => {
                      const propType = (p as Record<string, unknown>).type as PropertyType | undefined;
                      const sensitivity = (p as Record<string, unknown>).sensitivityLevel as SensitivityLevel | undefined;
                      const typeLabel = propType ? t(`editor:field_types.${propType}`, propType) : String(p.value);
                      return (
                        <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                          {propType && <FieldTypeIcon type={propType} size={14} />}
                          <span style={{ fontWeight: 500, flexShrink: 0 }}>{p.key}</span>
                          {sensitivity && <SensitivityBadge level={sensitivity} />}
                          <span style={{ color: 'var(--text-tertiary)', marginLeft: 'auto', flexShrink: 0 }}>
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
                  {/* ── Attachments section ─────────────────────────── */}
                  <div style={{ marginTop: 12, borderTop: '1px solid var(--border-subtle)', paddingTop: 10 }}>
                    <div
                      onClick={() => setExpandedSections(prev => ({ ...prev, attachments: !prev.attachments }))}
                      style={{ display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer', fontSize: 13, fontWeight: 600, userSelect: 'none' }}
                    >
                      <span style={{ transform: expandedSections.attachments ? 'rotate(90deg)' : 'none', transition: 'transform 0.15s', fontSize: 10 }}>▶</span>
                      {t('common:attachments')} ({detailItem.attachments.length + detailItem.deletedAttachments.length})
                    </div>
                      {expandedSections.attachments && (
                        <div style={{ marginTop: 8 }}>
                          {/* Active/Trash toggle */}
                          {(detailItem.deletedAttachments.length > 0) && (
                            <div style={{ display: 'flex', gap: 6, marginBottom: 8 }}>
                              <button
                                onClick={() => setShowTrashAttachments(false)}
                                style={{
                                  padding: '3px 10px', borderRadius: 4, border: '1px solid var(--border-subtle)', cursor: 'pointer', fontSize: 11,
                                  background: !showTrashAttachments ? 'var(--accent-primary)' : 'transparent',
                                  color: !showTrashAttachments ? 'white' : 'var(--text-secondary)',
                                }}
                              >{t('common:active')} ({detailItem.attachments.length})</button>
                              <button
                                onClick={() => setShowTrashAttachments(true)}
                                style={{
                                  padding: '3px 10px', borderRadius: 4, border: '1px solid var(--border-subtle)', cursor: 'pointer', fontSize: 11,
                                  background: showTrashAttachments ? 'var(--accent-primary)' : 'transparent',
                                  color: showTrashAttachments ? 'white' : 'var(--text-secondary)',
                                }}
                              >{t('common:trash')} ({detailItem.deletedAttachments.length})</button>
                            </div>
                          )}
                          {(showTrashAttachments ? detailItem.deletedAttachments : detailItem.attachments).length === 0 ? (
                            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', padding: '8px 0' }}>{t('common:no_data')}</p>
                          ) : (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                              {(showTrashAttachments ? detailItem.deletedAttachments : detailItem.attachments).map((a) => (
                                <div key={a.id} style={{ fontSize: 12, padding: '6px 8px', background: 'var(--bg-elevated-hover)', borderRadius: 6 }}>
                                  <div style={{ fontWeight: 500, marginBottom: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{a.fileName}</div>
                                  <div style={{ color: 'var(--text-tertiary)', fontSize: 11 }}>
                                    {formatBytes(a.sizeBytes)} · {new Date(a.createdAt).toLocaleDateString()}
                                  </div>
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      )}
                    </div>

                  {/* ── Snapshots/History section ──────────────────── */}
                  <div style={{ marginTop: 12, borderTop: '1px solid var(--border-subtle)', paddingTop: 10 }}>
                    <div
                      onClick={() => setExpandedSections(prev => ({ ...prev, snapshots: !prev.snapshots }))}
                      style={{ display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer', fontSize: 13, fontWeight: 600, userSelect: 'none' }}
                    >
                      <span style={{ transform: expandedSections.snapshots ? 'rotate(90deg)' : 'none', transition: 'transform 0.15s', fontSize: 10 }}>▶</span>
                      {t('settings:data_snapshots')} ({detailItem.snapshots.length})
                    </div>
                    {expandedSections.snapshots && (() => {
                      const snapIdx = historySnapIndex[detailItem.id] ?? 0;
                      const currentSnap = detailItem.snapshots[snapIdx];
                      const data = historySnapData[detailItem.id];
                      const loading = historySnapLoading[detailItem.id];
                      return (
                        <div style={{ marginTop: 8, fontSize: 12 }}>
                          {detailItem.snapshots.length > 1 && (
                            <div style={{ display: 'flex', gap: 6, marginBottom: 8 }}>
                              <button
                                disabled={snapIdx >= detailItem.snapshots.length - 1}
                                onClick={() => changeSnapshot(detailItem.id, detailItem.snapshots, snapIdx + 1)}
                                style={{
                                  padding: '3px 8px', border: '1px solid var(--border-subtle)', borderRadius: 4,
                                  cursor: 'pointer', fontSize: 11, background: 'var(--bg-subtle)', color: 'var(--text-secondary)',
                                  opacity: snapIdx >= detailItem.snapshots.length - 1 ? 0.4 : 1,
                                }}
                              >‹ {t('common:previous')}</button>
                              <span style={{ padding: '3px 0', color: 'var(--text-tertiary)' }}>{snapIdx + 1} / {detailItem.snapshots.length}</span>
                              <button
                                disabled={snapIdx <= 0}
                                onClick={() => changeSnapshot(detailItem.id, detailItem.snapshots, Math.max(0, snapIdx - 1))}
                                style={{
                                  padding: '3px 8px', border: '1px solid var(--border-subtle)', borderRadius: 4,
                                  cursor: 'pointer', fontSize: 11, background: 'var(--bg-subtle)', color: 'var(--text-secondary)',
                                  opacity: snapIdx <= 0 ? 0.4 : 1,
                                }}
                              >{t('common:next')} ›</button>
                            </div>
                          )}
                          {currentSnap && (
                            <div>
                              {/* Header */}
                              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 8px', background: 'var(--bg-elevated-hover)', borderRadius: 6, marginBottom: 6, minHeight: 32 }}>
                                <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
                                  {currentSnap.diffSummary && !(detailItem.snapshots.length > 2 && snapIdx === detailItem.snapshots.length - 1 && currentSnap.diffSummary === 'Created') && (
                                    <span style={{ padding: '2px 6px', borderRadius: 4, fontSize: 10, fontWeight: 500, background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)' }}>
                                      {t(`common:diff_${currentSnap.diffSummary}`, currentSnap.diffSummary)}
                                    </span>
                                  )}
                                  <SnapshotVersionBadge index={snapIdx} total={detailItem.snapshots.length} />
                                  <span style={{ padding: '2px 6px', borderRadius: 4, fontSize: 10, fontWeight: 500, background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)' }}>
                                    {t(`common:trigger_${currentSnap.triggeredBy}`, currentSnap.triggeredBy)}
                                  </span>
                                </div>
                                <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
                                  {new Date(currentSnap.timestamp).toLocaleString()}
                                </span>
                              </div>
                              {/* Content area — fixed minHeight prevents layout jump */}
                              <div style={{ minHeight: 60 }}>
                                {loading && !data && <p style={{ color: 'var(--text-tertiary)', padding: '8px 0' }}>{t('common:loading')}</p>}
                                {data && (() => {
                                  const d = data as Record<string, unknown>;
                                  const rawProps = d.properties as Record<string, unknown> | undefined;
                                  const tags: string[] = Array.isArray(d.tags) ? d.tags as string[] : [];
                                  const snapName = typeof d.name === 'string' ? d.name : '';
                                  // Build ordered fields with template metadata
                                  const orderedFields: { key: string; value: string; type?: PropertyType; sensitivityLevel?: SensitivityLevel }[] = [];
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
                                    // Orphaned fields not in template
                                    const known = new Set(detailTemplate.properties.map((p) => p.id));
                                    for (const [k, v] of Object.entries(rawProps)) {
                                      if (!k.startsWith('__') && !known.has(k) && v !== null && v !== undefined && v !== '') {
                                        orderedFields.push({ key: k, value: typeof v === 'string' ? v : JSON.stringify(v) });
                                      }
                                    }
                                  } else if (rawProps && typeof rawProps === 'object') {
                                    for (const [k, v] of Object.entries(rawProps)) {
                                      if (!k.startsWith('__') && v !== null && v !== undefined && v !== '') {
                                        orderedFields.push({ key: k, value: typeof v === 'string' ? v : JSON.stringify(v) });
                                      }
                                    }
                                  }
                                  return (
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                                      {snapName && (
                                        <div style={{ fontSize: 11, color: 'var(--text-tertiary)', textAlign: 'right' }}>{snapName}</div>
                                      )}
                                      {orderedFields.slice(0, 8).map((f) => (
                                        <div key={f.key} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, padding: '3px 0', borderBottom: '1px solid var(--border-subtle)' }}>
                                          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                                            {f.type && <FieldTypeIcon type={f.type} size={14} />}
                                            <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{f.key}</span>
                                            {f.sensitivityLevel && <SensitivityBadge level={f.sensitivityLevel} />}
                                          </div>
                                          <span style={{ color: 'var(--text-primary)', marginLeft: 'auto', textAlign: 'right' }}>{f.value}</span>
                                        </div>
                                      ))}
                                      {tags.length > 0 && (
                                        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
                                          {tags.map((tag) => (
                                            <span key={tag} style={{ padding: '1px 7px', borderRadius: 10, fontSize: 10, background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)', fontWeight: 500 }}>
                                              {tag}
                                            </span>
                                          ))}
                                        </div>
                                      )}
                                    </div>
                                  );
                                })()}
                              </div>
                            </div>
                          )}
                        </div>
                      );
                    })()}
                  </div>
                </>
              )}

              <div style={{ marginTop: 16, display: 'flex', gap: 8 }}>
                <Button size="sm" onClick={() => { doRestore([detailItem.id]); setDetailItem(null); }}>
                  <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
                </Button>
                <Button size="sm" variant="secondary" onClick={() => { doDelete([detailItem.id]); setDetailItem(null); }}>
                  {t('common:delete_permanently')}
                </Button>
              </div>
            </div>
          </>
        )}
        {/* ── Confirmation dialog ──────────────────────────── */}
        {confirmAction && (
          <>
            <div style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)', zIndex: 99 }} onClick={() => setConfirmAction(null)} />
            <div style={{
              position: 'fixed', top: '50%', left: '50%', transform: 'translate(-50%, -50%)',
              width: 340, zIndex: 100, background: 'var(--bg-elevated)', borderRadius: 12, padding: 24,
              boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
            }}>
              <h3 style={{ fontSize: 15, fontWeight: 600, margin: '0 0 8px' }}>
                {confirmAction.type === 'delete'
                  ? t('settings:confirm_delete_title')
                  : t('settings:confirm_restore_title')}
              </h3>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
                {confirmAction.type === 'delete'
                  ? t('settings:confirm_delete_desc', { count: confirmAction.count })
                  : t('settings:confirm_restore_desc', { count: confirmAction.count })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button size="sm" variant="secondary" onClick={() => setConfirmAction(null)}>
                  {t('common:cancel')}
                </Button>
                <Button
                  size="sm"
                  variant={confirmAction.type === 'delete' ? 'danger' : 'primary'}
                  onClick={async () => {
                    await confirmAction.callback();
                    setConfirmAction(null);
                  }}
                >
                  {confirmAction.type === 'delete' ? t('common:delete_permanently') : t('common:restore')}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </AppShell>
  );
}
