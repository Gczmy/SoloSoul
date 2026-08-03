// ============================================================
// TrashDetailPanel.tsx — 回收站详情模态外壳 + 内容编排
// P224-①：ObjectDetailContent 的 6 个渲染区块已拆至
// ./TrashDetailSections.tsx（纯展示子组件），历史快照展示域移至
// ./TrashSnapshotView.tsx。本文件仅保留模态外壳与状态编排。
// ============================================================

import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useSettingsStore } from '@/stores/settingsStore';
import { motion } from 'framer-motion';
import { FolderOpen } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ICON_SIZE } from '@/lib/constants';
import type { UserTemplate } from '@/types/template';
import type { TrashDetail, SnapshotEntry, TrashChildSummary } from './types';
import {
  TrashDetailHeader,
  TrashMetaInfo,
  TrashFieldList,
  TrashAttachmentsSection,
  SnapshotSummaryRow,
  TrashDetailActions,
} from './TrashDetailSections';

interface TrashDetailPanelProps {
  detailItem: TrashDetail;
  detailTemplate: UserTemplate | null;
  onClose: () => void;
  onRequestRestore: (id: string) => void;
  onRequestDelete: (id: string) => void;
}

function ObjectDetailContent({
  item,
  detailTemplate,
  onClose,
  onRequestRestore,
  onRequestDelete,
  showBackButton,
  onBack,
}: {
  item: TrashDetail;
  detailTemplate: UserTemplate | null;
  onClose: () => void;
  onRequestRestore: (id: string) => void;
  onRequestDelete: (id: string) => void;
  showBackButton?: boolean;
  onBack?: () => void;
}) {
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({});
  const [showTrashAttachments, setShowTrashAttachments] = useState(false);
  const [historySnapIndex, setHistorySnapIndex] = useState<Record<string, number>>({});
  const [historySnapData, setHistorySnapData] = useState<
    Record<string, Record<string, unknown> | null>
  >({});
  const [historySnapLoading, setHistorySnapLoading] = useState<Record<string, boolean>>({});

  const toggleSection = (key: string) => {
    setExpandedSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const loadSnapshotData = useCallback(async (detailId: string, snapshotId: string) => {
    setHistorySnapLoading((prev) => ({ ...prev, [detailId]: true }));
    try {
      const data = await invoke<Record<string, unknown> | null>('snapshot_get_data', {
        snapshotId: snapshotId,
      });
      setHistorySnapData((prev) => ({ ...prev, [detailId]: data }));
    } catch {
      setHistorySnapData((prev) => ({ ...prev, [detailId]: null }));
    } finally {
      setHistorySnapLoading((prev) => ({ ...prev, [detailId]: false }));
    }
  }, []);

  const changeSnapshot = (detailId: string, snapshots: SnapshotEntry[], newIdx: number) => {
    setHistorySnapIndex((prev) => ({ ...prev, [detailId]: newIdx }));
    if (snapshots[newIdx]) {
      loadSnapshotData(detailId, snapshots[newIdx].id);
    }
  };

  const currentSnapIdx = historySnapIndex[item.id] ?? 0;

  // Load first snapshot when mounted; skip if already cached
  useEffect(() => {
    if (item.snapshots.length > 0 && !historySnapData[item.id]) {
      loadSnapshotData(item.id, item.snapshots[0].id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item.id, item.snapshots, loadSnapshotData]);

  return (
    <>
      <TrashDetailHeader
        item={item}
        onClose={onClose}
        showBackButton={showBackButton}
        onBack={onBack}
      />
      <TrashMetaInfo item={item} customPages={customPages} />
      <TrashFieldList item={item} />
      {item.itemType !== 'template' && (
        <>
          <TrashAttachmentsSection
            activeAttachments={item.attachments}
            deletedAttachments={item.deletedAttachments}
            expanded={!!expandedSections.attachments}
            showTrash={showTrashAttachments}
            onToggle={() => toggleSection('attachments')}
            onSetShowTrash={setShowTrashAttachments}
          />
          <SnapshotSummaryRow
            item={item}
            expanded={!!expandedSections.snapshots}
            onToggle={() => toggleSection('snapshots')}
            currentSnapIdx={currentSnapIdx}
            data={historySnapData[item.id]}
            loading={historySnapLoading[item.id]}
            detailTemplate={detailTemplate}
            onChangeSnapshot={(newIdx) => changeSnapshot(item.id, item.snapshots, newIdx)}
          />
        </>
      )}
      <TrashDetailActions
        onRestore={() => {
          onRequestRestore(item.id);
          onClose();
        }}
        onDelete={() => {
          onRequestDelete(item.id);
          onClose();
        }}
      />
    </>
  );
}

export function TrashDetailPanel({
  detailItem,
  detailTemplate,
  onClose,
  onRequestRestore,
  onRequestDelete,
}: TrashDetailPanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [, setViewingChildId] = useState<string | null>(null);
  const [childDetail, setChildDetail] = useState<TrashDetail | null>(null);
  const [childLoading, setChildLoading] = useState(false);

  const handleViewChild = useCallback(async (child: TrashChildSummary) => {
    setViewingChildId(child.id);
    setChildLoading(true);
    try {
      const detail = await invoke<TrashDetail>('trash_get_detail', { trashId: child.id });
      setChildDetail(detail);
    } catch {
      setChildDetail(null);
    } finally {
      setChildLoading(false);
    }
  }, []);

  const handleBackToParent = useCallback(() => {
    setViewingChildId(null);
    setChildDetail(null);
  }, []);

  const showChildren = detailItem.itemType === 'page' && detailItem.childItems.length > 0;

  return (
    <>
      <div
        style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)', zIndex: 99 }}
        onClick={onClose}
      />
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        style={{
          position: 'fixed',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: 380,
          maxHeight: '80vh',
          overflowY: 'auto',
          zIndex: 100,
          background: 'var(--bg-elevated)',
          borderRadius: 12,
          padding: 24,
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
        }}
      >
        {/* Child detail loading */}
        {childLoading && (
          <div style={{ padding: '24px 0' }}>
            <LoadingPlaceholder variant="base" minHeight={60} />
          </div>
        )}

        {/* Child detail view */}
        {!childLoading && childDetail && (
          <ObjectDetailContent
            item={childDetail}
            detailTemplate={detailTemplate}
            onClose={onClose}
            onRequestRestore={onRequestRestore}
            onRequestDelete={onRequestDelete}
            showBackButton
            onBack={handleBackToParent}
          />
        )}

        {/* Main item view (hidden when drilling into child) */}
        {!childLoading && !childDetail && (
          <>
            <ObjectDetailContent
              item={detailItem}
              detailTemplate={detailTemplate}
              onClose={onClose}
              onRequestRestore={onRequestRestore}
              onRequestDelete={onRequestDelete}
            />

            {/* Child objects list for page-type items */}
            {showChildren && (
              <div
                style={{
                  marginTop: 16,
                  borderTop: '1px solid var(--border-subtle)',
                  paddingTop: 12,
                }}
              >
                <h4
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 600,
                    marginBottom: 8,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  <FolderOpen size={ICON_SIZE.sm} />
                  {t('settings:page_contains_objects', { defaultValue: '页面包含的对象' })} (
                  {detailItem.childItems.length})
                </h4>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  {detailItem.childItems.map((child) => (
                    <button
                      key={child.id}
                      onClick={() => handleViewChild(child)}
                      className="interactive-row"
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '8px 10px',
                        borderRadius: 6,
                        borderWidth: 1,
                        borderStyle: 'solid',
                        cursor: 'pointer',
                        fontFamily: 'inherit',
                        fontSize: 'var(--text-body-sm)',
                        color: 'var(--text-primary)',
                        textAlign: 'left',
                        width: '100%',
                      }}
                    >
                      <span
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: '50%',
                          background: 'var(--accent-primary)',
                          flexShrink: 0,
                        }}
                      />
                      <span
                        style={{
                          flex: 1,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {child.name}
                      </span>
                      <span
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: 'var(--text-tertiary)',
                          flexShrink: 0,
                        }}
                      >
                        ▶
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </motion.div>
    </>
  );
}
