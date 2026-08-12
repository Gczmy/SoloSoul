import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowLeft, ArrowUp, Images, X } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { PhotoAlbumGrid } from '@/components/attachment/PhotoAlbumGrid';
import { PhotoViewerOverlay } from '@/components/attachment/PhotoViewerOverlay';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { DropdownSelect } from '@/components/ui/DropdownSelect';
import { syncStatusBarStyle } from '@/lib/theme';
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

/** 照片集分组区块（startIndex 为区块首项在排序后可见列表中的下标，供查看器索引映射）。 */
interface AlbumSection {
  key: string;
  label: string | null;
  items: AttachmentItem[];
  startIndex: number;
}

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

/** 分组标题本地化：2026 / 2026年8月 / 2026年8月10日 / 对象名。 */
function formatGroupLabel(mode: AlbumGroupMode, key: string, lang: string): string {
  if (mode === 'object') return key;
  if (mode === 'none') return key;
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

  // ── Android 硬件返回分层守卫 ─────────────────────────────────────────
  // 相册打开时压入「相册层」历史标记（URL 不变），打开全屏查看器时再压入
  // 「查看器层」标记；Android 返回先弹查看器层（回到网格），再弹相册层
  // （关闭相册）——避免从全屏查看器直接退出到首页。层数记录于 layersRef，
  // 供卸载时 history.go(-n) 一次性清理残留标记，保持历史栈干净。
  const viewerIndexRef = useRef<number | null>(null);
  viewerIndexRef.current = viewerIndex;
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  const layersRef = useRef(0);

  // 相册层：挂载压入标记 + popstate 分层处理 + 卸载清理。
  // popstate 时浏览器已弹出顶层标记：若查看器开着则回网格（查看器层被弹），
  // 否则关闭相册（相册层被弹）。
  useEffect(() => {
    const prevState = window.history.state as { idx?: number } | null;
    window.history.pushState(
      { ...(prevState ?? {}), solosoulAlbumLayer: true, idx: (prevState?.idx ?? 0) + 1 },
      '',
    );
    layersRef.current += 1;

    const onPopState = () => {
      if (layersRef.current > 0) layersRef.current -= 1;
      if (viewerIndexRef.current !== null) {
        setViewerIndex(null);
      } else {
        onCloseRef.current();
      }
    };
    window.addEventListener('popstate', onPopState);
    return () => {
      window.removeEventListener('popstate', onPopState);
      // 仅当顶层仍是我们的标记（相册层/查看器层均带 solosoulAlbumLayer）时才清理——
      // 若相册打开期间叠加了外部历史条目（如 vault 锁定 navigate('/login')），
      // 顶层非标记则跳过，避免误弹外部条目。
      const top = window.history.state as { solosoulAlbumLayer?: boolean } | null;
      if (top?.solosoulAlbumLayer && layersRef.current > 0) {
        window.history.go(-layersRef.current);
      }
      layersRef.current = 0;
    };
  }, []);

  // 查看器层：打开时再压入一层标记（供返回先回网格）。
  // 关闭统一由 popstate / handleViewerBack 负责，本 effect 无需 cleanup。
  useEffect(() => {
    if (viewerIndex === null) return;
    const prevState = window.history.state as { idx?: number } | null;
    window.history.pushState(
      { ...(prevState ?? {}), solosoulViewerLayer: true, idx: (prevState?.idx ?? 0) + 1 },
      '',
    );
    layersRef.current += 1;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅在 null↔非 null 边界压层
  }, [viewerIndex === null]);

  /** 查看器左上角返回按钮：查看器层标记在栈顶时主动弹出（触发 popstate 回网格），
   *  否则直接回网格（防御性兜底）。 */
  const handleViewerBack = () => {
    const state = window.history.state as { solosoulViewerLayer?: boolean } | null;
    if (state?.solosoulViewerLayer) {
      window.history.back();
    } else {
      setViewerIndex(null);
    }
  };

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

  /** 标签分区（需求4）：全量去重标签，按出现次数降序、名称升序。 */
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
      .map(([tag]) => tag);
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

  /** 分组区块（需求5/6）。 */
  const sections = useMemo<AlbumSection[]>(() => {
    if (visibleItems.length === 0 || groupMode === 'none') {
      return [{ key: 'all', label: null, items: visibleItems, startIndex: 0 }];
    }
    const groups = new Map<string, AttachmentItem[]>();
    for (const item of visibleItems) {
      let key: string;
      if (groupMode === 'object') {
        key = item.objectName?.trim() || item.objectId;
      } else {
        key = timeGroupKey(groupMode, parseDate(item.createdAt));
      }
      const list = groups.get(key);
      if (list) list.push(item);
      else groups.set(key, [item]);
    }
    // 时间分组：按键值排序，倒序时时间新在前；对象分组：按名称字典序
    const keys = [...groups.keys()];
    keys.sort((a, b) => {
      if (groupMode === 'object') return a.localeCompare(b);
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
  }, [visibleItems, groupMode, sortDesc, i18n.language]);

  /** 相册内编辑描述/标签后：本地副本即时更新 + 通知父级。 */
  const handleItemMetaUpdated = (updated: AttachmentItem) => {
    setLocalItems((prev) => prev.map((i) => (i.id === updated.id ? updated : i)));
    onItemMetaUpdated?.(updated);
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
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {visibleItems.length}
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
            <div
              style={{
                overflowX: 'auto',
                paddingBottom: 2,
                marginBottom: -2,
              }}
            >
              <FilterChipGroup<string>
                value={filterTag}
                onChange={setFilterTag}
                toggle
                options={[
                  {
                    id: null,
                    label: t('common:filter_all', { defaultValue: 'All' }),
                  },
                  ...tagOptions.map((tag) => ({ id: tag, label: tag })),
                ]}
                size="caption"
                style={{ flexWrap: 'nowrap', width: 'max-content' }}
              />
            </div>
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
              <PhotoAlbumGrid
                items={section.items}
                onSelect={(_, i) => setViewerIndex(section.startIndex + i)}
              />
            </div>
          ))
        )}
      </div>

      {/* 全屏查看器（覆盖整个相册；浏览范围为当前筛选/排序后的可见列表） */}
      {viewerIndex !== null && visibleItems[viewerIndex] && (
        <PhotoViewerOverlay
          items={visibleItems}
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
