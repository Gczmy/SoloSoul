import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';
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

/** 可展开块外层**第一层**半透明主题色背景（展开态整块）。 */
const BLOCK_BG = 'color-mix(in srgb, var(--accent-primary) 6%, transparent)';
/** 折叠把手行**第二层**半透明主题色背景（叠在 BLOCK_BG 之上 → 颜色更深）。 */
const HANDLE_BG = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';

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
/**
 * Y001: 行点击的选区守卫。同一元素内拖选文本后 mouseup 触发 click 照常派发
 * （UI Events：click 派发到 mousedown/mouseup 目标的最近公共祖先），`userSelect:
 * 'text'` 只能让文本可选、无法阻止该 click——有非空选区时不切换折叠态，
 * 拖选复制与整行折叠点击因此互不冲突。
 */
function hasTextSelection(): boolean {
  const selection = window.getSelection();
  return selection != null && selection.toString().length > 0;
}

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
      onClick={(e) => {
        // 阻止冒泡：行容器整行可点（T003 触控优化），按钮点击不应再触发行切换造成双重展开/收起
        e.stopPropagation();
        onClick();
      }}
      aria-expanded={expanded}
      aria-label={label}
      title={label}
      // X002: 桌面指针设备 hover 反馈经 global.css `.toggle-arrow-btn`（media 限定）
      // 提供——触摸设备不挂该样式，与状态驱动展开高亮互不干扰。
      className="toggle-arrow-btn"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
        width: 18,
        height: 18,
        // 覆盖 global.css 移动端 button 触控基线（min-height/min-width: 44px）——
        // 否则安卓端按钮被撑成 44×44 视觉占两行；触控面积由整行可点承担。
        minWidth: 0,
        minHeight: 0,
        padding: 0,
        border: 'none',
        borderRadius: 4,
        // 状态驱动而非 hover/触摸事件驱动：展开态保持「点击效果」（accent 色 +
        // 高亮底），收起态恢复常态——安卓 WebView 触摸后合成 mouseover 无 mouseout
        // 会导致 hover 样式残留卡死（标签与描述按钮行为不一致的根因）。
        background: expanded ? 'var(--bg-hover)' : 'transparent',
        color: expanded ? 'var(--accent-primary)' : 'var(--text-tertiary)',
        cursor: 'pointer',
        // T003 移动端触控：消除 300ms 点击延迟与双击缩放误触发；
        // 触控面积由行容器整行可点承担，按钮不再扩展隐形热区（不占额外行）。
        touchAction: 'manipulation',
        transition: 'color 120ms ease, background 120ms ease',
      }}
    >
      {expanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
    </button>
  );
}

/**
 * 展开态折叠把手行：显示「描述」/「标签」标签，是展开态**唯一**可点击收起的位置——
 * 半透明主题色圆角背景提示可点击；下方展开内容（全文/chip）不可点击折叠，
 * 用户可自由选中/复制文本。
 */
function CollapseHandleRow({ label, onCollapse }: { label: string; onCollapse: () => void }) {
  const { t } = useTranslation(['common']);
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        // 半透明主题色圆角背景仅此把手行有（描述态叠在外层块背景之上 → 颜色更深，
        // 提示用户此行可点击收起）。
        background: HANDLE_BG,
        borderRadius: 6,
        // 触控友好：此行为展开态唯一折叠入口，垂直留白使其成为足够大的点击目标。
        padding: '6px 4px',
        cursor: 'pointer',
        touchAction: 'manipulation',
      }}
      onClick={() => {
        // Y001: 有文本选区（拖选复制/长按选择）时不切换折叠态。
        if (hasTextSelection()) return;
        onCollapse();
      }}
    >
      <span
        style={{
          fontSize: 'var(--text-caption)',
          fontWeight: 500,
          color: 'var(--text-secondary)',
          lineHeight: 1.2,
        }}
      >
        {label}
      </span>
      <ToggleButton
        expanded
        onClick={onCollapse}
        label={t('common:collapse', { defaultValue: '收起' })}
      />
    </div>
  );
}

/**
 * 名称/描述/标签共用的可展开块（T006–T008 三处重复收敛）：
 * - 收起态：内容单行省略 + 尾部箭头 + 整行可点展开（canExpand 时）
 * - 展开态：顶部折叠把手行（唯一折叠入口）+ 全量内容不可点击折叠（可选中/复制）
 * - 展开态整块叠**第一层**半透明主题色圆角背景，把手行 10% 叠加上层更深
 */
