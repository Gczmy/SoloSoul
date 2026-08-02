import type { RefObject } from 'react';
import { Paperclip, Image, FileText } from 'lucide-react';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentFileNameBlock } from '@/components/attachment/AttachmentFileNameBlock';
import { AttachmentActions } from '@/components/attachment/AttachmentActions';
import { type AttachmentItem } from '@/lib/attachmentUtils';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentListItemProps {
  item: AttachmentItem;
  compositeKey: string;
  checked: boolean;
  showTrash: boolean;
  isLast: boolean;
  renamingId: string | null;
  renameValue: string;
  renameInputRef: RefObject<HTMLInputElement | null>;
  onToggleSelect: (key: string) => void;
  onRenameValueChange: (v: string) => void;
  onConfirmRename: () => void;
  onCancelRename: () => void;
  onRestore: (item: AttachmentItem) => void;
  onPreview: (item: AttachmentItem) => void;
  onStartRename: (item: AttachmentItem) => void;
  onDownload: (item: AttachmentItem) => void;
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
  renamingId,
  renameValue,
  renameInputRef,
  onToggleSelect,
  onRenameValueChange,
  onConfirmRename,
  onCancelRename,
  onRestore,
  onPreview,
  onStartRename,
  onDownload,
  onDelete,
  onPermanentDelete,
}: AttachmentListItemProps) {
  const isRenaming = renamingId === item.id;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '8px 12px',
        borderBottom: isLast ? 'none' : '1px solid var(--border-subtle)',
        fontSize: 'var(--text-body-sm)',
      }}
    >
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
        metaStyle={{ fontSize: 'var(--text-badge)' }}
      />
      {isRenaming && !showTrash ? (
        <input
          ref={renameInputRef}
          value={renameValue}
          onChange={(e) => onRenameValueChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') onConfirmRename();
            if (e.key === 'Escape') onCancelRename();
          }}
          onBlur={onConfirmRename}
          style={{
            width: 100,
            padding: '3px 6px',
            fontSize: 'var(--text-caption)',
            borderRadius: 4,
            border: '1px solid var(--accent-primary)',
            background: 'transparent',
            color: 'var(--text-primary)',
            outline: 'none',
          }}
        />
      ) : null}
      <AttachmentActions
        showTrash={showTrash}
        isRenaming={isRenaming}
        onPreview={() => onPreview(item)}
        onStartRename={() => onStartRename(item)}
        onDownload={() => onDownload(item)}
        onSoftDelete={() => onDelete(item)}
        onRestore={() => onRestore(item)}
        onPermanentDelete={() => onPermanentDelete(item)}
      />
    </div>
  );
}
