import { useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { truncateFileName } from '@/lib/attachmentUtils';
import { formatBytes } from '@/lib/utils';

interface AttachmentFileNameBlockProps {
  fileName: string;
  sizeBytes: number;
  createdAt: string;
  showTrash: boolean;
  /** 可选元信息样式覆盖（如 AttachmentListItem 使用更小的 text-badge） */
  metaStyle?: CSSProperties;
  /** 可选：附件描述（存在时在元信息下方显示一行，截断；可展开全文） */
  description?: string | null;
  /** 可选：附件标签（存在时以小型 chips 显示，超过 4 个可展开） */
  tags?: string[];
}

/** 折叠态最多展示的标签数；超出部分收进「+N」并可用箭头展开。 */
const MAX_VISIBLE_TAGS = 4;

/**
 * 描述/标签的折叠展开箭头按钮（小号、透明、hover 高亮）。
 */
function ToggleButton({
  expanded,
  onClick,
  label,
}: {
  expanded: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-expanded={expanded}
      aria-label={label}
      title={label}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
        width: 18,
        height: 18,
        padding: 0,
        border: 'none',
        borderRadius: 4,
        background: 'transparent',
        color: 'var(--text-tertiary)',
        cursor: 'pointer',
        transition: 'color 120ms ease, background 120ms ease',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.color = 'var(--accent-primary)';
        e.currentTarget.style.background = 'var(--bg-hover)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = 'var(--text-tertiary)';
        e.currentTarget.style.background = 'transparent';
      }}
    >
      {expanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
    </button>
  );
}

/**
 * P226: 附件文件名块——名称（回收站态删除线/半透明）+ 大小·日期 + 描述（可展开）+ 标签（可展开）。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的文件名展示块；
 * 唯一差异是元信息行字号（text-caption vs text-badge），以 metaStyle 参数化。
 *
 * 描述与标签均默认折叠：
 * - 描述：折叠态单行省略，仅当文本实际溢出时显示展开箭头；展开后显示全文（保留换行）。
 * - 标签：折叠态显示前 4 个 +「+N」pill（与标签同尺寸垂直对齐），超过 4 个时显示展开箭头；
 *   展开后显示全部标签。
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
  const { t } = useTranslation(['common']);
  const visibleTags = (tags ?? []).filter((x) => x.trim());
  const trimmedDesc = description?.trim() ?? '';

  const [descExpanded, setDescExpanded] = useState(false);
  const [tagsExpanded, setTagsExpanded] = useState(false);
  const descRef = useRef<HTMLDivElement | null>(null);
  const [descOverflow, setDescOverflow] = useState(false);
  // 展开态实时镜像（供溢出检测守卫读取最新值，避免依赖数组包含 descExpanded 引发展开/收起重测）
  const descExpandedRef = useRef(descExpanded);
  descExpandedRef.current = descExpanded;

  // 折叠态检测描述是否溢出（仅溢出时展示展开箭头，避免短文本出现无意义的按钮）。
  // 展开态文本为多行（pre-wrap）不参与检测：若在展开态测量，scrollWidth ≤ clientWidth
  // 会误判无溢出，导致收起后箭头永久消失——故展开态直接跳过测量。descExpanded 经 ref
  // 读取最新值而非放入依赖数组，展开/收起不触发重测，保留折叠态测得的 descOverflow。
  useLayoutEffect(() => {
    if (descExpandedRef.current) return;
    const el = descRef.current;
    if (!el) return;
    setDescOverflow(el.scrollWidth > el.clientWidth + 1);
  }, [trimmedDesc, showTrash]);

  const hasMoreTags = visibleTags.length > MAX_VISIBLE_TAGS;
  const displayedTags = tagsExpanded ? visibleTags : visibleTags.slice(0, MAX_VISIBLE_TAGS);

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
      {trimmedDesc && (
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: 2,
            marginTop: 2,
            opacity: showTrash ? 0.6 : 1,
          }}
        >
          <div
            ref={descRef}
            style={{
              flex: 1,
              minWidth: 0,
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              overflow: 'hidden',
              textOverflow: descExpanded ? undefined : 'ellipsis',
              whiteSpace: descExpanded ? 'pre-wrap' : 'nowrap',
              wordBreak: 'break-word',
            }}
          >
            {trimmedDesc}
          </div>
          {descExpanded || descOverflow ? (
            <ToggleButton
              expanded={descExpanded}
              onClick={() => setDescExpanded((v) => !v)}
              label={
                descExpanded
                  ? t('common:collapse', { defaultValue: '收起' })
                  : t('common:expand', { defaultValue: '展开' })
              }
            />
          ) : null}
        </div>
      )}
      {visibleTags.length > 0 && (
        <div
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            alignItems: 'center',
            gap: 4,
            marginTop: 3,
          }}
        >
          {displayedTags.map((tag) => (
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
          {!tagsExpanded && hasMoreTags && (
            // +N 采用与标签一致的 pill 尺寸（同 padding/fontSize/行高），保证垂直对齐
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                padding: '1px 8px',
                borderRadius: 999,
                fontSize: 'var(--text-badge)',
                lineHeight: '1.4',
                color: 'var(--text-tertiary)',
                border: '1px dashed var(--border-subtle)',
                background: 'transparent',
                opacity: showTrash ? 0.5 : 1,
              }}
            >
              +{visibleTags.length - MAX_VISIBLE_TAGS}
            </span>
          )}
          {hasMoreTags && (
            <ToggleButton
              expanded={tagsExpanded}
              onClick={() => setTagsExpanded((v) => !v)}
              label={
                tagsExpanded
                  ? t('common:collapse', { defaultValue: '收起' })
                  : t('common:expand', { defaultValue: '展开' })
              }
            />
          )}
        </div>
      )}
    </div>
  );
}
