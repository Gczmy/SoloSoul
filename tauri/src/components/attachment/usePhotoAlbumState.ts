/**
 * PhotoAlbumOverlay 数据层 hook（P048 拆分：逻辑层与渲染层分离）。
 * 含相册全部状态、筛选/排序/分组派生、查看器浏览列表与硬件返回守卫。
 */
import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { syncStatusBarStyle } from '@/lib/theme';
import { useOverlayBackGuard } from '@/hooks/useOverlayBackGuard';
import type { AttachmentItem } from '@/lib/attachmentUtils';

/** 照片集分组模式：不分组 / 按年 / 按月 / 按日 / 按对象。 */
export type AlbumGroupMode = 'none' | 'year' | 'month' | 'day' | 'object';

export interface PhotoAlbumStateParams {
  items: AttachmentItem[];
  onClose: () => void;
  onItemMetaUpdated?: (updated: AttachmentItem) => void;
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
export type AlbumSection = AlbumLeafSection | AlbumGroupSection;

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

/** chip 数量徽标样式：标签按钮上显示该标签的照片数（当前选项数量在选项按钮上）。 */
export const tagCountBadge: CSSProperties = {
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

export function usePhotoAlbumState({ items, onClose, onItemMetaUpdated }: PhotoAlbumStateParams) {
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

  return {
    viewerIndex,
    setViewerIndex,
    filterTag,
    setFilterTag,
    sortDesc,
    setSortDesc,
    groupMode,
    setGroupMode,
    tagsExpanded,
    setTagsExpanded,
    tagFilterRef,
    handleTagFilterWheel,
    handleViewerBack,
    localItems,
    tagOptions,
    visibleItems,
    sections,
    viewerItems,
    handleItemMetaUpdated,
    distinctObjects,
    groupLabel,
    groupOptions,
    t,
  };
}
