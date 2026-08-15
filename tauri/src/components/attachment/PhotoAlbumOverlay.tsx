import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowLeft, ArrowUp, ChevronDown, ChevronUp, Images, X } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { PhotoAlbumGrid } from '@/components/attachment/PhotoAlbumGrid';
import { PhotoViewerOverlay } from '@/components/attachment/PhotoViewerOverlay';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { DropdownSelect } from '@/components/ui/DropdownSelect';
import { syncStatusBarStyle } from '@/lib/theme';
import { useOverlayBackGuard } from '@/hooks/useOverlayBackGuard';
import { ICON_SIZE, SAFE_AREA_BOTTOM, SAFE_AREA_TOP } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';

/** 照片集分组模式：不分组 / 按年 / 按月 / 按日 / 按对象。 */
export type AlbumGroupMode = 'none' | 'year' | 'month' | 'day' | 'object';

export interface PhotoAlbumOverlayProps {
  /** 已按 `previewItemByMime(...) === 'image'` 过滤的照片项（调用方负责过滤）。 */
  items: AttachmentItem[];
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
  /** 相册内编辑附件描述/标签后的回调（父级同步数据源）。 */
  onItemMetaUpdated?: (updated: AttachmentItem) => void;
  zIndex?: number | string;
}

interface AlbumSectionBase {
  key: string;
  label: string | null;
  items: AttachmentItem[];
}

/** 叶子区块：持有照片，startIndex 为区块首项在查看器浏览列表中的下标。 */
interface AlbumLeafSection extends AlbumSectionBase {
  startIndex: number;
  children?: undefined;
}

/** 对象分组模式的页面区块：只持有 children（对象子区块，渲染时缩进），自身不渲染网格。 */
interface AlbumGroupSection extends AlbumSectionBase {
  startIndex?: undefined;
  children: AlbumLeafSection[];
}

/** 照片集分组区块。对象分组为两级：页面区块（label=页面名）含 children=对象子区块。 */
type AlbumSection = AlbumLeafSection | AlbumGroupSection;

/** 解析 ISO 时间字符串为本地 Date；非法值回退 0 时刻保证排序稳定。 */
function parseDate(s: string): Date {
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? new Date(0) : d;
}

/** 时间分组键：'2026' / '2026-8' / '2026-8-10'。 */
function timeGroupKey(mode: Exclude<AlbumGroupMode, 'none' | 'object'>, d: Date): string {
  const y = d.getFullYear();
  const m = d.getMonth() + 1;
  const day = d.getDate();
  if (mode === 'year') return `${y}`;
  if (mode === 'month') return `${y}-${m}`;
  return `${y}-${m}-${day}`;
}

/** 分组标题本地化：2026 / 2026年8月 / 2026年8月10日。
 *  对象分组（页面→对象两级）与「不分组」在 sections 内单独处理，不经过本函数。 */
