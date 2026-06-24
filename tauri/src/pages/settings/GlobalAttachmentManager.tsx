import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useAttachmentPageSort } from '@/hooks/useAttachmentPageSort';
import {
  Paperclip,
  Trash2,
  RotateCcw,
  Eye,
  Edit2,
  Upload,
  X,
  ChevronRight,
  ChevronDown,
  Search,
} from 'lucide-react';
import { DEFAULT_CUSTOM_ICON, PAGE_ICON_MAP, CUSTOM_ICON_MAP } from '@/lib/pageIcons';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import type { PageIconKey, CustomIconId } from '@/lib/pageIcons';
import type { LucideIcon } from 'lucide-react';

/** Resolve icon from either PAGE_ICON_MAP (built-in) or CUSTOM_ICON_MAP (user-selectable). */
function resolvePageIcon(iconKey?: string | null): LucideIcon {
  const id = iconKey || DEFAULT_CUSTOM_ICON;
  if (id in PAGE_ICON_MAP) return PAGE_ICON_MAP[id as PageIconKey];
  if (id in CUSTOM_ICON_MAP) return CUSTOM_ICON_MAP[id as CustomIconId];
  return CUSTOM_ICON_MAP[DEFAULT_CUSTOM_ICON];
}

// ── Types ────────────────────────────────────────────────────

