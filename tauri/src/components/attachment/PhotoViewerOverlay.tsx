import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AnimatePresence, motion } from 'framer-motion';
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  FilePen,
  RotateCcw,
  X,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { AttachmentMetaEditDialog } from '@/components/attachment/AttachmentMetaEditDialog';
import { ICON_SIZE } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';
// P048: 翻页/加载/缩放/手势逻辑抽到 hook
import { usePhotoViewer, swipeNavigation } from './usePhotoViewer';
export { computeFitScale } from '@/lib/photoZoom';
export { swipeNavigation } from './usePhotoViewer';

export interface PhotoViewerOverlayProps {
  items: AttachmentItem[];
  initialIndex: number;
  /** 返回照片集网格。 */
  onBack: () => void;
  /** 关闭整个相册。 */
  onClose: () => void;
  onOpenExternal?: (item: AttachmentItem) => void;
  /** 附件描述/标签保存成功后的回调（照片集同步最新元数据） */
  onItemMetaUpdated?: (updated: AttachmentItem) => void;
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

interface NavButtonProps {
  direction: 'prev' | 'next';
  onClick: () => void;
  label: string;
}

/** 左右翻页按钮：半透明悬浮圆钮，让用户感知可左右切换照片；悬停时加深。 */
function NavButton({ direction, onClick, label }: NavButtonProps) {
  const [hovered, setHovered] = useState(false);
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      title={label}
      aria-label={label}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        position: 'absolute',
        top: '50%',
        transform: 'translateY(-50%)',
        [direction === 'prev' ? 'left' : 'right']: 12,
        zIndex: 2,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: 42,
        height: 42,
        borderRadius: '50%',
        border: '1px solid rgba(255,255,255,0.18)',
        background: hovered ? 'rgba(255,255,255,0.26)' : 'rgba(255,255,255,0.12)',
        color: 'rgba(255,255,255,0.92)',
        cursor: 'pointer',
        transition: 'background 0.15s ease',
      }}
    >
      {direction === 'prev' ? (
        <ChevronLeft size={ICON_SIZE.xl} />
      ) : (
        <ChevronRight size={ICON_SIZE.xl} />
      )}
    </button>
  );
}

/**
 * 照片集全屏查看器（附件照片集方案 §3.3）。
 *
 * - 顶栏左右两个返回按钮都回到网格；顶部显示「{current} / {total}」计数；
 * - 左右滑动切换（framer-motion `drag="x"` + 阈值判定），桌面端补方向键，
 *   内容区两侧另置半透明 ◀ ▶ 翻页按钮让用户感知可左右切换；
 * - 缩放采用与附件预览一致的 fit-scale 模型：初始 scale = 适应视口比例（相对原始尺寸，
 *   大图如 26%），图片始终按 `原始尺寸 × scale` 渲染——缩放比例真实、缩小始终可见；
 *   图片超出视口（scale > fit）时容器 overflow auto 可滚动平移并禁用横向滑动翻页，
 *   避免手势冲突（`global.css` 全局 `touch-action: manipulation` 在此容器覆写为 pan-y）；
 * - 加载策略：仅缓存当前 ±1（index 变化丢弃过期 resolve，修复「慢 IPC 覆盖新图」竞态）。
 */
