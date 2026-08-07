import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useAttachmentPageSort } from '@/hooks/useAttachmentPageSort';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { downloadViaStage, isUriPath } from '@/lib/mobileFileTransfer';
import { logger } from '@/lib/logger';
import { previewItemByMime, truncateFileName, downloadAttachmentFile } from '@/lib/attachmentUtils';
import { isMobilePlatformSync } from '@/lib/platform';
import type {
  AttachmentListAllResult,
  AttachmentMeta,
  AttachmentTreeObject,
  AttachmentTreePage,
  AttachmentToPurge,
} from '@/components/attachment/attachmentManagerTypes';

const getPageKey = (p: AttachmentTreePage) => p.pageId || p.pageName;
const getObjKey = (o: AttachmentTreeObject) => o.objectId;

/**
 * 附件管理器页面的数据加载、附件操作与批量操作逻辑（P024 拆分）。
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
  const [previewItem, setPreviewItem] = useState<AttachmentMeta | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameObjectId, setRenameObjectId] = useState<string>('');
  const [permDeleteItem, setPermDeleteItem] = useState<AttachmentToPurge | null>(null);
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

  // ── Attachment operations ──────────────────────────────────

  const openAttachmentExternal = async (item: AttachmentMeta) => {
    try {
      await invoke('attachment_open', {
        objectId: item.objectId,
        attachmentId: item.id,
      });
    } catch {
      showToast({
        type: 'error',
        message: t('common:cannot_open_file', {
          path: item.fileName,
          defaultValue: `Cannot open file: ${item.fileName}`,
        }),
      });
    }
  };

  const handlePreview = async (item: AttachmentMeta) => {
    // 移动端 PDF 无法直接在 WebView 遮罩中渲染，统一使用系统应用打开，与对象附件卡片保持一致。
    if (isMobilePlatformSync() && previewItemByMime(item) === 'pdf') {
      openAttachmentExternal(item);
      return;
    }
    setPreviewItem(item);
  };

  // P217: 重命名输入值由 AttachmentRow 内自包含的 RenameInput 本地管理，
  // 此处仅记录「正在重命名哪个附件」，确认时接收行内提交的新文件名。
  const handleStartRename = (item: AttachmentMeta, objectId: string) => {
    setRenamingId(item.id);
    setRenameObjectId(objectId);
  };

  const handleConfirmRename = async (newName: string) => {
    const trimmed = newName.trim();
    if (renamingId && trimmed && renameObjectId) {
      try {
        await invoke('attachment_rename', {
          objectId: renameObjectId,
          attachmentId: renamingId,
          newName: trimmed,
        });
        await loadData();
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:rename_failed')}: ${e}` });
      }
    }
    setRenamingId(null);
    setRenameObjectId('');
  };

  const handleUpload = async (objectId: string) => {
    const filePath = await pickFileToAttach();
    if (filePath) {
      try {
        await uploadSingleAttachment(filePath, objectId);
        await loadData();
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:upload_failed')}: ${e}` });
      }
    }
  };

  const handleSoftDelete = (item: AttachmentMeta, objectId: string) => {
    requestConfirm(
      t('common:confirm_delete_title', 'Delete attachment'),
      t('common:confirm_delete_body', {
        name: truncateFileName(item.fileName),
        defaultValue: `Delete "${truncateFileName(item.fileName)}"? It will be moved to trash.`,
      }),
      async () => {
        try {
          await invoke('attachment_soft_delete', { objectId: objectId, attachmentId: item.id });
          await loadData();
        } catch (e) {
          showToast({ type: 'error', message: `${t('common:delete_failed')}: ${e}` });
        }
      },
      { confirmLabel: t('common:delete'), cancelLabel: t('common:cancel') },
    );
  };

  const handleDownload = async (item: AttachmentMeta) => {
    // P011: 统一走共享下载入口（saveWithPause + downloadViaStage + toast）
    const filePath = item.vaultPath || item.srcPath;
    if (!filePath) {
      showToast({ type: 'error', message: t('common:no_file_path') });
      return;
    }
    await downloadAttachmentFile({
      filePath,
      fileName: item.fileName,
      invoke,
      showToast,
      t,
      downloadViaStage,
    });
  };

  const handleRestore = async (item: AttachmentMeta, objectId: string) => {
    try {
      await invoke('attachment_restore', { objectId: objectId, attachmentId: item.id });
      await loadData();
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:restore_failed')}: ${e}` });
    }
  };

  const handlePermanentDelete = (item: AttachmentMeta, objectId: string) => {
    setPermDeleteItem({ ...item, _objectId: objectId });
  };

  const doPermanentDelete = async () => {
    if (!permDeleteItem) return;
    try {
      await invoke('attachment_delete', {
        objectId: permDeleteItem._objectId,
        attachmentId: permDeleteItem.id,
      });
      await loadData();
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:perm_delete_failed')}: ${e}` });
    }
    setPermDeleteItem(null);
  };

  // ── Display data ───────────────────────────────────────────

  const rawPages = showTrash ? data?.trashPages || [] : data?.pages || [];
  const sortedPages = useAttachmentPageSort(rawPages);

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

  const {
    selectedIds,
    batchDeleteConfirm,
    batchRestoreConfirm,
    batchPermanentDeleteConfirm,
    allSelected,
    toggleSelect,
    handleSelectAll,
    clearSelection,
    setBatchDeleteConfirm,
    setBatchRestoreConfirm,
    setBatchPermanentDeleteConfirm,
  } = useBatchSelect(allVisibleKeys);

  // ── Batch operations ───────────────────────────────────────

  /** 根据选中的复合键批量下载附件 */
  const handleBatchDownload = async () => {
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    try {
      const { openWithPause } = await import('@/lib/dialog');
      const dirPath = await openWithPause({
        directory: true,
        multiple: false,
        title: t('common:select_download_directory', { defaultValue: 'Select download directory' }),
      });
      if (!dirPath) return;

      // Android 目录选择器返回 tree URI，无法直接用 std::fs 写入，暂不支持批量下载
      if (typeof dirPath === 'string' && isUriPath(dirPath)) {
        showToast({
          type: 'warning',
          message:
            t('common:batch_download_mobile_unsupported', { defaultValue: 'Batch download to a directory is not supported on mobile. Please download files individually.' }),
        });
        return;
      }

      // Map entries to actual attachment items
      const selectedItems: AttachmentMeta[] = [];
      for (const page of displayPages) {
        for (const obj of page.objects) {
          for (const att of obj.attachments) {
            if (entries.includes(`${obj.objectId}::${att.id}`)) {
              selectedItems.push(att);
            }
          }
        }
      }

      // P054: 逐条串行 await 改为 Promise.allSettled 并发（各附件独立 IPC + 独立目标文件，
      // 并发安全）；allSettled 不会因单项失败整体 reject，successCount 语义与串行一致。
      const results = await Promise.allSettled(
        selectedItems
          .filter((item) => !!(item.vaultPath || item.srcPath))
          .map((item) => {
            const filePath = item.vaultPath || (item.srcPath as string);
            const destPath = `${dirPath}/${item.fileName}`;
            return invoke('attachment_download', { srcPath: filePath, destPath: destPath });
          }),
      );
      const successCount = results.filter((r) => r.status === 'fulfilled').length;

      showToast({
        type: successCount === selectedItems.length ? 'success' : 'warning',
        message: t('common:batch_download_result', {
          success: successCount,
          total: selectedItems.length,
          defaultValue: `Downloaded ${successCount}/${selectedItems.length} files`,
        }),
      });
      clearSelection();
    } catch {
      // dialog cancelled
    }
  };

  /** 根据选中的复合键批量软删除附件 */
  const handleBatchDelete = async () => {
    setBatchDeleteConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [objectId, attachmentId] = key.split('::');
      if (!byObject.has(objectId)) byObject.set(objectId, []);
      byObject.get(objectId)!.push(attachmentId);
    }

    let successCount = 0;
    let failedCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_soft_delete', { objectId: objectId, attachmentIds: attachmentIds });
        successCount += attachmentIds.length;
      } catch (e) {
        // P121: 逐对象失败不再静默吞错——记录真实错误，批量结果提示失败数
        failedCount += attachmentIds.length;
        logger.warn('[AttachmentManager] Batch soft delete failed for object', objectId, ':', e);
      }
    }

    clearSelection();
    await loadData();
    const base = t('common:batch_delete_result', {
      success: successCount,
      total: entries.length,
      defaultValue: `Deleted ${successCount}/${entries.length} attachments`,
    });
    showToast({
      type: failedCount > 0 ? 'warning' : 'info',
      message:
        failedCount > 0
          ? `${base}${t('common:batch_op_failed_suffix', {
              count: failedCount,
              defaultValue: `（${failedCount} 项失败）`,
            })}`
          : base,
    });
  };

  /** 根据选中的复合键批量永久删除附件 */
  const handleBatchPermanentDelete = async () => {
    setBatchPermanentDeleteConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [objectId, attachmentId] = key.split('::');
      if (!byObject.has(objectId)) byObject.set(objectId, []);
      byObject.get(objectId)!.push(attachmentId);
    }

    let successCount = 0;
    let failedCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_delete', { objectId: objectId, attachmentIds: attachmentIds });
        successCount += attachmentIds.length;
      } catch (e) {
        // P121: 逐对象失败不再静默吞错——记录真实错误，批量结果提示失败数
        failedCount += attachmentIds.length;
        logger.warn('[AttachmentManager] Batch permanent delete failed for object', objectId, ':', e);
      }
    }

    clearSelection();
    await loadData();
    const base = t('common:batch_perm_delete_result', {
      success: successCount,
      total: entries.length,
      defaultValue: `Permanently deleted ${successCount}/${entries.length} attachments`,
    });
    showToast({
      type: failedCount > 0 ? 'warning' : 'info',
      message:
        failedCount > 0
          ? `${base}${t('common:batch_op_failed_suffix', {
              count: failedCount,
              defaultValue: `（${failedCount} 项失败）`,
            })}`
          : base,
    });
  };

  /** 根据选中的复合键批量恢复附件 */
  const handleBatchRestore = async () => {
    setBatchRestoreConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    // Group by objectId
    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [oid, attachmentId] = key.split('::');
      if (!byObject.has(oid)) byObject.set(oid, []);
      byObject.get(oid)!.push(attachmentId);
    }

    let successCount = 0;
    let failedCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_restore', { objectId: objectId, attachmentIds: attachmentIds });
        successCount += attachmentIds.length;
      } catch (e) {
        // P121: 逐对象失败不再静默吞错——记录真实错误，批量结果提示失败数
        failedCount += attachmentIds.length;
        logger.warn('[AttachmentManager] Batch restore failed for object', objectId, ':', e);
      }
    }
    clearSelection();
    await loadData();
    const base = t('common:batch_restore_result', {
      success: successCount,
      total: entries.length,
      defaultValue: `Restored ${successCount}/${entries.length} attachments`,
    });
    showToast({
      type: failedCount > 0 ? 'warning' : 'info',
      message:
        failedCount > 0
          ? `${base}${t('common:batch_op_failed_suffix', {
              count: failedCount,
              defaultValue: `（${failedCount} 项失败）`,
            })}`
          : base,
    });
  };

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
    previewItem,
    setPreviewItem,
    renamingId,
    setRenamingId,
    renameObjectId,
    setRenameObjectId,
    permDeleteItem,
    setPermDeleteItem,
    searchQuery,
    setSearchQuery,
    loadData,
    openAttachmentExternal,
    handlePreview,
    handleStartRename,
    handleConfirmRename,
    handleUpload,
    handleSoftDelete,
    handleDownload,
    handleRestore,
    handlePermanentDelete,
    doPermanentDelete,
    displayPages,
    allVisibleKeys,
    selectedIds,
    batchDeleteConfirm,
    batchRestoreConfirm,
    batchPermanentDeleteConfirm,
    allSelected,
    toggleSelect,
    handleSelectAll,
    clearSelection,
    setBatchDeleteConfirm,
    setBatchRestoreConfirm,
    setBatchPermanentDeleteConfirm,
    handleBatchDownload,
    handleBatchDelete,
    handleBatchPermanentDelete,
    handleBatchRestore,
    summaryStats,
  };
}
