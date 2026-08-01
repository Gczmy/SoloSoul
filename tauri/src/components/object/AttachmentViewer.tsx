import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import {
  Paperclip,
  X,
  RotateCw,
  Upload,
  Download,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useIsNarrowViewport } from '@/hooks/useIsNarrowViewport';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { downloadViaStage } from '@/lib/mobileFileTransfer';
import { isMobilePlatformSync } from '@/lib/platform';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';

import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { truncateFileName, previewItemByMime, type AttachmentItem } from '@/lib/attachmentUtils';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { AttachmentListItem } from '@/components/object/AttachmentListItem';
import { ICON_SIZE } from '@/lib/constants';
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
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);
  const [previewItem, setPreviewItem] = useState<AttachmentItem | null>(null);
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
        message:
          t('common:cannot_open_file', { path: item.fileName }) ||
          `Cannot open file. Make sure the file still exists: ${item.fileName}`,
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
            t('common:upload_refresh_failed') ||
            'Uploaded, but the list failed to refresh. Please reopen.',
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
      setItems((prev) =>
        prev.map((i) => (i.id === renamingId ? { ...i, fileName: renameValue.trim() } : i)),
      );
      invoke('attachment_rename', {
        objectId: objectId,
        attachmentId: renamingId,
        newName: renameValue.trim(),
      }).catch((err) => logger.warn('[AttachmentViewer] Rename failed:', err));
    }
    setRenamingId(null);
  };

  const handleDownload = async (item: AttachmentItem) => {
    const filePath = item.vaultPath || item.srcPath;
    if (!filePath) {
      showToast({ type: 'error', message: t('common:no_file_path') });
      return;
    }
    try {
      const { saveWithPause } = await import('@/lib/dialog');
      const destPath = await saveWithPause({
        defaultPath: item.fileName,
      });
      if (!destPath) return;
      await downloadViaStage(filePath, destPath, item.fileName, (src, dest) =>
        invoke('attachment_download', { srcPath: src, destPath: dest }),
      );
      showToast({
        type: 'success',
        message: t('common:download_result') || 'Downloaded successfully',
      });
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:download_failed')}: ${e}` });
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
        message: t('common:delete_failed') || `Delete failed: ${e}`,
      });
    });
    await loadAttachments();
    onCountChange?.();
  };

  const handleRestore = async (item: AttachmentItem) => {
    await invoke('attachment_restore', { objectId: objectId, attachmentId: item.id });
    await loadAttachments();
    onCountChange?.();
  };

  const handlePermanentDelete = async (item: AttachmentItem) => {
    setPermDeleteItem(null);
    await invoke('attachment_delete', { objectId: objectId, attachmentId: item.id });
    await loadAttachments();
    onCountChange?.();
  };

  const displayItems = showTrash ? trashItems : items;

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
    } catch {
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
    } catch {
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
          message: `${t('common:select_directory_failed') || 'Failed to pick directory'}: ${e}`,
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
        title: t('common:select_download_directory') || 'Select download directory',
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
      message:
        t('common:batch_download_result', {
          success: successCount,
          total: selectedItems.length,
        }) || `Downloaded ${successCount}/${selectedItems.length} files`,
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
    } catch {
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
          {/* Header */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '14px 18px',
              borderBottom: '1px solid var(--border-subtle)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <div
                style={{
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 600,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                {/* 窄视口只留图标，把空间让给 活跃/回收站 切换与右侧操作按钮 */}
                <Paperclip size={ICON_SIZE.sm} />
                {!isNarrowViewport && t('common:attachments')}
              </div>
              <div style={{ display: 'flex', gap: 4 }}>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    setShowTrash(false);
                    clearSelection();
                  }}
                  style={{
                    fontSize: 'var(--text-caption)',
                    ...(!showTrash
                      ? {
                          background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                          borderColor: 'var(--accent-primary)',
                          color: 'var(--accent-primary)',
                          boxShadow: '0 0 0 1px var(--accent-primary)',
                        }
                      : {}),
                  }}
                >
                  {t('common:attachments_active', { n: items.length })}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    setShowTrash(true);
                    clearSelection();
                  }}
                  className="interactive-danger-tab"
                  style={{
                    fontSize: 'var(--text-caption)',
                    ...(showTrash
                      ? {
                          background: 'color-mix(in srgb, #e74c3c 10%, transparent)',
                          borderColor: '#e74c3c',
                          color: '#e74c3c',
                          boxShadow: '0 0 0 1px #e74c3c',
                        }
                      : {}),
                  }}
                >
                  {t('common:attachments_trash', { n: trashItems.length })}
                </Button>
              </div>
            </div>
            {/* 右侧操作：窄视口下 Upload 改纯图标（与关闭按钮同款 44×44），避免头部溢出 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>                {!showTrash &&
                (isNarrowViewport ? (
                  <BadgeIconButton
                    Icon={Upload}
                    onClick={handleAdd}
                    title={t('common:upload') || 'Upload'}
                    iconSize={ICON_SIZE.sm}
                    disabled={uploading}
                  />
                ) : (
                  <Button variant="secondary" size="sm" onClick={handleAdd} disabled={uploading}>
                    {uploading ? (
                      <RotateCw size={ICON_SIZE.sm} style={{ animation: 'spin 1s linear infinite' }} />
                    ) : (
                      <Upload size={ICON_SIZE.sm} />
                    )}{' '}
                    {t('common:upload')}
                  </Button>
                ))}
              <BadgeIconButton
                Icon={X}
                onClick={onClose}
                title={t('common:close') || 'Close'}
                iconSize={ICON_SIZE.md}
              />
            </div>
          </div>
          {/* 批量操作工具栏 — 常驻显示 */}
          {displayItems.length > 0 && (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '8px 12px',
                borderBottom: '1px solid var(--border-subtle)',
                background:
                  selectedIds.size > 0
                    ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                    : 'var(--bg-toolbar)',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <SelectCheckbox
                  checked={allSelected}
                  onClick={() => handleSelectAll(allVisibleKeys)}
                  indeterminate={selectedIds.size > 0 && !allSelected}
                />
                <span
                  style={{ color: 'var(--text-secondary)', cursor: 'pointer', userSelect: 'none' }}
                  onClick={() => handleSelectAll(allVisibleKeys)}
                >
                  {allSelected ? t('common:deselect_all') : t('common:select_all')}
                </span>
                {selectedIds.size > 0 && (
                  <span style={{ color: 'var(--text-tertiary)' }}>
                    {t('common:selected_count', { n: selectedIds.size })}
                  </span>
                )}
              </div>
              {selectedIds.size > 0 && !showTrash ? (
                <div style={{ display: 'flex', gap: 6 }}>
                  <Button variant="secondary" size="sm" onClick={handleBatchDownload}>
                    <Download size={ICON_SIZE.sm} /> {t('common:download')}
                  </Button>
                  <DeleteButton
                    onClick={() => setBatchDeleteConfirm(true)}
                    title={t('common:delete')}
                  >
                    {t('common:delete')}
                  </DeleteButton>
                </div>
              ) : selectedIds.size > 0 && showTrash ? (
                <div style={{ display: 'flex', gap: 6 }}>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setBatchRestoreConfirm(true)}
                  >
                    <RotateCw size={ICON_SIZE.sm} /> {t('common:restore')}
                  </Button>
                  <DeleteButton
                    onClick={() => setBatchPermanentDeleteConfirm(true)}
                    title={t('common:delete_permanently')}
                  >
                    {t('common:delete_permanently')}
                  </DeleteButton>
                </div>
              ) : null}
            </div>
          )}
          {/* List */}
          <div style={{ flex: 1, overflow: 'auto' }}>
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
      {confirmDialog}
      {/* Confirmation dialogs */}
      <ConfirmDialog
        open={!!deleteItem}
        title={t('common:confirm_delete_title', 'Delete attachment')}
        body={
          t('common:confirm_delete_body', {
            name: deleteItem ? truncateFileName(deleteItem.fileName) : '',
          }) ||
          `Delete "${deleteItem ? truncateFileName(deleteItem.fileName) : ''}"? It will be moved to trash.`
        }
        confirmLabel={t('common:delete')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleConfirmDelete}
        onCancel={() => setDeleteItem(null)}
      />
      <ConfirmDialog
        open={batchDeleteConfirm}
        title={t('common:batch_delete_title')}
        body={t('common:batch_delete_body', { n: selectedIds.size })}
        confirmLabel={t('common:delete')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleBatchDelete}
        onCancel={() => setBatchDeleteConfirm(false)}
      />
      <ConfirmDialog
        open={batchRestoreConfirm}
        title={t('common:batch_restore_title')}
        body={t('common:batch_restore_body', { n: selectedIds.size })}
        confirmLabel={t('common:restore')}
        cancelLabel={t('common:cancel')}
        confirmStyle="primary"
        onConfirm={handleBatchRestore}
        onCancel={() => setBatchRestoreConfirm(false)}
      />
      <ConfirmDialog
        open={batchPermanentDeleteConfirm}
        title={t('common:batch_perm_delete_title')}
        body={t('common:batch_perm_delete_body', { n: selectedIds.size })}
        confirmLabel={t('common:delete_permanently')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleBatchPermanentDelete}
        onCancel={() => setBatchPermanentDeleteConfirm(false)}
      />
      <ConfirmDialog
        open={!!permDeleteItem}
        title={t('common:perm_delete_title')}
        body={t('common:perm_delete_body', {
          name: permDeleteItem ? truncateFileName(permDeleteItem.fileName) : '',
        })}
        confirmLabel={t('common:delete_permanently')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={() => permDeleteItem && handlePermanentDelete(permDeleteItem)}
        onCancel={() => setPermDeleteItem(null)}
      />
    </div>
  );
}