interface AttachmentMeta {
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

interface AttachmentTreeObject {
  objectId: string;
  objectName: string;
  templateName?: string | null;
  attachments: AttachmentMeta[];
}

interface AttachmentTreePage {
  pageId?: string | null;
  pageName: string;
  pageIcon?: string | null;
  objects: AttachmentTreeObject[];
}

interface AttachmentListAllResult {
  pages: AttachmentTreePage[];
  trashPages: AttachmentTreePage[];
}

// ── Helpers ──────────────────────────────────────────────────

function truncateFileName(fileName: string, maxLen = 28): string {
  const dotIndex = fileName.lastIndexOf('.');
  if (dotIndex <= 0) {
    if (fileName.length <= maxLen) return fileName;
    return fileName.slice(0, maxLen - 1) + '…';
  }
  const baseName = fileName.slice(0, dotIndex);
  const ext = fileName.slice(dotIndex);
  if (fileName.length <= maxLen) return fileName;
  const available = maxLen - ext.length - 2;
  if (available <= 1) return fileName.slice(0, maxLen - 1) + '…';
  return baseName.slice(0, available) + '…-' + ext;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const btnMini: React.CSSProperties = {
  width: 26,
  height: 26,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  border: 'none',
  borderRadius: 5,
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 11,
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

// ── 拖拽上传包裹组件 ─────────────────────────────────────────

/** 为单个对象行提供拖拽上传的 drop zone */
function ObjectDropTarget({
  objectId,
  loadData,
  children,
}: {
  objectId: string;
  loadData: () => void;
  children: React.ReactNode;
}) {
  const { ref, dragState } = useDragToAttach(objectId, { onComplete: loadData });
  return (
    <div ref={ref} style={{ position: 'relative' }}>
      {children}
      <DragUploadOverlay dragState={dragState} borderRadius={8} />
    </div>
  );
}

// ── Component ────────────────────────────────────────────────

export function GlobalAttachmentManager() {
  const { t } = useTranslation(['settings', 'common']);
  const navigate = useNavigate();
  const location = useLocation();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const [data, setData] = useState<AttachmentListAllResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [showTrash, setShowTrash] = useState(false);
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const [expandedObjects, setExpandedObjects] = useState<Set<string>>(new Set());
  const [previewItem, setPreviewItem] = useState<AttachmentMeta | null>(null);
  const [previewUrl, setPreviewUrl] = useState('');
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [renameObjectId, setRenameObjectId] = useState<string>('');
  const renameInputRef = useRef<HTMLInputElement>(null);

  const loadData = useCallback(async () => {
    if (!accountId) return;
    setLoading(true);
    try {
      const result = await invoke<AttachmentListAllResult>('attachment_list_all', {
        accountId,
      });
      setData(result);
    } catch {
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Expand all pages and objects by default when data loads
  useEffect(() => {
    if (!data) return;
    const allPages = new Set<string>();
    const allObjects = new Set<string>();
    const pages = data.pages.concat(data.trashPages);
    for (const page of pages) {
      const pageKey = getPageKey(page);
      allPages.add(pageKey);
      for (const obj of page.objects) {
        allObjects.add(`${pageKey}::${getObjKey(obj)}`);
      }
    }
    setExpandedPages(allPages);
    setExpandedObjects(allObjects);
  }, [data]);

  // ── Tree expansion ──────────────────────────────────────────

  const togglePage = (key: string) => {
    setExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const toggleObject = (key: string) => {
    setExpandedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const getPageKey = (p: AttachmentTreePage) => p.pageId || p.pageName;
  const getObjKey = (o: AttachmentTreeObject) => o.objectId;

  // ── Attachment operations ──────────────────────────────────

  const handlePreview = async (item: AttachmentMeta) => {
    const ext = item.fileName.split('.').pop()?.toLowerCase() || '';
    const isImage =
      item.mimeType.startsWith('image/') ||
      ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext);

    if (isImage) {
      setPreviewItem(item);
      setPreviewUrl('');
      try {
        const url = await invoke<string>('fs_read_file_as_data_url', {
          path: item.vaultPath || item.srcPath,
        });
        setPreviewUrl(url);
      } catch {
        setPreviewUrl('error');
      }
      return;
    }

    // For non-images, try opening with default app
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      const filePath = item.vaultPath || item.srcPath;
      if (filePath) await open(filePath);
    } catch {
      showToast({
        type: 'error',
        message: t('common:cannot_open_file', { path: item.fileName }) ||
          `Cannot open file: ${item.fileName}`,
      });
    }
  };

  const handleStartRename = (item: AttachmentMeta, objectId: string) => {
    setRenamingId(item.id);
    setRenameObjectId(objectId);
    setRenameValue(item.fileName);
    setTimeout(() => renameInputRef.current?.focus(), 50);
  };

  const handleConfirmRename = async () => {
    if (renamingId && renameValue.trim() && renameObjectId) {
      try {
        await invoke('attachment_rename', {
          objectId: renameObjectId,
          attachmentId: renamingId,
          newName: renameValue.trim(),
        });
        await loadData();
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:rename_failed')}: ${e}` });
      }
    }
    setRenamingId(null);
    setRenameObjectId('');
  };

  const handleUpload = async (objectId: string) => {
    const filePath = await pickFileToAttach();
    if (filePath) {
      try {
        await uploadSingleAttachment(filePath, objectId);
        await loadData();
      } catch (e) {
        showToast({ type: 'error', message: `${t('common:upload_failed')}: ${e}` });
      }
    }
  };

  const handleSoftDelete = (item: AttachmentMeta, objectId: string) => {
    requestConfirm(
      t('common:confirm_delete_title', 'Delete attachment'),
      t('common:confirm_delete_body', { name: truncateFileName(item.fileName) }) ||
        `Delete "${truncateFileName(item.fileName)}"? It will be moved to trash.`,
      async () => {
        try {
          await invoke('attachment_soft_delete', { objectId, attachmentId: item.id });
          await loadData();
        } catch (e) {
          showToast({ type: 'error', message: `${t('common:delete_failed')}: ${e}` });
        }
      },
      { confirmLabel: t('common:delete'), cancelLabel: t('common:cancel') },
    );
  };

  const handleRestore = async (item: AttachmentMeta, objectId: string) => {
    try {
      await invoke('attachment_restore', { objectId, attachmentId: item.id });
      await loadData();
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:restore_failed')}: ${e}` });
    }
  };

  const handlePermanentDelete = (item: AttachmentMeta, objectId: string) => {
    setPermDeleteItem({ ...item, _objectId: objectId });
  };

  const [permDeleteItem, setPermDeleteItem] = useState<(AttachmentMeta & { _objectId: string }) | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const doPermanentDelete = async () => {
    if (!permDeleteItem) return;
    try {
      await invoke('attachment_delete', { objectId: permDeleteItem._objectId, attachmentId: permDeleteItem.id });
      await loadData();
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:perm_delete_failed')}: ${e}` });
    }
    setPermDeleteItem(null);
  };

  // ── Display data ───────────────────────────────────────────

  const rawPages = showTrash ? (data?.trashPages || []) : (data?.pages || []);
  const sortedPages = useAttachmentPageSort(rawPages);

  // Filter pages/objects/attachments by search query (matches against file name)
  const displayPages = useMemo(() => {
    if (!searchQuery.trim()) return sortedPages;
    const q = searchQuery.toLowerCase();
    return sortedPages
      .map((page) => ({
        ...page,
        objects: page.objects
          .map((obj) => ({
            ...obj,
            attachments: obj.attachments.filter((att) =>
              att.fileName.toLowerCase().includes(q),
            ),
          }))
          .filter((obj) => obj.attachments.length > 0),
      }))
      .filter((page) => page.objects.length > 0);
  }, [sortedPages, searchQuery]);

  /** 收集当前显示的所有附件复合键 */
  const allVisibleKeys = useMemo(() => {
    const keys: string[] = [];
    for (const page of displayPages) {
      for (const obj of page.objects) {
        for (const att of obj.attachments) {
          keys.push(`${obj.objectId}::${att.id}`);
        }
      }
    }
    return keys;
  }, [displayPages]);

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

  // ── Batch operations ───────────────────────────────────────

  /** 根据选中的复合键批量软删除附件 */
  const handleBatchDelete = async () => {
    setBatchDeleteConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [objectId, attachmentId] = key.split('::');
      if (!byObject.has(objectId)) byObject.set(objectId, []);
      byObject.get(objectId)!.push(attachmentId);
    }

    let successCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_soft_delete', { objectId, attachmentIds });
        successCount += attachmentIds.length;
      } catch {
        // best effort per object
      }
    }

    clearSelection();
    await loadData();
    showToast({
      type: 'info',
      message: `${t('common:batch_delete_result', { success: successCount, total: entries.length }) || `Deleted ${successCount}/${entries.length} attachments`}`,
    });
  };

  /** 根据选中的复合键批量永久删除附件 */
  const handleBatchPermanentDelete = async () => {
    setBatchPermanentDeleteConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [objectId, attachmentId] = key.split('::');
      if (!byObject.has(objectId)) byObject.set(objectId, []);
      byObject.get(objectId)!.push(attachmentId);
    }

    let successCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_delete', { objectId, attachmentIds });
        successCount += attachmentIds.length;
      } catch {
        // best effort per object
      }
    }

    clearSelection();
    await loadData();
    showToast({
      type: 'info',
      message: `${t('common:batch_perm_delete_result', { success: successCount, total: entries.length }) || `Permanently deleted ${successCount}/${entries.length} attachments`}`,
    });
  };

