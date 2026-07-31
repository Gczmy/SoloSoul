import { useTranslation } from 'react-i18next';
import { ChevronRight, ChevronDown, Upload } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { DragUploadOverlay } from '@/components/object/DragUploadOverlay';
import type { ReactNode } from 'react';
import type { AttachmentTreeObject } from '@/components/attachment/attachmentManagerTypes';

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
  loadData: () => void;
  onToggle: () => void;
  onUpload: (objectId: string) => void;
  renderAttachments: () => ReactNode;
}

/** 对象分组行（移动端多行 / 桌面端单行），展开时渲染附件行。 */
export function AttachmentObjectGroup({
  obj,
  isExpanded,
  showTrash,
  loadData,
  onToggle,
  onUpload,
  renderAttachments,
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
                  title={t('common:upload') || 'Upload'}
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
                title={t('common:upload') || 'Upload'}
                iconSize={ICON_SIZE.sm}
              />
            )}
          </>
        )}
      </div>
      {isExpanded && renderAttachments()}
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
