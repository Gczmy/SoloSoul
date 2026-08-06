import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, ChevronDown, Upload } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import { AttachmentRow } from '@/components/attachment/AttachmentRow';
import type { ReactNode } from 'react';
import type {
  AttachmentMeta,
  AttachmentTreeObject,
} from '@/components/attachment/attachmentManagerTypes';

/** 为单个对象行提供拖拽上传的 drop zone */
function ObjectDropTarget({
  objectId,
  loadData,
  children,
}: {
  objectId: string;
  loadData: () => void;
  children: ReactNode;
}) {
  const { ref, dragState } = useDragToAttach(objectId, { onComplete: loadData });
  return (
    <div ref={ref} style={{ position: 'relative' }}>
      {children}
      <DragUploadOverlay dragState={dragState} borderRadius={8} />
    </div>
  );
}

interface AttachmentObjectGroupProps {
  obj: AttachmentTreeObject;
  isExpanded: boolean;
  showTrash: boolean;
  /** 当前选中附件复合键集合（`objectId::attachmentId`），用于行内勾选态。 */
  selectedIds: Set<string>;
  /** 当前重命名附件 ID，仅命中的行进入编辑态。 */
  renamingId: string | null;
  loadData: () => void;
  onToggle: () => void;
  onUpload: (objectId: string) => void;
  onToggleSelect: (compositeKey: string) => void;
  onRenameConfirm: (newName: string) => void;
  onRenameCancel: () => void;
  onPreview: (item: AttachmentMeta) => void;
  onStartRename: (item: AttachmentMeta, objectId: string) => void;
  onDownload: (item: AttachmentMeta) => void;
  onSoftDelete: (item: AttachmentMeta, objectId: string) => void;
  onRestore: (item: AttachmentMeta, objectId: string) => void;
  onPermanentDelete: (item: AttachmentMeta, objectId: string) => void;
}

/** 对象分组行（移动端多行 / 桌面端单行），展开时渲染附件行。 */
function AttachmentObjectGroupBase({
  obj,
  isExpanded,
  showTrash,
  selectedIds,
  renamingId,
  loadData,
  onToggle,
  onUpload,
  onToggleSelect,
  onRenameConfirm,
  onRenameCancel,
  onPreview,
  onStartRename,
  onDownload,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentObjectGroupProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const isMobile = isMobilePlatformSync();

  const row = (
    <div>
      <div
        onClick={onToggle}
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
        {isMobile ? (
          // 移动端：第1行 模板名+对象名+上传按钮，第2行 统计信息
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {obj.templateName && (
                <span
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-tertiary)',
                    flexShrink: 0,
                  }}
                >
                  {obj.templateName}
                </span>
              )}
              <span
                style={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  flex: 1,
                  minWidth: 0,
                }}
              >
                {obj.objectName}
              </span>
              {!showTrash && (
                <BadgeIconButton
                  Icon={Upload}
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpload(obj.objectId);
                  }}
                  title={t('common:upload', { defaultValue: 'Upload' })}
                  iconSize={ICON_SIZE.sm}
                />
              )}
            </div>
            <div
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                marginTop: 1,
              }}
            >
              {t('settings:attachments_count', { n: obj.attachments.length })} ·{' '}
              {formatBytes(obj.attachments.reduce((sum, a) => sum + a.sizeBytes, 0))}
            </div>
          </div>
        ) : (
          // 桌面端：单行布局
          <>
            <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {obj.templateName}
            </span>
            <span
              style={{
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                flex: 1,
              }}
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
              {formatBytes(obj.attachments.reduce((sum, a) => sum + a.sizeBytes, 0))}
            </span>
            {!showTrash && (
              <BadgeIconButton
                Icon={Upload}
                onClick={(e) => {
                  e.stopPropagation();
                  onUpload(obj.objectId);
                }}
                title={t('common:upload', { defaultValue: 'Upload' })}
                iconSize={ICON_SIZE.sm}
              />
            )}
          </>
        )}
      </div>
      {isExpanded && (
        <>
          {obj.attachments.map((att) => (
            <AttachmentRow
              key={att.id}
              item={att}
              objectId={obj.objectId}
              showTrash={showTrash}
              isChecked={selectedIds.has(`${obj.objectId}::${att.id}`)}
              isRenaming={renamingId === att.id}
              onToggleSelect={onToggleSelect}
              onRenameConfirm={onRenameConfirm}
              onRenameCancel={onRenameCancel}
              onPreview={onPreview}
              onStartRename={onStartRename}
              onDownload={onDownload}
              onSoftDelete={onSoftDelete}
              onRestore={onRestore}
              onPermanentDelete={onPermanentDelete}
            />
          ))}
        </>
      )}
    </div>
  );

  return showTrash ? (
    <div key={obj.objectId}>{row}</div>
  ) : (
    <ObjectDropTarget key={obj.objectId} objectId={obj.objectId} loadData={loadData}>
      {row}
    </ObjectDropTarget>
  );
}

/**
 * P217：memo 化——比较器只比较数据 props（obj/isExpanded/showTrash/selectedIds/renamingId），
 * 忽略全部回调身份。selectedIds/renamingId 作为数据透传，选中态/编辑态变化精确触发
 * 对应层级重渲染；回调持旧引用无害（显式参数 + 函数式 setState）。
 */
function attachmentObjectGroupPropsEqual(
  prev: AttachmentObjectGroupProps,
  next: AttachmentObjectGroupProps,
): boolean {
  return (
    prev.obj === next.obj &&
    prev.isExpanded === next.isExpanded &&
    prev.showTrash === next.showTrash &&
    prev.selectedIds === next.selectedIds &&
    prev.renamingId === next.renamingId
  );
}

export const AttachmentObjectGroup = memo(
  AttachmentObjectGroupBase,
  attachmentObjectGroupPropsEqual,
);
