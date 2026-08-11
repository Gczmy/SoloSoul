import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, ChevronDown } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { DEFAULT_CUSTOM_ICON, PAGE_ICON_MAP, CUSTOM_ICON_MAP } from '@/lib/pageIcons';
import { AttachmentObjectGroup } from '@/components/attachment/AttachmentObjectGroup';
import type { PageIconKey, CustomIconId } from '@/lib/pageIcons';
import type { LucideIcon } from 'lucide-react';
import type {
  AttachmentMeta,
  AttachmentTreePage,
} from '@/components/attachment/attachmentManagerTypes';

/** Resolve icon from either PAGE_ICON_MAP (built-in) or CUSTOM_ICON_MAP (user-selectable). */
function resolvePageIcon(iconKey?: string | null): LucideIcon {
  const id = iconKey || DEFAULT_CUSTOM_ICON;
  if (id in PAGE_ICON_MAP) return PAGE_ICON_MAP[id as PageIconKey];
  if (id in CUSTOM_ICON_MAP) return CUSTOM_ICON_MAP[id as CustomIconId];
  return CUSTOM_ICON_MAP[DEFAULT_CUSTOM_ICON];
}

interface AttachmentPageCardProps {
  page: AttachmentTreePage;
  pageKey: string;
  isExpanded: boolean;
  showTrash: boolean;
  /** 当前选中附件复合键集合（`objectId::attachmentId`）。 */
  selectedIds: Set<string>;
  /** 当前重命名附件 ID。 */
  renamingId: string | null;
  /** 当前展开对象键集合（`pageKey::objectId`）。 */
  expandedObjects: Set<string>;
  onToggle: () => void;
  onToggleObject: (key: string) => void;
  onUpload: (objectId: string) => void;
  loadData: () => void;
  onToggleSelect: (compositeKey: string) => void;
  onRenameConfirm: (newName: string) => void;
  onRenameCancel: () => void;
  onPreview: (item: AttachmentMeta) => void;
  onStartRename: (item: AttachmentMeta, objectId: string) => void;
  onDownload: (item: AttachmentMeta) => void;
  onShare: (item: AttachmentMeta) => void;
  /** 编辑描述与标签 */
  onEditMeta?: (item: AttachmentMeta, objectId: string) => void;
  onSoftDelete: (item: AttachmentMeta, objectId: string) => void;
  onRestore: (item: AttachmentMeta, objectId: string) => void;
  onPermanentDelete: (item: AttachmentMeta, objectId: string) => void;
}

/** 页面分组卡片，展开时渲染其下的对象分组。 */
function AttachmentPageCardBase({
  page,
  pageKey,
  isExpanded,
  showTrash,
  selectedIds,
  renamingId,
  expandedObjects,
  onToggle,
  onToggleObject,
  onUpload,
  loadData,
  onToggleSelect,
  onRenameConfirm,
  onRenameCancel,
  onPreview,
  onStartRename,
  onDownload,
  onShare,
  onEditMeta,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentPageCardProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const PageIconComp = resolvePageIcon(page.pageIcon);
  const isMobile = isMobilePlatformSync();

  const totalAttachments = page.objects.reduce((sum, o) => sum + o.attachments.length, 0);
  const totalBytes = page.objects.reduce(
    (sum, o) => sum + o.attachments.reduce((s, a) => s + a.sizeBytes, 0),
    0,
  );
  const pageLabel = page.pageId ? page.pageName : t(`navigation:${page.pageName}`);

  return (
    <Card style={{ padding: 0, overflow: 'hidden' }}>
      <div
        onClick={onToggle}
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
        {isMobile ? (
          // 移动端：第1行 页面名+展开箭头，第2行 统计信息
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span
                style={{
                  flex: 1,
                  minWidth: 0,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {pageLabel}
              </span>
              {isExpanded ? (
                <ChevronDown size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
              ) : (
                <ChevronRight size={ICON_SIZE.sm} style={{ flexShrink: 0 }} />
              )}
            </div>
            <div
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                marginTop: 2,
              }}
            >
              {t('settings:objects_count', { n: page.objects.length })} ·{' '}
              {t('settings:attachments_count', { n: totalAttachments })} · {formatBytes(totalBytes)}
            </div>
          </div>
        ) : (
          // 桌面端：单行布局
          <>
            <span style={{ flex: 1 }}>{pageLabel}</span>
            <span
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                whiteSpace: 'nowrap',
              }}
            >
              {t('settings:objects_count', { n: page.objects.length })} ·{' '}
              {t('settings:attachments_count', { n: totalAttachments })} · {formatBytes(totalBytes)}
            </span>
            {isExpanded ? (
              <ChevronDown size={ICON_SIZE.sm} />
            ) : (
              <ChevronRight size={ICON_SIZE.sm} />
            )}
          </>
        )}
      </div>
      {isExpanded && (
        <>
          {page.objects.map((obj) => {
            const objKey = `${pageKey}::${obj.objectId}`;
            return (
              <AttachmentObjectGroup
                key={obj.objectId}
                obj={obj}
                isExpanded={expandedObjects.has(objKey)}
                showTrash={showTrash}
                selectedIds={selectedIds}
                renamingId={renamingId}
                loadData={loadData}
                onToggle={() => onToggleObject(objKey)}
                onUpload={onUpload}
                onToggleSelect={onToggleSelect}
                onRenameConfirm={onRenameConfirm}
                onRenameCancel={onRenameCancel}
                onPreview={onPreview}
                onStartRename={onStartRename}
                onDownload={onDownload}
                onShare={onShare}
                onEditMeta={onEditMeta}
                onSoftDelete={onSoftDelete}
                onRestore={onRestore}
                onPermanentDelete={onPermanentDelete}
              />
            );
          })}
        </>
      )}
    </Card>
  );
}

/**
 * P217：memo 化——比较器只比较数据 props（page/pageKey/isExpanded/showTrash/selectedIds/
 * renamingId/expandedObjects），忽略全部回调身份。对象展开/选中/编辑态作为数据集合
 * 透传，变化时精确触发对应层级重渲染；回调持旧引用无害（显式参数 + 函数式 setState）。
 */
function attachmentPageCardPropsEqual(
  prev: AttachmentPageCardProps,
  next: AttachmentPageCardProps,
): boolean {
  return (
    prev.page === next.page &&
    prev.pageKey === next.pageKey &&
    prev.isExpanded === next.isExpanded &&
    prev.showTrash === next.showTrash &&
    prev.selectedIds === next.selectedIds &&
    prev.renamingId === next.renamingId &&
    prev.expandedObjects === next.expandedObjects
  );
}

export const AttachmentPageCard = memo(AttachmentPageCardBase, attachmentPageCardPropsEqual);
