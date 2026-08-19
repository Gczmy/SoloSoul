import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { useToastError } from '@/hooks/useToastError';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Info, Trash2, RotateCcw, FileX } from 'lucide-react';
import { logger } from '@/lib/logger';
import type { UserTemplate } from '@/types/template';
import type { TrashDetail, TrashConfirmAction } from '@/components/trash/types';

export const TIME_OPTIONS: { value: TrashTimeFilter; labelKey: string }[] = [
  { value: 'all', labelKey: 'all' },
  { value: '1d', labelKey: '1d' },
  { value: '3d', labelKey: '3d' },
  { value: '7d', labelKey: '7d' },
  { value: '30d', labelKey: '30d' },
  { value: 'half_year', labelKey: 'half_year' },
];

export const TYPE_OPTIONS: { value: TrashTypeFilter; i18nKey: string }[] = [
  { value: 'all', i18nKey: 'all' },
  { value: 'page', i18nKey: 'page' },
  { value: 'object', i18nKey: 'object' },
  { value: 'template', i18nKey: 'template' },
];

// P119: 回收站分页大小（参照 ObjectWorkspacePage 的 OBJECT_PAGE_SIZE=50 模式）
export const TRASH_PAGE_SIZE = 50;

/**
 * 回收站页的全部编排逻辑（P046 拆分：数据 hook）。
 * store 装配、过滤/分页、批量恢复/删除确认流、详情加载均收敛于此；
 * TrashPage 组件退化为纯展示组合层。
 */
export function useTrashPage() {
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
  const restoreBatch = useTrashStore((s) => s.restoreBatch);
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
  const [detailError, setDetailError] = useState<{ trashId: string; message: string } | null>(null);
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
        title: t('common:guide_trash_title', { defaultValue: 'Trash Guide' }),
        steps: [
          {
            icon: FileX,
            title: t('common:guide_trash_step1_title', { defaultValue: 'Soft Delete' }),
            description: t('common:guide_trash_step1_desc', {
              defaultValue:
                'Deleting an object or page moves it to trash instead of removing it immediately. Use time and type filters to find items.',
            }),
          },
          {
            icon: RotateCcw,
            title: t('common:guide_trash_step2_title', { defaultValue: 'Restore' }),
            description: t('common:guide_trash_step2_desc', {
              defaultValue:
                'Select items and tap Restore to recover them. Restoring a page may also restore its related objects.',
            }),
          },
          {
            icon: Trash2,
            title: t('common:guide_trash_step3_title', { defaultValue: 'Permanent Delete' }),
            description: t('common:guide_trash_step3_desc', {
              defaultValue:
                'Use the trash can button to permanently remove selected items. This action cannot be undone.',
            }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_trash', { defaultValue: 'Trash' }),
            description: t('common:guide_help_trash_desc', {
              defaultValue: 'Restore or permanently delete trashed items',
            }),
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
            // P014: 批量端点一次 IPC（替代逐项串行 restoreItem）；模板/对象由后端
            // 统一分派，级联恢复/已删除项幂等跳过，返回全部 outcome 供逐条 toast。
            const outcomes = await restoreBatch(ids);
            for (const outcome of outcomes) {
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
    [restoreBatch, onSuccess, onError, t, clearSelection, accountId],
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

  // TrashConfirmDialog 确认后的统一入口：失败保持对话框打开（可重试）
  const handleConfirmAction = useCallback(
    async (action: TrashConfirmAction) => {
      try {
        await action.callback();
        setConfirmAction(null);
      } catch (e) {
        // 失败不关闭对话框（可重试），toast 提示具体错误，避免 unhandled rejection
        onError(e, action.type === 'delete' ? t('common:delete_permanently') : t('common:restore'));
      }
    },
    [onError, t],
  );

  return {
    t,
    // store 状态
    items,
    timeFilter,
    typeFilter,
    searchQuery,
    isLoading,
    error,
    selectedIds,
    setTimeFilter,
    setTypeFilter,
    setSearchQuery,
    // 派生
    filtered,
    allFilteredSelected,
    hasSelection,
    visibleLimit,
    setVisibleLimit,
    trashGuidePages,
    // 详情
    detailItem,
    setDetailItem,
    detailTemplate,
    detailError,
    setDetailError,
    // 确认流
    confirmAction,
    setConfirmAction,
    // 操作
    doRestore,
    doDelete,
    handleRestoreOne,
    handleDeleteOne,
    openDetail,
    toggleSelection,
    selectAll,
    clearSelection,
    handleConfirmAction,
  };
}
