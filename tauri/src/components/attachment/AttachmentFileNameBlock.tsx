import type { CSSProperties } from 'react';
import { truncateFileName } from '@/lib/attachmentUtils';
import { formatBytes } from '@/lib/utils';

interface AttachmentFileNameBlockProps {
  fileName: string;
  sizeBytes: number;
  createdAt: string;
  showTrash: boolean;
  /** 可选元信息样式覆盖（如 AttachmentListItem 使用更小的 text-badge） */
  metaStyle?: CSSProperties;
  /** 可选：附件描述（存在时在元信息下方显示一行，截断） */
  description?: string | null;
  /** 可选：附件标签（存在时以小型 chips 显示） */
  tags?: string[];
}

/**
 * P226: 附件文件名块——名称（回收站态删除线/半透明）+ 大小·日期。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的文件名展示块；
 * 唯一差异是元信息行字号（text-caption vs text-badge），以 metaStyle 参数化。
 */
export function AttachmentFileNameBlock({
  fileName,
  sizeBytes,
  createdAt,
  showTrash,
  metaStyle,
  description,
  tags,
}: AttachmentFileNameBlockProps) {
  const visibleTags = (tags ?? []).filter((x) => x.trim());
  return (
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
        {truncateFileName(fileName)}
      </div>
      <div
        style={{
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
          marginTop: 1,
          ...metaStyle,
        }}
      >
        {formatBytes(sizeBytes)} · {new Date(createdAt).toLocaleDateString()}
      </div>
      {description?.trim() && (
        <div
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            marginTop: 2,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            opacity: showTrash ? 0.6 : 1,
          }}
        >
          {description.trim()}
        </div>
      )}
      {visibleTags.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginTop: 3 }}>
          {visibleTags.slice(0, 4).map((tag) => (
            <span
              key={tag}
              style={{
                padding: '1px 8px',
                borderRadius: 999,
                fontSize: 'var(--text-badge)',
                color: 'var(--accent-primary)',
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                opacity: showTrash ? 0.5 : 1,
              }}
            >
              {tag}
            </span>
          ))}
          {visibleTags.length > 4 && (
            <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
              +{visibleTags.length - 4}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
