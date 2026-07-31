import { useTranslation } from 'react-i18next';
import type { RefObject } from 'react';
import { Paperclip, Image, FileText, RotateCw, Eye, Edit2, Download } from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { truncateFileName, type AttachmentItem } from '@/lib/attachmentUtils';
import { formatBytes } from '@/lib/utils';
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
  const { t } = useTranslation(['common', 'editor']);
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
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontWeight: 500,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            textDecoration: showTrash ? 'line-through' : 'none',
            opacity: showTrash ? 0.5 : 1,
          }}
        >
          {truncateFileName(item.fileName)}
        </div>
        <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
          {formatBytes(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
        </div>
      </div>
      {showTrash ? (
        <>
          <BadgeIconButton
            Icon={RotateCw}
            onClick={() => onRestore(item)}
            title={t('common:restore')}
            iconSize={ICON_SIZE.sm}
          />
          <DeleteButton
            iconOnly
            onClick={() => onPermanentDelete(item)}
            title={t('common:delete_permanently')}
          />
        </>
      ) : (
        <>
          {isRenaming ? (
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
          ) : (
            <>
              <BadgeIconButton
                Icon={Eye}
                onClick={() => onPreview(item)}
                title="Preview"
                iconSize={ICON_SIZE.sm}
              />
              <BadgeIconButton
                Icon={Edit2}
                onClick={() => onStartRename(item)}
                title={t('common:rename')}
                iconSize={ICON_SIZE.sm}
              />
              <BadgeIconButton
                Icon={Download}
                onClick={() => onDownload(item)}
                title={t('common:download')}
                iconSize={ICON_SIZE.sm}
              />
            </>
          )}
          <DeleteButton
            iconOnly
            onClick={() => onDelete(item)}
            title={t('common:delete')}
          />
        </>
      )}
    </div>
  );
}
