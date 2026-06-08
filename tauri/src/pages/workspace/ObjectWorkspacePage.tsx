import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useSensitivityStore, SensitivityLevel } from '@/stores/sensitivityStore';
import { useRevealState } from '@/hooks/useRevealState';
import { Pencil, Trash2, Trash, Clock, ChevronLeft, ChevronRight, X, Paperclip, Edit2, RotateCw, Eye, EyeOff, Lock, Image, FileText, Copy, Check } from 'lucide-react';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';

// Labels resolved at render time via t() so they support i18n
const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
};

/** Extract displayable key-value pairs from object properties (filters internal __ fields). */
function flattenProperties(
  props: Record<string, unknown> | undefined
): { key: string; value: string }[] {
  if (!props) return [];
  const result: { key: string; value: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue; // skip internal fields like __attachments
    if (v === null || v === undefined || v === '') continue;
    if (typeof v === 'string') {
      result.push({ key: k, value: v });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      result.push({ key: k, value: String(v) });
    }
  }
  return result;
}

// =============================================================================
// HistoryViewer — flip-book style snapshot browser (§25.5)
// =============================================================================

interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

function HistoryViewer({ objectId, onClose }: { objectId: string; onClose: () => void }) {
  const [snapshots, setSnapshots] = useState<SnapshotEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentIdx, setCurrentIdx] = useState(0);
  const [animDir, setAnimDir] = useState<'left' | 'right' | null>(null);
  const { t } = useTranslation(['common', 'editor']);

  useEffect(() => {
    invoke<SnapshotEntry[]>('snapshot_get', { objectId })
      .then(setSnapshots)
      .finally(() => setLoading(false));
  }, [objectId]);

  const goPrev = () => {
    if (currentIdx < snapshots.length - 1) { setAnimDir('right'); setTimeout(() => { setCurrentIdx(i => i + 1); setAnimDir(null); }, 150); }
  };
  const goNext = () => {
    if (currentIdx > 0) { setAnimDir('left'); setTimeout(() => { setCurrentIdx(i => i - 1); setAnimDir(null); }, 150); }
  };

  const snap = snapshots[currentIdx];
  const total = snapshots.length;
  const isOldest = currentIdx >= total - 1;
  const isLatest = currentIdx <= 0;

  return (
    <div
      style={{
        position: 'fixed', inset: 0, zIndex: 2000, display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)', backdropFilter: 'blur(6px)',
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          position: 'relative', width: 460, maxHeight: '80vh', display: 'flex', flexDirection: 'column',
          background: 'var(--bg-elevated)', borderRadius: 16, boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
          border: '1px solid var(--border-subtle)',
          transform: animDir === 'left' ? 'perspective(1200px) rotateY(-8deg)' :
                    animDir === 'right' ? 'perspective(1200px) rotateY(8deg)' : 'perspective(1200px) rotateY(0)',
          transition: 'transform 0.15s ease',
          transformOrigin: animDir === 'left' ? 'left center' : animDir === 'right' ? 'right center' : 'center',
        }}
      >
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 18px', borderBottom: '1px solid var(--border-subtle)' }}>
          <div style={{ fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
            <Clock size={14} /> {t('common:history')}
            <span style={{ fontSize: 11, color: 'var(--text-tertiary)', fontWeight: 400 }}>{loading ? '' : `${currentIdx + 1} / ${total}`}</span>
          </div>
          <div style={{ display: 'flex', gap: 6 }}>
            <button onClick={goPrev} disabled={isOldest || loading} style={{ ...pgBtn, opacity: isOldest || loading ? 0.3 : 1 }}><ChevronLeft size={16} /></button>
            <button onClick={goNext} disabled={isLatest || loading} style={{ ...pgBtn, opacity: isLatest || loading ? 0.3 : 1 }}><ChevronRight size={16} /></button>
            <button onClick={onClose} style={{ ...pgBtn, marginLeft: 4 }}><X size={16} /></button>
          </div>
        </div>
        {/* Content */}
        <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
          {loading ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-tertiary)' }}>{t('common:loading')}</div>
          ) : !snap ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)', fontSize: 14 }}>{t('common:no_history')}</div>
          ) : (
            <SnapshotCard snap={snap} index={currentIdx} total={total} t={t} />
          )}
        </div>
        {/* Footer */}
        <div style={{ padding: '10px 18px', borderTop: '1px solid var(--border-subtle)', fontSize: 11, color: 'var(--text-tertiary)', textAlign: 'center' }}>
          {snap && `${t('common:version')} #${total - currentIdx} · ${new Date(snap.timestamp).toLocaleString()} · ${t(`common:trigger_${snap.triggeredBy}`)}`}
        </div>
      </div>
    </div>
  );
}

