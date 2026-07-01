import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '@/stores/settingsStore';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SnapshotVersionBadge } from '@/components/ui/SnapshotVersionBadge';
import {
  X,
  RotateCcw,
  ChevronLeft,
  ChevronRight,
  Image,
  FileText,
  Paperclip,
  FolderOpen,
  ArrowLeft,
} from 'lucide-react';
import { motion } from 'framer-motion';
import { formatBytes } from '@/lib/format';
import type { PropertyType, SensitivityLevel, UserTemplate } from '@/types/template';
import type { TrashDetail, SnapshotEntry, TrashAttachment, TrashChildSummary } from './types';
import { ICON_SIZE } from '@/lib/iconSizes';

/** Truncate long file names preserving extension: 'abcdefg…-.pdf' */
function truncateFileName(fileName: string, maxLen: number = 28): string {
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

interface TrashDetailPanelProps {
  detailItem: TrashDetail;
  detailTemplate: UserTemplate | null;
  onClose: () => void;
  onRequestRestore: (id: string) => void;
  onRequestDelete: (id: string) => void;
}

/** Shared content block reused for the main item and drilled-down child item. */
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
  const { t } = useTranslation(['settings', 'common', 'editor', 'navigation']);
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
        snapshotId,
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

  const activeAttachments: TrashAttachment[] = item.attachments;
  const deletedAttachments: TrashAttachment[] = item.deletedAttachments;

  return (
    <>
      {/* Back button (child drill-down) */}
      {showBackButton && onBack && (
        <button
          onClick={onBack}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = 'var(--accent-primary)';
            e.currentTarget.style.background =
              'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = 'var(--text-secondary)';
            e.currentTarget.style.background = 'none';
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            background: 'none',
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            padding: '4px 8px',
            color: 'var(--text-secondary)',
            fontSize: 'var(--text-body-sm)',
            fontFamily: 'inherit',
            transition: 'background 0.15s, color 0.15s',
            marginBottom: 8,
          }}
        >
          <ArrowLeft size={ICON_SIZE.sm} />
          {t('common:back', { defaultValue: 'Back' })}
        </button>
      )}

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: 16,
        }}
      >
        <div>
          <h3
            style={{
              fontSize: 'var(--text-section-title)',
              fontWeight: 600,
              margin: 0,
              overflowWrap: 'break-word',
              wordBreak: 'break-word',
            }}
          >
            {item.name}
          </h3>
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t(`settings:trash_type_${item.itemType}`)}
          </span>
        </div>
        <button
          onClick={onClose}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = 'var(--accent-primary)';
            e.currentTarget.style.background =
              'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = 'var(--text-tertiary)';
            e.currentTarget.style.background = 'none';
          }}
          style={{
            background: 'none',
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            padding: 4,
            color: 'var(--text-tertiary)',
            transition: 'background 0.15s, color 0.15s',
          }}
        >
          <X size={ICON_SIZE.lg} />
        </button>
      </div>

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          fontSize: 'var(--text-body-sm)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:delete_time')}</span>
          <span>{new Date(item.deletedAt).toLocaleString()}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:original_location')}</span>
          <span>
            {resolveCollectionLabel(item.sectionType || item.originalLocation, customPages, t)}
          </span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:remaining_retention')}</span>
          <span>
            {item.remainingDays != null
              ? t('settings:trash_expires_in', { days: item.remainingDays })
              : t('settings:never_delete')}
          </span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:deleted_by')}</span>
          <span>
            {item.deletedBy === 'user'
              ? t('settings:deleted_by_user')
              : t('settings:deleted_by_system')}
          </span>
        </div>
      </div>

      {item.previewProperties.length > 0 && (
        <div
          style={{
            marginTop: 16,
            borderTop: '1px solid var(--border-subtle)',
            paddingTop: 12,
          }}
        >
          <h4 style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, marginBottom: 8 }}>
            {t('settings:content_preview')}
          </h4>
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 6,
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
            }}
          >
            {item.previewProperties.map((p, i) => {
              const propType = (p as Record<string, unknown>).type as PropertyType | undefined;
              const sensitivity = (p as Record<string, unknown>).sensitivityLevel as
                | SensitivityLevel
                | undefined;
              const typeLabel = propType
                ? t(`editor:field_types.${propType}`, propType)
                : String(p.value);
              return (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  {propType && <FieldTypeIcon type={propType} size={ICON_SIZE.sm} />}
                  <span style={{ fontWeight: 500, flexShrink: 0 }}>{p.key}</span>
                  {sensitivity && <SensitivityBadge level={sensitivity} />}
                  <span
                    style={{
                      color: 'var(--text-tertiary)',
                      marginLeft: 'auto',
                      flexShrink: 0,
                    }}
                  >
                    {typeLabel}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {item.itemType !== 'template' && (
        <>
          {/* Attachments section */}
          <div
            style={{
              marginTop: 12,
              borderTop: '1px solid var(--border-subtle)',
              paddingTop: 10,
            }}
          >
            <div
              onClick={() => toggleSection('attachments')}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                cursor: 'pointer',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 600,
                userSelect: 'none',
              }}
            >
              <span
                style={{
                  transform: expandedSections.attachments ? 'rotate(90deg)' : 'none',
                  transition: 'transform 0.15s',
                  fontSize: 'var(--text-badge)',
                }}
              >
                ▶
              </span>
              {t('common:attachments')} ({activeAttachments.length + deletedAttachments.length})
            </div>
            {expandedSections.attachments && (
              <div style={{ marginTop: 8 }}>
                {deletedAttachments.length > 0 && (
                  <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
                    <button
                      onClick={() => setShowTrashAttachments(false)}
                      onMouseEnter={
                        !showTrashAttachments
                          ? undefined
                          : (e) => {
                              e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                            }
                      }
                      onMouseLeave={
                        !showTrashAttachments
                          ? undefined
                          : (e) => {
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                            }
                      }
                      style={{
                        padding: '4px 10px',
                        borderRadius: 6,
                        fontSize: 'var(--text-badge)',
                        fontWeight: 500,
                        border: showTrashAttachments
                          ? '1px solid var(--border-subtle)'
                          : '1px solid var(--accent-primary)',
                        background: showTrashAttachments
                          ? 'var(--bg-toolbar)'
                          : 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                        color: showTrashAttachments
                          ? 'var(--text-primary)'
                          : 'var(--accent-primary)',
                        boxShadow: showTrashAttachments
                          ? 'none'
                          : '0 0 0 1px var(--accent-primary)',
                        cursor: 'pointer',
                        transition:
                          'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                      }}
                    >
                      {t('common:active')} ({activeAttachments.length})
                    </button>
                    <button
                      onClick={() => setShowTrashAttachments(true)}
                      onMouseEnter={
                        showTrashAttachments
                          ? undefined
                          : (e) => {
                              e.currentTarget.style.borderColor = '#e74c3c';
                              e.currentTarget.style.background =
                                'color-mix(in srgb, #e74c3c 10%, transparent)';
                            }
                      }
                      onMouseLeave={
                        showTrashAttachments
                          ? undefined
                          : (e) => {
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                            }
                      }
                      style={{
                        padding: '4px 10px',
                        borderRadius: 6,
                        fontSize: 'var(--text-badge)',
                        fontWeight: 500,
                        border: showTrashAttachments
                          ? '1px solid #e74c3c'
                          : '1px solid var(--border-subtle)',
                        background: showTrashAttachments
                          ? 'color-mix(in srgb, #e74c3c 10%, transparent)'
                          : 'var(--bg-toolbar)',
                        color: showTrashAttachments ? '#e74c3c' : 'var(--text-primary)',
                        boxShadow: showTrashAttachments ? '0 0 0 1px #e74c3c' : 'none',
                        cursor: 'pointer',
                        transition:
                          'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                      }}
                    >
                      {t('common:trash')} ({deletedAttachments.length})
                    </button>
                  </div>
                )}
                {(showTrashAttachments ? deletedAttachments : activeAttachments).length === 0 ? (
                  <p
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      padding: '8px 0',
                    }}
                  >
                    {t('common:no_data')}
                  </p>
                ) : (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                    {(showTrashAttachments ? deletedAttachments : activeAttachments).map((a) => {
                      const ext = a.fileName.split('.').pop()?.toLowerCase() || '';
                      const isImage =
                        a.mimeType.startsWith('image/') ||
                        ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext);
                      const isPdf = a.mimeType === 'application/pdf' || ext === 'pdf';
                      const AttachIcon = isImage ? Image : isPdf ? FileText : Paperclip;
                      const iconColor = 'var(--text-tertiary)';
                      return (
                        <div
                          key={a.id}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.background =
                              'color-mix(in srgb, var(--accent-primary) 6%, transparent)';
                            e.currentTarget.style.borderColor =
                              'color-mix(in srgb, var(--accent-primary) 20%, transparent)';
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.background = 'var(--bg-elevated-hover)';
                            e.currentTarget.style.borderColor = 'transparent';
                          }}
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 8,
                            fontSize: 'var(--text-caption)',
                            padding: '8px 10px',
                            background: 'var(--bg-elevated-hover)',
                            borderRadius: 6,
                            border: '1px solid transparent',
                            cursor: 'default',
                            transition: 'background 0.15s, border-color 0.15s',
                          }}
                        >
                          <AttachIcon
                            size={ICON_SIZE.md}
                            style={{ color: iconColor, flexShrink: 0 }}
                          />
                          <div style={{ flex: 1, minWidth: 0 }}>
                            <div
                              style={{
                                fontWeight: 500,
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                                textDecoration: showTrashAttachments ? 'line-through' : 'none',
                              }}
                            >
                              {truncateFileName(a.fileName)}
                            </div>
                            <div
                              style={{
                                color: 'var(--text-tertiary)',
                                fontSize: 'var(--text-badge)',
                              }}
                            >
                              {formatBytes(a.sizeBytes)} ·{' '}
                              {new Date(a.createdAt).toLocaleDateString()}
                            </div>
                          </div>
                          <span
                            style={{
                              fontSize: 'var(--text-badge)',
                              padding: '2px 6px',
                              borderRadius: 4,
                              fontWeight: 500,
                              background:
                                'color-mix(in srgb, var(--text-tertiary) 10%, transparent)',
                              color: 'var(--text-tertiary)',
                              flexShrink: 0,
                              textDecoration: 'none',
                            }}
                          >
                            {ext.toUpperCase() || 'FILE'}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Snapshots section */}
          <div
            style={{
              marginTop: 12,
              borderTop: '1px solid var(--border-subtle)',
              paddingTop: 10,
            }}
          >
            <div
              onClick={() => toggleSection('snapshots')}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                cursor: 'pointer',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 600,
                userSelect: 'none',
                ...(expandedSections.snapshots
                  ? {}
                  : {
                      borderBottom: '1px solid var(--border-subtle)',
                      paddingBottom: 10,
                      marginBottom: 10,
                    }),
              }}
            >
              <span
                style={{
                  transform: expandedSections.snapshots ? 'rotate(90deg)' : 'none',
                  transition: 'transform 0.15s',
                  fontSize: 'var(--text-badge)',
                }}
              >
                ▶
              </span>
              {t('settings:data_snapshots')} ({item.snapshots.length})
            </div>
            {expandedSections.snapshots && (
              <SnapshotContent
                _detailId={item.id}
                snapshots={item.snapshots}
                currentSnapIdx={currentSnapIdx}
                data={historySnapData[item.id]}
                loading={historySnapLoading[item.id]}
                detailTemplate={detailTemplate}
                onChangeSnapshot={(newIdx) => changeSnapshot(item.id, item.snapshots, newIdx)}
              />
            )}
          </div>
        </>
      )}

      <div style={{ marginTop: 16, display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
        <Button
          size="sm"
          variant="tertiary"
          onClick={() => {
            onRequestRestore(item.id);
            onClose();
          }}
        >
          <RotateCcw size={ICON_SIZE.xs} style={{ marginRight: 4 }} /> {t('common:restore')}
        </Button>
        <DeleteButton
          onClick={() => {
            onRequestDelete(item.id);
            onClose();
          }}
          title={t('common:delete_permanently')}
        >
          {t('common:delete_permanently')}
        </DeleteButton>
      </div>
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
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background =
                          'color-mix(in srgb, var(--accent-primary) 8%, transparent)';
                        e.currentTarget.style.borderColor =
                          'color-mix(in srgb, var(--accent-primary) 25%, transparent)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'var(--bg-elevated-hover)';
                        e.currentTarget.style.borderColor = 'transparent';
                      }}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '8px 10px',
                        borderRadius: 6,
                        border: '1px solid transparent',
                        background: 'var(--bg-elevated-hover)',
                        cursor: 'pointer',
                        fontFamily: 'inherit',
                        fontSize: 'var(--text-body-sm)',
                        color: 'var(--text-primary)',
                        textAlign: 'left',
                        width: '100%',
                        transition: 'background 0.15s, border-color 0.15s',
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

interface SnapshotContentProps {
  _detailId: string;
  snapshots: SnapshotEntry[];
  currentSnapIdx: number;
  data: Record<string, unknown> | null | undefined;
  loading: boolean | undefined;
  detailTemplate: UserTemplate | null;
  onChangeSnapshot: (newIdx: number) => void;
}

function SnapshotContent({
  _detailId,
  snapshots,
  currentSnapIdx,
  data,
  loading,
  detailTemplate,
  onChangeSnapshot,
}: SnapshotContentProps) {
  const { t } = useTranslation(['settings', 'common', 'editor']);
  // Clamp index to prevent out-of-bounds when snapshots array changes after mount
  const clampedIdx = Math.min(currentSnapIdx, Math.max(0, snapshots.length - 1));
  const currentSnap = snapshots[clampedIdx];

  return (
    <div style={{ marginTop: 8, fontSize: 'var(--text-caption)' }}>
      {snapshots.length > 1 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          <button
            disabled={clampedIdx >= snapshots.length - 1}
            onClick={() => onChangeSnapshot(clampedIdx + 1)}
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              border: '1px solid var(--border-subtle)',
              borderRadius: 6,
              cursor: clampedIdx >= snapshots.length - 1 ? 'default' : 'pointer',
              fontSize: 'var(--text-badge)',
              background: 'transparent',
              color:
                clampedIdx >= snapshots.length - 1
                  ? 'var(--text-tertiary)'
                  : 'var(--text-secondary)',
              opacity: clampedIdx >= snapshots.length - 1 ? 0.35 : 1,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              if (clampedIdx < snapshots.length - 1) {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color =
                clampedIdx >= snapshots.length - 1
                  ? 'var(--text-tertiary)'
                  : 'var(--text-secondary)';
            }}
          >
            <ChevronLeft size={ICON_SIZE.sm} />
          </button>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 'var(--text-badge)',
              fontWeight: 500,
              color: 'var(--text-secondary)',
            }}
          >
            <span
              style={{
                color: 'var(--accent-primary)',
                fontWeight: 600,
                minWidth: 14,
                textAlign: 'center',
              }}
            >
              {clampedIdx + 1}
            </span>
            <span style={{ color: 'var(--text-tertiary)' }}>/</span>
            <span style={{ color: 'var(--text-tertiary)' }}>{snapshots.length}</span>
          </div>
          <button
            disabled={clampedIdx <= 0}
            onClick={() => onChangeSnapshot(Math.max(0, clampedIdx - 1))}
            style={{
              width: 28,
              height: 28,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              border: '1px solid var(--border-subtle)',
              borderRadius: 6,
              cursor: clampedIdx <= 0 ? 'default' : 'pointer',
              fontSize: 'var(--text-badge)',
              background: 'transparent',
              color: clampedIdx <= 0 ? 'var(--text-tertiary)' : 'var(--text-secondary)',
              opacity: clampedIdx <= 0 ? 0.35 : 1,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              if (clampedIdx > 0) {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color =
                clampedIdx <= 0 ? 'var(--text-tertiary)' : 'var(--text-secondary)';
            }}
          >
            <ChevronRight size={ICON_SIZE.sm} />
          </button>
        </div>
      )}
      {currentSnap && (
        <div>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              padding: '6px 8px',
              background: 'var(--bg-elevated-hover)',
              borderRadius: 6,
              marginBottom: 6,
              minHeight: 32,
            }}
          >
            <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
              {currentSnap.diffSummary &&
                !(
                  snapshots.length > 2 &&
                  clampedIdx === snapshots.length - 1 &&
                  currentSnap.diffSummary === 'Created'
                ) && (
                  <span
                    style={{
                      padding: '2px 6px',
                      borderRadius: 4,
                      fontSize: 'var(--text-badge)',
                      fontWeight: 500,
                      background: 'rgba(91,124,153,0.08)',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    {t(`common:diff_${currentSnap.diffSummary}`, currentSnap.diffSummary)}
                  </span>
                )}
              <SnapshotVersionBadge index={currentSnapIdx} total={snapshots.length} />
              <span
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  fontSize: 'var(--text-badge)',
                  fontWeight: 500,
                  background: 'rgba(91,124,153,0.08)',
                  color: 'var(--accent-primary)',
                }}
              >
                {t(`common:trigger_${currentSnap.triggeredBy}`, currentSnap.triggeredBy)}
              </span>
            </div>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginLeft: 'auto',
              }}
            >
              {new Date(currentSnap.timestamp).toLocaleString()}
            </span>
          </div>
          <div style={{ minHeight: 60 }}>
            {loading && !data && <LoadingPlaceholder variant="base" minHeight={60} />}
            {data && <SnapshotDataView data={data} detailTemplate={detailTemplate} />}
          </div>
        </div>
      )}
    </div>
  );
}

