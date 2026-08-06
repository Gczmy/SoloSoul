import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Paperclip, Info, FolderTree, Upload, Trash2 } from 'lucide-react';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAttachmentManager } from '@/hooks/useAttachmentManager';
import { AttachmentToolbar } from '@/components/attachment/AttachmentToolbar';
import { AttachmentPageCard } from '@/components/attachment/AttachmentPageCard';
import { AttachmentPreviewOverlay } from '@/components/attachment/AttachmentPreviewOverlay';
import { ConfirmDialog } from '@/components/attachment/ConfirmDialog';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { ICON_SIZE } from '@/lib/constants';
import { truncateFileName } from '@/lib/attachmentUtils';

/** P011: 顶层页面列表单次渲染上限（「加载更多」步进量）。 */
const VISIBLE_PAGE_SIZE = 20;

/** 全局附件管理器页面（P024 拆分：编排层 + useAttachmentManager hook + 子组件）。 */
export function GlobalAttachmentManager() {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const navigate = useNavigate();
  const location = useLocation();

  const {
    confirmDialog,
    data,
    loading,
    showTrash,
    setShowTrash,
    expandedPages,
    expandedObjects,
    togglePage,
    toggleObject,
    previewItem,
    setPreviewItem,
    renamingId,
    setRenamingId,
    setRenameObjectId,
    permDeleteItem,
    setPermDeleteItem,
    searchQuery,
    setSearchQuery,
    loadData,
    openAttachmentExternal,
    handlePreview,
    handleStartRename,
    handleConfirmRename,
    handleUpload,
    handleSoftDelete,
    handleDownload,
    handleRestore,
    handlePermanentDelete,
    doPermanentDelete,
    displayPages,
    allVisibleKeys,
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
    handleBatchDownload,
    handleBatchDelete,
    handleBatchPermanentDelete,
    handleBatchRestore,
    summaryStats,
  } = useAttachmentManager();

  // P011: 顶层页面列表分页「加载更多」，避免页面/对象多时一次全量挂载。
  const [visiblePageLimit, setVisiblePageLimit] = useState(VISIBLE_PAGE_SIZE);
  // 搜索词或回收站视图切换时重置分页游标。
  useEffect(() => {
    setVisiblePageLimit(VISIBLE_PAGE_SIZE);
  }, [searchQuery, showTrash]);
  const visiblePages = displayPages.slice(0, visiblePageLimit);

  const attachmentGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_attachment_title') ?? 'Attachment Guide',
        steps: [
          {
            icon: FolderTree,
            title: t('common:guide_attachment_step1_title') ?? 'Browse Tree',
            description:
              t('common:guide_attachment_step1_desc') ??
              'Attachments are grouped by page and object. Expand pages and objects to find the files you need.',
          },
          {
            icon: Upload,
            title: t('common:guide_attachment_step2_title') ?? 'Upload & Download',
            description:
              t('common:guide_attachment_step2_desc') ??
              'Upload new attachments or download existing ones. On mobile, select files from your device.',
          },
          {
            icon: Trash2,
            title: t('common:guide_attachment_step3_title') ?? 'Delete & Restore',
            description:
              t('common:guide_attachment_step3_desc') ??
              'Soft delete attachments to move them to trash, restore them later, or permanently delete them.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_attachments') ?? 'Attachment Management',
            description:
              t('common:guide_help_attachments_desc') ??
              'Upload, download, rename, and manage attachments in trash',
            href: '/help?id=attachments',
          },
        ],
      },
    ],
    [t],
  );

  return (
    <AppShell
      title={t('settings:items.global_attachments', { defaultValue: 'Attachments' })}
      onBack={() => {
        const state = location.state as { from?: string } | undefined;
        if (state?.from === '/home') navigate('/home');
        else navigate('/settings');
      }}
      actions={<PageGuideButton pages={attachmentGuidePages} />}
    >
      <PageContainer variant="medium" gap="default">
        <AttachmentToolbar
          showTrash={showTrash}
          searchQuery={searchQuery}
          loading={loading}
          hasData={!!data}
          summary={summaryStats}
          visibleKeys={allVisibleKeys}
          selectedCount={selectedIds.size}
          allSelected={allSelected}
          onSearchChange={setSearchQuery}
          onShowActive={() => {
            setShowTrash(false);
            clearSelection();
          }}
          onShowTrash={() => {
            setShowTrash(true);
            clearSelection();
          }}
          onRefresh={loadData}
          onSelectAll={() => handleSelectAll(allVisibleKeys)}
          onBatchDownload={handleBatchDownload}
          onBatchDelete={() => setBatchDeleteConfirm(true)}
          onBatchRestore={() => setBatchRestoreConfirm(true)}
          onBatchPermanentDelete={() => setBatchPermanentDeleteConfirm(true)}
        />

        {/* Content */}
        {!loading && data && (
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
                      ? t('common:no_search_results', { defaultValue: 'No matching attachments found.' })
                      : showTrash
                        ? t('settings:trash_empty', { defaultValue: 'Trash is empty.' })
                        : t('common:no_attachments', { defaultValue: 'No attachments found.' })}
                  </p>
                </div>
              </Card>
            ) : (
              <div
                style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-md)' }}
              >
                {visiblePages.map((page) => {
                  const pageKey = page.pageId || page.pageName;
                  return (
                    <AttachmentPageCard
                      key={pageKey}
                      page={page}
                      pageKey={pageKey}
                      isExpanded={expandedPages.has(pageKey)}
                      showTrash={showTrash}
                      selectedIds={selectedIds}
                      renamingId={renamingId}
                      expandedObjects={expandedObjects}
                      onToggle={() => togglePage(pageKey)}
                      onToggleObject={toggleObject}
                      onUpload={handleUpload}
                      loadData={loadData}
                      onToggleSelect={toggleSelect}
                      onRenameConfirm={handleConfirmRename}
                      onRenameCancel={() => {
                        setRenamingId(null);
                        setRenameObjectId('');
                      }}
                      onPreview={handlePreview}
                      onStartRename={handleStartRename}
                      onDownload={handleDownload}
                      onSoftDelete={handleSoftDelete}
                      onRestore={handleRestore}
                      onPermanentDelete={handlePermanentDelete}
                    />
                  );
                })}
                {displayPages.length > visiblePageLimit && (
                  <Button
                    variant="tertiary"
                    size="sm"
                    onClick={() => setVisiblePageLimit((n) => n + VISIBLE_PAGE_SIZE)}
                    style={{ marginTop: 4 }}
                  >
                    {t('load_more', { defaultValue: '加载更多' })}
                  </Button>
                )}
              </div>
            )}
          </motion.div>
        )}
      </PageContainer>

      {/* Preview overlay */}
      <AttachmentPreviewOverlay
        item={previewItem}
        onClose={() => setPreviewItem(null)}
        onOpenExternal={openAttachmentExternal}
      />

      {/* Confirmation dialogs */}
      <ConfirmDialog
        open={batchRestoreConfirm}
        title={t('common:batch_restore_title', { defaultValue: 'Batch restore' })}
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
        title={t('common:batch_delete_title', { defaultValue: 'Batch delete' })}
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
        title={t('common:batch_perm_delete_title', { defaultValue: 'Permanently delete selected?' })}
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
        title={t('common:perm_delete_title', { defaultValue: 'Permanently delete?' })}
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
