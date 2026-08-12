import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { downloadViaStage } from '@/lib/mobileFileTransfer';
import { isMobilePlatformSync } from '@/lib/platform';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { useAttachmentBatchOps } from './useAttachmentBatchOps';
import {
  previewItemByMime,
  downloadAttachmentFile,
  type AttachmentItem,
} from '@/lib/attachmentUtils';
import type { AttachmentMetaEditResult } from '@/components/attachment/AttachmentMetaEditDialog';
import { logger } from '@/lib/logger';

export interface AttachmentViewerProps {
  objectId: string;
  onClose: () => void;
  onCountChange?: () => void;
  zIndex?: number;
}

/** 上传/刷新链路的超时保护：防止原生 IPC 未结算导致 UI 永久卡死 */
const UPLOAD_TIMEOUT_MS = 45_000;
const REFRESH_TIMEOUT_MS = 15_000;

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(new Error(`${label} timeout after ${ms}ms`)), ms),
    ),
  ]);
}

/**
 * 对象附件查看器的全部编排逻辑（P046 拆分：数据 hook）。
 * 附件列表加载、上传/重命名/下载/转发/删除/恢复/永久删除、批量操作、
 * 照片集数据源、拖拽上传、预览/元数据编辑状态均收敛于此；
 * AttachmentViewer 组件退化为纯展示组合层。
 */
