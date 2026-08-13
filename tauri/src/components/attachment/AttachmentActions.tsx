import { useTranslation } from 'react-i18next';
import { Eye, Download, RotateCcw, Share2, FilePen } from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentActionsProps {
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

/**
 * P226: 附件行操作按钮组——回收站态（恢复 + 永久删除）与常规态（预览/下载/转发/编辑属性 + 删除）。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的操作按钮集合。
 * 「重命名」与「编辑描述和标签」合并为单一「编辑附件属性」按钮（FilePen），
 * 弹卡内提供名称/描述/标签三输入框；恢复图标统一为 RotateCcw。
 */
export function AttachmentActions({
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
