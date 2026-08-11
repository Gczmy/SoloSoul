import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { motion } from 'framer-motion';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { downloadViaStage } from '@/lib/mobileFileTransfer';
import { Images } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';

import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import {
  previewItemByMime,
  downloadAttachmentFile,
  type AttachmentItem,
} from '@/lib/attachmentUtils';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { PhotoAlbumOverlay } from '@/components/attachment/PhotoAlbumOverlay';
import { AttachmentListItem } from '@/components/object/AttachmentListItem';
import { AttachmentViewerHeader } from '@/components/object/AttachmentViewerHeader';
import { AttachmentBatchToolbar } from '@/components/object/AttachmentBatchToolbar';
import { AttachmentConfirmDialogs } from '@/components/object/AttachmentConfirmDialogs';
import { logger } from '@/lib/logger';

export type { AttachmentItem } from '@/lib/attachmentUtils';

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

export function AttachmentViewer({
  objectId,
  onClose,
  onCountChange,
  zIndex = 2000,
}: AttachmentViewerProps) {
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

  const { ref: dragRef, dragState } = useDragToAttach(objectId, {
    onComplete: () => {
      loadAttachments();
      onCountChange?.();
    },
  });

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(6px)',
      }}
      onClick={() => {
        // 有确认对话框打开时禁止背景点击关闭，避免移动端误触回到 workspace
        if (
          deleteItem ||
          permDeleteItem ||
          shareItem ||
          batchDeleteConfirm ||
          batchRestoreConfirm ||
          batchPermanentDeleteConfirm
        ) {
          return;
        }
        onClose();
      }}
    >
      {!loading && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          onClick={(e) => e.stopPropagation()}
          ref={dragRef}
          style={{
            width: 500,
            maxWidth: 'calc(100vw - 32px)',
            maxHeight: '80vh',
            display: 'flex',
            flexDirection: 'column',
            background: 'var(--bg-elevated)',
            borderRadius: 16,
            boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
            border: '1px solid var(--border-subtle)',
            position: 'relative',
            margin: 16,
          }}
        >
          {/* Header（P013 拆分） */}
          <AttachmentViewerHeader
            isNarrowViewport={isNarrowViewport}
            showTrash={showTrash}
            activeCount={items.length}
            trashCount={trashItems.length}
            uploading={uploading}
            onShowActive={() => {
              setShowTrash(false);
              clearSelection();
            }}
            onShowTrash={() => {
              setShowTrash(true);
              clearSelection();
            }}
            onAdd={handleAdd}
            onClose={onClose}
          />
          {/* 批量操作工具栏 — 常驻显示（P013 拆分） */}
          {displayItems.length > 0 && (
            <AttachmentBatchToolbar
              showTrash={showTrash}
              allSelected={allSelected}
              selectedCount={selectedIds.size}
              onToggleSelectAll={() => handleSelectAll(allVisibleKeys)}
              onBatchDownload={handleBatchDownload}
              onBatchDelete={() => setBatchDeleteConfirm(true)}
              onBatchRestore={() => setBatchRestoreConfirm(true)}
              onBatchPermanentDelete={() => setBatchPermanentDeleteConfirm(true)}
            />
          )}
          {/* List */}
          <div style={{ flex: 1, overflow: 'auto' }}>
            {/* 照片集入口：活跃/回收站视图各有对应数据源的照片集（附件照片集方案 §3.2） */}
            {displayPhotoItems.length > 0 && (
              <button
                type="button"
                onClick={() => setPhotoAlbumOpen(true)}
                className="interactive-toolbar"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  width: 'calc(100% - 24px)',
                  margin: '8px 12px 4px',
                  padding: '10px 12px',
                  borderRadius: 10,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  cursor: 'pointer',
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-primary)',
                }}
              >
                <Images size={ICON_SIZE.sm} style={{ color: 'var(--accent-primary)' }} />
                <span style={{ flex: 1, textAlign: 'left' }}>
                  {t('common:photo_album', 'Photo Album')}
                </span>
                <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {displayPhotoItems.length}
                </span>
              </button>
            )}
            {displayItems.length === 0 ? (
              <div
                style={{
                  textAlign: 'center',
                  padding: 48,
                  color: 'var(--text-secondary)',
                  fontSize: 'var(--text-body)',
                }}
              >
                {showTrash ? t('common:attachments_trash_empty') : t('common:no_attachments')}
              </div>
            ) : (
              displayItems.map((item, idx) => {
                const compositeKey = `${objectId}::${item.id}`;
                const checked = selectedIds.has(compositeKey);
                return (
                  <AttachmentListItem
                    key={item.id}
                    item={item}
                    compositeKey={compositeKey}
                    checked={checked}
                    showTrash={showTrash}
                    isLast={idx === displayItems.length - 1}
                    renamingId={renamingId}
                    renameValue={renameValue}
                    renameInputRef={renameInputRef}
                    onToggleSelect={toggleSelect}
                    onRenameValueChange={setRenameValue}
                    onConfirmRename={handleConfirmRename}
                    onCancelRename={() => setRenamingId(null)}
                    onRestore={handleRestore}
                    onPreview={handlePreview}
                    onStartRename={handleStartRename}
                    onDownload={handleDownload}
                    onShare={handleShare}
                    onDelete={handleDelete}
                    onPermanentDelete={setPermDeleteItem}
                  />
                );
              })
            )}
          </div>
          {/* 拖拽上传覆盖层 */}
          <DragUploadOverlay dragState={dragState} borderRadius={16} />
        </motion.div>
      )}{' '}
      {/* Preview overlay */}
      <AttachmentPreviewOverlay
        item={previewItem}
        onClose={() => setPreviewItem(null)}
        onOpenExternal={openAttachmentExternal}
      />
      {/* Photo album overlay（对象级照片集） */}
      {photoAlbumOpen && displayPhotoItems.length > 0 && (
        <PhotoAlbumOverlay
          items={displayPhotoItems}
          onClose={() => setPhotoAlbumOpen(false)}
          onOpenExternal={openAttachmentExternal}
          zIndex={2100}
        />
      )}
      {confirmDialog}
      {/* Confirmation dialogs（P013 拆分） */}
      <AttachmentConfirmDialogs
        deleteItem={deleteItem}
        permDeleteItem={permDeleteItem}
        shareItem={shareItem}
        batchDeleteConfirm={batchDeleteConfirm}
        batchRestoreConfirm={batchRestoreConfirm}
        batchPermanentDeleteConfirm={batchPermanentDeleteConfirm}
        selectedCount={selectedIds.size}
        onConfirmDelete={handleConfirmDelete}
        onCancelDelete={() => setDeleteItem(null)}
        onConfirmPermanentDelete={() => permDeleteItem && handlePermanentDelete(permDeleteItem)}
        onCancelPermanentDelete={() => setPermDeleteItem(null)}
        onConfirmShare={doShare}
        onCancelShare={() => setShareItem(null)}
        onConfirmBatchDelete={handleBatchDelete}
        onCancelBatchDelete={() => setBatchDeleteConfirm(false)}
        onConfirmBatchRestore={handleBatchRestore}
        onCancelBatchRestore={() => setBatchRestoreConfirm(false)}
        onConfirmBatchPermanentDelete={handleBatchPermanentDelete}
        onCancelBatchPermanentDelete={() => setBatchPermanentDeleteConfirm(false)}
      />
    </div>
  );
}