  /** 根据选中的复合键批量恢复附件 */
  const handleBatchRestore = async () => {
    setBatchRestoreConfirm(false);
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    // Group by objectId
    const byObject = new Map<string, string[]>();
    for (const key of entries) {
      const [oid, attachmentId] = key.split('::');
      if (!byObject.has(oid)) byObject.set(oid, []);
      byObject.get(oid)!.push(attachmentId);
    }

    let successCount = 0;
    for (const [objectId, attachmentIds] of byObject) {
      try {
        await invoke('attachment_batch_restore', { objectId, attachmentIds });
        successCount += attachmentIds.length;
      } catch {
        // best effort per object
      }
    }
    clearSelection();
    await loadData();
    showToast({
      type: 'info',
      message: `${t('common:batch_restore_result', { success: successCount, total: entries.length }) || `Restored ${successCount}/${entries.length} attachments`}`,
    });
  };

  // Auto-expand all pages/objects when searching
  useEffect(() => {
    if (!searchQuery.trim() || !data) return;
    const allPages = new Set<string>();
    const allObjects = new Set<string>();
    for (const page of displayPages) {
      const pageKey = getPageKey(page);
      allPages.add(pageKey);
      for (const obj of page.objects) {
        allObjects.add(`${pageKey}::${getObjKey(obj)}`);
      }
    }
    setExpandedPages(allPages);
    setExpandedObjects(allObjects);
  }, [displayPages, searchQuery, data]);

