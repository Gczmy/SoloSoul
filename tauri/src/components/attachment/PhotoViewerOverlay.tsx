import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AnimatePresence, motion } from 'framer-motion';
import { ArrowLeft, ExternalLink, RotateCcw, X, ZoomIn, ZoomOut } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ICON_SIZE } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { loadFullPreviewUrl } from '@/lib/photoAlbumPreview';

export interface PhotoViewerOverlayProps {
  items: AttachmentItem[];
  initialIndex: number;
  /** 返回照片集网格。 */
  onBack: () => void;
  /** 关闭整个相册。 */
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
}

/** 横向滑动翻页阈值（px）。 */
const SWIPE_THRESHOLD = 60;
const MIN_SCALE = 0.1;
const MAX_SCALE = 5;
const ZOOM_STEP = 1.2;

/** 纯函数：根据拖拽横向位移判断翻页方向（左滑 → 下一张，右滑 → 上一张）。 */
export function swipeNavigation(offsetX: number, threshold = SWIPE_THRESHOLD): -1 | 0 | 1 {
  if (offsetX <= -threshold) return 1; // 下一张
  if (offsetX >= threshold) return -1; // 上一张
  return 0;
}

const iconButtonStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: 36,
  height: 36,
  borderRadius: 8,
  border: 'none',
  background: 'transparent',
  color: 'rgba(255,255,255,0.85)',
  cursor: 'pointer',
  flexShrink: 0,
};

/**
 * 照片集全屏查看器（附件照片集方案 §3.3）。
 *
 * - 左上角返回按钮回到网格；顶部显示「{current} / {total}」计数；
 * - 左右滑动切换（framer-motion `drag="x"` + 阈值判定），桌面端补方向键；
 * - 仅 scale ≤ 1（未放大）时响应横向 swipe——放大后容器 overflow auto 滚动优先，
 *   避免手势冲突（`global.css` 全局 `touch-action: manipulation` 在此容器覆写为 pan-y）；
 * - 加载策略：仅缓存当前 ±1（index 变化丢弃过期 resolve，修复「慢 IPC 覆盖新图」竞态）。
 */
