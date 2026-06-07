import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { invoke } from '@tauri-apps/api/core';
import { Trash2, RotateCcw, FileText, X, Info } from 'lucide-react';

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
  previewProperties: { key: string; value: unknown }[];
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
];

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
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    items, timeFilter, typeFilter, searchQuery,
    loadItems, setTimeFilter, setTypeFilter, setSearchQuery,
    restoreItem, permanentDelete, isLoading,
    selectedIds, toggleSelection, selectAll, clearSelection,
  } = useTrashStore();

  const [detailItem, setDetailItem] = useState<TrashDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);

  useEffect(() => {
    if (accountId) loadItems(accountId);
  }, [accountId, timeFilter]);

  const filtered = items
    .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
    .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase()));

  const allFilteredSelected = filtered.length > 0 && filtered.every((i) => selectedIds.has(i.id));
  const hasSelection = selectedIds.size > 0;

  const openDetail = async (trashId: string) => {
    setLoadingDetail(true);
    try {
      const d = await invoke<TrashDetail>('trash_get_detail', { trashId });
      setDetailItem(d);
    } catch {
      setDetailItem(null);
    } finally {
      setLoadingDetail(false);
    }
  };

  return (
    <AppShell title={t('settings:trash')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {/* Filters */}
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
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
          <span style={{ width: 1, background: 'var(--border-subtle)', margin: '2px 4px' }} />
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

        <Input
          placeholder={t('settings:search_logs')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

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

        {/* List */}
        {isLoading ? (
          <Card><p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 24 }}>{t('common:loading')}</p></Card>
        ) : filtered.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: '48px 24px' }}>
              <Trash2 size={48} style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>{t('settings:trash_empty')}</p>
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
                <Button size="sm" onClick={() => restoreItem(item.id)} title={t('common:restore')}>
                  <RotateCcw size={13} />
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
            <Button size="sm" onClick={async () => { for (const id of selectedIds) await restoreItem(id); clearSelection(); }}>
              <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
            </Button>
            <Button size="sm" variant="secondary" onClick={async () => { await permanentDelete(Array.from(selectedIds)); clearSelection(); }}>
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
                  <span>{detailItem.sectionType || detailItem.originalLocation}</span>
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
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12, color: 'var(--text-secondary)' }}>
                    {detailItem.previewProperties.map((p, i) => (
                      <div key={i} style={{ display: 'flex', gap: 8 }}>
                        <span style={{ fontWeight: 500, flexShrink: 0 }}>{p.key}:</span>
                        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                          {typeof p.value === 'string' ? p.value : JSON.stringify(p.value)}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div style={{ marginTop: 16, display: 'flex', gap: 8 }}>
                <Button size="sm" onClick={async () => { await restoreItem(detailItem.id); setDetailItem(null); }}>
                  <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
                </Button>
                <Button size="sm" variant="secondary" onClick={async () => { await permanentDelete([detailItem.id]); setDetailItem(null); }}>
                  {t('common:delete_permanently')}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </AppShell>
  );
}
