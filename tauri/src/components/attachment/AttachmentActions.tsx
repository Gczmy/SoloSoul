import { useTranslation } from 'react-i18next';
import { Eye, Edit2, Download, RotateCcw, Share2 } from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { ICON_SIZE } from '@/lib/constants';

interface AttachmentActionsProps {
  showTrash: boolean;
  /** 重命名进行中：隐藏预览/重命名/下载/转发按钮（AttachmentListItem 语义：保留删除按钮） */
  isRenaming?: boolean;
  onPreview: () => void;
  onStartRename: () => void;
  onDownload: () => void;
  onShare: () => void;
  onSoftDelete: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
}

/**
 * P226: 附件行操作按钮组——回收站态（恢复 + 永久删除）与常规态（预览/重命名/下载 + 删除）。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的操作按钮集合；
 * 差异点参数化：isRenaming 时常规态隐藏前三个按钮但保留删除（AttachmentListItem 语义，
 * AttachmentRow 由调用方在 isRenaming 时整体不渲染）。恢复图标统一为 RotateCcw，
 * AttachmentListItem 原 Preview 硬编码标题统一走 i18n。
 */
export function AttachmentActions({
  showTrash,
  isRenaming = false,
  onPreview,
  onStartRename,
  onDownload,
  onShare,
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
        <DeleteButton
          iconOnly
          onClick={onPermanentDelete}
          title={t('common:delete_permanently')}
        />
      </>
    );
  }

  return (
    <>
      {!isRenaming && (
        <>
          <BadgeIconButton
            Icon={Eye}
            onClick={onPreview}
            title={t('common:preview')}
            iconSize={ICON_SIZE.sm}
          />
          <BadgeIconButton
            Icon={Edit2}
            onClick={onStartRename}
            title={t('common:rename')}
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
        </>
      )}
      <DeleteButton iconOnly onClick={onSoftDelete} title={t('common:delete')} />
    </>
  );
}
