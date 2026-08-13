import type { CSSProperties } from 'react';
import { Paperclip, Image, FileText, type LucideIcon } from 'lucide-react';
import { previewItemByMime } from '@/lib/attachmentUtils';

/** 附件格式分类（回收站对象详情卡附件项同一套判定的扁平化：text 归入 other）。 */
export type AttachmentFormatType = 'image' | 'pdf' | 'other';

/**
 * 按 mimeType + 文件扩展名判定附件格式（image/pdf/other）。
 * 复用 previewItemByMime 单一分类源（text 文件图标归回形针，故映射为 other），
 * 保证附件管理树与回收站展示口径一致。
 */
export function getAttachmentFormatType(item: {
  fileName: string;
  mimeType: string;
}): AttachmentFormatType {
  const kind = previewItemByMime(item);
  return kind === 'text' ? 'other' : kind;
}

const FORMAT_ICONS: Record<AttachmentFormatType, LucideIcon> = {
  image: Image,
  pdf: FileText,
  other: Paperclip,
};

/** 附件格式图标（图片→Image / PDF→FileText / 其余→回形针）。 */
export function AttachmentTypeIcon({
  item,
  size,
  style,
}: {
  item: { fileName: string; mimeType: string };
  size: number;
  style?: CSSProperties;
}) {
  const Icon = FORMAT_ICONS[getAttachmentFormatType(item)];
  return <Icon size={size} style={style} />;
}

/**
 * 附件格式名称徽章（如 [PNG] [PDF]），样式与回收站对象详情卡附件项的徽章一致：
 * text-badge 字号、2px 6px 内边距、4px 圆角、半透明弱化底色；无扩展名时回退 FILE。
 */
export function AttachmentExtBadge({ fileName }: { fileName: string }) {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  return (
    <span
      style={{
        fontSize: 'var(--text-badge)',
        padding: '2px 6px',
        borderRadius: 4,
        fontWeight: 500,
        background: 'color-mix(in srgb, var(--text-tertiary) 10%, transparent)',
        color: 'var(--text-tertiary)',
        flexShrink: 0,
        textDecoration: 'none',
      }}
    >
      {ext.toUpperCase() || 'FILE'}
    </span>
  );
}