function ExpandableBlock({
  expanded,
  canExpand,
  onExpand,
  onCollapse,
  label,
  marginTop,
  opacity,
  children,
}: {
  expanded: boolean;
  /** 收起态是否可展开（溢出/超限才显示箭头与整行可点） */
  canExpand: boolean;
  onExpand: () => void;
  onCollapse: () => void;
  /** 折叠把手行标签（展开态出现） */
  label: string;
  marginTop?: number;
  /** 整块不透明度（回收站态半透明） */
  opacity: number;
  children: ReactNode;
}) {
  const { t } = useTranslation(['common']);
  return (
    <div
      style={{
        marginTop,
        minWidth: 0,
        opacity,
        // 展开态：整块叠**第一层**半透明主题色圆角背景，把折叠把手行与内容圈在一起；
        // 收起态无背景、维持原样。
        background: expanded ? BLOCK_BG : undefined,
        borderRadius: expanded ? 8 : undefined,
        padding: expanded ? 4 : undefined,
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}
    >
      {expanded && <CollapseHandleRow label={label} onCollapse={onCollapse} />}
      <div
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          gap: 2,
          minWidth: 0,
          // T003/T006：**仅收起态**整行可点（点内容任意处展开）——展开态内容不可点击
          // 折叠（避免用户无法选中文本），折叠把手移交标签行。
          cursor: !expanded && canExpand ? 'pointer' : 'default',
          touchAction: 'manipulation',
        }}
        onClick={
          !expanded && canExpand
            ? () => {
                // Y001: 有文本选区（拖选复制/长按选择）时不切换折叠态——同一元素内
                // 拖选后 click 仍照常派发，仅靠 userSelect 无法阻止。
                if (hasTextSelection()) return;
                onExpand();
              }
            : undefined
        }
      >
        {children}
        {!expanded && canExpand ? (
          <ToggleButton
            expanded={false}
            onClick={onExpand}
            label={t('common:expand', { defaultValue: '展开' })}
          />
        ) : null}
      </div>
    </div>
  );
}