  // ── Count summaries ────────────────────────────────────────

  const activeCount = data?.pages.reduce((acc, p) =>
    acc + p.objects.reduce((acc2, o) => acc2 + o.attachments.length, 0), 0) ?? 0;
  const activeBytes = data?.pages.reduce((acc, p) =>
    acc + p.objects.reduce((acc2, o) => acc2 + o.attachments.reduce((s, a) => s + a.sizeBytes, 0), 0), 0) ?? 0;
  const trashCount = data?.trashPages.reduce((acc, p) =>
    acc + p.objects.reduce((acc2, o) => acc2 + o.attachments.length, 0), 0) ?? 0;
  const trashBytes = data?.trashPages.reduce((acc, p) =>
    acc + p.objects.reduce((acc2, o) => acc2 + o.attachments.reduce((s, a) => s + a.sizeBytes, 0), 0), 0) ?? 0;

  const summaryStats = useMemo(() => {
    const pages = showTrash ? (data?.trashPages || []) : (data?.pages || []);
    let totalAttachments = 0;
    let totalBytes = 0;
    let totalObjects = 0;
    for (const page of pages) {
      for (const obj of page.objects) {
        totalObjects++;
        for (const att of obj.attachments) {
          totalAttachments++;
          totalBytes += att.sizeBytes;
        }
      }
    }
    return { totalObjects, totalAttachments, totalBytes };
  }, [data, showTrash]);

  // ── Render attachment row ──────────────────────────────────

  const renderAttachment = (item: AttachmentMeta, objectId: string) => {
    const isRenaming = renamingId === item.id;

    const compositeKey = `${objectId}::${item.id}`;
    const isChecked = selectedIds.has(compositeKey);

    return (
      <div
        key={item.id}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '6px 8px 6px 40px',
          fontSize: 12,
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <SelectCheckbox
          checked={isChecked}
          onClick={(e) => { e.stopPropagation(); toggleSelect(compositeKey); }}
          size={14}
          borderRadius={3}
        />
        {showTrash ? (
          <Trash2 size={12} style={{ color: '#e74c3c', flexShrink: 0 }} />
        ) : (
          <Paperclip size={12} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
        )}

        {isRenaming ? (
          <input
            ref={renameInputRef}
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleConfirmRename();
              if (e.key === 'Escape') {
                setRenamingId(null);
                setRenameObjectId('');
              }
            }}
            onBlur={handleConfirmRename}
            style={{
              flex: 1,
              minWidth: 0,
              padding: '2px 6px',
              fontSize: 12,
              borderRadius: 4,
              border: '1px solid var(--accent-primary)',
              background: 'transparent',
              color: 'var(--text-primary)',
              outline: 'none',
            }}
          />
        ) : (
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
        )}

