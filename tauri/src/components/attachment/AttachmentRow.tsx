import { useTranslation } from 'react-i18next';
import {
  Paperclip,
  RotateCcw,
  Eye,
  Edit2,
  Download,
} from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { ICON_SIZE } from '@/lib/constants';
import { truncateFileName } from '@/lib/attachmentUtils';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AttachmentMeta } from '@/components/attachment/attachmentManagerTypes';

interface AttachmentRowProps {
  item: AttachmentMeta;
  objectId: string;
  showTrash: boolean;
  isChecked: boolean;
  isRenaming: boolean;
  renameValue: string;
  renameInputRef: React.RefObject<HTMLInputElement | null>;
  onToggleSelect: (compositeKey: string) => void;
  onRenameChange: (value: string) => void;
  onRenameConfirm: () => void;
  onRenameCancel: () => void;
  onPreview: (item: AttachmentMeta) => void;
  onStartRename: (item: AttachmentMeta, objectId: string) => void;
  onDownload: (item: AttachmentMeta) => void;
  onSoftDelete: (item: AttachmentMeta, objectId: string) => void;
  onRestore: (item: AttachmentMeta, objectId: string) => void;
  onPermanentDelete: (item: AttachmentMeta, objectId: string) => void;
}

/** 附件单行（移动端多行布局 / 桌面端单行布局）。 */
export function AttachmentRow({
  item,
  objectId,
  showTrash,
  isChecked,
  isRenaming,
  renameValue,
  renameInputRef,
  onToggleSelect,
  onRenameChange,
  onRenameConfirm,
  onRenameCancel,
  onPreview,
  onStartRename,
  onDownload,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentRowProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);

  const compositeKey = `${objectId}::${item.id}`;
  const isMobile = isMobilePlatformSync();

  const renameInput = (
    <input
      ref={renameInputRef}
      value={renameValue}
      onChange={(e) => onRenameChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') onRenameConfirm();
        if (e.key === 'Escape') onRenameCancel();
      }}
      onBlur={onRenameConfirm}
      style={{
        flex: 1,
        minWidth: 0,
        padding: '2px 6px',
        fontSize: 'var(--text-sm)',
        borderRadius: 4,
        border: '1px solid var(--accent-primary)',
        background: 'transparent',
        color: 'var(--text-primary)',
        outline: 'none',
      }}
    />
  );

  const fileNameBlock = (
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
      <div
        style={{
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
          marginTop: 1,
        }}
      >
        {formatBytes(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
      </div>
    </div>
  );

  const actionButtons = showTrash ? (
    <>
      <BadgeIconButton
        Icon={RotateCcw}
        onClick={() => onRestore(item, objectId)}
        title={t('common:restore')}
        iconSize={ICON_SIZE.sm}
      />
      <DeleteButton
        iconOnly
        onClick={() => onPermanentDelete(item, objectId)}
        title={t('common:delete_permanently')}
      />
    </>
  ) : (
    <>
      <BadgeIconButton
        Icon={Eye}
        onClick={() => onPreview(item)}
        title={t('common:preview')}
        iconSize={ICON_SIZE.sm}
      />
      <BadgeIconButton
        Icon={Edit2}
        onClick={() => onStartRename(item, objectId)}
        title={t('common:rename')}
        iconSize={ICON_SIZE.sm}
      />
      <BadgeIconButton
        Icon={Download}
        onClick={() => onDownload(item)}
        title={t('common:download')}
        iconSize={ICON_SIZE.sm}
      />
      <DeleteButton
        iconOnly
        onClick={() => onSoftDelete(item, objectId)}
        title={t('common:delete')}
      />
    </>
  );

  // 移动端：多行布局 — 第1行 勾选框+图标+文件名，第2行 大小·时间，第3行 操作按钮
  if (isMobile) {
    return (
      <div
        key={item.id}
        style={{
          display: 'flex',
          gap: 6,
          padding: '6px 8px 6px 40px',
          fontSize: 'var(--text-sm)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <SelectCheckbox
          checked={isChecked}
          onClick={(e) => {
            e.stopPropagation();
            onToggleSelect(compositeKey);
          }}
        />
        <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />

        {isRenaming ? (
          renameInput
        ) : (
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
            <div
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                marginTop: 1,
              }}
            >
              {formatBytes(item.sizeBytes)} · {new Date(item.createdAt).toLocaleDateString()}
            </div>
            <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>{actionButtons}</div>
          </div>
        )}
      </div>
    );
  }

  // 桌面端：单行布局
  return (
    <div
      key={item.id}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '6px 8px 6px 40px',
        fontSize: 'var(--text-sm)',
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      <SelectCheckbox
        checked={isChecked}
        onClick={(e) => {
          e.stopPropagation();
          onToggleSelect(compositeKey);
        }}
      />
      <Paperclip size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />

      {isRenaming ? renameInput : fileNameBlock}

      <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
        {showTrash ? actionButtons : !isRenaming && actionButtons}
      </div>
    </div>
  );
}
