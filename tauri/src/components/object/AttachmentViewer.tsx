import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { Paperclip, X, Trash2, RotateCw, Eye, Image, FileText, Edit2, Scan } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';

export interface AttachmentItem {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
  deletedAt?: string | null;
  srcPath?: string | null;
  vaultPath?: string | null;
}

/** Truncate a file name preserving its extension: "abcdefg…-.pdf" instead of "abcdefg…" */
function truncateFileName(fileName: string, maxLen: number = 28): string {
  const dotIndex = fileName.lastIndexOf('.');
  if (dotIndex <= 0) {
    // No extension or hidden file like ".gitignore"
    if (fileName.length <= maxLen) return fileName;
    return fileName.slice(0, maxLen - 1) + '…';
  }
  const baseName = fileName.slice(0, dotIndex);
  const ext = fileName.slice(dotIndex); // includes the dot, e.g. ".pdf"
  if (fileName.length <= maxLen) return fileName;
  const available = maxLen - ext.length - 2; // 2 for "…-"
  if (available <= 1) return fileName.slice(0, maxLen - 1) + '…';
  return baseName.slice(0, available) + '…-' + ext;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const pgBtn: React.CSSProperties = {
  width: 30,
  height: 30,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  color: 'var(--text-secondary)',
  fontSize: 14,
  transition: 'background 0.15s, color 0.15s',
};

const miniBtn: React.CSSProperties = {
  width: 28,
  height: 28,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 6,
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 12,
  color: 'var(--text-secondary)',
  transition: 'background 0.15s, color 0.15s',
};

const btnHoverEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
  e.currentTarget.style.color = 'var(--accent-primary)';
};
const btnHoverLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
  e.currentTarget.style.background = 'transparent';
  e.currentTarget.style.color = 'var(--text-secondary)';
};
const btnDelEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
};
const btnDelLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
  e.currentTarget.style.background = 'transparent';
};

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
  const [previewUrl, setPreviewUrl] = useState<string>('');
  const { t } = useTranslation(['common', 'editor']);
  const navigate = useNavigate();
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const handleOcr = (item: AttachmentItem) => {
    const filePath = item.vaultPath || item.srcPath;
    if (!filePath) {
      showToast({
        type: 'error',
        message: t('common:ocr_no_path') || 'Cannot locate attachment file for OCR',
      });
      return;
    }
    navigate('/ocr', { state: { filePath } });
  };

  const openWithDefault = async (path: string) => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(path);
    } catch {
      showToast({
        type: 'error',
        message: t('common:cannot_open_file', { path }) ||
          `Cannot open file. Make sure the file still exists at: ${path}`,
      });
    }
  };

  const handlePreview = async (item: AttachmentItem) => {
    const ext = item.fileName.split('.').pop()?.toLowerCase() || '';
    const isImage =
      item.mimeType.startsWith('image/') ||
      ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext);
    const isPdf = item.mimeType === 'application/pdf' || ext === 'pdf';
    const isText =
      item.mimeType.startsWith('text/') || ['json', 'xml', 'csv', 'md', 'txt'].includes(ext);

    const filePath = item.vaultPath || item.srcPath;
    if (filePath && (isImage || isPdf || isText)) {
      setPreviewItem(item);
      setPreviewUrl('');
      try {
        const url = await invoke<string>('fs_read_file_as_data_url', { path: filePath });
        setPreviewUrl(url);
      } catch {
        setPreviewUrl('error');
      }
    } else if (filePath) {
      openWithDefault(filePath);
    } else {
      openWithDefault(item.fileName);
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
      }).catch(() => {});
    }
    setRenamingId(null);
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
      <div
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
                fontSize: 13,
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Paperclip size={14} /> {t('common:attachments')}
            </div>
            <div style={{ display: 'flex', gap: 4 }}>
              <button
                onClick={() => { setShowTrash(false); clearSelection(); }}
                onMouseEnter={!showTrash ? undefined : (e) => {
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 8%, transparent)';
                }}
                onMouseLeave={!showTrash ? undefined : (e) => {
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.background = 'transparent';
                }}
                style={{
                  padding: '3px 8px',
                  borderRadius: 6,
                  fontSize: 11,
                  border: '1px solid var(--border-subtle)',
                  background: !showTrash ? 'var(--accent-primary)' : 'transparent',
                  color: !showTrash ? 'white' : 'var(--text-secondary)',
                  cursor: 'pointer',
                  transition: 'background 0.15s, border-color 0.15s, color 0.15s',
                }}
              >
                {t('common:attachments_active', { n: items.length })}
              </button>
              <button
                onClick={() => { setShowTrash(true); clearSelection(); }}
                onMouseEnter={showTrash ? undefined : (e) => {
                  e.currentTarget.style.borderColor = '#e74c3c';
                  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 8%, transparent)';
                }}
                onMouseLeave={showTrash ? undefined : (e) => {
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.background = 'transparent';
                }}
                style={{
                  padding: '3px 8px',
                  borderRadius: 6,
                  fontSize: 11,
                  border: '1px solid var(--border-subtle)',
                  background: showTrash ? '#e74c3c' : 'transparent',
                  color: showTrash ? 'white' : 'var(--text-secondary)',
                  cursor: 'pointer',
                  transition: 'background 0.15s, border-color 0.15s, color 0.15s',
                }}
              >
                {t('common:attachments_trash', { n: trashItems.length })}
              </button>
            </div>
          </div>
          {!showTrash && (
            <button
              onClick={handleAdd}
              onMouseEnter={btnHoverEnter}
              onMouseLeave={btnHoverLeave}
              style={{ ...pgBtn, fontSize: 11, fontWeight: 600, whiteSpace: 'nowrap' }}
            >
              + {t('common:create')}
            </button>
          )}
          <button onClick={onClose} onMouseEnter={btnHoverEnter} onMouseLeave={btnHoverLeave} style={pgBtn}>
            <X size={16} />
          </button>
        </div>
        {/* 批量操作工具栏 */}
        {selectedIds.size > 0 && displayItems.length > 0 && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '8px 12px',
              borderBottom: '1px solid var(--border-subtle)',
              background: 'color-mix(in srgb, var(--accent-primary) 6%, transparent)',
              fontSize: 13,
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <SelectCheckbox
                checked={allSelected}
                onClick={() => handleSelectAll(allVisibleKeys)}
              />
              <span style={{ color: 'var(--text-secondary)' }}>
                {t('common:selected_count', { n: selectedIds.size })}
              </span>
            </div>
            {!showTrash ? (
              <button
                onClick={() => setBatchDeleteConfirm(true)}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = '#c0392b';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = '#e74c3c';
                }}
                style={{
                  padding: '4px 12px',
                  borderRadius: 6,
                  border: 'none',
                  background: '#e74c3c',
                  color: 'white',
                  fontSize: 12,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                }}
              >
                <Trash2 size={12} /> {t('common:delete')}
              </button>
            ) : (
              <div style={{ display: 'flex', gap: 6 }}>
                <button
                  onClick={() => setBatchPermanentDeleteConfirm(true)}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 20%, transparent)';
                    e.currentTarget.style.borderColor = '#e74c3c';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                  style={{
                    padding: '4px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'transparent',
                    color: '#e74c3c',
                    fontSize: 12,
                    fontWeight: 500,
                    cursor: 'pointer',
                    transition: 'all 0.15s ease',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                  }}
                >
                  <Trash2 size={12} /> {t('common:delete_permanently')}
                </button>
                <button
                  onClick={() => setBatchRestoreConfirm(true)}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 20%, transparent)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                  }}
                  style={{
                    padding: '4px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--accent-primary)',
                    background: 'transparent',
                    color: 'var(--accent-primary)',
                    fontSize: 12,
                    fontWeight: 500,
                    cursor: 'pointer',
                    transition: 'all 0.15s ease',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                  }}
                >
                  <RotateCw size={12} /> {t('common:restore')}
                </button>
              </div>
            )}
          </div>
        )}
        {/* List */}
        <div style={{ flex: 1, overflow: 'auto', padding: 12 }}>
          {loading ? (
            <LoadingPlaceholder variant="elevated" minHeight={120} />
          ) : displayItems.length === 0 ? (
            <div
              style={{
                textAlign: 'center',
                padding: 48,
                color: 'var(--text-secondary)',
                fontSize: 14,
              }}
            >
              {showTrash ? t('common:attachments_trash_empty') : t('common:no_attachments')}
            </div>
          ) : (
            displayItems.map((item) => {
              const compositeKey = `${objectId}::${item.id}`;
              const checked = selectedIds.has(compositeKey);
              return (
              <div
                key={item.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 6px',
                  borderBottom: '1px solid var(--border-subtle)',
                  fontSize: 13,
                }}
              >
                <SelectCheckbox
                  checked={checked}
                  onClick={(e) => { e.stopPropagation(); toggleSelect(compositeKey); }}
                />
                {showTrash ? (
                  <Trash2 size={14} style={{ color: '#e74c3c', flexShrink: 0 }} />
                ) : item.mimeType.startsWith('image/') ? (
                  <Image size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                ) : item.mimeType === 'application/pdf' ? (
                  <FileText size={14} style={{ color: '#e74c3c', flexShrink: 0 }} />
                ) : (
                  <Paperclip size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
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
                  <div style={{ fontSize: 10, color: 'var(--text-tertiary)' }}>
                    {formatSize(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
                  </div>
                </div>
                {showTrash ? (
                  <>
                    <button
                      onClick={() => handleRestore(item)}
                      onMouseEnter={btnHoverEnter}
                      onMouseLeave={btnHoverLeave}
                      style={miniBtn}
                      title={t('common:restore')}
                    >
                      <RotateCw size={12} />
                    </button>
                    <button
                      onClick={() => setPermDeleteItem(item)}
                      onMouseEnter={btnDelEnter}
                      onMouseLeave={btnDelLeave}
                      style={{ ...miniBtn, color: '#e74c3c' }}
                      title={t('common:delete_permanently')}
                    >
                      <Trash2 size={12} />
                    </button>
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
                          fontSize: 12,
                          borderRadius: 4,
                          border: '1px solid var(--accent-primary)',
                          background: 'transparent',
                          color: 'var(--text-primary)',
                          outline: 'none',
                        }}
                      />
                    ) : (
                      <>
                        <button
                          onClick={() => handlePreview(item)}
                          onMouseEnter={btnHoverEnter}
                          onMouseLeave={btnHoverLeave}
                          style={miniBtn}
                          title="Preview"
                        >
                          <Eye size={12} />
                        </button>
                        {item.mimeType.startsWith('image/') && (
                          <button
                            onClick={() => handleOcr(item)}
                            onMouseEnter={btnHoverEnter}
                            onMouseLeave={btnHoverLeave}
                            style={miniBtn}
                            title={t('common:ocr') || 'OCR'}
                          >
                            <Scan size={12} />
                          </button>
                        )}
                        <button
                          onClick={() => handleStartRename(item)}
                          onMouseEnter={btnHoverEnter}
                          onMouseLeave={btnHoverLeave}
                          style={miniBtn}
                          title={t('common:rename')}
                        >
                          <Edit2 size={12} />
                        </button>
                      </>
                    )}
                    <button
                      onClick={() => handleDelete(item)}
                      onMouseEnter={btnDelEnter}
                      onMouseLeave={btnDelLeave}
                      style={{ ...miniBtn, color: '#e74c3c' }}
                      title={t('common:delete')}
                    >
                      <Trash2 size={12} />
                    </button>
                  </>
                )}
              </div>
            );            })
           )}
        </div>
        {/* 拖拽上传覆盖层 */}
        <DragUploadOverlay dragState={dragState} borderRadius={16} />
      </div>
      {/* Preview: image */}
      {previewItem && previewItem.mimeType.startsWith('image/') && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            flexDirection: 'column',
            background: 'rgba(0,0,0,0.8)',
            backdropFilter: 'blur(12px)',
          }}
          onClick={() => setPreviewItem(null)}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '10px 18px',
              background: 'var(--bg-toolbar)',
            }}
          >
            <span style={{ fontSize: 13, fontWeight: 500 }}>{previewItem.fileName}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setPreviewItem(null);
              }}
              style={{
                color: 'var(--text-secondary)',
                background: 'transparent',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              <X size={18} />
            </button>
          </div>
          <div
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: 24,
            }}
          >
            {previewUrl === 'error' ? (
              <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
            ) : previewUrl ? (
              <img
                src={previewUrl}
                alt={previewItem.fileName}
                style={{ maxWidth: '90%', maxHeight: '90%', objectFit: 'contain', borderRadius: 8 }}
              />
            ) : (
              <LoadingPlaceholder variant="toolbar" minHeight={120} />
            )}
          </div>
        </div>
      )}
      {/* Preview: PDF */}
      {previewItem && previewItem.mimeType === 'application/pdf' && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            flexDirection: 'column',
            background: 'rgba(0,0,0,0.8)',
            backdropFilter: 'blur(12px)',
          }}
          onClick={() => setPreviewItem(null)}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '10px 18px',
              background: 'var(--bg-toolbar)',
            }}
          >
            <span style={{ fontSize: 13, fontWeight: 500 }}>{previewItem.fileName}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setPreviewItem(null);
              }}
              style={{
                color: 'var(--text-secondary)',
                background: 'transparent',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              <X size={18} />
            </button>
          </div>
          <div style={{ flex: 1, padding: 24 }}>
            {previewUrl === 'error' ? (
              <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
            ) : previewUrl ? (
              <iframe
                src={previewUrl}
                style={{
                  width: '100%',
                  height: '100%',
                  border: 'none',
                  borderRadius: 8,
                  background: 'white',
                }}
              />
            ) : (
              <LoadingPlaceholder variant="toolbar" minHeight={120} />
            )}
          </div>
        </div>
      )}
      {confirmDialog}
      {/* Batch delete confirmation */}
      {batchDeleteConfirm && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.4)',
          }}
          onClick={() => setBatchDeleteConfirm(false)}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('common:batch_delete_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:batch_delete_body', { n: selectedIds.size })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button
                onClick={() => setBatchDeleteConfirm(false)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  cursor: 'pointer',
                  fontSize: 13,
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:cancel')}
              </button>
              <button
                onClick={handleBatchDelete}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: '#e74c3c',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = '#c0392b'; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = '#e74c3c'; }}
              >
                {t('common:delete')}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Batch restore confirmation */}
      {batchRestoreConfirm && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.4)',
          }}
          onClick={() => setBatchRestoreConfirm(false)}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('common:batch_restore_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:batch_restore_body', { n: selectedIds.size })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button
                onClick={() => setBatchRestoreConfirm(false)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  cursor: 'pointer',
                  fontSize: 13,
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:cancel')}
              </button>
              <button
                onClick={handleBatchRestore}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--accent-primary)',
                  background: 'var(--accent-primary)',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.opacity = '0.85'; }}
                onMouseLeave={(e) => { e.currentTarget.style.opacity = '1'; }}
              >
                {t('common:restore')}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Batch permanent delete confirmation */}
      {batchPermanentDeleteConfirm && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.4)',
          }}
          onClick={() => setBatchPermanentDeleteConfirm(false)}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('common:batch_perm_delete_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:batch_perm_delete_body', { n: selectedIds.size })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button
                onClick={() => setBatchPermanentDeleteConfirm(false)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  cursor: 'pointer',
                  fontSize: 13,
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:cancel')}
              </button>
              <button
                onClick={handleBatchPermanentDelete}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: '#e74c3c',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = '#c0392b'; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = '#e74c3c'; }}
              >
                {t('common:delete_permanently')}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Permanent delete confirmation */}
      {permDeleteItem && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.4)',
          }}
          onClick={() => setPermDeleteItem(null)}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: '24px 28px',
              maxWidth: 360,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
              {t('common:perm_delete_title')}
            </h3>
            <p
              style={{
                margin: '0 0 20px',
                fontSize: 14,
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:perm_delete_body', { name: truncateFileName(permDeleteItem.fileName) })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button
                onClick={() => setPermDeleteItem(null)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  cursor: 'pointer',
                  fontSize: 13,
                  color: 'var(--text-primary)',
                }}
              >
                {t('common:cancel')}
              </button>
              <button
                onClick={() => handlePermanentDelete(permDeleteItem)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: '#e74c3c',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                }}
              >
                {t('common:delete_permanently')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
