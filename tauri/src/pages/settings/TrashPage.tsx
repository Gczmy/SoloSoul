import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { invoke } from '@tauri-apps/api/core';
import { Trash2, RotateCcw, FileText, Info, Loader2, Folder, LayoutTemplate } from 'lucide-react';
import { PluginBadge } from '@/components/template/PluginBadge';
import type { UserTemplate } from '@/types/template';
import { TrashDetailPanel } from '@/components/trash/TrashDetailPanel';
import { TrashConfirmDialog } from '@/components/trash/TrashConfirmDialog';
import type { TrashDetail, TrashConfirmAction } from '@/components/trash/types';

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
  const location = useLocation();
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    items,
    timeFilter,
    typeFilter,
    searchQuery,
    loadItems,
    setTimeFilter,
    setTypeFilter,
    setSearchQuery,
    restoreItem,
    permanentDelete,
    isLoading,
    error,
    selectedIds,
    toggleSelection,
    selectAll,
    clearSelection,
  } = useTrashStore();

  const [detailItem, setDetailItem] = useState<TrashDetail | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<UserTemplate | null>(null);
  const [, setLoadingDetail] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);
  const { getTemplate } = useTemplateStore();

  const [confirmAction, setConfirmAction] = useState<TrashConfirmAction | null>(null);

  useEffect(() => {
    setTypeFilter('all');
  }, [setTypeFilter]);

  useEffect(() => {
    if (accountId) loadItems(accountId);
  }, [accountId, timeFilter, loadItems]);

  useEffect(() => {
    if (!isLoading && !error) {
      setHasLoaded(true);
    }
  }, [isLoading, error]);

  const filtered = items
    .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
    .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase()));

  const allFilteredSelected = filtered.length > 0 && filtered.every((i) => selectedIds.has(i.id));
  const hasSelection = selectedIds.size > 0;

  const doRestore = (ids: string[]) => {
    const count = ids.length;
    setConfirmAction({
      type: 'restore',
      ids,
      count,
      callback: async () => {
        for (const id of ids) await restoreItem(id);
        clearSelection();
        if (accountId)
          useSettingsStore
            .getState()
            .loadCustomPages(accountId)
            .catch(() => {});
      },
    });
  };

  const doDelete = (ids: string[]) => {
    const count = ids.length;
    setConfirmAction({
      type: 'delete',
      ids,
      count,
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
      if (d.templateId) {
        getTemplate(d.templateId).then((tpl) => setDetailTemplate(tpl));
      } else {
        setDetailTemplate(null);
      }
    } catch {
      setDetailItem(null);
      setDetailTemplate(null);
    } finally {
      setLoadingDetail(false);
    }
  };

  return (
    <AppShell title={t('settings:trash')} onBack={() => {
            const state = location.state as { fromHome?: boolean } | undefined;
            if (state?.fromHome) {
              navigate('/home');
            } else {
              navigate('/settings');
            }
          }}>
      <PageContainer variant="small" gap="default">
        <Input
          placeholder={t('settings:search_trash')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
        />

        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {TIME_OPTIONS.map((opt) => {
            const isActive = timeFilter === opt.value;
            return (
              <button
                key={opt.value}
                onClick={() => setTimeFilter(opt.value)}
                onMouseEnter={!isActive ? (e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                } : undefined}
                onMouseLeave={!isActive ? (e) => {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                } : undefined}
                style={{
                  padding: '5px 12px',
                  borderRadius: 6,
                  border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                  background: isActive ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                  color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                  boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                  fontSize: 12,
                  cursor: 'pointer',
                  transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                }}
              >
                {t(`settings:${opt.labelKey}`, opt.labelKey)}
              </button>
            );
          })}
        </div>

        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {TYPE_OPTIONS.map((opt) => {
            const isActive = typeFilter === opt.value;
            return (
              <button
                key={opt.value}
                onClick={() => setTypeFilter(opt.value)}
                onMouseEnter={!isActive ? (e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                } : undefined}
                onMouseLeave={!isActive ? (e) => {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                } : undefined}
                style={{
                  padding: '5px 12px',
                  borderRadius: 6,
                  border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                  background: isActive ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                  color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                  boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                  fontSize: 12,
                  cursor: 'pointer',
                  transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                }}
              >
                {t(`settings:trash_type_${opt.value}`)}
              </button>
            );
          })}
        </div>

        {filtered.length > 0 && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              fontSize: 12,
              color: 'var(--text-secondary)',
              padding: '4px 0',
            }}
          >
            <input
              type="checkbox"
              checked={allFilteredSelected}
              ref={(el) => {
                if (el) el.indeterminate = !allFilteredSelected && hasSelection;
              }}
              onChange={() =>
                allFilteredSelected ? clearSelection() : selectAll(filtered.map((i) => i.id))
              }
              style={{ accentColor: 'var(--accent-primary)' }}
            />
            <span>
              {t('settings:select_all')} ({filtered.length})
            </span>
            {hasSelection && (
              <span style={{ marginLeft: 'auto' }}>
                {selectedIds.size} {t('settings:selected')}
              </span>
            )}
          </div>
        )}

        {error && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--error)', padding: 16, fontSize: 13 }}>
              {error}
            </p>
          </Card>
        )}

        {isLoading && !hasLoaded ? (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={120} />
          </Card>
        ) : (
          <div style={{ position: 'relative' }}>
            {isLoading && hasLoaded && (
              <div
                style={{
                  position: 'absolute',
                  inset: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  background: 'var(--bg-elevated)',
                  opacity: 0.85,
                  zIndex: 1,
                  borderRadius: 12,
                }}
              >
                <Loader2
                  size={24}
                  style={{ animation: 'spin 1s linear infinite', color: 'var(--accent-primary)' }}
                />
              </div>
            )}
            {filtered.length === 0 ? (
              <Card>
                <div style={{ textAlign: 'center', padding: '48px 24px' }}>
                  <Trash2
                    size={48}
                    style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
                  />
                  <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>
                    {items.length > 0
                      ? t('settings:trash_empty_filtered')
                      : t('settings:trash_empty')}
                  </p>
                </div>
              </Card>
            ) : (
              filtered.map((item) => (
                <Card
                  key={item.id}
                  interactive
                  onClick={() => openDetail(item.id)}
                  style={{
                    cursor: 'pointer',
                    transition: 'transform 0.15s ease, box-shadow 0.15s ease',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <input
                      type="checkbox"
                      checked={selectedIds.has(item.id)}
                      onChange={() => toggleSelection(item.id)}
                      onClick={(e) => e.stopPropagation()}
                      style={{ accentColor: 'var(--accent-primary)', flexShrink: 0 }}
                    />
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 10,
                        flex: 1,
                        minWidth: 0,
                      }}
                    >
                      {(() => {
                        const Icon = item.itemType === 'template' ? LayoutTemplate : item.itemType === 'page' ? Folder : FileText;
                        return <Icon size={18} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />;
                      })()}
                      <div style={{ minWidth: 0 }}>
                        <div
                          style={{
                            fontSize: 13,
                            fontWeight: 500,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {item.name}
                        </div>
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 4,
                            fontSize: 11,
                            color: 'var(--text-tertiary)',
                          }}
                        >
                          {t(`settings:trash_type_${item.itemType}`)} · {timeAgo(item.deletedAt, t)}
                          {item.expiresAt &&
                            ` · ${t('settings:trash_expires_in', { days: Math.max(0, Math.floor((item.expiresAt - Date.now()) / 86400000)) })}`}
                          <PluginBadge contractTypeId={item.contractTypeId} size="sm" />
                        </div>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="secondary"
                      style={{
                        border: '1px solid var(--accent-primary)',
                        color: 'var(--accent-primary)',
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        doRestore([item.id]);
                      }}
                      title={t('common:restore')}
                    >
                      <RotateCcw size={13} style={{ color: 'var(--accent-primary)' }} />
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      style={{
                        color: '#e74c3c',
                        border: '1px solid rgba(231,76,60,0.3)',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
                        e.currentTarget.style.borderColor = 'rgba(231,76,60,0.5)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'var(--bg-toolbar)';
                        e.currentTarget.style.borderColor = 'rgba(231,76,60,0.3)';
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        doDelete([item.id]);
                      }}
                      title={t('common:delete_permanently')}
                    >
                      <Trash2 size={13} style={{ color: '#e74c3c' }} />
                    </Button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        openDetail(item.id);
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.color = 'var(--accent-primary)';
                        e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                        e.currentTarget.style.background = 'none';
                      }}
                      style={{
                        background: 'none',
                        border: 'none',
                        cursor: 'pointer',
                        padding: 4,
                        borderRadius: 4,
                        color: 'var(--text-tertiary)',
                        transition: 'background 0.15s, color 0.15s',
                      }}
                      title={t('common:details')}
                    >
                      <Info size={16} />
                    </button>
                  </div>
                </Card>
              ))
            )}
          </div>
        )}

        {hasSelection && (
          <div
            style={{
              position: 'sticky',
              bottom: 0,
              padding: '12px 16px',
              background: 'var(--bg-elevated)',
              borderRadius: 10,
              border: '1px solid var(--border-subtle)',
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              boxShadow: '0 -2px 12px rgba(0,0,0,0.08)',
            }}
          >
            <span style={{ fontSize: 13, color: 'var(--text-secondary)', marginRight: 'auto' }}>
              {selectedIds.size} {t('settings:selected')}
            </span>
            <Button size="sm" variant="secondary" style={{ border: '1px solid var(--accent-primary)', color: 'var(--accent-primary)' }} onClick={() => doRestore(Array.from(selectedIds))}>
              <RotateCcw size={13} style={{ marginRight: 4 }} /> {t('common:restore')}
            </Button>
            <Button
              size="sm"
              variant="secondary"
              style={{
                color: '#e74c3c',
                border: '1px solid rgba(231,76,60,0.3)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
                e.currentTarget.style.borderColor = 'rgba(231,76,60,0.5)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'rgba(231,76,60,0.3)';
              }}
              onClick={() => doDelete(Array.from(selectedIds))}
            >
              {t('common:delete_permanently')}
            </Button>
          </div>
        )}

        {detailItem && (
          <TrashDetailPanel
            detailItem={detailItem}
            detailTemplate={detailTemplate}
            onClose={() => setDetailItem(null)}
            onRequestRestore={(id) => doRestore([id])}
            onRequestDelete={(id) => doDelete([id])}
          />
        )}

        {confirmAction && (
          <TrashConfirmDialog
            action={confirmAction}
            onClose={() => setConfirmAction(null)}
            onConfirm={async () => {
              await confirmAction.callback();
              setConfirmAction(null);
            }}
          />
        )}
      </PageContainer>
    </AppShell>
  );
}