        <div style={{ display: 'flex', gap: 1, flexShrink: 0 }}>
          {showTrash ? (
            <>
              <button
                onClick={() => handleRestore(item, objectId)}
                onMouseEnter={btnHoverEnter}
                onMouseLeave={btnHoverLeave}
                style={btnMini}
                title={t('common:restore')}
              >
                <RotateCcw size={10} />
              </button>
              <button
                onClick={() => handlePermanentDelete(item, objectId)}
                onMouseEnter={btnDelEnter}
                onMouseLeave={btnDelLeave}
                style={{ ...btnMini, color: '#e74c3c' }}
                title={t('common:delete_permanently')}
              >
                <Trash2 size={10} />
              </button>
            </>
          ) : (
            !isRenaming && (
              <>
                <button
                  onClick={() => handlePreview(item)}
                  onMouseEnter={btnHoverEnter}
                  onMouseLeave={btnHoverLeave}
                  style={btnMini}
                  title={t('common:preview')}
                >
                  <Eye size={10} />
                </button>
                <button
                  onClick={() => handleStartRename(item, objectId)}
                  onMouseEnter={btnHoverEnter}
                  onMouseLeave={btnHoverLeave}
                  style={btnMini}
                  title={t('common:rename')}
                >
                  <Edit2 size={10} />
                </button>
                <button
                  onClick={() => handleSoftDelete(item, objectId)}
                  onMouseEnter={btnDelEnter}
                  onMouseLeave={btnDelLeave}
                  style={{ ...btnMini, color: '#e74c3c' }}
                  title={t('common:delete')}
                >
                  <Trash2 size={10} />
                </button>
              </>
            )
          )}
        </div>
      </div>
    );
  };

  // ── Render object group ────────────────────────────────────

  const renderObject = (obj: AttachmentTreeObject, pageKey: string) => {
    const objKey = `${pageKey}::${obj.objectId}`;
    const isExpanded = expandedObjects.has(objKey);

    const row = (
      <div>
        <div
          onClick={() => toggleObject(objKey)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '7px 8px 7px 28px',
            cursor: 'pointer',
            fontSize: 12,
            fontWeight: 500,
            color: 'var(--text-primary)',
            borderBottom: '1px solid var(--border-subtle)',
            transition: 'background 0.15s',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}
        >
          {isExpanded ? <ChevronDown size={12} style={{ flexShrink: 0 }} /> : <ChevronRight size={12} style={{ flexShrink: 0 }} />}
          <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
            {obj.templateName}
          </span>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>
            {obj.objectName}
          </span>
          <span style={{ fontSize: 11, color: 'var(--text-tertiary)', flexShrink: 0, whiteSpace: 'nowrap' }}>
            {t('settings:attachments_count', { n: obj.attachments.length })} · {formatSize(obj.attachments.reduce((sum, a) => sum + a.sizeBytes, 0))}
          </span>
          {!showTrash && (
            <button
              onClick={(e) => { e.stopPropagation(); handleUpload(obj.objectId); }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = 'var(--accent-primary)';
                e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = 'var(--text-secondary)';
                e.currentTarget.style.background = 'transparent';
              }}
              style={{
                ...btnMini,
                width: 22,
                height: 22,
                fontSize: 10,
              }}
              title={t('common:upload') || 'Upload'}
            >
              <Upload size={10} />
            </button>
          )}
        </div>
        {isExpanded && obj.attachments.map((att) => renderAttachment(att, obj.objectId))}
      </div>
    );

    return showTrash ? (
      <div key={obj.objectId}>{row}</div>
    ) : (
      <ObjectDropTarget key={obj.objectId} objectId={obj.objectId} loadData={loadData}>
        {row}
      </ObjectDropTarget>
    );
  };

  // ── Render page group ──────────────────────────────────────

  const renderPage = (page: AttachmentTreePage) => {
    const pageKey = getPageKey(page);
    const isExpanded = expandedPages.has(pageKey);
    const PageIconComp = resolvePageIcon(page.pageIcon);

    return (
      <Card key={pageKey} style={{ padding: 0, overflow: 'hidden' }}>
        <div
          onClick={() => togglePage(pageKey)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '10px 14px',
            cursor: 'pointer',
            fontSize: 13,
            fontWeight: 600,
            color: 'var(--text-primary)',
            background: 'var(--bg-toolbar)',
            borderBottom: isExpanded ? '1px solid var(--border-subtle)' : 'none',
            transition: 'background 0.15s',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 8%, var(--bg-toolbar))'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'var(--bg-toolbar)'; }}
        >
          <PageIconComp size={16} style={{ flexShrink: 0, color: 'var(--accent-primary)' }} />
          <span style={{ flex: 1 }}>{page.pageName}</span>
          <span style={{ fontSize: 11, color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>
            {t('settings:objects_count', { n: page.objects.length })} · {t('settings:attachments_count', { n: page.objects.reduce((sum, o) => sum + o.attachments.length, 0) })} · {formatSize(page.objects.reduce((sum, o) => sum + o.attachments.reduce((s, a) => s + a.sizeBytes, 0), 0))}
          </span>
          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </div>
        {isExpanded && page.objects.map((obj) => renderObject(obj, pageKey))}
      </Card>
    );
  };

  // ── Main render ────────────────────────────────────────────

  return (
    <AppShell title={t('settings:items.global_attachments') || 'Attachments'}
      onBack={() => {
        const state = location.state as { from?: string } | undefined;
        if (state?.from === '/home') navigate('/home');
        else navigate('/settings');
      }}
    >
      <div
        style={{
          maxWidth: 600,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        {/* Search */}
        <Input
          placeholder={showTrash ? t('common:search_trash') || 'Search trash...' : t('common:search_attachments') || 'Search attachments...'}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
          icon={<Search size={14} style={{ color: 'var(--text-tertiary)' }} />}
        />

        {/* Tab pills */}
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <button
            onClick={() => { setShowTrash(false); clearSelection(); }}
            onMouseEnter={showTrash ? (e) => {
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            } : undefined}
            onMouseLeave={showTrash ? (e) => {
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.background = 'var(--bg-toolbar)';
            } : undefined}
            style={{
              padding: '5px 12px',
              borderRadius: 6,
              fontSize: 12,
              fontWeight: 500,
              border: !showTrash ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
              background: !showTrash ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
              color: !showTrash ? 'var(--accent-primary)' : 'var(--text-primary)',
              boxShadow: !showTrash ? '0 0 0 1px var(--accent-primary)' : 'none',
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
            }}
          >
            {t('common:attachments_active', { n: activeCount }) || `Attachments (${activeCount})`}
            <span style={{ marginLeft: 4, fontSize: 10, opacity: 0.7 }}>{formatSize(activeBytes)}</span>
          </button>
          <button
            onClick={() => { setShowTrash(true); clearSelection(); }}
            onMouseEnter={!showTrash ? (e) => {
              e.currentTarget.style.borderColor = '#e74c3c';
              e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 10%, transparent)';
            } : undefined}
            onMouseLeave={!showTrash ? (e) => {
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.background = 'var(--bg-toolbar)';
            } : undefined}
            style={{
              padding: '5px 12px',
              borderRadius: 6,
              fontSize: 12,
              fontWeight: 500,
              border: showTrash ? '1px solid #e74c3c' : '1px solid var(--border-subtle)',
              background: showTrash ? 'color-mix(in srgb, #e74c3c 10%, transparent)' : 'var(--bg-toolbar)',
              color: showTrash ? '#e74c3c' : 'var(--text-primary)',
              boxShadow: showTrash ? '0 0 0 1px #e74c3c' : 'none',
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
            }}
          >
            {t('common:attachments_trash', { n: trashCount }) || `Trash (${trashCount})`}
            <span style={{ marginLeft: 4, fontSize: 10, opacity: 0.7 }}>{formatSize(trashBytes)}</span>
          </button>

          <div style={{ flex: 1 }} />

          <button
            onClick={loadData}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.color = 'var(--accent-primary)';
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color = 'var(--text-secondary)';
              e.currentTarget.style.background = 'var(--bg-toolbar)';
            }}
            style={{
              padding: '5px 12px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
              color: 'var(--text-secondary)',
              fontSize: 12,
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s',
            }}
          >
            {t('common:refresh') || 'Refresh'}
          </button>
        </div>

        {/* Summary card */}
        {!loading && data && (
          <Card style={{ padding: '12px 16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 20, fontSize: 12 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <Paperclip size={14} style={{ color: 'var(--accent-primary)' }} />
                <span style={{ color: 'var(--text-tertiary)' }}>{t('common:attachments')}</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{summaryStats.totalAttachments}</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ color: 'var(--text-tertiary)' }}>{t('common:size')}</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{formatSize(summaryStats.totalBytes)}</span>
              </div>
              <div style={{ flex: 1 }} />
              <div style={{ color: 'var(--text-tertiary)', fontSize: 11 }}>
                {t('settings:objects_count', { n: summaryStats.totalObjects })}
              </div>
            </div>
          </Card>
        )}

        {/* Batch toolbar (活跃标签 → 批量删除，回收站标签 → 批量恢复) */}
        {selectedIds.size > 0 && (
          <Card style={{ padding: '8px 14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12 }}>
              <div
                onClick={() => handleSelectAll(allVisibleKeys)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  cursor: 'pointer',
                  color: 'var(--text-secondary)',
                  userSelect: 'none',
                }}
              >
                <SelectCheckbox checked={allSelected} size={14} borderRadius={3} />
                {allSelected ? t('common:deselect_all') : t('common:select_all')}
              </div>

              <div style={{ flex: 1 }} />

              <span style={{ color: 'var(--text-tertiary)', fontSize: 11 }}>
                {t('common:selected_count', { n: selectedIds.size })}
              </span>

              {!showTrash ? (
                <button
                  onClick={() => setBatchDeleteConfirm(true)}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
                    e.currentTarget.style.borderColor = '#e74c3c';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    padding: '4px 10px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'transparent',
                    color: '#e74c3c',
                    fontSize: 12,
                    fontWeight: 500,
                    cursor: 'pointer',
                    transition: 'all 0.15s ease',
                  }}
                >
                  <Trash2 size={12} /> {t('common:delete')}
                </button>
              ) : (
                <div style={{ display: 'flex', gap: 6 }}>
                  <button
                    onClick={() => setBatchRestoreConfirm(true)}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                      e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'transparent';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      padding: '4px 10px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'transparent',
                      color: 'var(--accent-primary)',
                      fontSize: 12,
                      fontWeight: 500,
                      cursor: 'pointer',
                      transition: 'all 0.15s ease',
                    }}
                  >
                    <RotateCcw size={12} /> {t('common:restore')}
                  </button>
                  <button
                    onClick={() => setBatchPermanentDeleteConfirm(true)}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
                      e.currentTarget.style.borderColor = '#e74c3c';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'transparent';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      padding: '4px 10px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'transparent',
                      color: '#e74c3c',
                      fontSize: 12,
                      fontWeight: 500,
                      cursor: 'pointer',
                      transition: 'all 0.15s ease',
                    }}
                  >
                    <Trash2 size={12} /> {t('common:delete_permanently')}
                  </button>
                </div>
              )}
            </div>
          </Card>
        )}

        {/* Content */}
        {loading ? (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={200} />
          </Card>
        ) : displayPages.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: '48px 24px' }}>
              <Paperclip
                size={48}
                style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
              />
              <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>
                {searchQuery.trim()
                  ? (t('common:no_search_results') || 'No matching attachments found.')
                  : showTrash
                    ? (t('settings:trash_empty') || 'Trash is empty.')
                    : (t('common:no_attachments') || 'No attachments found.')}
              </p>
            </div>
          </Card>
        ) : (
          displayPages.map(renderPage)
        )}
      </div>

      {/* Image preview overlay */}
      {previewItem && (
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
              onClick={(e) => { e.stopPropagation(); setPreviewItem(null); }}
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
              <LoadingPlaceholder variant="toolbar" minHeight={120} />
            )}
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
              {t('common:batch_restore_title') || 'Batch restore'}
            </h3>
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:batch_restore_body', { n: selectedIds.size }) ||
                `Restore ${selectedIds.size} selected attachment(s) from trash?`}
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
                  border: 'none',
                  background: 'var(--accent-primary)',
                  color: 'white',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                }}
              >
                {t('common:restore')}
              </button>
            </div>
          </div>
        </div>
      )}

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
              {t('common:batch_delete_title') || 'Batch delete'}
            </h3>
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:batch_delete_body', { n: selectedIds.size }) ||
                `Delete ${selectedIds.size} selected attachment(s)? They will be moved to trash.`}
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
                }}
              >
                {t('common:delete')}
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
              {t('common:batch_perm_delete_title') || 'Permanently delete selected?'}
            </h3>
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:batch_perm_delete_body', { n: selectedIds.size }) ||
                `Permanently delete ${selectedIds.size} selected attachment(s)? This cannot be undone.`}
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
                }}
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
              {t('common:perm_delete_title') || 'Permanently delete?'}
            </h3>
            <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
              {t('common:perm_delete_body', { name: truncateFileName(permDeleteItem.fileName) }) ||
                `Delete "${truncateFileName(permDeleteItem.fileName)}"? This cannot be undone.`}
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
                onClick={doPermanentDelete}
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

      {confirmDialog}
    </AppShell>
  );
}
