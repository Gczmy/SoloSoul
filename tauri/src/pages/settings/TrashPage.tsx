import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { useToastError } from '@/hooks/useToastError';
import { useSettingsStore } from '@/stores/settingsStore';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { useTemplateStore } from '@/stores/templateStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Trash2, RotateCcw, Info, Search, FileX } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { logger } from '@/lib/logger';
import type { UserTemplate } from '@/types/template';
import { TrashDetailPanel } from '@/components/trash/TrashDetailPanel';
import { TrashConfirmDialog } from '@/components/trash/TrashConfirmDialog';
import { TrashItemCard } from '@/components/trash/TrashItemCard';
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

// P119: 回收站分页大小（参照 ObjectWorkspacePage 的 OBJECT_PAGE_SIZE=50 模式）
const TRASH_PAGE_SIZE = 50;

export function TrashPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['settings', 'common', 'editor']);
  const { onSuccess, onError } = useToastError();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  // P055: 分字段 selector，避免 store 任何变化触发整页重渲染（函数引用稳定）
  const items = useTrashStore((s) => s.items);
  const timeFilter = useTrashStore((s) => s.timeFilter);
  const typeFilter = useTrashStore((s) => s.typeFilter);
  const searchQuery = useTrashStore((s) => s.searchQuery);
  const loadItems = useTrashStore((s) => s.loadItems);
  const setTimeFilter = useTrashStore((s) => s.setTimeFilter);
  const setTypeFilter = useTrashStore((s) => s.setTypeFilter);
  const setSearchQuery = useTrashStore((s) => s.setSearchQuery);
  const restoreItem = useTrashStore((s) => s.restoreItem);
  const permanentDelete = useTrashStore((s) => s.permanentDelete);
  const isLoading = useTrashStore((s) => s.isLoading);
  const error = useTrashStore((s) => s.error);
  const selectedIds = useTrashStore((s) => s.selectedIds);
  const toggleSelection = useTrashStore((s) => s.toggleSelection);
  const selectAll = useTrashStore((s) => s.selectAll);
  const clearSelection = useTrashStore((s) => s.clearSelection);

  const [detailItem, setDetailItem] = useState<TrashDetail | null>(null);
  const [detailTemplate, setDetailTemplate] = useState<UserTemplate | null>(null);
  const [, setLoadingDetail] = useState(false);
  // N-11: 详情加载失败态——与「无数据」区分，失败时显示错误占位 + 重试。
  const [detailError, setDetailError] = useState<{ trashId: string; message: string } | null>(
    null,
  );
  const getTemplate = useTemplateStore((s) => s.getTemplate);

  const [confirmAction, setConfirmAction] = useState<TrashConfirmAction | null>(null);

  // P119: 分页游标——回收站可达数百条，仅挂载前 TRASH_PAGE_SIZE 条，
  // 「加载更多」追加；搜索词/类型过滤/条目集变化时重置。
  const [visibleLimit, setVisibleLimit] = useState(TRASH_PAGE_SIZE);
  useEffect(() => {
    setVisibleLimit(TRASH_PAGE_SIZE);
  }, [searchQuery, typeFilter, items]);

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

  // P119: filtered useMemo——搜索击键/过滤切换不再每次渲染重建过滤数组
  const filtered = useMemo(
    () =>
      items
        .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
        .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase())),
    [items, typeFilter, searchQuery],
  );

  const allFilteredSelected = filtered.length > 0 && filtered.every((i) => selectedIds.has(i.id));
  const hasSelection = selectedIds.size > 0;

  // P119: 回调 useCallback 稳定化——父级重新渲染时不再新建卡片闭包
  const doRestore = useCallback(
    (ids: string[]) => {
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
                  t('settings:trash_restored_with_cascaded_page', {
                    page: outcome.cascadedPageName,
                  }),
                );
              } else if (outcome.rebuiltPageName) {
                onSuccess(
                  t('settings:trash_restored_with_rebuilt_page', {
                    page: outcome.rebuiltPageName,
                  }),
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
            // R2-V3：内层不吞错——重新抛出，外层 TrashConfirmDialog onConfirm 捕获后
            // 保持对话框打开可重试；已恢复项在重试时按「Trash item not found」幂等处理。
            throw err;
          }
        },
      });
    },
    [restoreItem, onSuccess, onError, t, clearSelection, accountId],
  );

  const doDelete = useCallback(
    (ids: string[]) => {
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
    },
    [items, permanentDelete, clearSelection],
  );

  const handleRestoreOne = useCallback((trashId: string) => doRestore([trashId]), [doRestore]);
  const handleDeleteOne = useCallback((trashId: string) => doDelete([trashId]), [doDelete]);

  const openDetail = useCallback(
    async (trashId: string) => {
      setLoadingDetail(true);
      setDetailError(null);
      try {
        const d = await invoke<TrashDetail>('trash_get_detail', { trashId: trashId });
        setDetailItem(d);
        if (d.templateId) {
          getTemplate(d.templateId)
            .then((tpl) => setDetailTemplate(tpl))
            .catch((err) => logger.warn('[TrashPage] Load detail template failed:', err));
        } else {
          setDetailTemplate(null);
        }
      } catch (e) {
        // P122: 加载失败不再静默——toast 区分「无数据」与「加载失败」
        logger.warn('[TrashPage] Load trash detail failed:', e);
        onError(e, t('settings:trash_detail_load_failed', { defaultValue: '加载回收站详情失败' }));
        setDetailItem(null);
        setDetailTemplate(null);
        // N-11: 记录失败态（含原 trashId 供重试），UI 不再与「无数据」同态。
        setDetailError({
          trashId,
          message: typeof e === 'string' ? e : e instanceof Error ? e.message : String(e),
        });
      } finally {
        setLoadingDetail(false);
      }
    },
    [getTemplate, onError, t],
  );

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
          <FilterChipGroup
            options={TIME_OPTIONS.map((opt) => ({
              id: opt.value,
              label: t(`settings:${opt.labelKey}`, opt.labelKey),
            }))}
            value={timeFilter}
            onChange={(v) => {
              if (v) setTimeFilter(v);
            }}
          />
          <FilterChipGroup
            options={TYPE_OPTIONS.map((opt) => ({
              id: opt.value,
              label: t(`settings:trash_type_${opt.value}`),
            }))}
            value={typeFilter}
            onChange={(v) => {
              if (v) setTypeFilter(v);
            }}
          />
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
              <>
                {filtered.slice(0, visibleLimit).map((item) => (
                  <TrashItemCard
                    key={item.id}
                    item={item}
                    isSelected={selectedIds.has(item.id)}
                    onOpenDetail={openDetail}
                    onRestore={handleRestoreOne}
                    onDelete={handleDeleteOne}
                    onToggle={toggleSelection}
                  />
                ))}
                {filtered.length > visibleLimit && (
                  <Button
                    variant="tertiary"
                    size="sm"
                    onClick={() => setVisibleLimit((n) => n + TRASH_PAGE_SIZE)}
                    style={{ marginTop: 4 }}
                  >
                    {t('common:load_more', { defaultValue: '加载更多' })}
                  </Button>
                )}
              </>
            )}
          </div>
        )}

        {detailItem && (
          <TrashDetailPanel
            detailItem={detailItem}
            detailTemplate={detailTemplate}
            onClose={() => {
              setDetailItem(null);
              setDetailError(null);
            }}
            onRequestRestore={(id) => doRestore([id])}
            onRequestDelete={(id) => doDelete([id])}
          />
        )}

        {detailError && !detailItem && (
          <Card>
            <div
              style={{
                textAlign: 'center',
                padding: '32px 24px',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 12,
              }}
            >
              <Info size={ICON_SIZE['2xl']} style={{ opacity: 0.4, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
                {t('settings:trash_detail_load_failed', { defaultValue: '加载回收站详情失败' })}
              </p>
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  maxWidth: 420,
                  wordBreak: 'break-word',
                }}
              >
                {detailError.message}
              </p>
              <Button
                variant="primary"
                size="sm"
                onClick={() => openDetail(detailError.trashId)}
              >
                {t('common:retry')}
              </Button>
            </div>
          </Card>
        )}

        {confirmAction && (
          <TrashConfirmDialog
            action={confirmAction}
            onClose={() => setConfirmAction(null)}
            onConfirm={async () => {
              try {
                await confirmAction.callback();
                setConfirmAction(null);
              } catch (e) {
                // 失败不关闭对话框（可重试），toast 提示具体错误，避免 unhandled rejection
                onError(
                  e,
                  confirmAction.type === 'delete'
                    ? t('common:delete_permanently')
                    : t('common:restore'),
                );
              }
            }}
          />
        )}
      </PageContainer>
    </AppShell>
  );
}
