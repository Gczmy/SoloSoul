import { motion } from 'framer-motion';
import { Images } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { AttachmentMetaEditDialog } from '@/components/attachment/AttachmentMetaEditDialog';
import { PhotoAlbumOverlay } from '@/components/attachment/PhotoAlbumOverlay';
import { AttachmentListItem } from '@/components/object/AttachmentListItem';
import { AttachmentViewerHeader } from '@/components/object/AttachmentViewerHeader';
import { AttachmentBatchToolbar } from '@/components/object/AttachmentBatchToolbar';
import { AttachmentConfirmDialogs } from '@/components/object/AttachmentConfirmDialogs';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';

import { useAttachmentViewer, type AttachmentViewerProps } from './useAttachmentViewer';

export type { AttachmentViewerProps } from './useAttachmentViewer';
export type { AttachmentItem } from '@/lib/attachmentUtils';

/**
 * 对象附件查看器 — P046 拆分后为纯展示组合层：
 * 全部编排逻辑（列表加载、上传/重命名/下载/转发/删除/恢复/永久删除、批量操作、
 * 照片集数据源、拖拽上传）收敛于 useAttachmentViewer 数据 hook；
 * 本组件仅负责 JSX 组合与子组件装配。
 */
export function AttachmentViewer(props: AttachmentViewerProps) {
  const {
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
    // 批量选择
    selectedIds,
    allSelected,
    allVisibleKeys,
    toggleSelect,
    handleSelectAll,
    clearSelection,
    batchDeleteConfirm,
    batchRestoreConfirm,
    batchPermanentDeleteConfirm,
    setBatchDeleteConfirm,
    setBatchRestoreConfirm,
    setBatchPermanentDeleteConfirm,
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
    handleBatchDelete,
    handleBatchRestore,
    handleBatchDownload,
    handleBatchPermanentDelete,
    // 确认对话框
    confirmDialog,
  } = useAttachmentViewer(props);

  const { onClose } = props;

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
                    onEditMeta={setMetaEditItem}
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
        onItemUpdated={handleMetaSaved}
      />
      {/* 附件描述/标签编辑对话框 */}
      {metaEditItem && (
        <AttachmentMetaEditDialog
          item={metaEditItem}
          onClose={() => setMetaEditItem(null)}
          onSaved={handleMetaSaved}
        />
      )}
      {/* Photo album overlay（对象级照片集） */}
      {photoAlbumOpen && displayPhotoItems.length > 0 && (
        <PhotoAlbumOverlay
          items={displayPhotoItems}
          onClose={() => setPhotoAlbumOpen(false)}
          onOpenExternal={openAttachmentExternal}
          onItemMetaUpdated={handleMetaSaved}
          // 相对查看器层级 +1：详情模态下查看器为 5100，固定 2100 会被其背景遮住
          zIndex={zIndex + 1}
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
