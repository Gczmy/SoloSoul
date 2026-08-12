import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useUiStore } from '@/stores/uiStore';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { logger } from '@/lib/logger';

export interface UseAttachmentBatchOpsOptions {
  objectId: string;
  /** 当前视图（活跃/回收站）所有可见附件的复合键（`${objectId}::${id}`），用于全选判定。 */
  allVisibleKeys: string[];
  /** 当前视图（活跃/回收站）的可见附件列表，批量下载按此查找选中项。 */
  displayItems: AttachmentItem[];
  /** 批量操作完成后的刷新回调（父 hook 传入：loadAttachments + onCountChange）。 */
  loadAttachments: () => Promise<void>;
  onCountChange?: () => void;
}

/**
 * 附件批量操作编排（W001-② 拆分：批量段收敛于此）。
 * 从 useAttachmentViewer 抽出——批量选择状态（useBatchSelect）与
 * 批量删除/恢复/下载/永久删除四个 handler 集中在此，父 hook 仅组合。
 */
export function useAttachmentBatchOps({
  objectId,
  allVisibleKeys,
  displayItems,
  loadAttachments,
  onCountChange,
}: UseAttachmentBatchOpsOptions) {
  const { t } = useTranslation(['common', 'editor']);
  const showToast = useUiStore((s) => s.showToast);

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

  const handleBatchDelete = async () => {
    setBatchDeleteConfirm(false);
    const keys = Array.from(selectedIds);
    const attachmentIds = keys.map((k) => k.split('::')[1]);
    try {
      await invoke('attachment_batch_soft_delete', { objectId: objectId, attachmentIds: attachmentIds });
      showToast({
        type: 'success',
        message: t('common:batch_delete_result', { success: keys.length, total: keys.length }),
      });
    } catch (err) {
      logger.warn('[AttachmentViewer] Batch soft delete failed:', err);
      showToast({
        type: 'warning',
        message: t('common:batch_delete_result', { success: 0, total: keys.length }),
      });
    }
    clearSelection();
    await loadAttachments();
    onCountChange?.();
  };

  const handleBatchRestore = async () => {
    setBatchRestoreConfirm(false);
    const keys = Array.from(selectedIds);
    const attachmentIds = keys.map((k) => k.split('::')[1]);
    try {
      await invoke('attachment_batch_restore', { objectId: objectId, attachmentIds: attachmentIds });
      showToast({
        type: 'success',
        message: t('common:batch_restore_result', { success: keys.length, total: keys.length }),
      });
    } catch (err) {
      logger.warn('[AttachmentViewer] Batch restore failed:', err);
      showToast({
        type: 'warning',
        message: t('common:batch_restore_result', { success: 0, total: keys.length }),
      });
    }
    clearSelection();
    await loadAttachments();
    onCountChange?.();
  };

  const handleBatchDownload = async () => {
    const keys = Array.from(selectedIds);
    const attachmentIds = keys.map((k) => k.split('::')[1]);
    const selectedItems = displayItems.filter((item) => attachmentIds.includes(item.id));
    if (selectedItems.length === 0) return;

    let dirPath: string | null;
    if (isMobilePlatformSync()) {
      // 移动端：使用自定义 SAF 目录选择器（plugin-dialog 的 directory 模式在 Android 不支持）
      const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
        (m) => m.useAutoLockPauseStore.getState(),
      );
      pause();
      try {
        const result = await invoke<{ uri: string | null }>('attachment_pick_tree_uri');
        dirPath = result.uri;
      } catch (e) {
        // 显示后端返回的具体错误，便于定位（如 NO_TREE_PICKER_HANDLER）
        showToast({
          type: 'error',
          message: `${t('common:select_directory_failed', { defaultValue: 'Failed to pick directory' })}: ${e}`,
        });
        return;
      } finally {
        resume();
      }
    } else {
      const { openWithPause } = await import('@/lib/dialog');
      dirPath = (await openWithPause({
        directory: true,
        multiple: false,
        title: t('common:select_download_directory', { defaultValue: 'Select download directory' }),
      })) as string | null;
    }
    if (!dirPath) return;

    // P054: 逐条串行 await 改为 Promise.allSettled 并发（各附件独立 IPC + 独立目标文件，
    // 并发安全）；allSettled 不会因单项失败整体 reject，successCount 语义与串行一致。
    // 平台检测只取一次（纯常量），避免逐项重复调用。
    const isMobile = isMobilePlatformSync();
    const downloadTasks = selectedItems
      .filter((item) => !!(item.vaultPath || item.srcPath))
      .map((item) => {
        const filePath = item.vaultPath || (item.srcPath as string);
        if (isMobile) {
          // 移动端 SAF 目录返回的是 content://tree/... URI，走 Android 专用命令
          return invoke('attachment_export_tree_uri', {
            srcPath: filePath,
            treeUri: dirPath,
            fileName: item.fileName,
            mimeType: item.mimeType,
          });
        }
        const destPath = `${dirPath}/${item.fileName}`;
        return invoke('attachment_download', { srcPath: filePath, destPath: destPath });
      });
    const results = await Promise.allSettled(downloadTasks);
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
  };

  const handleBatchPermanentDelete = async () => {
    setBatchPermanentDeleteConfirm(false);
    const keys = Array.from(selectedIds);
    const attachmentIds = keys.map((k) => k.split('::')[1]);
    try {
      await invoke('attachment_batch_delete', { objectId: objectId, attachmentIds: attachmentIds });
      showToast({
        type: 'success',
        message: t('common:batch_perm_delete_result', { success: keys.length, total: keys.length }),
      });
    } catch (err) {
      logger.warn('[AttachmentViewer] Batch permanent delete failed:', err);
      showToast({
        type: 'warning',
        message: t('common:batch_perm_delete_result', { success: 0, total: keys.length }),
      });
    }
    clearSelection();
    await loadAttachments();
    onCountChange?.();
  };

  return {
    selectedIds,
    allSelected,
    toggleSelect,
    handleSelectAll,
    clearSelection,
    batchDeleteConfirm,
    batchRestoreConfirm,
    batchPermanentDeleteConfirm,
    setBatchDeleteConfirm,
    setBatchRestoreConfirm,
    setBatchPermanentDeleteConfirm,
    handleBatchDelete,
    handleBatchRestore,
    handleBatchDownload,
    handleBatchPermanentDelete,
  };
}
