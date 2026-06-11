import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Paperclip, X, Trash2, RotateCw, Eye, Image, FileText, Edit2 } from 'lucide-react';

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
};

export interface AttachmentViewerProps {
  objectId: string;
  onClose: () => void;
  onCountChange?: () => void;
  zIndex?: number;
}

export function AttachmentViewer({ objectId, onClose, onCountChange, zIndex = 2000 }: AttachmentViewerProps) {
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

  const openWithDefault = async (path: string) => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(path);
    } catch {
      alert('Cannot open file. Make sure the file still exists at: ' + path);
    }
  };

  const handlePreview = async (item: AttachmentItem) => {
    const ext = item.fileName.split('.').pop()?.toLowerCase() || '';
    const isImage =
      item.mimeType.startsWith('image/') ||
      ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext);
    const isPdf = item.mimeType === 'application/pdf' || ext === 'pdf';
    const isText =
      item.mimeType.startsWith('text/') ||
      ['json', 'xml', 'csv', 'md', 'txt'].includes(ext);

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
    const { open } = await import('@tauri-apps/plugin-dialog');
    const filePath = await open({ multiple: false, title: 'Select file to attach' });
    if (filePath && typeof filePath === 'string') {
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file';
      const sizeBytes = await invoke<number>('fs_get_file_size', { path: filePath }).catch(() => 0);
      const ext = fileName.split('.').pop()?.toLowerCase() || '';
      const mimeMap: Record<string, string> = {
        jpg: 'image/jpeg',
        jpeg: 'image/jpeg',
        png: 'image/png',
        gif: 'image/gif',
        webp: 'image/webp',
        svg: 'image/svg+xml',
        pdf: 'application/pdf',
        txt: 'text/plain',
        md: 'text/markdown',
        json: 'application/json',
        xml: 'application/xml',
        csv: 'text/csv',
      };
      const id = crypto.randomUUID();
      const vaultPath = await invoke<string>('attachment_copy_to_vault', {
        srcPath: filePath,
        objectId,
        attachmentId: id,
        fileName,
      }).catch(() => filePath);
      await invoke('attachment_save', {
        objectId,
        meta: {
          id,
          objectId,
          fileName,
          mimeType: mimeMap[ext] || 'application/octet-stream',
          sizeBytes,
          createdAt: new Date().toISOString(),
          srcPath: filePath,
          vaultPath,
        },
      });
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
        prev.map((i) => (i.id === renamingId ? { ...i, fileName: renameValue.trim() } : i))
      );
      invoke('attachment_rename', {
        objectId,
        attachmentId: renamingId,
        newName: renameValue.trim(),
      }).catch(() => {});
    }
    setRenamingId(null);
  };

  const handleDelete = async (item: AttachmentItem) => {
    if (!confirm(item.fileName)) return;
    await invoke('attachment_soft_delete', { objectId, attachmentId: item.id }).catch((e) =>
      alert('Delete failed: ' + e)
    );
    await loadAttachments();
    onCountChange?.();
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
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 500,
          maxHeight: '80vh',
          display: 'flex',
          flexDirection: 'column',
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
          border: '1px solid var(--border-subtle)',
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
            <div style={{ fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Paperclip size={14} /> {t('common:attachments')}
            </div>
            <div style={{ display: 'flex', gap: 4 }}>
              <button
                onClick={() => setShowTrash(false)}
                style={{
                  padding: '3px 8px',
                  borderRadius: 6,
                  fontSize: 11,
                  border: '1px solid var(--border-subtle)',
                  background: !showTrash ? 'var(--accent-primary)' : 'transparent',
                  color: !showTrash ? 'white' : 'var(--text-secondary)',
                  cursor: 'pointer',
                }}
              >
                {t('common:attachments_active', { n: items.length })}
              </button>
              <button
                onClick={() => setShowTrash(true)}
                style={{
                  padding: '3px 8px',
                  borderRadius: 6,
                  fontSize: 11,
                  border: '1px solid var(--border-subtle)',
                  background: showTrash ? '#e74c3c' : 'transparent',
                  color: showTrash ? 'white' : 'var(--text-secondary)',
                  cursor: 'pointer',
                }}
              >
                {t('common:attachments_trash', { n: trashItems.length })}
              </button>
            </div>
          </div>
          {!showTrash && (
            <button onClick={handleAdd} style={{ ...pgBtn, fontSize: 11, fontWeight: 600, whiteSpace: 'nowrap' }}>
              + {t('common:create')}
            </button>
          )}
          <button onClick={onClose} style={pgBtn}>
            <X size={16} />
          </button>
        </div>
        {/* List */}
        <div style={{ flex: 1, overflow: 'auto', padding: 12 }}>
          {loading ? (
            <div style={{ textAlign: 'center', padding: 32, color: 'var(--text-tertiary)' }}>
              {t('common:loading')}
            </div>
          ) : displayItems.length === 0 ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)', fontSize: 14 }}>
              {showTrash ? t('common:attachments_trash_empty') : t('common:no_attachments')}
            </div>
          ) : (
            displayItems.map((item) => (
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
                    {item.fileName}
                  </div>
                  <div style={{ fontSize: 10, color: 'var(--text-tertiary)' }}>
                    {formatSize(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
                  </div>
                </div>
                {showTrash ? (
                  <>
                    <button onClick={() => handleRestore(item)} style={miniBtn} title={t('common:restore')}>
                      <RotateCw size={12} />
                    </button>
                    <button
                      onClick={() => setPermDeleteItem(item)}
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
                        <button onClick={() => handlePreview(item)} style={miniBtn} title="Preview">
                          <Eye size={12} />
                        </button>
                        <button onClick={() => handleStartRename(item)} style={miniBtn} title={t('common:rename')}>
                          <Edit2 size={12} />
                        </button>
                      </>
                    )}
                    <button
                      onClick={() => handleDelete(item)}
                      style={{ ...miniBtn, color: '#e74c3c' }}
                      title={t('common:delete')}
                    >
                      <Trash2 size={12} />
                    </button>
                  </>
                )}
              </div>
            ))
          )}
        </div>
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
              style={{ color: 'var(--text-secondary)', background: 'transparent', border: 'none', cursor: 'pointer' }}
            >
              <X size={18} />
            </button>
          </div>
          <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 24 }}>
            {previewUrl === 'error' ? (
              <div style={{ color: '#e74c3c', padding: 24 }}>Failed to load preview.</div>
            ) : previewUrl ? (
              <img
                src={previewUrl}
                alt={previewItem.fileName}
                style={{ maxWidth: '90%', maxHeight: '90%', objectFit: 'contain', borderRadius: 8 }}
              />
            ) : (
              <div style={{ color: 'var(--text-tertiary)', padding: 24 }}>Loading...</div>
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
              style={{ color: 'var(--text-secondary)', background: 'transparent', border: 'none', cursor: 'pointer' }}
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
                style={{ width: '100%', height: '100%', border: 'none', borderRadius: 8, background: 'white' }}
              />
            ) : (
              <div style={{ color: 'var(--text-tertiary)', padding: 24 }}>Loading...</div>
            )}
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
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:perm_delete_body', { name: permDeleteItem.fileName })}
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