/**
 * P226: 附件文件名块——名称（超长折叠展开/回收站态删除线）+ 大小·日期 + 描述（可展开）+ 标签（可展开）。
 *
 * 收敛自 AttachmentRow 与 AttachmentListItem 两处逐字节相同的文件名展示块；
 * 唯一差异是元信息行字号（text-caption vs text-badge），以 metaStyle 参数化。
 *
 * 名称、描述与标签均默认折叠：
 * - 名称：折叠态单行省略，仅当文本实际溢出时显示展开箭头；展开后显示全名（换行）。
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

  const [nameExpanded, setNameExpanded] = useState(false);
  const [descExpanded, setDescExpanded] = useState(false);
  const [tagsExpanded, setTagsExpanded] = useState(false);
  const nameRef = useRef<HTMLDivElement | null>(null);
  const descRef = useRef<HTMLDivElement | null>(null);
  const tagsContainerRef = useRef<HTMLDivElement | null>(null);
  const [nameOverflow, setNameOverflow] = useState(false);
  const [descOverflow, setDescOverflow] = useState(false);
  const [tagsOverflow, setTagsOverflow] = useState(false);
  // 展开态实时镜像（供溢出检测守卫读取最新值，避免依赖数组包含 expanded 引发展开/收起重测）
  const nameExpandedRef = useRef(nameExpanded);
  nameExpandedRef.current = nameExpanded;
  const descExpandedRef = useRef(descExpanded);
  descExpandedRef.current = descExpanded;
  const tagsExpandedRef = useRef(tagsExpanded);
  tagsExpandedRef.current = tagsExpanded;

  // 折叠态检测描述/标签是否溢出（仅溢出时展示展开箭头，避免短文本出现无意义的按钮）。
  // 展开态不参与检测：描述展开为多行（pre-wrap）、标签展开为 wrap 后 scrollWidth ≤
  // clientWidth 会误判无溢出，导致收起后箭头永久消失——故展开态直接跳过测量。
  // descExpanded/tagsExpanded 经 ref 读取最新值而非放入依赖数组，展开/收起不触发重测，
  // 保留折叠态测得的溢出值。
  //
  // T005: 除内容/回收站态变化外，还注册 ResizeObserver 监听容器尺寸变化（列表变窄、
  // 侧栏展开/收起、窗口缩放等）时重测——否则容器变窄后原本 ≤4 个的标签被截断时
  // 用户无任何展开途径（无 +N 也无按钮）。
  useLayoutEffect(() => {
    const measure = () => {
      if (!nameExpandedRef.current) {
        const el = nameRef.current;
        if (el) {
          setNameOverflow(el.scrollWidth > el.clientWidth + 1);
        }
      }
      if (!descExpandedRef.current) {
        const el = descRef.current;
        if (el) {
          setDescOverflow(el.scrollWidth > el.clientWidth + 1);
        }
      }
      if (!tagsExpandedRef.current) {
        setTagsOverflow(hasTagOverflow(tagsContainerRef.current));
      }
    };
    measure();
    // U006: 字体晚加载兜底——ResizeObserver 监听 border-box 尺寸，字体加载只改变
    // scrollWidth（文本渲染宽度）不改变元素盒尺寸时不触发；`document.fonts.ready`
    // 在全部字体加载完成后 resolve，此时重测一次，避免字体加载前测量偏小漏判。
    // 组件卸载后触发重测仅 setState no-op（React 18 无警告），无需清理。
    document.fonts?.ready?.then(() => measure()).catch(() => undefined);
    // jsdom 无 ResizeObserver（测试环境跳过，仅依赖变化重测）
    if (typeof ResizeObserver === 'undefined') {
      return undefined;
    }
    const observer = new ResizeObserver(() => measure());
    if (nameRef.current) observer.observe(nameRef.current);
    if (descRef.current) observer.observe(descRef.current);
    if (tagsContainerRef.current) observer.observe(tagsContainerRef.current);
    return () => observer.disconnect();
  }, [fileName, trimmedDesc, tags, showTrash]);

  const hasMoreTags = visibleTags.length > MAX_VISIBLE_TAGS;
  const displayedTags = tagsExpanded ? visibleTags : visibleTags.slice(0, MAX_VISIBLE_TAGS);
  // 折叠按钮条件：标签数量超限 **或** 任一标签被省略号截断（4 个以内的长标签同样可展开）
  const showTagsToggle = hasMoreTags || tagsOverflow;

  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      {/* 名称行：超长时折叠/展开（ExpandableBlock 共享） */}
      <ExpandableBlock
        expanded={nameExpanded}
        canExpand={nameOverflow}
        onExpand={() => setNameExpanded(true)}
        onCollapse={() => setNameExpanded(false)}
        label={t('common:name', { defaultValue: '名称' })}
        // 回收站态：整块（含把手行/箭头）统一半透明；文本删除线在下方内容层保留
        opacity={showTrash ? 0.5 : 1}
      >
        <div
          ref={nameRef}
          style={{
            flex: 1,
            minWidth: 0,
            fontWeight: 500,
            overflow: 'hidden',
            textOverflow: nameExpanded ? undefined : 'ellipsis',
            whiteSpace: nameExpanded ? 'pre-wrap' : 'nowrap',
            wordBreak: 'break-word',
            textDecoration: showTrash ? 'line-through' : 'none',
            userSelect: 'text',
          }}
        >
          {fileName}
        </div>
      </ExpandableBlock>
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
        <ExpandableBlock
          expanded={descExpanded}
          canExpand={descOverflow}
          onExpand={() => setDescExpanded(true)}
          onCollapse={() => setDescExpanded(false)}
          label={t('common:attachment_description', { defaultValue: '描述' })}
          marginTop={2}
          opacity={showTrash ? 0.6 : 1}
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
              // X002: 整行可点下显式声明文本可选（拖选复制的前提）。
              // Y001: 机制修正——同一元素内拖选后 click 照常派发（按下/释放目标相同），
              // 防误翻转改由行 onClick 的选区守卫承担（有选区则不切换）。
              userSelect: 'text',
            }}
          >
            {trimmedDesc}
          </div>
        </ExpandableBlock>
      )}
      {visibleTags.length > 0 && (
        <ExpandableBlock
          expanded={tagsExpanded}
          canExpand={showTagsToggle}
          onExpand={() => setTagsExpanded(true)}
          onCollapse={() => setTagsExpanded(false)}
          label={t('common:attachment_tags', { defaultValue: '标签' })}
          marginTop={3}
          opacity={showTrash ? 0.6 : 1}
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
                  // 空间不足时优先截断标签文本而非换行。
                  flexShrink: tagsExpanded ? 0 : 1,
                  minWidth: 0,
                  padding: '1px 8px',
                  borderRadius: 999,
                  fontSize: 'var(--text-badge)',
                  color: 'var(--accent-primary)',
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  // 展开态允许 chip 内文本换行：**必须加 maxWidth: 100%**——flex-basis: auto
                  // 的 item 默认宽度为内容 max-content，长标签会撑出容器而溢出，whiteSpace:
                  // normal 永远没有换行机会；maxWidth 将 item 宽度钳制在容器宽度内，
                  // wordBreak 兜底长单词/超长串，换行才会在 chip 内部生效。
                  maxWidth: tagsExpanded ? '100%' : undefined,
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
        </ExpandableBlock>
      )}
    </div>
  );
}