interface SnapshotDataViewProps {
  data: Record<string, unknown>;
  detailTemplate: UserTemplate | null;
}

function SnapshotDataView({ data, detailTemplate }: SnapshotDataViewProps) {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { t } = useTranslation(['editor']);
  const rawProps = data.properties as Record<string, unknown> | undefined;
  const tags: string[] = Array.isArray(data.tags) ? (data.tags as string[]) : [];
  const snapName = typeof data.name === 'string' ? data.name : '';

  const orderedFields: {
    key: string;
    value: string;
    type?: PropertyType;
    sensitivityLevel?: SensitivityLevel;
  }[] = [];

  if (rawProps && typeof rawProps === 'object' && detailTemplate) {
    for (const p of detailTemplate.properties) {
      const v = rawProps[p.id];
      if (v !== null && v !== undefined && v !== '' && !String(p.id).startsWith('__')) {
        orderedFields.push({
          key: p.name,
          value: typeof v === 'string' ? v : JSON.stringify(v),
          type: p.type,
          sensitivityLevel: (p.sensitivityLevel || 'internal') as SensitivityLevel,
        });
      }
    }
    const known = new Set(detailTemplate.properties.map((p) => p.id));
    for (const [k, v] of Object.entries(rawProps)) {
      if (!k.startsWith('__') && !known.has(k) && v !== null && v !== undefined && v !== '') {
        orderedFields.push({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        });
      }
    }
  } else if (rawProps && typeof rawProps === 'object') {
    for (const [k, v] of Object.entries(rawProps)) {
      if (!k.startsWith('__') && v !== null && v !== undefined && v !== '') {
        orderedFields.push({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        });
      }
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      {snapName && (
        <div
          style={{
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            textAlign: 'right',
            overflowWrap: 'break-word',
            wordBreak: 'break-word',
          }}
        >
          {snapName}
        </div>
      )}
      {orderedFields.slice(0, 8).map((f) => (
        <div
          key={f.key}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            fontSize: 'var(--text-caption)',
            padding: '3px 0',
            borderBottom: '1px solid var(--border-subtle)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            {f.type && <FieldTypeIcon type={f.type} size={ICON_SIZE.sm} />}
            <span style={{ fontWeight: 500, color: 'var(--text-secondary)' }}>{f.key}</span>
            {f.sensitivityLevel && <SensitivityBadge level={f.sensitivityLevel} />}
          </div>
          <span style={{ color: 'var(--text-primary)', marginLeft: 'auto', textAlign: 'right' }}>
            {f.value}
          </span>
        </div>
      ))}
      {tags.length > 0 && (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
          {tags.map((tag) => (
            <span
              key={tag}
              style={{
                padding: '1px 7px',
                borderRadius: 10,
                fontSize: 'var(--text-badge)',
                background: 'rgba(91,124,153,0.08)',
                color: 'var(--accent-primary)',
                fontWeight: 500,
              }}
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
