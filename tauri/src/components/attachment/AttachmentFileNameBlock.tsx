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

/** 标签 chip 的 data 标记，供溢出检测定位（避免误扫 +N/按钮等兄弟元素）。 */
const TAG_CHIP_ATTR = 'data-tag-chip';

/**
 * 折叠态下是否存在被压缩省略的标签 chip（scrollWidth > clientWidth 即出现省略号）。
 *
 * 与描述溢出检测同策略：只要任一标签被省略（哪怕数量不超过 MAX_VISIBLE_TAGS），
 * 就需要折叠按钮让用户展开查看完整内容。
 */
export function hasTagOverflow(container: HTMLElement | null): boolean {
  if (!container) return false;
  return Array.from(container.querySelectorAll(`[${TAG_CHIP_ATTR}]`)).some(
    (el) => el.scrollWidth > el.clientWidth + 1,
  );
}

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
 * - 标签：折叠态显示前 4 个 +「+N」pill（与标签同尺寸垂直对齐），超过 4 个**或任一标签被
 *   省略号截断**时显示展开箭头；展开后显示全部标签。
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
  const tagsContainerRef = useRef<HTMLDivElement | null>(null);
  const [descOverflow, setDescOverflow] = useState(false);
  const [tagsOverflow, setTagsOverflow] = useState(false);
  // 展开态实时镜像（供溢出检测守卫读取最新值，避免依赖数组包含 expanded 引发展开/收起重测）
  const descExpandedRef = useRef(descExpanded);
  descExpandedRef.current = descExpanded;
  const tagsExpandedRef = useRef(tagsExpanded);
  tagsExpandedRef.current = tagsExpanded;

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

  // 折叠态检测是否有标签被省略号截断（任一 chip 溢出即需要展开按钮）；
  // 展开态 wrap 不压缩不参与检测（同描述策略，tagsExpanded 经 ref 读取避免依赖数组）。
  useLayoutEffect(() => {
    if (tagsExpandedRef.current) return;
    setTagsOverflow(hasTagOverflow(tagsContainerRef.current));
  }, [tags, showTrash]);

  const hasMoreTags = visibleTags.length > MAX_VISIBLE_TAGS;
  const displayedTags = tagsExpanded ? visibleTags : visibleTags.slice(0, MAX_VISIBLE_TAGS);
  // 折叠按钮条件：标签数量超限 **或** 任一标签被省略号截断（4 个以内的长标签同样可展开）
  const showTagsToggle = hasMoreTags || tagsOverflow;

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
        // 外层 flex 与描述块同构：chips 区域独立 wrap，折叠按钮固定在右侧、顶部对齐——
        // 展开/收起时按钮位置不变（不随标签流漂移到最后一行末尾）。
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: 2,
            marginTop: 3,
            minWidth: 0,
            opacity: showTrash ? 0.6 : 1,
          }}
        >
          <div
            ref={tagsContainerRef}
            style={{
              flex: 1,
              minWidth: 0,
              display: 'flex',
              // 折叠态强制单行：标签 chip 可压缩并以省略号截断，保证 +N 始终完整留在行尾；
              // 展开态恢复换行显示全部标签（长标签在 chip 内换行，阅读舒适）。
              flexWrap: tagsExpanded ? 'wrap' : 'nowrap',
              alignItems: 'center',
              gap: 4,
              overflow: tagsExpanded ? 'visible' : 'hidden',
            }}
          >
            {displayedTags.map((tag) => (
              <span
                key={tag}
                {...{ [TAG_CHIP_ATTR]: true }}
                style={{
                  // 折叠态允许压缩（flexShrink: 1 + minWidth: 0 使省略号生效），
                  // 空间不足时优先截断标签文本而非换行；展开态不压缩、完整显示。
                  flexShrink: tagsExpanded ? 0 : 1,
                  minWidth: 0,
                  padding: '1px 8px',
                  borderRadius: 999,
                  fontSize: 'var(--text-badge)',
                  color: 'var(--accent-primary)',
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  // 展开态允许 chip 内文本换行（wordBreak 兜底长单词/超长串）
                  whiteSpace: tagsExpanded ? 'normal' : 'nowrap',
                  overflow: tagsExpanded ? 'visible' : 'hidden',
                  textOverflow: tagsExpanded ? undefined : 'ellipsis',
                  wordBreak: 'break-word',
                  lineHeight: 1.4,
                }}
              >
                {tag}
              </span>
            ))}
            {!tagsExpanded && hasMoreTags && (
              // +N 采用与标签一致的 pill 尺寸（同 padding/fontSize/行高），保证垂直对齐；
              // flexShrink: 0 使其永不被压缩、始终位于行尾
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  flexShrink: 0,
                  padding: '1px 8px',
                  borderRadius: 999,
                  fontSize: 'var(--text-badge)',
                  lineHeight: '1.4',
                  color: 'var(--text-tertiary)',
                  border: '1px dashed var(--border-subtle)',
                  background: 'transparent',
                }}
              >
                +{visibleTags.length - MAX_VISIBLE_TAGS}
              </span>
            )}
          </div>
          {showTagsToggle && (
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