export function PhotoViewerOverlay({
  items,
  initialIndex,
  onBack,
  onClose,
  onOpenExternal,
  onItemMetaUpdated,
}: PhotoViewerOverlayProps) {
  const { t } = useTranslation('common');
  const [metaEditOpen, setMetaEditOpen] = useState(false);
  const {
    index,
    direction,
    url,
    setUrl,
    error,
    setError,
    loading,
    naturalSize,
    setNaturalSize,
    scale,
    contentRef,
    total,
    item,
    fitsViewport,
    goNext,
    goPrev,
    zoomIn,
    zoomOut,
    resetZoom,
    onZoomPointer,
    onZoomKeyboard,
    pinchActive,
  } = usePhotoViewer({ items, initialIndex, onClose });

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
    const displayWidth = naturalSize ? Math.round(naturalSize.w * scale) : undefined;
    const displayHeight = naturalSize ? Math.round(naturalSize.h * scale) : undefined;
    return (
      // 定宽高包装层：margin auto 居中——内容小于视口时水平垂直居中；放大超出视口时
      // auto 边距归零、按起始位置对齐，保证溢出边缘仍可滚动到达（与附件预览一致）。
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          margin: 'auto',
          display: 'inline-block',
          width: displayWidth,
          height: displayHeight,
          maxWidth: displayWidth !== undefined ? undefined : '100%',
          maxHeight: displayHeight !== undefined ? undefined : '100%',
          minWidth: 'auto',
          minHeight: 'auto',
          flexShrink: 0,
        }}
      >
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
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            borderRadius: 8,
            display: 'block',
            userSelect: 'none',
          }}
        />
      </div>
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
        // 点背景关闭（桌面端习惯）；图片放大超出视口时避免误触
        if (e.target === e.currentTarget && fitsViewport) onClose();
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
        {/* 描述/标签编辑入口（全屏照片，需求 2+3） */}
        <button
          type="button"
          onClick={() => setMetaEditOpen(true)}
          title={t('common:edit_meta', 'Edit Attachment Attributes')}
          aria-label={t('common:edit_meta', 'Edit Attachment Attributes')}
          style={iconButtonStyle}
          className="interactive-icon"
        >
          <FilePen size={ICON_SIZE.md} />
        </button>
        <button
          type="button"
          onClick={onBack}
          title={t('common:close', 'Close')}
          aria-label={t('common:close', 'Close')}
          style={iconButtonStyle}
          className="interactive-icon"
        >
          <X size={ICON_SIZE.md} />
        </button>
      </div>

      {/* 描述/标签编辑对话框（覆盖查看器） */}
      {metaEditOpen && item && (
        <AttachmentMetaEditDialog
          item={item}
          onClose={() => setMetaEditOpen(false)}
          onSaved={(updated) => {
            onItemMetaUpdated?.({ ...item, ...updated });
            setMetaEditOpen(false);
          }}
        />
      )}

      {/* 内容区：AnimatePresence 方向性滑动切换 + drag="x" 翻页；手势（捏合/双击）绑定在此容器 */}
      <div
        ref={contentRef}
        data-testid="photo-viewer-content"
        style={{ flex: 1, position: 'relative', overflow: 'hidden' }}
      >
        <AnimatePresence initial={false} custom={direction}>
          <motion.div
            key={index}
            custom={direction}
            initial={{ opacity: 0, x: 48 * direction }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -48 * direction }}
            transition={{ duration: 0.18, ease: 'easeOut' }}
            drag={pinchActive ? false : fitsViewport ? 'x' : false}
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
              overflow: fitsViewport ? 'hidden' : 'auto',
              // 覆写全局 touch-action: manipulation——图片未超出视口时横向拖拽交给 framer-motion；
              // 超出后容器溢出滚动优先，需恢复 auto 让浏览器处理双向滚动
              touchAction: fitsViewport ? 'pan-y' : 'auto',
            }}
          >
            {renderContent()}
          </motion.div>
        </AnimatePresence>

        {/* 左右翻页按钮：多图时显示，半透明提示可左右切换 */}
        {total > 1 && (
          <>
            <NavButton direction="prev" onClick={goPrev} label={t('common:previous', 'Previous')} />
            <NavButton direction="next" onClick={goNext} label={t('common:next', 'Next')} />
          </>
        )}
      </div>

      {/* 缩放控件（仅图片就绪时显示）
          交互加固：缩放条悬浮于 framer-motion drag 层之上，显式 zIndex 确保绘制在上层、
          touch-action: manipulation 声明本区域仅为点按；按钮主路径走 onPointerDown（触控/鼠标
          按下即响应），键盘激活经 click(detail===0) 兜底，右键/中键不缩放。 */}
      {showZoomControls && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            position: 'absolute',
            bottom: 24,
            left: '50%',
            transform: 'translateX(-50%)',
            zIndex: 10,
            touchAction: 'manipulation',
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
            type="button"
            onPointerDown={onZoomPointer(zoomOut)}
            onClick={onZoomKeyboard(zoomOut)}
            title={t('common:attachment_zoom_out', 'Zoom Out')}
            style={{ ...iconButtonStyle, width: 28, height: 28 }}
          >
            <ZoomOut size={ICON_SIZE.lg} />
          </button>
          <span style={{ minWidth: 52, textAlign: 'center' }}>{Math.round(scale * 100)}%</span>
          <button
            type="button"
            onPointerDown={onZoomPointer(zoomIn)}
            onClick={onZoomKeyboard(zoomIn)}
            title={t('common:attachment_zoom_in', 'Zoom In')}
            style={{ ...iconButtonStyle, width: 28, height: 28 }}
          >
            <ZoomIn size={ICON_SIZE.lg} />
          </button>
          <button
            type="button"
            onPointerDown={onZoomPointer(resetZoom)}
            onClick={onZoomKeyboard(resetZoom)}
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
