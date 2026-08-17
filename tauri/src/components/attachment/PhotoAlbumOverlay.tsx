import { Suspense } from 'react';
import { ArrowDown, ArrowLeft, ArrowUp, ChevronDown, ChevronUp, Images, X } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { PhotoAlbumGrid } from '@/components/attachment/PhotoAlbumGrid';
import { LazyPhotoViewerOverlay } from '@/components/attachment/LazyPhotoViewerOverlay';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { DropdownSelect } from '@/components/ui/DropdownSelect';
import { ICON_SIZE, SAFE_AREA_BOTTOM, SAFE_AREA_TOP } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';
// P048: 数据层抽到 hook（state/筛选/排序/分组/查看器列表/硬件返回守卫）
import { usePhotoAlbumState, tagCountBadge } from './usePhotoAlbumState';
import type { AlbumGroupMode } from './usePhotoAlbumState';

export type { AlbumGroupMode } from './usePhotoAlbumState';

export interface PhotoAlbumOverlayProps {
  /** 已按 `previewItemByMime(...) === 'image'` 过滤的照片项（调用方负责过滤）。 */
  items: AttachmentItem[];
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
  /** 相册内编辑附件描述/标签后的回调（父级同步数据源）。 */
  onItemMetaUpdated?: (updated: AttachmentItem) => void;
  zIndex?: number | string;
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
  const {
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
    handleItemMetaUpdated: onAlbumItemMetaUpdated,
    groupLabel,
    groupOptions,
    t,
  } = usePhotoAlbumState({ items, onClose, onItemMetaUpdated });

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
        <Suspense
          fallback={
            <div
              style={{
                position: 'fixed',
                inset: 0,
                zIndex: 'var(--z-preview-overlay)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: '#000',
              }}
            >
              <div className="spinner" style={{ width: 24, height: 24, borderTopColor: 'var(--text-secondary)' }} />
            </div>
          }
        >
          <LazyPhotoViewerOverlay
            items={viewerItems}
            initialIndex={viewerIndex}
            onBack={handleViewerBack}
            onClose={onClose}
            onOpenExternal={onOpenExternal}
            onItemMetaUpdated={onAlbumItemMetaUpdated}
          />
        </Suspense>
      )}
    </div>
  );
}
