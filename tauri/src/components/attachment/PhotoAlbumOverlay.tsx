import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeft, Images, X } from 'lucide-react';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { PhotoAlbumGrid } from '@/components/attachment/PhotoAlbumGrid';
import { PhotoViewerOverlay } from '@/components/attachment/PhotoViewerOverlay';
import { syncStatusBarStyle } from '@/lib/theme';
import { ICON_SIZE, SAFE_AREA_BOTTOM, SAFE_AREA_TOP } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';

export interface PhotoAlbumOverlayProps {
  /** 已按 `previewItemByMime(...) === 'image'` 过滤的照片项（调用方负责过滤）。 */
  items: AttachmentItem[];
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
  zIndex?: number | string;
}

/**
 * 照片集相册组合层（附件照片集方案 §3.2/§3.3）：
 * - 顶栏（返回/关闭 + 标题 + 数量）；
 * - 网格视图（PhotoAlbumGrid）↔ 全屏查看器（PhotoViewerOverlay）状态机；
 * - 打开时切深色状态栏，关闭恢复主题对应样式（与 AttachmentPreviewOverlay 一致）。
 */
export function PhotoAlbumOverlay({
  items,
  onClose,
  onOpenExternal,
  zIndex = 'var(--z-preview-overlay)',
}: PhotoAlbumOverlayProps) {
  const { t } = useTranslation('common');
  const [viewerIndex, setViewerIndex] = useState<number | null>(null);

  useEffect(() => {
    void syncStatusBarStyle('dark');
    return () => {
      const currentTheme = document.documentElement.getAttribute('data-theme');
      void syncStatusBarStyle(currentTheme === 'dark' ? 'dark' : 'light');
    };
  }, []);

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
            {items.length}
          </span>
        </div>
        <BadgeIconButton
          Icon={X}
          onClick={onClose}
          title={t('common:close', 'Close')}
          iconSize={ICON_SIZE.md}
        />
      </div>

      {/* 网格 */}
      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {items.length === 0 ? (
          <div
            style={{
              textAlign: 'center',
              padding: 48,
              color: 'var(--text-secondary)',
              fontSize: 'var(--text-body)',
            }}
          >
            {t('common:no_attachments', 'No attachments')}
          </div>
        ) : (
          <PhotoAlbumGrid items={items} onSelect={(_, i) => setViewerIndex(i)} />
        )}
      </div>

      {/* 全屏查看器（覆盖整个相册） */}
      {viewerIndex !== null && items[viewerIndex] && (
        <PhotoViewerOverlay
          items={items}
          initialIndex={viewerIndex}
          onBack={() => setViewerIndex(null)}
          onClose={onClose}
          onOpenExternal={onOpenExternal}
        />
      )}
    </div>
  );
}
