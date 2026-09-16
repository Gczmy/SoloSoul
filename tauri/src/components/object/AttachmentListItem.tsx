import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentFileNameBlock } from '@/components/attachment/AttachmentFileNameBlock';
import { AttachmentActions } from '@/components/attachment/AttachmentActions';
import {
  AttachmentTypeIcon,
  AttachmentExtBadge,
} from '@/components/attachment/AttachmentFormatBadge';
import { type AttachmentItem } from '@/lib/attachmentUtils';
import { ICON_SIZE } from '@/lib/constants';
import { isAndroidSync, isMobilePlatformSync } from '@/lib/platform';

interface AttachmentListItemProps {
  item: AttachmentItem;
  compositeKey: string;
  checked: boolean;
  showTrash: boolean;
  isLast: boolean;
  onToggleSelect: (key: string) => void;
  onRestore: (item: AttachmentItem) => void;
  onPreview: (item: AttachmentItem) => void;
  onDownload: (item: AttachmentItem) => void;
  onShare: (item: AttachmentItem) => void;
  /** 编辑附件属性（名称/描述/标签） */
  onEditMeta?: (item: AttachmentItem) => void;
  onDelete: (item: AttachmentItem) => void;
  onPermanentDelete: (item: AttachmentItem) => void;
}

/**
 * 附件管理器单行：选择框 + 类型图标 + 元信息 + 操作（预览/重命名/下载/删除/恢复/永久删除）。
 * 从 AttachmentViewer 抽出。
 */
export function AttachmentListItem({
  item,
  compositeKey,
  checked,
  showTrash,
  isLast,
  onToggleSelect,
  onRestore,
  onPreview,
  onDownload,
  onShare,
  onEditMeta,
  onDelete,
  onPermanentDelete,
}: AttachmentListItemProps) {
  // Android 将操作收进单个菜单按钮，与附件信息并排；其他移动端保留独立按钮行。
  const isAndroid = isAndroidSync();
  const stackActions = isMobilePlatformSync() && !isAndroid;
  const typeIcon = (
    <AttachmentTypeIcon
      item={item}
      size={ICON_SIZE.sm}
      style={{
        color: 'var(--text-tertiary)',
        flexShrink: 0,
        opacity: !isAndroid && showTrash ? 0.5 : 1,
      }}
    />
  );
  const checkbox = (
    <SelectCheckbox
      checked={checked}
      onClick={(e) => {
        e.stopPropagation();
        onToggleSelect(compositeKey);
      }}
    />
  );

  const infoRow = (
    <div
      style={{
        display: 'flex',
        alignItems: isAndroid ? 'flex-start' : 'center',
        gap: 8,
        minWidth: 0,
        flex: 1,
      }}
    >
      {isAndroid ? (
        // 与全局附件行一致：勾选框只对齐名称首行，不随描述/标签增高而下移。
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            height: 'calc(var(--text-body-sm) * 1.4)',
            flexShrink: 0,
          }}
        >
          {checkbox}
        </div>
      ) : (
        checkbox
      )}
      {!isAndroid && typeIcon}
      <AttachmentFileNameBlock
        fileName={item.fileName}
        sizeBytes={item.sizeBytes}
        createdAt={item.createdAt}
        showTrash={showTrash}
        description={item.description}
        tags={item.tags}
        metaStyle={{ fontSize: 'var(--text-badge)' }}
        metaLeadingIcon={
          isAndroid ? (
            <>
              {typeIcon}
              <AttachmentExtBadge fileName={item.fileName} />
            </>
          ) : undefined
        }
      />
    </div>
  );

  const actions = (
    <AttachmentActions
      fileName={item.fileName}
      showTrash={showTrash}
      onPreview={() => onPreview(item)}
      onDownload={() => onDownload(item)}
      onShare={() => onShare(item)}
      onEditMeta={onEditMeta ? () => onEditMeta(item) : undefined}
      onSoftDelete={() => onDelete(item)}
      onRestore={() => onRestore(item)}
      onPermanentDelete={() => onPermanentDelete(item)}
    />
  );

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: stackActions ? 'column' : 'row',
        alignItems: stackActions ? 'stretch' : 'center',
        gap: 8,
        padding: '8px 12px',
        borderBottom: isLast ? 'none' : '1px solid var(--border-subtle)',
        fontSize: 'var(--text-body-sm)',
        lineHeight: isAndroid ? 1.4 : undefined,
      }}
    >
      {infoRow}
      {stackActions ? (
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 2 }}>{actions}</div>
      ) : (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            marginLeft: 'auto',
            gap: 2,
            flexShrink: 0,
          }}
        >
          {actions}
        </div>
      )}
    </div>
  );
}
