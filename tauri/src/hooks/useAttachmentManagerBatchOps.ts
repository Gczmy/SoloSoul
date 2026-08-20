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
      // P023: 三重 for + if 改为 flatMap 链（行为等价）
      const selectedItems: AttachmentMeta[] = displayPages.flatMap((page) =>
        page.objects.flatMap((obj) =>
          obj.attachments.filter((att) => entries.includes(`${obj.objectId}::${att.id}`)),
        ),
      );

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

  /**
   * P017: 软删/永久删/恢复三函数参数化合并——逐行相同的分组、IPC 循环、
   * 失败统计与 toast 文案结构收敛为单一实现，仅 IPC 命令名与 i18n key 不同。
   * 逐个对象串行 await（P016 另行并行化）。
   */
  const runBatchOperation = useCallback(
    async (op: {
      command: 'attachment_batch_soft_delete' | 'attachment_batch_delete' | 'attachment_batch_restore';
      closeConfirm: () => void;
      resultKey: 'batch_delete_result' | 'batch_perm_delete_result' | 'batch_restore_result';
      opLabel: string;
      defaultMessage: string;
    }) => {
      op.closeConfirm();
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
          await invoke(op.command, {
            objectId: objectId,
            attachmentIds: attachmentIds,
          });
          successCount += attachmentIds.length;
        } catch (e) {
          // P121: 逐对象失败不再静默吞错——记录真实错误，批量结果提示失败数
          failedCount += attachmentIds.length;
          logger.warn(`[AttachmentManager] ${op.opLabel} failed for object`, objectId, ':', e);
        }
      }

      clearSelection();
      await loadData();
      const base = t(`common:${op.resultKey}`, {
        success: successCount,
        total: entries.length,
        defaultValue: op.defaultMessage,
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
    },
    [selectedIds, clearSelection, loadData, t, showToast],
  );

  /** 根据选中的复合键批量软删除附件 */
  const handleBatchDelete = useCallback(() => {
    return runBatchOperation({
      command: 'attachment_batch_soft_delete',
      closeConfirm: setBatchDeleteConfirm.bind(null, false),
      resultKey: 'batch_delete_result',
      opLabel: 'Batch soft delete',
      defaultMessage: `Deleted ${selectedIds.size}/${selectedIds.size} attachments`,
    });
  }, [runBatchOperation, selectedIds.size, setBatchDeleteConfirm]);

  /** 根据选中的复合键批量永久删除附件 */
  const handleBatchPermanentDelete = useCallback(() => {
    return runBatchOperation({
      command: 'attachment_batch_delete',
      closeConfirm: setBatchPermanentDeleteConfirm.bind(null, false),
      resultKey: 'batch_perm_delete_result',
      opLabel: 'Batch permanent delete',
      defaultMessage: `Permanently deleted ${selectedIds.size}/${selectedIds.size} attachments`,
    });
  }, [runBatchOperation, selectedIds.size, setBatchPermanentDeleteConfirm]);

  /** 根据选中的复合键批量恢复附件 */
  const handleBatchRestore = useCallback(() => {
    return runBatchOperation({
      command: 'attachment_batch_restore',
      closeConfirm: setBatchRestoreConfirm.bind(null, false),
      resultKey: 'batch_restore_result',
      opLabel: 'Batch restore',
      defaultMessage: `Restored ${selectedIds.size}/${selectedIds.size} attachments`,
    });
  }, [runBatchOperation, selectedIds.size, setBatchRestoreConfirm]);

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