export function PhotoViewerOverlay({
  items,
  initialIndex,
  onBack,
  onClose,
  onOpenExternal,
}: PhotoViewerOverlayProps) {
  const { t } = useTranslation('common');
  const [index, setIndex] = useState(initialIndex);
  const [direction, setDirection] = useState(1);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(null);
  const [scale, setScale] = useState(1);

  const total = items.length;
  const item = items[index];
  const zoomed = scale > 1;

  const goTo = useCallback(
    (targetRaw: number) => {
      if (total === 0) return;
      const target = ((targetRaw % total) + total) % total;
      setDirection(target === index ? 1 : target > index ? 1 : -1);
      setIndex(target);
    },
    [index, total],
  );
  const goNext = useCallback(() => goTo(index + 1), [goTo, index]);
  const goPrev = useCallback(() => goTo(index - 1), [goTo, index]);

  // 竞态保护：index（item）变化时丢弃过期 resolve，避免慢 IPC 覆盖新图。
  useEffect(() => {
    if (!item) return;
    let stale = false;
    setLoading(true);
    setError(false);
    setUrl(null);
    setNaturalSize(null);
    setScale(1);
    loadFullPreviewUrl(item)
      .then((u) => {
        if (!stale) setUrl(u);
      })
      .catch(() => {
        if (!stale) setError(true);
      })
      .finally(() => {
        if (!stale) setLoading(false);
      });
    return () => {
      stale = true;
    };
  }, [item]);

  // 桌面端方向键 + Esc（goNext/goPrev 经 useCallback 保持新鲜闭包）。
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'ArrowLeft') goPrev();
      else if (e.key === 'ArrowRight') goNext();
      else if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [goNext, goPrev, onClose]);

  const clampScale = (v: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, v));
  const zoomIn = () => setScale((s) => clampScale(s * ZOOM_STEP));
  const zoomOut = () => setScale((s) => clampScale(s / ZOOM_STEP));
  const resetZoom = () => setScale(1);

  const showZoomControls = !!url && !loading && !error;

  const renderContent = () => {
    if (!item) return null;
    if (error) {
      return (
        <div style={{ margin: 'auto', color: '#fff', padding: 24, textAlign: 'center' }}>
          <div>{t('common:attachment_preview_failed', 'Failed to load preview.')}</div>
          {onOpenExternal && (
            <button
              onClick={() => onOpenExternal(item)}
              style={{
                marginTop: 16,
                display: 'inline-flex',
                alignItems: 'center',
                gap: 8,
                padding: '8px 16px',
                borderRadius: 8,
                border: '1px solid rgba(255,255,255,0.2)',
                background: 'rgba(28,28,30,0.8)',
                color: '#fff',
                cursor: 'pointer',
              }}
            >
              <ExternalLink size={ICON_SIZE.md} />
              <span>{t('common:attachment_open_system', 'Open with system app')}</span>
            </button>
          )}
        </div>
      );
    }
    if (loading) {
      return <LoadingPlaceholder variant="toolbar" minHeight={120} style={{ margin: 'auto' }} />;
    }
    if (!url) return null;
    return (
      <img
        src={url}
        alt={item.fileName}
        draggable={false}
        onLoad={(e) =>
          setNaturalSize({ w: e.currentTarget.naturalWidth, h: e.currentTarget.naturalHeight })
        }
        onError={() => {
          // data URL 拿到但浏览器无法渲染（如 HEIC）→ 进入错误态降级「使用系统应用打开」
          setError(true);
          setUrl(null);
        }}
        style={{
          margin: 'auto',
          maxWidth: zoomed ? 'none' : '100%',
          maxHeight: zoomed ? 'none' : '100%',
          width: zoomed && naturalSize ? naturalSize.w * scale : 'auto',
          height: zoomed && naturalSize ? naturalSize.h * scale : 'auto',
          objectFit: 'contain',
          borderRadius: 8,
          transition: 'width 0.15s ease, height 0.15s ease',
          userSelect: 'none',
        }}
      />
    );
  };

  return (
    <div
      data-testid="photo-viewer"
      style={{
        position: 'absolute',
        inset: 0,
        zIndex: 1,
        display: 'flex',
        flexDirection: 'column',
        background: 'rgba(0,0,0,0.94)',
        backdropFilter: 'blur(12px)',
      }}
      onClick={(e) => {
        // 点背景关闭（桌面端习惯）；缩放时避免误触
        if (e.target === e.currentTarget && scale <= 1) onClose();
      }}
    >
      {/* 顶栏：返回网格 + 文件名 + 计数 + 关闭 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '10px 14px',
          background: 'rgba(28,28,30,0.6)',
          borderBottom: '1px solid rgba(255,255,255,0.08)',
          flexShrink: 0,
        }}
      >
        <button
          type="button"
          onClick={onBack}
          title={t('common:back_to_album', 'Back to album')}
          aria-label={t('common:back_to_album', 'Back to album')}
          style={iconButtonStyle}
          className="interactive-icon"
        >
          <ArrowLeft size={ICON_SIZE.md} />
        </button>
        <span
          style={{
            flex: 1,
            minWidth: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            color: '#fff',
          }}
        >
          {item?.fileName}
        </span>
        <span
          data-testid="photo-viewer-counter"
          aria-label={t('common:photo_album_counter', {
            current: index + 1,
            total,
          })}
          style={{
            fontSize: 'var(--text-caption)',
            color: 'rgba(255,255,255,0.7)',
            whiteSpace: 'nowrap',
          }}
        >
          {index + 1} / {total}
        </span>
        <button
          type="button"
          onClick={onClose}
          title={t('common:close', 'Close')}
          aria-label={t('common:close', 'Close')}
          style={iconButtonStyle}
          className="interactive-icon"
        >
          <X size={ICON_SIZE.md} />
        </button>
      </div>

      {/* 内容区：AnimatePresence 方向性滑动切换 + drag="x" 翻页 */}
      <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
        <AnimatePresence initial={false} custom={direction}>
          <motion.div
            key={index}
            custom={direction}
            initial={{ opacity: 0, x: 48 * direction }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -48 * direction }}
            transition={{ duration: 0.18, ease: 'easeOut' }}
            drag={scale <= 1 ? 'x' : false}
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.12}
            onDragEnd={(_, info) => {
              const nav = swipeNavigation(info.offset.x);
              if (nav === 1) goNext();
              else if (nav === -1) goPrev();
            }}
            style={{
              position: 'absolute',
              inset: 0,
              display: 'flex',
              overflow: zoomed ? 'auto' : 'hidden',
              // 覆写全局 touch-action: manipulation——未放大时横向拖拽交给 framer-motion；
              // 放大后容器溢出滚动优先，需恢复 auto 让浏览器处理双向滚动
              touchAction: zoomed ? 'auto' : 'pan-y',
            }}
          >
            {renderContent()}
          </motion.div>
        </AnimatePresence>
      </div>

      {/* 缩放控件（仅图片就绪时显示） */}
      {showZoomControls && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            position: 'absolute',
            bottom: 24,
            left: '50%',
            transform: 'translateX(-50%)',
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: '8px 14px',
            borderRadius: 24,
            background: 'rgba(28,28,30,0.85)',
            border: '1px solid rgba(255,255,255,0.12)',
            color: '#fff',
            fontSize: 'var(--text-body-sm)',
            userSelect: 'none',
          }}
        >
          <button
            onClick={zoomOut}
            title={t('common:attachment_zoom_out', 'Zoom Out')}
            style={{ ...iconButtonStyle, width: 28, height: 28 }}
          >
            <ZoomOut size={ICON_SIZE.lg} />
          </button>
          <span style={{ minWidth: 52, textAlign: 'center' }}>
            {Math.round(scale * 100)}%
          </span>
          <button
            onClick={zoomIn}
            title={t('common:attachment_zoom_in', 'Zoom In')}
            style={{ ...iconButtonStyle, width: 28, height: 28 }}
          >
            <ZoomIn size={ICON_SIZE.lg} />
          </button>
          <button
            onClick={resetZoom}
            title={t('common:attachment_zoom_fit', 'Fit to window')}
            style={{ ...iconButtonStyle, width: 28, height: 28 }}
          >
            <RotateCcw size={ICON_SIZE.lg} />
          </button>
        </div>
      )}
    </div>
  );
}
