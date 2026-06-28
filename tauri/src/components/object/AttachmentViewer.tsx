import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Paperclip, X, RotateCw, Eye, Image, FileText, Edit2, Upload, Download } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { truncateFileName, formatSize, isImageMime, type AttachmentItem } from '@/lib/attachmentUtils';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { ICON_SIZE } from '@/lib/iconSizes';


export type { AttachmentItem } from '@/lib/attachmentUtils';

export interface AttachmentViewerProps {
  objectId: string;
  onClose: () => void;
  onCountChange?: () => void;
  zIndex?: number;
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
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);
  const [previewItem, setPreviewItem] = useState<AttachmentItem | null>(null);
  const { t } = useTranslation(['common', 'editor']);
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();


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
    const isImg = isImageMime(item.mimeType, item.fileName);
    if (isImg) {
      setPreviewItem(item);
    } else {
      openAttachmentExternal(item);
    }
  };

  const loadAttachments = useCallback(async () => {
    setLoading(true);
    try {
      const [active, deleted] = await Promise.all([
        invoke<AttachmentItem[]>('attachment_list', { objectId, showDeleted: false }),
        invoke<AttachmentItem[]>('attachment_list', { objectId, showDeleted: true }),
      ]);
      setItems(active);
      setTrashItems(deleted);
    } catch {
      setItems([]);
      setTrashItems([]);
    } finally {
      setLoading(false);
    }
  }, [objectId]);

  useEffect(() => {
    loadAttachments();
  }, [loadAttachments]);

  const handleAdd = async () => {
    const filePath = await pickFileToAttach();
    if (filePath) {
      await uploadSingleAttachment(filePath, objectId);
      await loadAttachments();
      onCountChange?.();
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
        objectId,
        attachmentId: renamingId,
        newName: renameValue.trim(),
      }).catch((err) => console.warn('[AttachmentViewer] Rename failed:', err));
    }
    setRenamingId(null);
  };

  const handleDownload = async (item: AttachmentItem) => {
    const filePath = item.vaultPath || item.srcPath;
    if (!filePath) {
      showToast({ type: 'error', message: 'No file path available' });
      return;
    }
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const destPath = await save({
        defaultPath: item.fileName,
      });
      if (!destPath) return;
      await invoke('attachment_download', { srcPath: filePath, destPath });
      showToast({ type: 'success', message: t('common:download_result') || 'Downloaded successfully' });
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:download_failed')}: ${e}` });
    }
  };

  const handleDelete = (item: AttachmentItem) => {
    const truncatedName = truncateFileName(item.fileName);
    requestConfirm(
      t('common:confirm_delete_title', 'Delete attachment'),
      t('common:confirm_delete_body', { name: truncatedName }) ||
        `Delete "${truncatedName}"? It will be moved to trash.`,
      async () => {
        await invoke('attachment_soft_delete', { objectId, attachmentId: item.id }).catch((e) => {
          showToast({
            type: 'error',
            message: t('common:delete_failed') || `Delete failed: ${e}`,
          });
        });
        await loadAttachments();
        onCountChange?.();
      },
      { confirmLabel: t('common:delete'), cancelLabel: t('common:cancel') },
    );
  };

  const handleRestore = async (item: AttachmentItem) => {
    await invoke('attachment_restore', { objectId, attachmentId: item.id });
    await loadAttachments();
    onCountChange?.();
  };

  const handlePermanentDelete = async (item: AttachmentItem) => {
    setPermDeleteItem(null);
    await invoke('attachment_delete', { objectId, attachmentId: item.id });
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
      await invoke('attachment_batch_soft_delete', { objectId, attachmentIds });
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
      await invoke('attachment_batch_restore', { objectId, attachmentIds });
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

    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const dirPath = await open({ directory: true, multiple: false, title: t('common:select_download_directory') || 'Select download directory' });
      if (!dirPath) return;

      let successCount = 0;
      for (const item of selectedItems) {
        const filePath = item.vaultPath || item.srcPath;
        if (!filePath) continue;
        const destPath = `${dirPath}/${item.fileName}`;
        try {
          await invoke('attachment_download', { srcPath: filePath, destPath });
          successCount++;
        } catch {
          // continue with next file
        }
      }

      showToast({
        type: successCount === selectedItems.length ? 'success' : 'warning',
        message: t('common:batch_download_result', { success: successCount, total: selectedItems.length }) ||
          `Downloaded ${successCount}/${selectedItems.length} files`,
      });
      clearSelection();
    } catch {
      // dialog cancelled
    }
  };

  const handleBatchPermanentDelete = async () => {
    setBatchPermanentDeleteConfirm(false);
    const keys = Array.from(selectedIds);
    const attachmentIds = keys.map((k) => k.split('::')[1]);
    try {
      await invoke('attachment_batch_delete', { objectId, attachmentIds });
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
      onClick={onClose}
    >
      {!loading && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          onClick={(e) => e.stopPropagation()}          ref={dragRef}
          style={{
            width: 500,
            maxHeight: '80vh',
            display: 'flex',
            flexDirection: 'column',
            background: 'var(--bg-elevated)',
            borderRadius: 16,
            boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
            border: '1px solid var(--border-subtle)',
            position: 'relative',
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
              <Paperclip size={ICON_SIZE.sm} /> {t('common:attachments')}
            </div>
            <div style={{ display: 'flex', gap: 4 }}>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => { setShowTrash(false); clearSelection(); }}
                style={{
                  fontSize: 'var(--text-caption)',
                  ...(!showTrash ? {
                    background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                    borderColor: 'var(--accent-primary)',
                    color: 'var(--accent-primary)',
                    boxShadow: '0 0 0 1px var(--accent-primary)',
                  } : {}),
                }}
              >
                {t('common:attachments_active', { n: items.length })}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => { setShowTrash(true); clearSelection(); }}
                style={{
                  fontSize: 'var(--text-caption)',
                  ...(showTrash ? {
                    background: 'color-mix(in srgb, #e74c3c 10%, transparent)',
                    borderColor: '#e74c3c',
                    color: '#e74c3c',
                    boxShadow: '0 0 0 1px #e74c3c',
                  } : {}),
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = '#e74c3c';
                  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 10%, transparent)';
                }}
                onMouseLeave={(e) => {
                  if (!showTrash) {
                    e.currentTarget.style.borderColor = '';
                    e.currentTarget.style.background = '';
                  }
                }}
              >
                {t('common:attachments_trash', { n: trashItems.length })}
              </Button>
            </div>
          </div>
          {!showTrash && (
            <Button variant="secondary" size="sm" onClick={handleAdd}>
              <Upload size={ICON_SIZE.sm} /> {t('common:upload')}
            </Button>
          )}
          <BadgeIconButton Icon={X} onClick={onClose} title={t('common:close') || 'Close'} iconSize={ICON_SIZE.md} />
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
              background: selectedIds.size > 0
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
              <span style={{ color: 'var(--text-secondary)', cursor: 'pointer', userSelect: 'none' }} onClick={() => handleSelectAll(allVisibleKeys)}>
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
                <DeleteButton onClick={() => setBatchDeleteConfirm(true)} title={t('common:delete')}>
                  {t('common:delete')}
                </DeleteButton>
              </div>
            ) : selectedIds.size > 0 && showTrash ? (
              <div style={{ display: 'flex', gap: 6 }}>
                <Button variant="secondary" size="sm" onClick={() => setBatchRestoreConfirm(true)}>
                  <RotateCw size={ICON_SIZE.sm} /> {t('common:restore')}
                </Button>
                <DeleteButton onClick={() => setBatchPermanentDeleteConfirm(true)} title={t('common:delete_permanently')}>
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
              <div
                key={item.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 12px',
                  borderBottom: idx < displayItems.length - 1 ? '1px solid var(--border-subtle)' : 'none',
                  fontSize: 'var(--text-body-sm)',
                }}
              >
                <SelectCheckbox
                  checked={checked}
                  onClick={(e) => { e.stopPropagation(); toggleSelect(compositeKey); }}
                />
                {item.mimeType.startsWith('image/') ? (
                  <Image size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0, opacity: showTrash ? 0.5 : 1 }} />
                ) : item.mimeType === 'application/pdf' ? (
                  <FileText size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0, opacity: showTrash ? 0.5 : 1 }} />
                ) : (
                  <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0, opacity: showTrash ? 0.5 : 1 }} />
                )}
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div
                    style={{
                      fontWeight: 500,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      textDecoration: showTrash ? 'line-through' : 'none',
                      opacity: showTrash ? 0.5 : 1,
                    }}
                  >
                    {truncateFileName(item.fileName)}
                  </div>
                  <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                    {formatSize(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
                  </div>
                </div>
                {showTrash ? (
                  <>
                    <BadgeIconButton
                      Icon={RotateCw}
                      onClick={() => handleRestore(item)}
                      title={t('common:restore')}
                      iconSize={ICON_SIZE.sm}
                    />
                    <DeleteButton
                      iconOnly
                      onClick={() => setPermDeleteItem(item)}
                      title={t('common:delete_permanently')}
                    />
                  </>
                ) : (
                  <>
                    {renamingId === item.id ? (
                      <input
                        ref={renameInputRef}
                        value={renameValue}
                        onChange={(e) => setRenameValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleConfirmRename();
                          if (e.key === 'Escape') setRenamingId(null);
                        }}
                        onBlur={handleConfirmRename}
                        style={{
                          width: 100,
                          padding: '3px 6px',
                          fontSize: 'var(--text-caption)',
                          borderRadius: 4,
                          border: '1px solid var(--accent-primary)',
                          background: 'transparent',
                          color: 'var(--text-primary)',
                          outline: 'none',
                        }}
                      />
                    ) : (
                      <>
                        <BadgeIconButton
                          Icon={Eye}
                          onClick={() => handlePreview(item)}
                          title="Preview"
                          iconSize={ICON_SIZE.sm}
                        />
                        <BadgeIconButton
                          Icon={Edit2}
                          onClick={() => handleStartRename(item)}
                          title={t('common:rename')}
                          iconSize={ICON_SIZE.sm}
                        />
                        <BadgeIconButton
                          Icon={Download}
                          onClick={() => handleDownload(item)}
                          title={t('common:download')}
                          iconSize={ICON_SIZE.sm}
                        />
                      </>
                    )}
                    <DeleteButton
                      iconOnly
                      onClick={() => handleDelete(item)}
                      title={t('common:delete')}
                    />
                  </>
                )}
              </div>
            );            })
           )}
        </div>
        {/* 拖拽上传覆盖层 */}
        <DragUploadOverlay dragState={dragState} borderRadius={16} />
        </motion.div>
      )}      {/* Preview overlay */}
      <AttachmentPreviewOverlay item={previewItem} onClose={() => setPreviewItem(null)} />
      {confirmDialog}
      {/* Confirmation dialogs */}
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
        body={t('common:perm_delete_body', { name: permDeleteItem ? truncateFileName(permDeleteItem.fileName) : '' })}
        confirmLabel={t('common:delete_permanently')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={() => permDeleteItem && handlePermanentDelete(permDeleteItem)}
        onCancel={() => setPermDeleteItem(null)}
      />
    </div>
  );
}