export function useAttachmentViewer(props: AttachmentViewerProps) {
  const { objectId, onCountChange, zIndex = 2000 } = props;

  const [items, setItems] = useState<AttachmentItem[]>([]);
  const [trashItems, setTrashItems] = useState<AttachmentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [showTrash, setShowTrash] = useState(false);
  const [permDeleteItem, setPermDeleteItem] = useState<AttachmentItem | null>(null);
  const [deleteItem, setDeleteItem] = useState<AttachmentItem | null>(null);
  const [shareItem, setShareItem] = useState<AttachmentItem | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);
  const [previewItem, setPreviewItem] = useState<AttachmentItem | null>(null);
  const [metaEditItem, setMetaEditItem] = useState<AttachmentItem | null>(null);
  const [photoAlbumOpen, setPhotoAlbumOpen] = useState(false);
  const [uploading, setUploading] = useState(false);
  const { t } = useTranslation(['common', 'editor']);
  const showToast = useUiStore((s) => s.showToast);
  const { dialog: confirmDialog } = useConfirm();
  const isNarrowViewport = useIsNarrowViewport();

  const openAttachmentExternal = async (item: AttachmentItem) => {
    try {
      await invoke('attachment_open', {
        objectId,
        attachmentId: item.id,
      });
    } catch {
      showToast({
        type: 'error',
        message: t('common:cannot_open_file', {
          path: item.fileName,
          defaultValue: `Cannot open file. Make sure the file still exists: ${item.fileName}`,
        }),
      });
    }
  };

  const handlePreview = async (item: AttachmentItem) => {
    // 移动端 PDF 使用原生 PdfRenderer 预览，不在 WebView 遮罩中尝试渲染 data URL。
    if (isMobilePlatformSync() && previewItemByMime(item) === 'pdf') {
      openAttachmentExternal(item);
      return;
    }
    setPreviewItem(item);
  };

  const loadAttachments = useCallback(async () => {
    setLoading(true);
    try {
      const [active, deleted] = await Promise.all([
        invoke<AttachmentItem[]>('attachment_list', { objectId: objectId, showDeleted: false }),
        invoke<AttachmentItem[]>('attachment_list', { objectId: objectId, showDeleted: true }),
      ]);
      setItems(active);
      setTrashItems(deleted);
    } catch (e) {
      logger.warn('[AttachmentViewer] Failed to load attachments:', e);
      // 保留旧列表，避免加载失败时界面被清空
    } finally {
      setLoading(false);
    }
  }, [objectId]);

  useEffect(() => {
    loadAttachments();
  }, [loadAttachments]);

  const handleAdd = async () => {
    const filePath = await pickFileToAttach();
    if (!filePath) return;
    setUploading(true);
    try {
      // 真机上曾出现首个附件上传后 IPC 未结算导致 uploading 卡死、按钮永久禁用，
      // 关键 await 均加超时兜底，保证按钮必然复位
      await withTimeout(
        uploadSingleAttachment(filePath, objectId),
        UPLOAD_TIMEOUT_MS,
        'upload',
      );
      showToast({
        type: 'success',
        message: t('common:upload_success'),
      });
      // 刷新失败不影响上传结果本身，单独捕获并明确提示
      try {
        await withTimeout(loadAttachments(), REFRESH_TIMEOUT_MS, 'refresh');
        onCountChange?.();
      } catch (refreshErr) {
        logger.warn('[AttachmentViewer] refresh after upload failed:', refreshErr);
        showToast({
          type: 'warning',
          message:
            t('common:upload_refresh_failed', { defaultValue: 'Uploaded, but the list failed to refresh. Please reopen.' }),
        });
      }
    } catch (e) {
      showToast({
        type: 'error',
        message: `${t('common:upload_failed')}: ${e}`,
      });
    } finally {
      setUploading(false);
    }
  };

  const handleStartRename = (item: AttachmentItem) => {
    setRenamingId(item.id);
    setRenameValue(item.fileName);
    setTimeout(() => renameInputRef.current?.focus(), 50);
  };

  const handleConfirmRename = async () => {
    if (renamingId && renameValue.trim()) {
      const newName = renameValue.trim();
      // 乐观更新前先取原名，失败时回滚，避免前端显示新名、后端仍是旧名的状态不一致
      const prevName = items.find((i) => i.id === renamingId)?.fileName;
      setItems((prev) =>
        prev.map((i) => (i.id === renamingId ? { ...i, fileName: newName } : i)),
      );
      try {
        await invoke('attachment_rename', {
          objectId,
          attachmentId: renamingId,
          newName,
        });
      } catch (err) {
        logger.warn('[AttachmentViewer] Rename failed:', err);
        if (prevName !== undefined) {
          setItems((prev) =>
            prev.map((i) => (i.id === renamingId ? { ...i, fileName: prevName } : i)),
          );
        }
        showToast({ type: 'error', message: t('common:rename_failed') });
      }
    }
    setRenamingId(null);
  };

  const handleDownload = async (item: AttachmentItem) => {
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

  /** 转发：先弹确认框（明文离开 Vault 警示），确认后调用 attachment_share。 */
  const handleShare = (item: AttachmentItem) => {
    setShareItem(item);
  };

  const doShare = async () => {
    if (!shareItem) return;
    const item = shareItem;
    setShareItem(null);
    try {
      await invoke('attachment_share', {
        objectId,
        attachmentId: item.id,
      });
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:forward_failed')}: ${e}` });
    }
  };

  /** 附件描述/标签保存成功：就地更新列表与预览中的附件元数据。 */
  const handleMetaSaved = (updated: AttachmentMetaEditResult) => {
    const patch = (list: AttachmentItem[]) =>
      list.map((i) =>
        i.id === metaEditItem?.id ? { ...i, description: updated.description, tags: updated.tags } : i,
      );
    setItems((prev) => patch(prev));
    setTrashItems((prev) => patch(prev));
    setPreviewItem((prev) =>
      prev && prev.id === metaEditItem?.id ? { ...prev, ...updated } : prev,
    );
  };

  const handleDelete = (item: AttachmentItem) => {
    setDeleteItem(item);
  };

  const handleConfirmDelete = async () => {
    if (!deleteItem) return;
    const item = deleteItem;
    setDeleteItem(null);
    await invoke('attachment_soft_delete', { objectId: objectId, attachmentId: item.id }).catch((e) => {
      showToast({
        type: 'error',
        message: t('common:delete_failed', { defaultValue: `Delete failed: ${e}` }),
      });
    });
    await loadAttachments();
    onCountChange?.();
  };

  const handleRestore = async (item: AttachmentItem) => {
    try {
      await invoke('attachment_restore', { objectId: objectId, attachmentId: item.id });
    } catch (err) {
      // P227: 单条恢复失败原为未捕获 rejection（无任何 UI 反馈），补 toast + 留痕。
      logger.warn('[AttachmentViewer] Restore failed:', err);
      showToast({
        type: 'error',
        message: `${t('common:restore_failed', { defaultValue: 'Restore failed' })}: ${err}`,
      });
    }
    await loadAttachments();
    onCountChange?.();
  };

  const handlePermanentDelete = async (item: AttachmentItem) => {
    setPermDeleteItem(null);
    try {
      await invoke('attachment_delete', { objectId: objectId, attachmentId: item.id });
    } catch (err) {
      // P227: 永久删除失败原为未捕获 rejection，补 toast + 留痕。
      logger.warn('[AttachmentViewer] Permanent delete failed:', err);
      showToast({
        type: 'error',
        message: `${t('common:perm_delete_failed', { defaultValue: 'Delete failed' })}: ${err}`,
      });
    }
    await loadAttachments();
    onCountChange?.();
  };

  const displayItems = showTrash ? trashItems : items;

  /** 活跃附件中的图片。 */
  const activePhotoItems = useMemo(
    () => items.filter((item) => previewItemByMime(item) === 'image'),
    [items],
  );
  /** 回收站附件中的图片。 */
  const trashPhotoItems = useMemo(
    () => trashItems.filter((item) => previewItemByMime(item) === 'image'),
    [trashItems],
  );
  /** 当前视图（活跃/回收站）对应的照片集数据源（附件照片集方案 §3.2）。 */
  const displayPhotoItems = showTrash ? trashPhotoItems : activePhotoItems;

  const allVisibleKeys = useMemo(
    () => displayItems.map((item) => `${objectId}::${item.id}`),
    [displayItems, objectId],
  );

  // W001-②: 批量选择状态与批量操作（删除/恢复/下载/永久删除）收敛于 useAttachmentBatchOps
  const batchOps = useAttachmentBatchOps({
    objectId,
    allVisibleKeys,
    displayItems,
    loadAttachments,
    onCountChange,
  });

  const { ref: dragRef, dragState } = useDragToAttach(objectId, {
    onComplete: () => {
      loadAttachments();
      onCountChange?.();
    },
  });

  return {
    zIndex,
    objectId,
    t,
    // 列表数据
    items,
    trashItems,
    loading,
    showTrash,
    setShowTrash,
    displayItems,
    displayPhotoItems,
    uploading,
    isNarrowViewport,
    // 批量选择（useAttachmentBatchOps 收敛）
    allVisibleKeys,
    ...batchOps,
    // 单项操作状态
    deleteItem,
    setDeleteItem,
    permDeleteItem,
    setPermDeleteItem,
    shareItem,
    setShareItem,
    renamingId,
    setRenamingId,
    renameValue,
    setRenameValue,
    renameInputRef,
    previewItem,
    setPreviewItem,
    metaEditItem,
    setMetaEditItem,
    photoAlbumOpen,
    setPhotoAlbumOpen,
    // 拖拽上传
    dragRef,
    dragState,
    // handlers
    openAttachmentExternal,
    handlePreview,
    handleAdd,
    handleStartRename,
    handleConfirmRename,
    handleDownload,
    handleShare,
    doShare,
    handleMetaSaved,
    handleDelete,
    handleConfirmDelete,
    handleRestore,
    handlePermanentDelete,
    // 确认对话框
    confirmDialog,
  };
}
