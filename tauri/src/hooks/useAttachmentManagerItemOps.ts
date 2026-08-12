import { useState, useCallback } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { downloadViaStage } from '@/lib/mobileFileTransfer';
import { previewItemByMime, truncateFileName, downloadAttachmentFile } from '@/lib/attachmentUtils';
import { isMobilePlatformSync } from '@/lib/platform';
import type { Toast } from '@/stores/uiStore';
import type { AttachmentMeta, AttachmentToPurge } from '@/components/attachment/attachmentManagerTypes';

export interface UseAttachmentManagerItemOpsOptions {
  /** 数据刷新（操作成功后同步）。 */
  loadData: () => Promise<void>;
  /** 通用确认框（软删除前置确认）。 */
  requestConfirm: (
    title: string,
    message: string,
    onConfirm: () => void,
    options?: { confirmLabel?: string; cancelLabel?: string },
  ) => void;
  t: TFunction;
  showToast: (payload: Omit<Toast, 'id'>) => void;
}

/**
 * 附件管理器的单项附件操作域（W002-① 拆分：数据 hook）。
 * 打开/预览/重命名/上传/软删/下载/转发/恢复/永久删除 12 个 handler 与
 * 各自状态（preview/share/rename/permDelete）收敛于此；父 hook 仅透传
 * loadData/requestConfirm 并展开返回值。
 */
export function useAttachmentManagerItemOps({
  loadData,
  requestConfirm,
  t,
  showToast,
}: UseAttachmentManagerItemOpsOptions) {
  const [previewItem, setPreviewItem] = useState<AttachmentMeta | null>(null);
  const [shareItem, setShareItem] = useState<AttachmentMeta | null>(null);
  /** 正在编辑描述/标签的附件（非空时渲染 AttachmentMetaEditDialog） */
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameObjectId, setRenameObjectId] = useState<string>('');
  const [permDeleteItem, setPermDeleteItem] = useState<AttachmentToPurge | null>(null);

  const openAttachmentExternal = useCallback(
    async (item: AttachmentMeta) => {
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
    },
    [t, showToast],
  );

  const handlePreview = useCallback(
    async (item: AttachmentMeta) => {
      // 移动端 PDF 无法直接在 WebView 遮罩中渲染，统一使用系统应用打开，与对象附件卡片保持一致。
      if (isMobilePlatformSync() && previewItemByMime(item) === 'pdf') {
        openAttachmentExternal(item);
        return;
      }
      setPreviewItem(item);
    },
    [openAttachmentExternal],
  );

  // P217: 重命名输入值由 AttachmentRow 内自包含的 RenameInput 本地管理，
  // 此处仅记录「正在重命名哪个附件」，确认时接收行内提交的新文件名。
  const handleStartRename = useCallback((item: AttachmentMeta, objectId: string) => {
    setRenamingId(item.id);
    setRenameObjectId(objectId);
  }, []);

  const handleConfirmRename = useCallback(
    async (newName: string) => {
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
    },
    [renamingId, renameObjectId, loadData, t, showToast],
  );

  const handleUpload = useCallback(
    async (objectId: string) => {
      const filePath = await pickFileToAttach();
      if (filePath) {
        try {
          await uploadSingleAttachment(filePath, objectId);
          await loadData();
        } catch (e) {
          showToast({ type: 'error', message: `${t('common:upload_failed')}: ${e}` });
        }
      }
    },
    [loadData, t, showToast],
  );

  const handleSoftDelete = useCallback(
    (item: AttachmentMeta, objectId: string) => {
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
    },
    [requestConfirm, t, loadData, showToast],
  );

  const handleDownload = useCallback(
    async (item: AttachmentMeta) => {
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
    },
    [t, showToast],
  );

  /** 转发：先弹确认框（明文离开 Vault 警示），确认后调用 attachment_share。 */
  const handleShare = useCallback((item: AttachmentMeta) => {
    setShareItem(item);
  }, []);

  const doShare = useCallback(async () => {
    if (!shareItem) return;
    try {
      await invoke('attachment_share', {
        objectId: shareItem.objectId,
        attachmentId: shareItem.id,
      });
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:forward_failed')}: ${e}` });
    }
    setShareItem(null);
  }, [shareItem, t, showToast]);

  const handleRestore = useCallback(
    async (item: AttachmentMeta, objectId: string) => {
      try {
        await invoke('attachment_restore', { objectId: objectId, attachmentId: item.id });
        await loadData();
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:restore_failed')}: ${e}` });
      }
    },
    [loadData, t, showToast],
  );

  const handlePermanentDelete = useCallback((item: AttachmentMeta, objectId: string) => {
    setPermDeleteItem({ ...item, _objectId: objectId });
  }, []);

  const doPermanentDelete = useCallback(async () => {
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
  }, [permDeleteItem, loadData, t, showToast]);

  return {
    previewItem,
    setPreviewItem,
    shareItem,
    setShareItem,
    renamingId,
    setRenamingId,
    renameObjectId,
    setRenameObjectId,
    permDeleteItem,
    setPermDeleteItem,
    openAttachmentExternal,
    handlePreview,
    handleStartRename,
    handleConfirmRename,
    handleUpload,
    handleSoftDelete,
    handleDownload,
    handleShare,
    doShare,
    handleRestore,
    handlePermanentDelete,
    doPermanentDelete,
  };
}
