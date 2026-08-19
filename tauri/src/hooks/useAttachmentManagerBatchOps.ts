import { useCallback } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { isUriPath } from '@/lib/mobileFileTransfer';
import { logger } from '@/lib/logger';
import type { Toast } from '@/stores/uiStore';
import type {
  AttachmentMeta,
  AttachmentTreePage,
} from '@/components/attachment/attachmentManagerTypes';

export interface UseAttachmentManagerBatchOpsOptions {
  /** 当前显示的所有附件复合键（useBatchSelect 的 allSelected 推导依据）。 */
  allVisibleKeys: string[];
  /** 当前视图（活跃/回收站 + 搜索过滤后）的树数据，用于复合键 → 附件映射。 */
  displayPages: AttachmentTreePage[];
  /** 数据刷新（批量操作后同步）。 */
  loadData: () => Promise<void>;
  t: TFunction;
  showToast: (payload: Omit<Toast, 'id'>) => void;
}

/**
 * 附件管理器的批量操作域（W002-① 拆分：数据 hook）。
 * 批量选择状态（useBatchSelect）与四个批量 handler（下载/软删/永久删除/恢复）
 * 收敛于此；父 hook 仅透传 displayPages 与 loadData 并展开返回值。
 */
export function useAttachmentManagerBatchOps({
  allVisibleKeys,
  displayPages,
  loadData,
  t,
  showToast,
}: UseAttachmentManagerBatchOpsOptions) {
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

  /** 根据选中的复合键批量下载附件 */
  const handleBatchDownload = useCallback(async () => {
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
          message: t('common:batch_download_mobile_unsupported', {
            defaultValue:
              'Batch download to a directory is not supported on mobile. Please download files individually.',
          }),
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
    } catch (e) {
      // P016: dialog 取消经 openWithPause 返回 null 提前 return（不抛异常），
      // 走到 catch 的是真实错误（dialog 插件失败 / 动态 import 失败等），
      // 不再误判为「用户取消」静默吞掉——留痕日志便于排查。
      logger.warn('[AttachmentManager] Batch download failed:', e);
    }
  }, [selectedIds, displayPages, t, showToast, clearSelection]);

  /** 根据选中的复合键批量软删除附件 */
  const handleBatchDelete = useCallback(async () => {
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
        await invoke('attachment_batch_soft_delete', {
          objectId: objectId,
          attachmentIds: attachmentIds,
        });
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
  }, [selectedIds, setBatchDeleteConfirm, clearSelection, loadData, t, showToast]);

  /** 根据选中的复合键批量永久删除附件 */
  const handleBatchPermanentDelete = useCallback(async () => {
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
        await invoke('attachment_batch_delete', {
          objectId: objectId,
          attachmentIds: attachmentIds,
        });
        successCount += attachmentIds.length;
      } catch (e) {
        // P121: 逐对象失败不再静默吞错——记录真实错误，批量结果提示失败数
        failedCount += attachmentIds.length;
        logger.warn(
          '[AttachmentManager] Batch permanent delete failed for object',
          objectId,
          ':',
          e,
        );
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
  }, [selectedIds, setBatchPermanentDeleteConfirm, clearSelection, loadData, t, showToast]);

  /** 根据选中的复合键批量恢复附件 */
  const handleBatchRestore = useCallback(async () => {
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
        await invoke('attachment_batch_restore', {
          objectId: objectId,
          attachmentIds: attachmentIds,
        });
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
  }, [selectedIds, setBatchRestoreConfirm, clearSelection, loadData, t, showToast]);

  return {
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
  };
}
