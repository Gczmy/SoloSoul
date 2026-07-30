import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { useToastError } from '@/hooks/useToastError';
import { useSettingsStore } from '@/stores/settingsStore';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { useTemplateStore } from '@/stores/templateStore';
import { invoke } from '@tauri-apps/api/core';
import { Trash2, RotateCcw, FileText, Info, Folder, LayoutTemplate, Search, FileX } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { isMobilePlatformSync } from '@/lib/platform';
import { logger } from '@/lib/logger';
import { PluginBadge } from '@/components/template/PluginBadge';
import type { UserTemplate } from '@/types/template';
import { TrashDetailPanel } from '@/components/trash/TrashDetailPanel';
import { TrashConfirmDialog } from '@/components/trash/TrashConfirmDialog';
import type { TrashDetail, TrashConfirmAction } from '@/components/trash/types';
import { ICON_SIZE } from '@/lib/constants';

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
  const { onSuccess, onError } = useToastError();
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
  const { getTemplate } = useTemplateStore();

  const [confirmAction, setConfirmAction] = useState<TrashConfirmAction | null>(null);

  const trashGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_trash_title') ?? 'Trash Guide',
        steps: [
          {
            icon: FileX,
            title: t('common:guide_trash_step1_title') ?? 'Soft Delete',
            description:
              t('common:guide_trash_step1_desc') ??
              'Deleting an object or page moves it to trash instead of removing it immediately. Use time and type filters to find items.',
          },
          {
            icon: RotateCcw,
            title: t('common:guide_trash_step2_title') ?? 'Restore',
            description:
              t('common:guide_trash_step2_desc') ??
              'Select items and tap Restore to recover them. Restoring a page may also restore its related objects.',
          },
          {
            icon: Trash2,
            title: t('common:guide_trash_step3_title') ?? 'Permanent Delete',
            description:
              t('common:guide_trash_step3_desc') ??
              'Use the trash can button to permanently remove selected items. This action cannot be undone.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_trash') ?? 'Trash',
            description:
              t('common:guide_help_trash_desc') ??
              'Restore or permanently delete trashed items',
            href: '/help?id=trash',
          },
        ],
      },
    ],
    [t],
  );

  useEffect(() => {
    setTypeFilter('all');
  }, [setTypeFilter]);

  useEffect(() => {
    if (accountId) loadItems(accountId);
  }, [accountId, timeFilter, loadItems]);

  const filtered = items
    .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
    .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase()));

  const allFilteredSelected = filtered.length > 0 && filtered.every((i) => selectedIds.has(i.id));
  const hasSelection = selectedIds.size > 0;

  const doRestore = (ids: string[]) => {
    setConfirmAction({
      type: 'restore',
      ids,
      count: ids.length,
      callback: async () => {
        try {
          for (const id of ids) {
            const outcome = await restoreItem(id);
            if (outcome.cascadedPageName) {
              // 恢复对象触发了页面级联恢复：同时显示两条 toast
              onSuccess(t('settings:trash_restored', { name: outcome.name }));
              onSuccess(
                t('settings:trash_restored_with_cascaded_page', { page: outcome.cascadedPageName }),
              );
            } else if (outcome.rebuiltPageName) {
              onSuccess(
                t('settings:trash_restored_with_rebuilt_page', { page: outcome.rebuiltPageName }),
              );
            } else if ((outcome.cascadedCount ?? 0) > 0) {
              onSuccess(
                t('settings:trash_restored_with_count', { count: outcome.cascadedCount }),
              );
            } else {
              onSuccess(t('settings:trash_restored', { name: outcome.name }));
            }
          }
          clearSelection();
          if (accountId)
            useSettingsStore
              .getState()
              .loadCustomPages(accountId)
              .catch((err) =>
                  logger.warn('[TrashPage] Load custom pages after restore failed:', err),
                );
        } catch (err) {
          onError(err, t('common:restore_failed'));
        }
      },
    });
  };

  const doDelete = (ids: string[]) => {
    const selectedItems = items.filter((i) => ids.includes(i.id));
    const pageSectionTypes = new Set(
      selectedItems
        .filter((i) => i.itemType === 'page' && i.originalSectionType)
        .map((i) => i.originalSectionType as string),
    );
    let pageChildCount: number | undefined;
    if (pageSectionTypes.size > 0) {
      const count = items.filter(
        (i) => i.itemType === 'object' && pageSectionTypes.has(i.originalSectionType ?? ''),
      ).length;
      if (count > 0) pageChildCount = count;
    }
    setConfirmAction({
      type: 'delete',
      ids,
      count: ids.length,
      pageChildCount,
      callback: async () => {
        await permanentDelete(ids);
        clearSelection();
      },
    });
  };

  const openDetail = async (trashId: string) => {
    setLoadingDetail(true);
    try {
      const d = await invoke<TrashDetail>('trash_get_detail', { trash_id: trashId });
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
    <AppShell
      title={t('settings:trash')}
      actions={<PageGuideButton pages={trashGuidePages} />}
      onBack={() => {
        const state = location.state as { fromHome?: boolean } | undefined;
        if (state?.fromHome) {
          navigate('/home');
        } else {
          navigate('/settings');
        }
      }}
    >
      <PageContainer variant="medium" gap="default">

        <Input
          placeholder={t('settings:search_trash')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
        />

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {TIME_OPTIONS.map((opt) => {
              const isActive = timeFilter === opt.value;
              return (
                <button
                  key={opt.value}
                  onClick={() => setTimeFilter(opt.value)}
                  onMouseEnter={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background =
                            'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                        }
                      : undefined
                  }
                  onMouseLeave={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        }
                      : undefined
                  }
                  style={{
                    padding: '5px 12px',
                    borderRadius: 6,
                    border: isActive
                      ? '1px solid var(--accent-primary)'
                      : '1px solid var(--border-subtle)',
                    background: isActive
                      ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                      : 'var(--bg-toolbar)',
                    color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                    boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                    fontSize: 'var(--text-sm)',
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
                  onMouseEnter={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background =
                            'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                        }
                      : undefined
                  }
                  onMouseLeave={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        }
                      : undefined
                  }
                  style={{
                    padding: '5px 12px',
                    borderRadius: 6,
                    border: isActive
                      ? '1px solid var(--accent-primary)'
                      : '1px solid var(--border-subtle)',
                    background: isActive
                      ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                      : 'var(--bg-toolbar)',
                    color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                    boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                    fontSize: 'var(--text-sm)',
                    cursor: 'pointer',
                    transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                  }}
                >
                  {t(`settings:trash_type_${opt.value}`)}
                </button>
              );
            })}
          </div>
        </div>

        {filtered.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                fontSize: 'var(--text-sm)',
                color: 'var(--text-secondary)',
                padding: '4px 0',
              }}
            >
              <SelectCheckbox
                checked={allFilteredSelected}
                indeterminate={!allFilteredSelected && hasSelection}
                onChange={() =>
                  allFilteredSelected ? clearSelection() : selectAll(filtered.map((i) => i.id))
                }
              />
              <span>
                {t('settings:select_all')} ({filtered.length})
              </span>
            </div>

            {/* 批量操作栏：嵌入页面内，紧贴全选勾选框下方 */}
            {hasSelection && (
              <div
                style={{
                  padding: '10px 14px',
                  background: 'var(--bg-toolbar)',
                  borderRadius: 10,
                  border: '1px solid var(--border-subtle)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                <Button
                  size="sm"
                  variant="tertiary"
                  onClick={() => doRestore(Array.from(selectedIds))}
                >
                  <RotateCcw size={ICON_SIZE.xs} style={{ marginRight: 4 }} />
                  {t('common:restore_all')}
                </Button>
                <DeleteButton
                  onClick={() => doDelete(Array.from(selectedIds))}
                  title={t('common:delete_permanently_all')}
                >
                  {t('common:delete_permanently_all')}
                </DeleteButton>
              </div>
            )}
          </div>
        )}

        {error && (
          <Card>
            <p
              style={{
                textAlign: 'center',
                color: 'var(--error)',
                padding: 16,
                fontSize: 'var(--text-body-sm)',
              }}
            >
              {error}
            </p>
          </Card>
        )}

        {isLoading && items.length === 0 ? (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={120} />
          </Card>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {filtered.length === 0 ? (
              <Card>
                <div style={{ textAlign: 'center', padding: '48px 24px' }}>
                  <Trash2
                    size={ICON_SIZE['5xl']}
                    style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
                  />
                  <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
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
                  {(() => {
                    const isMobile = isMobilePlatformSync();
                    const icon = (() => {
                      const Icon =
                        item.itemType === 'template'
                          ? LayoutTemplate
                          : item.itemType === 'page'
                            ? Folder
                            : FileText;
                      return (
                        <Icon
                          size={ICON_SIZE.xl}
                          style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                        />
                      );
                    })();

                    const meta = (
                      <div style={{ minWidth: 0, flex: 1 }}>
                        <div
                          style={{
                            fontSize: 'var(--text-body)',
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
                            fontSize: 'var(--text-caption)',
                            color: 'var(--text-tertiary)',
                          }}
                        >
                          {t(`settings:trash_type_${item.itemType}`)} · {timeAgo(item.deletedAt, t)}
                          {item.expiresAt &&
                            ` · ${t('settings:trash_expires_in', { days: Math.max(0, Math.floor((item.expiresAt - Date.now()) / 86400000)) })}`}
                          <PluginBadge contractTypeId={item.contractTypeId} size="sm" />
                        </div>
                      </div>
                    );

                    const actions = (
                      <>
                        <Button
                          size="sm"
                          variant="tertiary"
                          onClick={(e) => {
                            e.stopPropagation();
                            doRestore([item.id]);
                          }}
                          title={t('common:restore')}
                        >
                          <RotateCcw size={ICON_SIZE.sm} />
                        </Button>
                        <DeleteButton
                          onClick={(e) => {
                            e.stopPropagation();
                            doDelete([item.id]);
                          }}
                          title={t('common:delete_permanently')}
                        />
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            openDetail(item.id);
                          }}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.color = 'var(--accent-primary)';
                            e.currentTarget.style.background =
                              'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
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
                          <Info size={ICON_SIZE.lg} />
                        </button>
                      </>
                    );

                    return isMobile ? (
                      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
                        <SelectCheckbox
                          checked={selectedIds.has(item.id)}
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleSelection(item.id);
                          }}
                        />
                        <div
                          style={{
                            display: 'flex',
                            flexDirection: 'column',
                            gap: 8,
                            flex: 1,
                            minWidth: 0,
                          }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                            {icon}
                            {meta}
                          </div>
                          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
                            {actions}
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <SelectCheckbox
                          checked={selectedIds.has(item.id)}
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleSelection(item.id);
                          }}
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
                          {icon}
                          {meta}
                        </div>
                        {actions}
                      </div>
                    );
                  })()}
                </Card>
              ))
            )}
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
