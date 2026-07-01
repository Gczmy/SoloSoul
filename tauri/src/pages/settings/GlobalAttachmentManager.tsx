import { motion } from 'framer-motion';
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { useAttachmentPageSort } from '@/hooks/useAttachmentPageSort';
import {
  Paperclip,
  RotateCcw,
  Eye,
  Edit2,
  Upload,
  ChevronRight,
  ChevronDown,
  Search,
  Download,
} from 'lucide-react';
import { DEFAULT_CUSTOM_ICON, PAGE_ICON_MAP, CUSTOM_ICON_MAP } from '@/lib/pageIcons';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { useBatchSelect } from '@/hooks/useBatchSelect';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import type { PageIconKey, CustomIconId } from '@/lib/pageIcons';
import type { LucideIcon } from 'lucide-react';
import { truncateFileName, formatSize } from '@/lib/attachmentUtils';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { ICON_SIZE } from '@/lib/iconSizes';

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

// ── Helpers (rest) ────────────────────────────────────────────

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
  const { t } = useTranslation(['settings', 'common', 'navigation']);
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
      return;
    }

    // For non-images, open with the system default app via a trusted Rust command.
    try {
      await invoke('attachment_open', {
        objectId: item.objectId,
        attachmentId: item.id,
      });
    } catch {
      showToast({
        type: 'error',
        message:
          t('common:cannot_open_file', { path: item.fileName }) ||
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

  const handleDownload = async (item: AttachmentMeta) => {
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
      showToast({
        type: 'success',
        message: t('common:download_result') || 'Downloaded successfully',
      });
    } catch (e) {
      showToast({ type: 'error', message: `${t('common:download_failed')}: ${e}` });
    }
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

  const [permDeleteItem, setPermDeleteItem] = useState<
    (AttachmentMeta & { _objectId: string }) | null
  >(null);
  const [searchQuery, setSearchQuery] = useState('');

  const doPermanentDelete = async () => {
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
  };

  // ── Display data ───────────────────────────────────────────

  const rawPages = showTrash ? data?.trashPages || [] : data?.pages || [];
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
            attachments: obj.attachments.filter((att) => att.fileName.toLowerCase().includes(q)),
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

  /** 根据选中的复合键批量下载附件 */
  const handleBatchDownload = async () => {
    const entries = [...selectedIds];
    if (entries.length === 0) return;

    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const dirPath = await open({
        directory: true,
        multiple: false,
        title: t('common:select_download_directory') || 'Select download directory',
      });
      if (!dirPath) return;

      // Map entries to actual attachment items
      const selectedItems: AttachmentMeta[] = [];
      for (const page of displayPages) {
        for (const obj of page.objects) {
          for (const att of obj.attachments) {
            if (entries.includes(`${obj.objectId}::${att.id}`)) {
              selectedItems.push(att);
            }
          }
        }
      }

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
        message:
          t('common:batch_download_result', {
            success: successCount,
            total: selectedItems.length,
          }) || `Downloaded ${successCount}/${selectedItems.length} files`,
      });
      clearSelection();
    } catch {
      // dialog cancelled
    }
  };

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

  // ── Count summaries (unified via summaryStats) ─────────────

  const summaryStats = useMemo(() => {
    const activePages = data?.pages || [];
    const trashPages = data?.trashPages || [];
    let activeAttachments = 0,
      activeBytes = 0,
      activeObjects = 0;
    for (const page of activePages) {
      for (const obj of page.objects) {
        activeObjects++;
        for (const att of obj.attachments) {
          activeAttachments++;
          activeBytes += att.sizeBytes;
        }
      }
    }
    let trashAttachments = 0,
      trashBytes = 0,
      trashObjects = 0;
    for (const page of trashPages) {
      for (const obj of page.objects) {
        trashObjects++;
        for (const att of obj.attachments) {
          trashAttachments++;
          trashBytes += att.sizeBytes;
        }
      }
    }
    return {
      activeAttachments,
      activeBytes,
      activeObjects,
      trashAttachments,
      trashBytes,
      trashObjects,
    };
  }, [data]);

  const {
    activeAttachments: activeCount,
    activeBytes,
    activeObjects,
    trashAttachments: trashCount,
    trashBytes,
    trashObjects,
  } = summaryStats;

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
          fontSize: 'var(--text-sm)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <SelectCheckbox
          checked={isChecked}
          onClick={(e) => {
            e.stopPropagation();
            toggleSelect(compositeKey);
          }}
        />
        <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />

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
              fontSize: 'var(--text-sm)',
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
            <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {formatSize(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
            </div>
          </div>
        )}

        <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
          {showTrash ? (
            <>
              <BadgeIconButton
                Icon={RotateCcw}
                onClick={() => handleRestore(item, objectId)}
                title={t('common:restore')}
                iconSize={ICON_SIZE.sm}
              />
              <DeleteButton
                iconOnly
                onClick={() => handlePermanentDelete(item, objectId)}
                title={t('common:delete_permanently')}
              />
            </>
          ) : (
            !isRenaming && (
              <>
                <BadgeIconButton
                  Icon={Eye}
                  onClick={() => handlePreview(item)}
                  title={t('common:preview')}
                  iconSize={ICON_SIZE.sm}
                />
                <BadgeIconButton
                  Icon={Edit2}
                  onClick={() => handleStartRename(item, objectId)}
                  title={t('common:rename')}
                  iconSize={ICON_SIZE.sm}
                />
                <BadgeIconButton
                  Icon={Download}
                  onClick={() => handleDownload(item)}
                  title={t('common:download')}
                  iconSize={ICON_SIZE.sm}
                />
                <DeleteButton
                  iconOnly
                  onClick={() => handleSoftDelete(item, objectId)}
                  title={t('common:delete')}
                />
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
            fontSize: 'var(--text-sm)',
            fontWeight: 500,
            color: 'var(--text-primary)',
            borderBottom: '1px solid var(--border-subtle)',
            transition: 'background 0.15s',
          }}
          className="interactive-accent-light"
        >
          {isExpanded ? (
            <ChevronDown size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
          ) : (
            <ChevronRight size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
          )}
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {obj.templateName}
          </span>
          <span
            style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}
          >
            {obj.objectName}
          </span>
          <span
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              flexShrink: 0,
              whiteSpace: 'nowrap',
            }}
          >
            {t('settings:attachments_count', { n: obj.attachments.length })} ·{' '}
            {formatSize(obj.attachments.reduce((sum, a) => sum + a.sizeBytes, 0))}
          </span>
          {!showTrash && (
            <BadgeIconButton
              Icon={Upload}
              onClick={(e) => {
                e.stopPropagation();
                handleUpload(obj.objectId);
              }}
              title={t('common:upload') || 'Upload'}
              iconSize={ICON_SIZE.sm}
            />
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
            fontSize: 'var(--text-sm)',
            fontWeight: 600,
            color: 'var(--text-primary)',
            background: 'var(--bg-toolbar)',
            borderBottom: isExpanded ? '1px solid var(--border-subtle)' : 'none',
            transition: 'background 0.15s',
          }}
          className="interactive-toolbar"
        >
          <PageIconComp
            size={ICON_SIZE.xl}
            style={{ flexShrink: 0, color: 'var(--accent-primary)' }}
          />
          <span style={{ flex: 1 }}>
            {page.pageId ? page.pageName : t(`navigation:${page.pageName}`)}
          </span>
          <span
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              whiteSpace: 'nowrap',
            }}
          >
            {t('settings:objects_count', { n: page.objects.length })} ·{' '}
            {t('settings:attachments_count', {
              n: page.objects.reduce((sum, o) => sum + o.attachments.length, 0),
            })}{' '}
            ·{' '}
            {formatSize(
              page.objects.reduce(
                (sum, o) => sum + o.attachments.reduce((s, a) => s + a.sizeBytes, 0),
                0,
              ),
            )}
          </span>
          {isExpanded ? <ChevronDown size={ICON_SIZE.sm} /> : <ChevronRight size={ICON_SIZE.sm} />}
        </div>
        {isExpanded && page.objects.map((obj) => renderObject(obj, pageKey))}
      </Card>
    );
  };

  // ── Main render ────────────────────────────────────────────

  return (
    <AppShell
      title={t('settings:items.global_attachments') || 'Attachments'}
      onBack={() => {
        const state = location.state as { from?: string } | undefined;
        if (state?.from === '/home') navigate('/home');
        else navigate('/settings');
      }}
    >
      <PageContainer variant="medium" gap="default">
        {/* Search */}
        <Input
          placeholder={
            showTrash
              ? t('common:search_trash') || 'Search trash...'
              : t('common:search_attachments') || 'Search attachments...'
          }
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
        />

        {/* Tab pills */}
        {!loading && data && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-md)' }}
          >
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  setShowTrash(false);
                  clearSelection();
                }}
                style={
                  !showTrash
                    ? {
                        background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                        borderColor: 'var(--accent-primary)',
                        color: 'var(--accent-primary)',
                        boxShadow: '0 0 0 1px var(--accent-primary)',
                      }
                    : undefined
                }
              >
                {t('common:attachments_active', { n: activeCount }) ||
                  `Attachments (${activeCount})`}
                <span style={{ marginLeft: 4, fontSize: 'var(--text-caption)', opacity: 0.7 }}>
                  {formatSize(activeBytes)}
                </span>
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  setShowTrash(true);
                  clearSelection();
                }}
                style={
                  showTrash
                    ? {
                        background: 'color-mix(in srgb, #e74c3c 10%, transparent)',
                        borderColor: '#e74c3c',
                        color: '#e74c3c',
                        boxShadow: '0 0 0 1px #e74c3c',
                      }
                    : undefined
                }
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
                {t('common:attachments_trash', { n: trashCount }) || `Trash (${trashCount})`}
                <span style={{ marginLeft: 4, fontSize: 'var(--text-caption)', opacity: 0.7 }}>
                  {formatSize(trashBytes)}
                </span>
              </Button>

              <div style={{ flex: 1 }} />

              <Button variant="secondary" size="sm" onClick={loadData}>
                <RotateCcw size={ICON_SIZE.sm} /> {t('common:refresh') || 'Refresh'}
              </Button>
            </div>

            {/* Summary card */}
            <Card style={{ padding: '12px 16px' }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 20,
                  fontSize: 'var(--text-sm)',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--accent-primary)' }} />
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('common:attachments')}</span>
                  <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                    {showTrash ? trashCount : activeCount}
                  </span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('common:size')}</span>
                  <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                    {formatSize(showTrash ? trashBytes : activeBytes)}
                  </span>
                </div>
                <div style={{ flex: 1 }} />
                <div style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
                  {t('settings:objects_count', { n: showTrash ? trashObjects : activeObjects })}
                </div>
              </div>
            </Card>

            {/* Batch toolbar */}
            <Card style={{ padding: '8px 14px' }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  fontSize: 'var(--text-sm)',
                }}
              >
                <div
                  onClick={() => handleSelectAll(allVisibleKeys)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    cursor: displayPages.length > 0 ? 'pointer' : 'default',
                    color: 'var(--text-secondary)',
                    userSelect: 'none',
                  }}
                >
                  <SelectCheckbox
                    checked={allSelected}
                    indeterminate={selectedIds.size > 0 && !allSelected}
                    disabled={displayPages.length === 0}
                  />
                  {allSelected ? t('common:deselect_all') : t('common:select_all')}
                </div>

                <div style={{ flex: 1 }} />

                <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
                  {t('common:selected_count', { n: selectedIds.size })}
                </span>

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
                      <RotateCcw size={ICON_SIZE.sm} /> {t('common:restore')}
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
            </Card>

            {/* Content */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.2 }}
            >
              {displayPages.length === 0 ? (
                <Card>
                  <div style={{ textAlign: 'center', padding: '48px 24px' }}>
                    <Paperclip
                      size={ICON_SIZE['5xl']}
                      style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
                    />
                    <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)' }}>
                      {searchQuery.trim()
                        ? t('common:no_search_results') || 'No matching attachments found.'
                        : showTrash
                          ? t('settings:trash_empty') || 'Trash is empty.'
                          : t('common:no_attachments') || 'No attachments found.'}
                    </p>
                  </div>
                </Card>
              ) : (
                <div
                  style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-md)' }}
                >
                  {displayPages.map(renderPage)}
                </div>
              )}
            </motion.div>
          </motion.div>
        )}
      </PageContainer>

      {/* Preview overlay */}
      <AttachmentPreviewOverlay item={previewItem} onClose={() => setPreviewItem(null)} />

      {/* Confirmation dialogs */}
      <ConfirmDialog
        open={batchRestoreConfirm}
        title={t('common:batch_restore_title') || 'Batch restore'}
        body={
          t('common:batch_restore_body', { n: selectedIds.size }) ||
          `Restore ${selectedIds.size} selected attachment(s) from trash?`
        }
        confirmLabel={t('common:restore')}
        cancelLabel={t('common:cancel')}
        confirmStyle="primary"
        onConfirm={handleBatchRestore}
        onCancel={() => setBatchRestoreConfirm(false)}
      />
      <ConfirmDialog
        open={batchDeleteConfirm}
        title={t('common:batch_delete_title') || 'Batch delete'}
        body={
          t('common:batch_delete_body', { n: selectedIds.size }) ||
          `Delete ${selectedIds.size} selected attachment(s)? They will be moved to trash.`
        }
        confirmLabel={t('common:delete')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleBatchDelete}
        onCancel={() => setBatchDeleteConfirm(false)}
      />
      <ConfirmDialog
        open={batchPermanentDeleteConfirm}
        title={t('common:batch_perm_delete_title') || 'Permanently delete selected?'}
        body={
          t('common:batch_perm_delete_body', { n: selectedIds.size }) ||
          `Permanently delete ${selectedIds.size} selected attachment(s)? This cannot be undone.`
        }
        confirmLabel={t('common:delete_permanently')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={handleBatchPermanentDelete}
        onCancel={() => setBatchPermanentDeleteConfirm(false)}
      />
      <ConfirmDialog
        open={!!permDeleteItem}
        title={t('common:perm_delete_title') || 'Permanently delete?'}
        body={
          permDeleteItem
            ? t('common:perm_delete_body', { name: truncateFileName(permDeleteItem.fileName) }) ||
              `Delete "${truncateFileName(permDeleteItem.fileName)}"? This cannot be undone.`
            : ''
        }
        confirmLabel={t('common:delete_permanently')}
        cancelLabel={t('common:cancel')}
        confirmStyle="danger"
        onConfirm={doPermanentDelete}
        onCancel={() => setPermDeleteItem(null)}
      />

      {confirmDialog}
    </AppShell>
  );
}