function formatGroupLabel(
  mode: Exclude<AlbumGroupMode, 'none' | 'object'>,
  key: string,
  lang: string,
): string {
  const [y, m, d] = key.split('-').map(Number);
  if (mode === 'year') {
    return new Intl.DateTimeFormat(lang, { year: 'numeric' }).format(new Date(y, 0, 1));
  }
  if (mode === 'month') {
    return new Intl.DateTimeFormat(lang, { year: 'numeric', month: 'long' }).format(
      new Date(y, (m || 1) - 1, 1),
    );
  }
  return new Intl.DateTimeFormat(lang, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(new Date(y, (m || 1) - 1, d || 1));
}

/**
 * 照片集相册组合层（附件照片集方案 §3.2/§3.3）：
 * - 顶栏（返回/关闭 + 标题 + 数量）；
 * - 工具栏：标签分区筛选（需求4）+ 时间正/倒序排序（需求5）+ 年/月/日/对象分组（需求5/6）；
 * - 网格视图（PhotoAlbumGrid）↔ 全屏查看器（PhotoViewerOverlay）状态机；
 * - 打开时切深色状态栏，关闭恢复主题对应样式（与 AttachmentPreviewOverlay 一致）。
 */
export function PhotoAlbumOverlay({
  items,
  onClose,
  onOpenExternal,
  onItemMetaUpdated,
  zIndex = 'var(--z-preview-overlay)',
}: PhotoAlbumOverlayProps) {
  const { t, i18n } = useTranslation('common');
  const [viewerIndex, setViewerIndex] = useState<number | null>(null);
  const [filterTag, setFilterTag] = useState<string | null>(null);
  const [sortDesc, setSortDesc] = useState(true);
  const [groupMode, setGroupMode] = useState<AlbumGroupMode>('none');
  // 标签筛选横向滚动容器：隐藏滚动条但支持左右滑动（对齐侧边栏搜索卡片
  // SearchPopover.filterBar 的页面选项滚动方式）。
  const tagFilterRef = useRef<HTMLDivElement>(null);
  // 标签展开态：折叠 = 单行横向滚动（省空间）；展开 = 全部标签换行平铺（高度
  // 自适应，便于一眼看到全部标签）。展开后箭头按钮切换为折叠。
  const [tagsExpanded, setTagsExpanded] = useState(false);

  // ── Android 硬件返回分层守卫（共享 useOverlayBackGuard）──
  // 相册打开时压入「相册层」历史标记（URL 不变），打开全屏查看器时再压入
  // 「查看器层」标记；Android 返回先弹查看器层（回到网格），再弹相册层
  // （关闭相册）——避免从全屏查看器直接退出到首页。三个入口（首页/附件管理/
  // 对象详情）均渲染本 overlay，故统一获得该行为。
  const { handleInnerBack: handleViewerBack } = useOverlayBackGuard({
    innerOpen: viewerIndex !== null,
    onCloseInner: () => setViewerIndex(null),
    onClose,
  });

  // 本地副本：相册内编辑描述/标签后即时生效，父级刷新后经 props 同步。
  // 注意：仅同步数据、不重置筛选/查看器下标——相册内编辑元数据后父级 setItems/
  // loadData 会产生新的 items 引用，若在此重置 viewerIndex 会把用户从全屏查看器
  // 踢回网格（需求 2+3 要求在全屏照片中编辑后留在原处）。越界由渲染守卫
  // `visibleItems[viewerIndex]` 兜底。
  const [localItems, setLocalItems] = useState<AttachmentItem[]>(items);
  useEffect(() => {
    setLocalItems(items);
  }, [items]);

  useEffect(() => {
    void syncStatusBarStyle('dark');
    return () => {
      const currentTheme = document.documentElement.getAttribute('data-theme');
      void syncStatusBarStyle(currentTheme === 'dark' ? 'dark' : 'light');
    };
  }, []);

  /** 标签分区（需求4）：全量去重标签，按出现次数降序、名称升序；
   *  count 供 chip 上的数量徽标（当前选项照片数显示在每个选项按钮上）。 */
  const tagOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const item of localItems) {
      for (const tag of item.tags ?? []) {
        const k = tag.trim();
        if (k) counts.set(k, (counts.get(k) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .map(([tag, count]) => ({ tag, count }));
  }, [localItems]);

  /** 可见列表 = 标签筛选 + 时间排序（上传时间 created_at）。 */
  const visibleItems = useMemo(() => {
    const filtered = filterTag
      ? localItems.filter((i) => (i.tags ?? []).includes(filterTag))
      : localItems;
    return [...filtered].sort((a, b) => {
      const diff = parseDate(a.createdAt).getTime() - parseDate(b.createdAt).getTime();
      return sortDesc ? -diff : diff;
    });
  }, [localItems, filterTag, sortDesc]);

  /** 分组区块（需求5/6）。对象分组为 页面→对象 两级：
   *  顶层页面区块（页面名，系统页经 navigation 命名空间翻译），
   *  其 children 为对象子区块（对象名缩进展示，照片在子区块内）。 */
  const sections = useMemo<AlbumSection[]>(() => {
    if (visibleItems.length === 0 || groupMode === 'none') {
      return [{ key: 'all', label: null, items: visibleItems, startIndex: 0 }];
    }
    if (groupMode === 'object') {
      // 页面 → 对象 两级分组（页面信息由 collectPhotoItems 从树携带）
      const pageGroups = new Map<string, Map<string, AttachmentItem[]>>();
      const pageLabels = new Map<string, string>();
      for (const item of visibleItems) {
        const pageKey = item.pageId || item.pageName || '';
        if (!pageLabels.has(pageKey)) {
          pageLabels.set(
            pageKey,
            item.pageName
              ? item.pageId
                ? item.pageName
                : t(`navigation:${item.pageName}`, { defaultValue: item.pageName })
              : t('common:unknown_page', { defaultValue: 'Unknown page' }),
          );
        }
        let objs = pageGroups.get(pageKey);
        if (!objs) {
          objs = new Map();
          pageGroups.set(pageKey, objs);
        }
        const objKey = item.objectName?.trim() || item.objectId;
        const list = objs.get(objKey);
        if (list) list.push(item);
        else objs.set(objKey, [item]);
      }
      // 页面按标签字典序、页内对象按名称字典序（与既有对象分组排序一致）
      const pageKeys = [...pageGroups.keys()].sort((a, b) =>
        (pageLabels.get(a) ?? a).localeCompare(pageLabels.get(b) ?? b),
      );
      let startIndex = 0;
      return pageKeys.map((pageKey) => {
        const objs = pageGroups.get(pageKey)!;
        const objKeys = [...objs.keys()].sort((a, b) => a.localeCompare(b));
        const children: AlbumLeafSection[] = objKeys.map((objKey) => {
          const items = objs.get(objKey)!;
          const child: AlbumLeafSection = { key: objKey, label: objKey, items, startIndex };
          startIndex += items.length;
          return child;
        });
        return {
          key: pageKey,
          label: pageLabels.get(pageKey) ?? pageKey,
          items: children.flatMap((c) => c.items),
          children,
        };
      });
    }
    const groups = new Map<string, AttachmentItem[]>();
    for (const item of visibleItems) {
      const key = timeGroupKey(groupMode, parseDate(item.createdAt));
      const list = groups.get(key);
      if (list) list.push(item);
      else groups.set(key, [item]);
    }
    // 时间分组：按键值排序，倒序时时间新在前
    const keys = [...groups.keys()];
    keys.sort((a, b) => {
      const na = Number(a.split('-')[0]);
      const nb = Number(b.split('-')[0]);
      if (na !== nb) return sortDesc ? nb - na : na - nb;
      return sortDesc ? b.localeCompare(a) : a.localeCompare(b);
    });
    let startIndex = 0;
    return keys.map((key) => {
      const groupItems = groups.get(key)!;
      const section: AlbumSection = {
        key,
        label: formatGroupLabel(groupMode, key, i18n.language),
        items: groupItems,
        startIndex,
      };
      startIndex += groupItems.length;
      return section;
    });
  }, [visibleItems, groupMode, sortDesc, i18n.language, t]);

  /** 查看器浏览列表 = 分组渲染顺序拍平（对象分组下与网格可见顺序一致，
   *  保证 startIndex + i 索引正确；时间分组下等价于 createdAt 排序）。 */
  const viewerItems = useMemo(() => {
    const out: AttachmentItem[] = [];
    for (const s of sections) {
      if (s.children) {
        for (const c of s.children) out.push(...c.items);
      } else {
        out.push(...s.items);
      }
    }
    return out;
  }, [sections]);

  /** 相册内编辑描述/标签后：本地副本即时更新 + 通知父级。 */
  const handleItemMetaUpdated = (updated: AttachmentItem) => {
    setLocalItems((prev) => prev.map((i) => (i.id === updated.id ? updated : i)));
    onItemMetaUpdated?.(updated);
  };

  /**
   * 垂直滚轮转为横向滚动（与 SearchPopover.filterBar 同一交互）：标签区
   * 横向可滚动但隐藏滚动条，鼠标滚轮上下滚动时改为左右滑动标签；无横向
   * 溢出时不拦截（保持默认行为）。
   */
  const handleTagFilterWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    const el = tagFilterRef.current;
    if (!el || el.scrollWidth <= el.clientWidth) return;
    if (e.deltaY !== 0) {
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    }
  };

  /** 仅单个对象（对象级相册）时隐藏「按对象」分组选项。 */
  const distinctObjects = useMemo(
    () => new Set(localItems.map((i) => i.objectName?.trim() || i.objectId)).size,
    [localItems],
  );

  const groupLabel =
    groupMode === 'none'
      ? t('common:group_none', { defaultValue: 'No grouping' })
      : groupMode === 'object'
        ? t('common:group_by_object', { defaultValue: 'By object' })
        : groupMode === 'year'
          ? t('common:group_by_year', { defaultValue: 'By year' })
          : groupMode === 'month'
            ? t('common:group_by_month', { defaultValue: 'By month' })
            : t('common:group_by_day', { defaultValue: 'By day' });

  const groupOptions = [
    { value: 'none', label: t('common:group_none', { defaultValue: 'No grouping' }) },
    { value: 'year', label: t('common:group_by_year', { defaultValue: 'By year' }) },
    { value: 'month', label: t('common:group_by_month', { defaultValue: 'By month' }) },
    { value: 'day', label: t('common:group_by_day', { defaultValue: 'By day' }) },
    ...(distinctObjects > 1
      ? [{ value: 'object', label: t('common:group_by_object', { defaultValue: 'By object' }) }]
      : []),
  ];

  // chip 数量徽标样式：标签按钮上显示该标签的照片数（当前选项数量在选项按钮上）
  const tagCountBadge: CSSProperties = {
    display: 'inline-block',
    marginLeft: 5,
    padding: '0 5px',
    borderRadius: 8,
    background: 'color-mix(in srgb, var(--accent-primary) 12%, transparent)',
    color: 'var(--text-tertiary)',
    fontSize: 'var(--text-badge)',
    fontWeight: 500,
    lineHeight: '14px',
    verticalAlign: 'middle',
  };

  return (
    <div
      data-testid="photo-album-overlay"
      // 阻止冒泡：AttachmentViewer 外层容器点击背景即关闭，照片集内部点击不应触发
      onClick={(e) => e.stopPropagation()}
      style={{
        position: 'fixed',
        top: SAFE_AREA_TOP,
        left: 0,
        right: 0,
        bottom: SAFE_AREA_BOTTOM,
        zIndex,
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg-elevated)',
      }}
    >
      {/* 标签筛选区横向滚动：隐藏 webkit 滚动条但保留滚动能力（与
          SearchPopover.filterBar 的 scrollbar-width:none 方案一致）。 */}
      <style>{`
        .photo-album-tag-filter::-webkit-scrollbar { display: none; }
      `}</style>
      {/* 顶栏 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '10px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          background: 'var(--bg-toolbar)',
          flexShrink: 0,
        }}
      >
        <BadgeIconButton
          Icon={ArrowLeft}
          onClick={onClose}
          title={t('common:back', 'Back')}
          iconSize={ICON_SIZE.md}
        />
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
          <Images size={ICON_SIZE.sm} style={{ color: 'var(--accent-primary)' }} />
          <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600 }}>
            {t('common:photo_album', 'Photo Album')}
          </span>
          {/* 标题右侧显示全部照片数量（当前选项数量移到标签 chip 与工具栏右侧） */}
          <span
            data-testid="album-total-count"
            style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}
          >
            {localItems.length}
          </span>
        </div>
        <BadgeIconButton
          Icon={X}
          onClick={onClose}
          title={t('common:close', 'Close')}
          iconSize={ICON_SIZE.md}
        />
      </div>

      {/* 工具栏：标签筛选 + 排序 + 分组 */}
      {(tagOptions.length > 0 || visibleItems.length > 0) && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            padding: '10px 16px',
            borderBottom: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            flexShrink: 0,
          }}
        >
          {tagOptions.length > 0 && (
            <>
              <div
                ref={tagFilterRef}
                onWheel={handleTagFilterWheel}
                className="photo-album-tag-filter"
                style={
                  tagsExpanded
                    ? {
                        // 展开态：不做横向滚动约束，chip 换行平铺，高度自适应标签数量
                        paddingBottom: 2,
                        marginBottom: -2,
                      }
                    : {
                        overflowX: 'auto',
                        overflowY: 'hidden',
                        scrollbarWidth: 'none',
                        paddingBottom: 2,
                        marginBottom: -2,
                      }
                }
              >
                <FilterChipGroup<string>
                  value={filterTag}
                  onChange={setFilterTag}
                  toggle
                  options={[
                    {
                      id: null,
                      label: (
                        <>
                          {t('common:filter_all', { defaultValue: 'All' })}
                          <span style={tagCountBadge}>{localItems.length}</span>
                        </>
                      ),
                    },
                    ...tagOptions.map(({ tag, count }) => ({
                      id: tag,
                      label: (
                        <>
                          {tag}
                          <span style={tagCountBadge}>{count}</span>
                        </>
                      ),
                    })),
                  ]}
                  size="caption"
                  style={
                    tagsExpanded
                      ? { flexWrap: 'wrap', width: '100%' }
                      : { flexWrap: 'nowrap', width: 'max-content' }
                  }
                />
              </div>
              {/* 展开/折叠按钮：滚动栏下方，展开后切换为折叠图标 */}
              <button
                type="button"
                onClick={() => setTagsExpanded((v) => !v)}
                title={t(tagsExpanded ? 'common:collapse' : 'common:expand')}
                aria-label={t(tagsExpanded ? 'common:collapse' : 'common:expand')}
                className="interactive-icon"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  alignSelf: 'center',
                  width: 24,
                  height: 20,
                  padding: 0,
                  border: 'none',
                  borderRadius: 6,
                  background: 'transparent',
                  color: 'var(--text-tertiary)',
                  cursor: 'pointer',
                  flexShrink: 0,
                }}
              >
                {tagsExpanded ? (
                  <ChevronUp size={14} style={{ flexShrink: 0 }} />
                ) : (
                  <ChevronDown size={14} style={{ flexShrink: 0 }} />
                )}
              </button>
            </>
          )}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button
              type="button"
              onClick={() => setSortDesc((v) => !v)}
              title={t(
                sortDesc ? 'common:sort_asc' : 'common:sort_desc',
                sortDesc ? 'Switch to oldest first' : 'Switch to newest first',
              )}
              className="interactive-toolbar"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                padding: '5px 10px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)',
                color: 'var(--text-secondary)',
                fontSize: 'var(--text-caption)',
                cursor: 'pointer',
                whiteSpace: 'nowrap',
              }}
            >
              {sortDesc ? <ArrowDown size={ICON_SIZE.sm} /> : <ArrowUp size={ICON_SIZE.sm} />}
              {sortDesc
                ? t('common:sort_desc', { defaultValue: 'Newest first' })
                : t('common:sort_asc', { defaultValue: 'Oldest first' })}
            </button>
            <DropdownSelect
              value={groupMode}
              onChange={(v) => setGroupMode(v as AlbumGroupMode)}
              options={groupOptions}
              triggerLabel={groupLabel}
              ariaLabel={t('common:album_group_mode', { defaultValue: 'Group by' })}
              width={110}
            />
            {/* 排序/分组按钮右侧：当前选项的照片数量 */}
            <span
              data-testid="album-current-count"
              style={{
                marginLeft: 'auto',
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                whiteSpace: 'nowrap',
              }}
            >
              {visibleItems.length}
            </span>
          </div>
        </div>
      )}

      {/* 网格（按区块分隔渲染） */}
      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {visibleItems.length === 0 ? (
          <div
            style={{
              textAlign: 'center',
              padding: 48,
              color: 'var(--text-secondary)',
              fontSize: 'var(--text-body)',
            }}
          >
            {filterTag
              ? t('common:no_photos_for_filter', {
                  tag: filterTag,
                  defaultValue: `No photos with tag "${filterTag}"`,
                })
              : t('common:no_attachments', 'No attachments')}
          </div>
        ) : (
          sections.map((section) => (
            <div key={section.key} style={{ marginBottom: 20 }}>
              {section.label && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    marginBottom: 10,
                  }}
                >
                  <span
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 600,
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {section.label}
                  </span>
                  <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                    {section.items.length}
                  </span>
                  <div style={{ flex: 1, height: 1, background: 'var(--border-subtle)' }} />
                </div>
              )}
              {section.children ? (
                // 对象分组：对象子区块缩进展示（对象名 + 照片）
                section.children.map((child) => (
                  <div key={child.key} style={{ paddingLeft: 16, marginBottom: 16 }}>
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        marginBottom: 10,
                      }}
                    >
                      <span
                        style={{
                          fontSize: 'var(--text-caption)',
                          fontWeight: 600,
                          color: 'var(--text-tertiary)',
                        }}
                      >
                        {child.label}
                      </span>
                      <span
                        style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}
                      >
                        {child.items.length}
                      </span>
                    </div>
                    <PhotoAlbumGrid
                      items={child.items}
                      onSelect={(_, i) => setViewerIndex(child.startIndex + i)}
                    />
                  </div>
                ))
              ) : (
                <PhotoAlbumGrid
                  items={section.items}
                  onSelect={(_, i) => setViewerIndex(section.startIndex + i)}
                />
              )}
            </div>
          ))
        )}
      </div>

      {/* 全屏查看器（覆盖整个相册；浏览范围为分组渲染顺序拍平的可见列表） */}
      {viewerIndex !== null && viewerItems[viewerIndex] && (
        <PhotoViewerOverlay
          items={viewerItems}
          initialIndex={viewerIndex}
          onBack={handleViewerBack}
          onClose={onClose}
          onOpenExternal={onOpenExternal}
          onItemMetaUpdated={handleItemMetaUpdated}
        />
      )}
    </div>
  );
}
