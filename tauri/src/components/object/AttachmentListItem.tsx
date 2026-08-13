import { Paperclip, Image, FileText } from 'lucide-react';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentFileNameBlock } from '@/components/attachment/AttachmentFileNameBlock';
import { AttachmentActions } from '@/components/attachment/AttachmentActions';
import { type AttachmentItem } from '@/lib/attachmentUtils';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';

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

function TypeIcon({ item, showTrash }: { item: AttachmentItem; showTrash: boolean }) {
  const style = {
    color: 'var(--text-tertiary)',
    flexShrink: 0,
    opacity: showTrash ? 0.5 : 1,
  };
  if (item.mimeType.startsWith('image/')) {
    return <Image size={ICON_SIZE.sm} style={style} />;
  }
  if (item.mimeType === 'application/pdf') {
    return <FileText size={ICON_SIZE.sm} style={style} />;
  }
  return <Paperclip size={ICON_SIZE.sm} style={style} />;
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
  // 安卓端：附件信息（图标/名称/大小时间）与操作按钮并排会被按钮挤占，
  // 按钮单独一行放在信息下方；桌面端保持原横向布局。
  const isMobile = isMobilePlatformSync();

  const infoRow = (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
      <SelectCheckbox
        checked={checked}
        onClick={(e) => {
          e.stopPropagation();
          onToggleSelect(compositeKey);
        }}
      />
      <TypeIcon item={item} showTrash={showTrash} />
      <AttachmentFileNameBlock
        fileName={item.fileName}
        sizeBytes={item.sizeBytes}
        createdAt={item.createdAt}
        showTrash={showTrash}
        description={item.description}
        tags={item.tags}
        metaStyle={{ fontSize: 'var(--text-badge)' }}
      />
    </div>
  );

  const actions = (
    <AttachmentActions
      showTrash={showTrash}
      onPreview={() => onPreview(item)}
      onDownload={() => onDownload(item)}
      onShare={() => onShare(item)}
      onEditMeta={() => onEditMeta?.(item)}
      onSoftDelete={() => onDelete(item)}
      onRestore={() => onRestore(item)}
      onPermanentDelete={() => onPermanentDelete(item)}
    />
  );

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: isMobile ? 'column' : 'row',
        alignItems: isMobile ? 'stretch' : 'center',
        gap: 8,
        padding: '8px 12px',
        borderBottom: isLast ? 'none' : '1px solid var(--border-subtle)',
        fontSize: 'var(--text-body-sm)',
      }}
    >
      {infoRow}
      {isMobile ? (
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 2 }}>{actions}</div>
      ) : (
        // 桌面端：marginLeft auto 将按钮组推到行尾右对齐
        <div style={{ display: 'flex', alignItems: 'center', marginLeft: 'auto', gap: 2 }}>
          {actions}
        </div>
      )}
    </div>
  );
}