function SnapshotCard({ snap, index, total, t }: { snap: SnapshotEntry; index: number; total: number; t: (k: string) => string }) {
  const [snapData, setSnapData] = useState<Record<string, unknown> | null>(null);

  useEffect(() => {
    invoke<Record<string, unknown> | null>('snapshot_get_data', { snapshotId: snap.id })
      .then(setSnapData);
  }, [snap.id]);

  const rawProps = snapData && typeof snapData === 'object' && 'properties' in snapData
    ? (snapData.properties as Record<string, unknown> | undefined)
    : undefined;
  const fields = flattenProperties(rawProps);
  const snapName = snapData && typeof snapData === 'object' && 'name' in snapData ? String(snapData.name) : '';
  const tags: string[] = snapData && typeof snapData === 'object' && 'tags' in snapData && Array.isArray(snapData.tags)
    ? snapData.tags as string[] : [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {/* Version badge */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          {index <= 1 && (
            <span style={{ padding: '3px 8px', borderRadius: 6, fontSize: 10, fontWeight: 600, background: index === 0 ? 'rgba(39,174,96,0.12)' : 'rgba(91,124,153,0.08)', color: index === 0 ? '#27ae60' : 'var(--accent-primary)' }}>
              {index === 0 ? t('common:current_version') : t('common:previous_version')}
            </span>
          )}
        </div>
        <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
          {snapName}
        </div>
      </div>
      {/* Properties */}
      {fields.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginTop: 4 }}>
          {fields.map((f) => (
            <div key={f.key} style={{ display: 'flex', gap: 8, fontSize: 12, padding: '4px 0', borderBottom: '1px solid var(--border-subtle)' }}>
              <span style={{ fontWeight: 500, color: 'var(--text-secondary)', minWidth: 90 }}>{t(`editor:fields.${f.key}`)}</span>
              <span style={{ color: 'var(--text-primary)' }}>{f.value}</span>
            </div>
          ))}
        </div>
      )}
      {/* Tags */}
      {tags.length > 0 && (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
          {tags.map((tag) => (
            <span key={tag} style={{ padding: '1px 7px', borderRadius: 10, fontSize: 10, background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)', fontWeight: 500 }}>
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

const pgBtn: React.CSSProperties = {
  width: 30, height: 30, display: 'flex', alignItems: 'center', justifyContent: 'center',
  border: 'none', borderRadius: 6, background: 'transparent', cursor: 'pointer',
  color: 'var(--text-secondary)', fontSize: 14,
};

// =============================================================================
// AttachmentViewer — attachment list for an object (§25.6)
// =============================================================================

interface AttachmentItem {
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

function AttachmentViewer({ objectId, onClose, onCountChange }: { objectId: string; onClose: () => void; onCountChange: () => void }) {
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
    const isImage = item.mimeType.startsWith('image/') || ['png','jpg','jpeg','gif','webp','svg'].includes(ext);
    const isPdf = item.mimeType === 'application/pdf' || ext === 'pdf';
    const isText = item.mimeType.startsWith('text/') || ['json','xml','csv','md','txt'].includes(ext);

    const filePath = item.vaultPath || item.srcPath; // vault copy preferred, original as fallback
    if (filePath && (isImage || isPdf || isText)) {
      // Built-in preview via data URL
      setPreviewItem(item);
      setPreviewUrl('');
      try {
        const url = await invoke<string>('fs_read_file_as_data_url', { path: filePath });
        setPreviewUrl(url);
      } catch {
        setPreviewUrl('error');
      }
    } else if (filePath) {
      // Office docs, etc. — open with system default
      openWithDefault(filePath);
    } else {
      // No path — likely an old attachment; try to open by name as fallback
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
    } catch { setItems([]); setTrashItems([]); }
    finally { setLoading(false); }
  }, [objectId]);

  useEffect(() => { loadAttachments(); }, [loadAttachments]);

  const handleAdd = async () => {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const filePath = await open({ multiple: false, title: 'Select file to attach' });
    if (filePath && typeof filePath === 'string') {
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file';
      const sizeBytes = await invoke<number>('fs_get_file_size', { path: filePath }).catch(() => 0);
      const ext = fileName.split('.').pop()?.toLowerCase() || '';
      const mimeMap: Record<string, string> = { jpg:'image/jpeg', jpeg:'image/jpeg', png:'image/png', gif:'image/gif', webp:'image/webp', svg:'image/svg+xml', pdf:'application/pdf', txt:'text/plain', md:'text/markdown', json:'application/json', xml:'application/xml', csv:'text/csv' };
      const id = crypto.randomUUID();
      // Copy file into vault storage so it survives original deletion
      const vaultPath = await invoke<string>('attachment_copy_to_vault', { srcPath: filePath, objectId, attachmentId: id, fileName }).catch(() => filePath);
      await invoke('attachment_save', {
        objectId, meta: {
          id, objectId,
          fileName, mimeType: mimeMap[ext] || 'application/octet-stream',
          sizeBytes, createdAt: new Date().toISOString(), srcPath: filePath, vaultPath,
        },
      });
      await loadAttachments();
      onCountChange();
    }
  };

  const handleStartRename = (item: AttachmentItem) => {
    setRenamingId(item.id);
    setRenameValue(item.fileName);
    setTimeout(() => renameInputRef.current?.focus(), 50);
  };

  const handleConfirmRename = async () => {
    if (renamingId && renameValue.trim()) {
      // Update locally first, avoid flash from full reload
      setItems((prev) => prev.map((i) => i.id === renamingId ? { ...i, fileName: renameValue.trim() } : i));
      invoke('attachment_rename', { objectId, attachmentId: renamingId, newName: renameValue.trim() }).catch(() => {});
    }
    setRenamingId(null);
  };

  const handleDelete = async (item: AttachmentItem) => {
    if (!confirm(item.fileName)) return;
    await invoke('attachment_soft_delete', { objectId, attachmentId: item.id }).catch((e) => alert('Delete failed: ' + e));
    await loadAttachments();
      onCountChange();
  };

  const handleRestore = async (item: AttachmentItem) => {
    await invoke('attachment_restore', { objectId, attachmentId: item.id });
    await loadAttachments();
      onCountChange();
  };

  const handlePermanentDelete = async (item: AttachmentItem) => {
    setPermDeleteItem(null);
    await invoke('attachment_delete', { objectId, attachmentId: item.id });
    await loadAttachments();
      onCountChange();
  };

  const displayItems = showTrash ? trashItems : items;

  return (
    <div
      style={{
        position: 'fixed', inset: 0, zIndex: 2000, display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)', backdropFilter: 'blur(6px)',
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 500, maxHeight: '80vh', display: 'flex', flexDirection: 'column',
          background: 'var(--bg-elevated)', borderRadius: 16, boxShadow: '0 24px 80px rgba(0,0,0,0.25)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 18px', borderBottom: '1px solid var(--border-subtle)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Paperclip size={14} /> {t('common:attachments')}
            </div>
            <div style={{ display: 'flex', gap: 4 }}>
              <button onClick={() => setShowTrash(false)} style={{ padding: '3px 8px', borderRadius: 6, fontSize: 11, border: '1px solid var(--border-subtle)', background: !showTrash ? 'var(--accent-primary)' : 'transparent', color: !showTrash ? 'white' : 'var(--text-secondary)', cursor: 'pointer' }}>{t('common:attachments_active', { n: items.length })}</button>
              <button onClick={() => setShowTrash(true)} style={{ padding: '3px 8px', borderRadius: 6, fontSize: 11, border: '1px solid var(--border-subtle)', background: showTrash ? '#e74c3c' : 'transparent', color: showTrash ? 'white' : 'var(--text-secondary)', cursor: 'pointer' }}>{t('common:attachments_trash', { n: trashItems.length })}</button>
            </div>
          </div>
          {!showTrash && <button onClick={handleAdd} style={{ ...pgBtn, fontSize: 11, fontWeight: 600, whiteSpace: 'nowrap' }}>+ {t('common:create')}</button>}
          <button onClick={onClose} style={pgBtn}><X size={16} /></button>
        </div>
        {/* List */}
        <div style={{ flex: 1, overflow: 'auto', padding: 12 }}>
          {loading ? (
            <div style={{ textAlign: 'center', padding: 32, color: 'var(--text-tertiary)' }}>{t('common:loading')}</div>
          ) : displayItems.length === 0 ? (
            <div style={{ textAlign: 'center', padding: 48, color: 'var(--text-secondary)', fontSize: 14 }}>
              {showTrash ? t('common:attachments_trash_empty') : t('common:no_attachments')}
            </div>
          ) : displayItems.map((item) => (
            <div key={item.id} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 6px', borderBottom: '1px solid var(--border-subtle)', fontSize: 13 }}>
              {showTrash ? <Trash2 size={14} style={{ color: '#e74c3c', flexShrink: 0 }} /> : (
                item.mimeType.startsWith('image/') ? <Image size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} /> :
                item.mimeType === 'application/pdf' ? <FileText size={14} style={{ color: '#e74c3c', flexShrink: 0 }} /> :
                <Paperclip size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
              )}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', textDecoration: showTrash ? 'line-through' : 'none', opacity: showTrash ? 0.5 : 1 }}>{item.fileName}</div>
                <div style={{ fontSize: 10, color: 'var(--text-tertiary)' }}>
                  {formatSize(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
                </div>
              </div>
              {showTrash ? (
                <>
                  <button onClick={() => handleRestore(item)} style={miniBtn} title={t('common:restore')}><RotateCw size={12} /></button>
                  <button onClick={() => setPermDeleteItem(item)} style={{ ...miniBtn, color: '#e74c3c' }} title={t('common:delete_permanently')}><Trash2 size={12} /></button>
                </>
              ) : (
                <>
                  {renamingId === item.id ? (
                    <input ref={renameInputRef} value={renameValue} onChange={(e) => setRenameValue(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') handleConfirmRename(); if (e.key === 'Escape') setRenamingId(null); }} onBlur={handleConfirmRename} style={{ width: 100, padding: '3px 6px', fontSize: 12, borderRadius: 4, border: '1px solid var(--accent-primary)', background: 'transparent', color: 'var(--text-primary)', outline: 'none' }} />
                  ) : (
                    <>
                    <button onClick={() => handlePreview(item)} style={miniBtn} title="Preview"><Eye size={12} /></button>
                    <button onClick={() => handleStartRename(item)} style={miniBtn} title={t('common:rename')}><Edit2 size={12} /></button>
                    </>
                  )}
                  <button onClick={() => handleDelete(item)} style={{ ...miniBtn, color: '#e74c3c' }} title={t('common:delete')}><Trash2 size={12} /></button>
                </>
              )}
            </div>
          ))}
        </div>
      </div>
      {/* Preview: image */}
      {previewItem && previewItem.mimeType.startsWith('image/') && (
        <div style={{ position:'fixed', inset:0, zIndex:9999, display:'flex', flexDirection:'column', background:'rgba(0,0,0,0.8)', backdropFilter:'blur(12px)' }} onClick={() => setPreviewItem(null)}>
          <div style={{ display:'flex', alignItems:'center', justifyContent:'space-between', padding:'10px 18px', background:'var(--bg-toolbar)' }}>
            <span style={{ fontSize:13, fontWeight:500 }}>{previewItem.fileName}</span>
            <button onClick={(e) => { e.stopPropagation(); setPreviewItem(null); }} style={{ color:'var(--text-secondary)', background:'transparent', border:'none', cursor:'pointer' }}><X size={18} /></button>
          </div>
          <div style={{ flex:1, display:'flex', alignItems:'center', justifyContent:'center', padding:24 }}>
            {previewUrl === 'error' ? <div style={{ color:'#e74c3c', padding:24 }}>Failed to load preview.</div> : previewUrl ? <img src={previewUrl} alt={previewItem.fileName} style={{ maxWidth:'90%', maxHeight:'90%', objectFit:'contain', borderRadius:8 }} /> : <div style={{ color:'var(--text-tertiary)', padding:24 }}>Loading...</div>}
          </div>
        </div>
      )}
      {/* Preview: PDF */}
      {previewItem && previewItem.mimeType === 'application/pdf' && (
        <div style={{ position:'fixed', inset:0, zIndex:9999, display:'flex', flexDirection:'column', background:'rgba(0,0,0,0.8)', backdropFilter:'blur(12px)' }} onClick={() => setPreviewItem(null)}>
          <div style={{ display:'flex', alignItems:'center', justifyContent:'space-between', padding:'10px 18px', background:'var(--bg-toolbar)' }}>
            <span style={{ fontSize:13, fontWeight:500 }}>{previewItem.fileName}</span>
            <button onClick={(e) => { e.stopPropagation(); setPreviewItem(null); }} style={{ color:'var(--text-secondary)', background:'transparent', border:'none', cursor:'pointer' }}><X size={18} /></button>
          </div>
          <div style={{ flex:1, padding:24 }}>
            {previewUrl === 'error' ? <div style={{ color:'#e74c3c', padding:24 }}>Failed to load preview.</div> : previewUrl ? <iframe src={previewUrl} style={{ width:'100%', height:'100%', border:'none', borderRadius:8, background:'white' }} /> : <div style={{ color:'var(--text-tertiary)', padding:24 }}>Loading...</div>}
          </div>
        </div>
      )}
      {/* Permanent delete confirmation */}
      {permDeleteItem && (
        <div style={{ position: 'fixed', inset: 0, zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.4)' }} onClick={() => setPermDeleteItem(null)}>
          <div style={{ background: 'var(--bg-elevated)', borderRadius: 12, padding: '24px 28px', maxWidth: 360, width: '90%', boxShadow: 'var(--shadow-lg)', border: '1px solid var(--border-subtle)' }} onClick={(e) => e.stopPropagation()}>
            <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>{t('common:perm_delete_title')}</h3>
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:perm_delete_body', { name: permDeleteItem.fileName })}
            </p>
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button onClick={() => setPermDeleteItem(null)} style={{ padding: '8px 16px', borderRadius: 8, border: '1px solid var(--border-subtle)', background: 'var(--bg-elevated)', cursor: 'pointer', fontSize: 13, color: 'var(--text-primary)' }}>{t('common:cancel')}</button>
              <button onClick={() => handlePermanentDelete(permDeleteItem)} style={{ padding: '8px 16px', borderRadius: 8, border: 'none', background: '#e74c3c', color: 'white', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('common:delete_permanently')}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

const miniBtn: React.CSSProperties = {
  width: 28, height: 28, display: 'flex', alignItems: 'center', justifyContent: 'center',
  border: 'none', borderRadius: 6, background: 'transparent', cursor: 'pointer',
  fontSize: 12, color: 'var(--text-secondary)',
};

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const [searchQuery, setSearchQuery] = useState('');
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [confirmPageDelete, setConfirmPageDelete] = useState(false);
  const [historyObjId, setHistoryObjId] = useState<string | null>(null);
  const [snapshotCounts, setSnapshotCounts] = useState<Record<string, number>>({});
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);
  const [attachmentCounts, setAttachmentCounts] = useState<Record<string, number>>({});
  const [detailObj, setDetailObj] = useState<typeof visibleObjects[number] | null>(null);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const { objects, loadObjects, deleteObject, isLoading, error } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const removeCustomPage = useSettingsStore((s) => s.removeCustomPage);
  const { map: sensitivityMap, loadMap } = useSensitivityStore();
  const { maskValue, isRevealed, reveal } = useRevealState();
  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const activeCategoryLabel = sectionFilter ? t(`navigation:${sectionFilter}`, sectionFilter) : null;

  // Load sensitivity map for field-level masking
  useEffect(() => { loadMap(); }, []);

  /** Password verification for critical field reveal. Uses inline prompt dialog. */
  const passwordVerify = useCallback(async (): Promise<boolean> => {
    // Prompt user for password via built-in prompt() as fallback
    // In production, replace with PasswordVerificationDialog
    const pw = prompt(t('common:current_password'));
    if (!pw) return false;
    try {
      await invoke('login', { accountId: accountId, password: pw });
      return true;
    } catch {
      alert(t('common:current_password_incorrect'));
      return false;
    }
  }, [accountId, t]);

  /** Resolve sensitivity level for a property key within an object's collection. */
  const getFieldSensitivity = (collectionType: string, fieldKey: string): SensitivityLevel => {
    const fieldId = `${collectionType}.${fieldKey}`;
    if (sensitivityMap?.entries?.[fieldId]) return sensitivityMap.entries[fieldId];
    // Also try snake_case normalized version (map entries use snake_case, fields may be camelCase)
    const snakeKey = fieldKey.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase());
    const snakeFieldId = `${collectionType}.${snakeKey}`;
    if (snakeFieldId !== fieldId && sensitivityMap?.entries?.[snakeFieldId]) return sensitivityMap.entries[snakeFieldId];
    // Fallback: match any entry ending with .{fieldKey} (regardless of case)
    for (const [id, level] of Object.entries(sensitivityMap?.entries || {})) {
      if (id.endsWith(`.${fieldKey}`)) return level;
      if (id.endsWith(`.${snakeKey}`)) return level;
    }
    return 'public'; // default level: public (only sensitive fields are explicitly protected)
  };

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId]);

  const visibleObjects = objects.filter(
    (obj) =>
      obj.collectionType !== 'page' &&
      obj.collectionType !== 'unknown' &&
      obj.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Load snapshot counts for visible objects
  useEffect(() => {
    const ids = visibleObjects.map(o => o.id);
    if (ids.length === 0) return;
    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then(setSnapshotCounts)
      .catch(() => {});
  }, [visibleObjects.length]);

  // Load attachment counts for visible objects
  const refreshAttachmentCounts = useCallback(() => {
    const ids = visibleObjects.map(o => o.id);
    if (ids.length === 0) return;
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: ids })
      .then(setAttachmentCounts)
      .catch(() => {});
  }, [visibleObjects.length]);

  useEffect(() => { refreshAttachmentCounts(); }, [refreshAttachmentCounts]);


  const newObjectUrl = pageId
    ? `/editor?parentId=${pageId}`
    : `/editor${sectionFilter ? `?section=${sectionFilter}` : ''}`;

  const handleDelete = async (objectId: string) => {
    setConfirmDelete(null);
    setDeletingId(objectId);
    try {
      await deleteObject(objectId);
    } finally {
      setDeletingId(null);
    }
  };

  return (
    <AppShell
      title={customPage?.name || activeCategoryLabel || t('objects')}
      actions={
        <div style={{ display: 'flex', gap: 8 }}>
          <button
            onClick={() => navigate(newObjectUrl)}
            style={{
              padding: '8px 16px', borderRadius: 8, border: 'none',
              background: 'var(--accent-primary)', color: 'white',
              fontSize: 13, fontWeight: 500, cursor: 'pointer',
            }}
          >
            + {t('create')}
          </button>
          {pageId && customPage && (
            <button
              onClick={() => setConfirmPageDelete(true)}
              title={t('delete')}
              style={{
                padding: '8px 12px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: 'transparent', color: '#e74c3c', cursor: 'pointer',
                fontSize: 13, display: 'flex', alignItems: 'center', gap: 4,
              }}
            >
              <Trash size={14} /> {t('delete')} {customPage?.name || t('objects')}
            </button>
          )}
        </div>
      }
    >
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }} onMouseDown={(e) => { if (e.detail > 1) e.preventDefault(); }}>
        {!pageId && (
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            {CATEGORY_TYPES.map((catType) => (
              <button
                key={catType}
                onClick={() => navigate(`/workspace?section=${catType}`)}
                style={{
                  padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                  background: sectionFilter === catType ? 'var(--accent-primary)' : 'transparent',
                  color: sectionFilter === catType ? 'white' : 'var(--text-primary)',
                  fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
                }}
              >
                <span>{React.createElement(CATEGORY_ICONS[catType], { size: 16 })}</span>
                {t(`navigation:${catType}`, catType)}
              </button>
            ))}
            {sectionFilter && (
              <button
                onClick={() => navigate('/workspace')}
                style={{
                  padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                  background: 'transparent', color: 'var(--text-tertiary)',
                  fontSize: 13, cursor: 'pointer',
                }}
              >
                {t('clear')}
              </button>
            )}
          </div>
        )}

        <Input
          placeholder={t('search_objects_placeholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        {isLoading && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
              {t('loading')}
            </p>
          </Card>
        )}
        {!isLoading && error && (
          <Card>
            <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>{error}</p>
          </Card>
        )}
        {!isLoading && !error && visibleObjects.length === 0 && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: 14 }}>
              {searchQuery ? t('no_matching_objects') : t('no_objects')}
            </p>
          </Card>
        )}
        {!isLoading &&
          visibleObjects.map((obj) => {
            const fields = flattenProperties(obj.properties as Record<string, unknown> | undefined);
            return (
              <Card key={obj.id} interactive onClick={() => setDetailObj(obj)}>
                {/* Header row */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: fields.length > 0 ? 8 : 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <PAGE_ICON_MAP.custom size={22} />
                    <div>
                      <span style={{ fontSize: 14, fontWeight: 600 }}>{obj.name}</span>
                      <span style={{
                        fontSize: 10, color: 'var(--text-tertiary)', marginLeft: 8,
                        padding: '1px 5px', borderRadius: 4, background: 'var(--bg-elevated)',
                      }}>
                        {t(`navigation:${obj.collectionType}`, obj.collectionType)}
                      </span>
                    </div>
                  </div>
                  {/* Edit + Delete + History actions */}
                  <div style={{ display: 'flex', gap: 2 }} onClick={(e) => e.stopPropagation()}>
                    <div style={{ position: 'relative' }}>
                      <button
                        onClick={() => setHistoryObjId(obj.id)}
                        title="History"
                        style={{
                          width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                          border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                          color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                        }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = 'rgba(128,128,128,0.08)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                      >
                        <Clock size={14} />
                      </button>
                      {/* Badge count */}
                      {snapshotCounts[obj.id] !== undefined && snapshotCounts[obj.id] > 0 && (
                        <span style={{
                          position: 'absolute', top: -2, right: -2, minWidth: 14, height: 14,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          background: 'var(--accent-primary)', color: 'white', fontSize: 9, fontWeight: 700,
                          borderRadius: 7, padding: '0 3px', lineHeight: 1,
                        }}>
                          {snapshotCounts[obj.id]}
                        </span>
                      )}
                    </div>
                    {/* Attachment button */}
                    <div style={{ position: 'relative' }}>
                      <button
                        onClick={() => setAttachmentObjId(obj.id)}
                        title="Attachments"
                        style={{
                          width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                          border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                          color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                        }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = 'rgba(128,128,128,0.08)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                      >
                        <Paperclip size={14} />
                      </button>
                      {attachmentCounts[obj.id] !== undefined && attachmentCounts[obj.id] > 0 && (
                        <span style={{
                          position: 'absolute', top: -2, right: -2, minWidth: 14, height: 14,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          background: 'var(--accent-primary)', color: 'white', fontSize: 9, fontWeight: 700,
                          borderRadius: 7, padding: '0 3px', lineHeight: 1,
                        }}>
                          {attachmentCounts[obj.id]}
                        </span>
                      )}
                    </div>
                    <button
                      onClick={() => navigate(`/editor/${obj.id}`)}
                      title="Edit"
                      style={{
                        width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                        border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                        color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(128,128,128,0.08)';
                        e.currentTarget.style.color = 'var(--text-primary)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'transparent';
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                      }}
                    >
                      <Pencil size={14} />
                    </button>
                    <button
                      onClick={() => setConfirmDelete({ id: obj.id, name: obj.name })}
                      title="Move to trash"
                      style={{
                        width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                        border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                        color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
                        e.currentTarget.style.color = '#e74c3c';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'transparent';
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                      }}
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
                {/* Property chips */}
                {fields.length > 0 && (
                  <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                    {fields.map((f) => {
                      const sens = getFieldSensitivity(obj.collectionType, f.key);
                      const isMasked = sens !== 'public';
                      return (
                      <span
                        key={f.key}
                        style={{
                          padding: '3px 8px', borderRadius: 6, fontSize: 11,
                          background: 'var(--bg-toolbar)', color: 'var(--text-secondary)',
                          border: '1px solid var(--border-subtle)',
                          maxWidth: 180, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                          ...(isMasked ? {
                            filter: 'blur(4px)',
                            cursor: 'default',
                            userSelect: 'none',
                          } : {}),
                        }}
                        title={isMasked ? t('sensitive_label') : `${t(`editor:fields.${f.key}`, f.key)}: ${f.value}`}
                      >
                        {f.value}
                      </span>
                      );
                    })}
                  </div>
                )}
                {/* Tag pills */}
                {obj.tags && obj.tags.length > 0 && (
                  <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 6 }}>
                    {obj.tags.map((tag) => (
                      <span key={tag} style={{
                        padding: '1px 7px', borderRadius: 10, fontSize: 10,
                        background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)',
                        fontWeight: 500,
                      }}>
                        {tag}
                      </span>
                    ))}
                  </div>
                )}
              </Card>
            );
          })}

        {/* Page delete confirmation dialog */}
        {confirmPageDelete && pageId && customPage && (
          <div
            style={{
              position: 'fixed', inset: 0, zIndex: 1000,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)', backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmPageDelete(false)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)', borderRadius: 12, padding: '24px 28px',
                maxWidth: 360, width: '90%', boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>{t('object_delete_confirm_title')}</h3>
              <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                {t('object_delete_confirm_body', { name: customPage.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmPageDelete(false)}>{t('cancel')}</Button>
                <button
                  onClick={async () => {
                    setConfirmPageDelete(false);
                    if (accountId) {
                      await removeCustomPage(accountId, pageId);
                      navigate('/');
                    }
                  }}
                  style={{
                    padding: '8px 16px', borderRadius: 8, border: 'none',
                    background: '#e74c3c', color: 'white',
                    fontSize: 13, fontWeight: 500, cursor: 'pointer',
                  }}
                >
                  {t('delete')}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Delete confirmation dialog */}
        {/* Object detail modal */}
        {detailObj && (() => {
          const dFields = flattenProperties(detailObj.properties as Record<string, unknown> | undefined);
          const handleCopyField = async (value: string, key: string) => {
            try { await navigator.clipboard.writeText(value); setCopiedField(key); setTimeout(() => setCopiedField(null), 1500); } catch {}
          };
          return (
        <div
          style={{ position: 'fixed', inset: 0, zIndex: 3000, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.35)', backdropFilter: 'blur(4px)' }}
          onClick={() => { setDetailObj(null); setCopiedField(null); }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{ background: 'var(--bg-elevated)', borderRadius: 16, padding: '28px 32px', maxWidth: 560, width: '90%', maxHeight: '80vh', overflowY: 'auto', boxShadow: 'var(--shadow-lg)', border: '1px solid var(--border-subtle)' }}
          >
            {/* Title row */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 20 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <PAGE_ICON_MAP.custom size={24} />
                <div>
                  <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0 }}>{detailObj.name}</h2>
                  <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                    {t('navigation:' + detailObj.collectionType, detailObj.collectionType)} · {t('common:created')}: {(detailObj as any).createdAt?.slice(0, 10) || '—'} · {t('common:updated')}: {(detailObj as any).updatedAt?.slice(0, 10) || '—'}
                  </span>
                </div>
              </div>
              <button onClick={() => { setDetailObj(null); setCopiedField(null); }} style={{ padding: 6, borderRadius: 8, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}>
                <X size={20} />
              </button>
            </div>

            {/* Divider */}
            <div style={{ height: 1, background: 'var(--border-subtle)', marginBottom: 16 }} />

            {/* Properties */}
            {dFields.length === 0 ? (
              <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center', padding: '16px 0' }}>{t('no_properties')}</p>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {dFields.map((f) => {
                  const sens = getFieldSensitivity(detailObj.collectionType, f.key);
                  const fieldId = sensitivityMap?.entries ? `${detailObj.collectionType}.${f.key}` : '';
                  const revealed = fieldId ? isRevealed(fieldId) : true;
                  const needsReveal = sens === 'sensitive' || sens === 'critical';
                  const handleReveal = async () => {
                    if (sens === 'critical') {
                      // Critical requires password verification
                      const verified = await passwordVerify();
                      if (verified) reveal(fieldId);
                    } else {
                      reveal(fieldId);
                    }
                  };
                  return (
                    <div key={f.key} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, padding: '8px 12px', borderRadius: 8, background: 'var(--bg-toolbar)', border: '1px solid var(--border-subtle)' }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
                          <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)' }}>
                            {t('editor:fields.' + f.key, f.key)}
                          </span>
                          <SensitivityBadge level={sens} />
                        </div>
                        <div style={{ fontSize: 14, color: (needsReveal && !revealed) ? 'var(--text-tertiary)' : 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                          {revealed ? f.value : maskValue(f.value, fieldId, sens)}
                        </div>
                      </div>
                      <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                        {needsReveal && !revealed && (
                          <button
                            onClick={handleReveal}
                            title={sens === 'critical' ? t('common:password_required') : t('common:reveal')}
                            style={{ padding: '4px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)', background: sens === 'critical' ? 'rgba(220,38,38,0.06)' : 'transparent', cursor: 'pointer', fontSize: 11, color: sens === 'critical' ? '#dc2626' : 'var(--text-tertiary)', display: 'flex', alignItems: 'center', gap: 4 }}
                          >
                            {sens === 'critical' ? <Lock size={12} /> : <Eye size={12} />} {sens === 'critical' ? t('common:unlock') : t('common:reveal')}
                          </button>
                        )}
                        <button
                          onClick={() => handleCopyField(revealed ? f.value : maskValue(f.value, fieldId, sens), f.key)}
                          style={{ padding: '4px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)', background: 'transparent', cursor: 'pointer', fontSize: 11, color: copiedField === f.key ? '#27ae60' : 'var(--text-tertiary)', display: 'flex', alignItems: 'center', gap: 4, transition: 'all 0.15s' }}
                        >
                          {copiedField === f.key ? <><Check size={12} /> {t('common:copied') || 'Copied'}</> : <><Copy size={12} /> {t('common:copy') || 'Copy'}</>}
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Tags */}
            {detailObj.tags && detailObj.tags.length > 0 && (
              <div style={{ marginTop: 16, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                {detailObj.tags.map((tag: string) => (
                  <span key={tag} style={{ padding: '2px 8px', borderRadius: 10, fontSize: 11, background: 'var(--bg-toolbar)', color: 'var(--text-secondary)', border: '1px solid var(--border-subtle)' }}>{tag}</span>
                ))}
              </div>
            )}
          </div>
        </div>
          );
        })()}

        {confirmDelete && (
          <div
            style={{
              position: 'fixed', inset: 0, zIndex: 1000,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)', backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmDelete(null)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)', borderRadius: 12, padding: '24px 28px',
                maxWidth: 360, width: '90%', boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>{t('object_delete_confirm_title')}</h3>
              <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                {t('object_delete_confirm_body', { name: confirmDelete.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmDelete(null)}>{t('cancel')}</Button>
                <button
                  onClick={() => handleDelete(confirmDelete.id)}
                  style={{
                    padding: '8px 16px', borderRadius: 8, border: 'none',
                    background: '#e74c3c', color: 'white',
                    fontSize: 13, fontWeight: 500, cursor: 'pointer',
                  }}
                >
                  {t('delete')}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
      {historyObjId && <HistoryViewer objectId={historyObjId} onClose={() => setHistoryObjId(null)} />}
      {attachmentObjId && <AttachmentViewer objectId={attachmentObjId} onClose={() => setAttachmentObjId(null)} onCountChange={refreshAttachmentCounts} />}
    </AppShell>
  );
}
