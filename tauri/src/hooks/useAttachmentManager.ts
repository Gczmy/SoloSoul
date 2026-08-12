import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useAttachmentPageSort } from '@/hooks/useAttachmentPageSort';
import { collectPhotoItems } from '@/lib/attachmentUtils';
import { useAttachmentManagerBatchOps } from '@/hooks/useAttachmentManagerBatchOps';
import { useAttachmentManagerItemOps } from '@/hooks/useAttachmentManagerItemOps';
import type {
  AttachmentListAllResult,
  AttachmentMeta,
  AttachmentTreePage,
  AttachmentTreeObject,
} from '@/components/attachment/attachmentManagerTypes';

const getPageKey = (p: AttachmentTreePage) => p.pageId || p.pageName;
const getObjKey = (o: AttachmentTreeObject) => o.objectId;

/**
 * 附件管理器页面的编排逻辑（P024 拆分：数据 hook；W002-① 再拆后为组合层）。
 * 数据加载、树展开、展示数据派生与统计收敛于此；批量操作收敛于
 * useAttachmentManagerBatchOps，单项附件操作收敛于 useAttachmentManagerItemOps。
 * 返回主组件渲染所需的状态与回调。
 */
export function useAttachmentManager() {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const [data, setData] = useState<AttachmentListAllResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [showTrash, setShowTrash] = useState(false);
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const [expandedObjects, setExpandedObjects] = useState<Set<string>>(new Set());
  /** 正在编辑描述/标签的附件（非空时渲染 AttachmentMetaEditDialog） */
  const [metaEditItem, setMetaEditItem] = useState<AttachmentMeta | null>(null);
  const [albumOpen, setAlbumOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const loadData = useCallback(async () => {
    if (!accountId) return;
    setLoading(true);
    try {
      const result = await invoke<AttachmentListAllResult>('attachment_list_all', {
        accountId: accountId,
      });
      setData(result);
    } catch {
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Expand all pages and objects by default when data loads
  useEffect(() => {
    if (!data) return;
    const allPages = new Set<string>();
    const allObjects = new Set<string>();
    const pages = data.pages.concat(data.trashPages);
    for (const page of pages) {
      const pageKey = getPageKey(page);
      allPages.add(pageKey);
      for (const obj of page.objects) {
        allObjects.add(`${pageKey}::${getObjKey(obj)}`);
      }
    }
    setExpandedPages(allPages);
    setExpandedObjects(allObjects);
  }, [data]);

  // ── Tree expansion ──────────────────────────────────────────

  const togglePage = (key: string) => {
    setExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const toggleObject = (key: string) => {
    setExpandedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  // ── Display data ───────────────────────────────────────────

  const rawPages = showTrash ? data?.trashPages || [] : data?.pages || [];
  const sortedPages = useAttachmentPageSort(rawPages);

  /** 活跃附件中的全部图片。 */
  const activePhotoItems = useMemo(() => collectPhotoItems(data?.pages), [data]);
  /** 回收站软删除附件中的全部图片。 */
  const trashPhotoItems = useMemo(() => collectPhotoItems(data?.trashPages), [data]);
  /** 当前视图（活跃/回收站）对应的照片集数据源（附件照片集方案 §3.2）。 */
  const photoItems = useMemo(
    () => (showTrash ? trashPhotoItems : activePhotoItems),
    [showTrash, trashPhotoItems, activePhotoItems],
  );

  // Filter pages/objects/attachments by search query (matches against file name)
  const displayPages = useMemo(() => {
    if (!searchQuery.trim()) return sortedPages;
    const q = searchQuery.toLowerCase();
    return sortedPages
      .map((page) => ({
        ...page,
        objects: page.objects
          .map((obj) => ({
            ...obj,
            attachments: obj.attachments.filter((att) => att.fileName.toLowerCase().includes(q)),
          }))
          .filter((obj) => obj.attachments.length > 0),
      }))
      .filter((page) => page.objects.length > 0);
  }, [sortedPages, searchQuery]);

  /** 收集当前显示的所有附件复合键 */
  const allVisibleKeys = useMemo(() => {
    const keys: string[] = [];
    for (const page of displayPages) {
      for (const obj of page.objects) {
        for (const att of obj.attachments) {
          keys.push(`${obj.objectId}::${att.id}`);
        }
      }
    }
    return keys;
  }, [displayPages]);

  // ── 批量操作（useAttachmentManagerBatchOps 收敛）────────────
  const batchOps = useAttachmentManagerBatchOps({
    allVisibleKeys,
    displayPages,
    loadData,
    t,
    showToast,
  });

  // ── 单项附件操作（useAttachmentManagerItemOps 收敛）─────────
  const itemOps = useAttachmentManagerItemOps({
    loadData,
    requestConfirm,
    t,
    showToast,
  });

  // Auto-expand all pages/objects when searching
  useEffect(() => {
    if (!searchQuery.trim() || !data) return;
    const allPages = new Set<string>();
    const allObjects = new Set<string>();
    for (const page of displayPages) {
      const pageKey = getPageKey(page);
      allPages.add(pageKey);
      for (const obj of page.objects) {
        allObjects.add(`${pageKey}::${getObjKey(obj)}`);
      }
    }
    setExpandedPages(allPages);
    setExpandedObjects(allObjects);
  }, [displayPages, searchQuery, data]);

  // ── Count summaries (unified via summaryStats) ─────────────

  const summaryStats = useMemo(() => {
    const activePages = data?.pages || [];
    const trashPages = data?.trashPages || [];
    let activeAttachments = 0,
      activeBytes = 0,
      activeObjects = 0;
    for (const page of activePages) {
      for (const obj of page.objects) {
        activeObjects++;
        for (const att of obj.attachments) {
          activeAttachments++;
          activeBytes += att.sizeBytes;
        }
      }
    }
    let trashAttachments = 0,
      trashBytes = 0,
      trashObjects = 0;
    for (const page of trashPages) {
      for (const obj of page.objects) {
        trashObjects++;
        for (const att of obj.attachments) {
          trashAttachments++;
          trashBytes += att.sizeBytes;
        }
      }
    }
    return {
      activeAttachments,
      activeBytes,
      activeObjects,
      trashAttachments,
      trashBytes,
      trashObjects,
    };
  }, [data]);

  return {
    t,
    showToast,
    confirmDialog,
    data,
    loading,
    showTrash,
    setShowTrash,
    expandedPages,
    expandedObjects,
    togglePage,
    toggleObject,
    photoItems,
    metaEditItem,
    setMetaEditItem,
    albumOpen,
    setAlbumOpen,
    searchQuery,
    setSearchQuery,
    loadData,
    displayPages,
    allVisibleKeys,
    summaryStats,
    // 单项附件操作（useAttachmentManagerItemOps 展开）
    ...itemOps,
    // 批量操作（useAttachmentManagerBatchOps 展开）
    ...batchOps,
  };
}
