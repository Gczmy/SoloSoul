import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Eye, Download, RotateCcw, Share2, FilePen, MoreHorizontal, Trash2 } from 'lucide-react';
import { AndroidSheet } from '@/components/android/AndroidSheet';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';
import { isAndroidSync } from '@/lib/platform';

interface AttachmentActionsProps {
  fileName: string;
  showTrash: boolean;
  onPreview: () => void;
  onDownload: () => void;
  onShare: () => void;
  /** 编辑附件属性（名称/描述/标签——原「重命名」与「编辑描述和标签」两按钮合并而来） */
  onEditMeta?: () => void;
  onSoftDelete: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
}

function AndroidAttachmentActions({ fileName, showTrash, ...handlers }: AttachmentActionsProps) {
  const { t } = useTranslation('common');
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const options = showTrash
    ? [
        { label: t('restore'), Icon: RotateCcw, action: handlers.onRestore, danger: false },
        {
          label: t('delete_permanently'),
          Icon: Trash2,
          action: handlers.onPermanentDelete,
          danger: true,
        },
      ]
    : [
        { label: t('preview'), Icon: Eye, action: handlers.onPreview, danger: false },
        { label: t('download'), Icon: Download, action: handlers.onDownload, danger: false },
        { label: t('forward'), Icon: Share2, action: handlers.onShare, danger: false },
        ...(handlers.onEditMeta
          ? [{ label: t('edit_meta'), Icon: FilePen, action: handlers.onEditMeta, danger: false }]
          : []),
        { label: t('delete'), Icon: Trash2, action: handlers.onSoftDelete, danger: true },
      ];

  return (
    <>
      <button
        ref={trigger}
        type="button"
        className="android-icon-button"
        aria-label={t('attachment_actions_for', { name: fileName })}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={(event) => {
          event.stopPropagation();
          setOpen(true);
        }}
      >
        <MoreHorizontal size={24} />
      </button>
      {open && (
        <AndroidSheet
          title={t('more_actions')}
          onClose={() => setOpen(false)}
          trigger={trigger.current}
          zIndex="var(--z-preview-overlay)"
        >
          <p className="android-attachment-menu-name" title={fileName}>
            {fileName}
          </p>
          <div className="android-attachment-menu-actions">
            {options.map(({ label, Icon, action, danger }) => (
              <button
                key={label}
                type="button"
                data-danger={danger || undefined}
                onClick={() => {
                  setOpen(false);
                  action();
                }}
              >
                <Icon size={24} aria-hidden="true" />
                <span>{label}</span>
              </button>
            ))}
          </div>
        </AndroidSheet>
      )}
    </>
  );
}

/**
 * P226: 附件行操作按钮组——回收站态（恢复 + 永久删除）与常规态（预览/下载/转发/编辑属性 + 删除）。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的操作按钮集合。
 * 「重命名」与「编辑描述和标签」合并为单一「编辑附件属性」按钮（FilePen），
 * 弹卡内提供名称/描述/标签三输入框；恢复图标统一为 RotateCcw。
 */
export function AttachmentActions({
  fileName,
  showTrash,
  onPreview,
  onDownload,
  onShare,
  onEditMeta,
  onSoftDelete,
  onRestore,
  onPermanentDelete,
}: AttachmentActionsProps) {
  const { t } = useTranslation(['common']);

  if (isAndroidSync()) {
    return (
      <AndroidAttachmentActions
        fileName={fileName}
        showTrash={showTrash}
        onPreview={onPreview}
        onDownload={onDownload}
        onShare={onShare}
        onEditMeta={onEditMeta}
        onSoftDelete={onSoftDelete}
        onRestore={onRestore}
        onPermanentDelete={onPermanentDelete}
      />
    );
  }

  if (showTrash) {
    return (
      <>
        <BadgeIconButton
          Icon={RotateCcw}
          onClick={onRestore}
          title={t('common:restore')}
          iconSize={ICON_SIZE.sm}
        />
        <DeleteButton iconOnly onClick={onPermanentDelete} title={t('common:delete_permanently')} />
      </>
    );
  }

  return (
    <>
      <BadgeIconButton
        Icon={Eye}
        onClick={onPreview}
        title={t('common:preview')}
        iconSize={ICON_SIZE.sm}
      />
      <BadgeIconButton
        Icon={Download}
        onClick={onDownload}
        title={t('common:download')}
        iconSize={ICON_SIZE.sm}
      />
      <BadgeIconButton
        Icon={Share2}
        onClick={onShare}
        title={t('common:forward')}
        iconSize={ICON_SIZE.sm}
      />
      {onEditMeta && (
        <BadgeIconButton
          Icon={FilePen}
          onClick={onEditMeta}
          title={t('common:edit_meta', { defaultValue: 'Edit Attachment Attributes' })}
          iconSize={ICON_SIZE.sm}
        />
      )}
      <DeleteButton iconOnly onClick={onSoftDelete} title={t('common:delete')} />
    </>
  );
}
