/**
 * PhotoViewerOverlay 数据层 hook（P048 拆分：翻页/加载/缩放/手势逻辑与渲染分离）。
 * 含索引状态机、±1 缓存加载（竞态保护）、fit-scale 模型与安卓捏合/双击手势。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTouchZoom } from '@/hooks/useTouchZoom';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { loadFullPreviewUrl } from '@/lib/photoAlbumPreview';
import { MIN_SCALE, MAX_SCALE, ZOOM_STEP, clampScale, computeFitScale } from '@/lib/photoZoom';

/** 横向滑动翻页阈值（px）。 */
export const SWIPE_THRESHOLD = 60;

/** 纯函数：根据拖拽横向位移判断翻页方向（左滑 → 下一张，右滑 → 上一张）。 */
export function swipeNavigation(offsetX: number, threshold = SWIPE_THRESHOLD): -1 | 0 | 1 {
  if (offsetX <= -threshold) return 1; // 下一张
  if (offsetX >= threshold) return -1; // 上一张
  return 0;
}

export interface UsePhotoViewerParams {
  items: AttachmentItem[];
  initialIndex: number;
  onClose: () => void;
}

export function usePhotoViewer({ items, initialIndex, onClose }: UsePhotoViewerParams) {
  const [index, setIndex] = useState(initialIndex);
  const [direction, setDirection] = useState(1);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(null);
  const [scale, setScale] = useState(1);
  /** 适应视口比例（相对原始尺寸，见 fitToView）；图片未放大时 scale 即为此值。 */
  const [fitScale, setFitScale] = useState(1);
  /** 图片展示区（内容区容器，跨照片稳定存在）：fitToView 以 clientWidth/Height 计算适应比例，
   *  双指捏合/双击手势也绑定在此（motion.div 随 index 重建，不宜作为监听宿主）。 */
  const contentRef = useRef<HTMLDivElement>(null);

  const total = items.length;
  const item = items[index];
  /** 图片渲染尺寸是否未超出视口：超出时容器 overflow auto 可平移、禁用左右滑动翻页。
   *  带 epsilon 容忍缩放 ×1.2 ÷1.2 浮点回环的亚像素误差（如 fit=0.487 回环后 0.4870000000000001）。 */
  const fitsViewport = naturalSize !== null ? scale <= fitScale + 0.001 : true;

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
    setFitScale(1);
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

  const zoomIn = () => setScale((s) => clampScale(s * ZOOM_STEP));
  const zoomOut = () => setScale((s) => clampScale(s / ZOOM_STEP));

  /** 计算「适应视口」缩放比例（相对图片原始尺寸）并与附件预览一致：
   *  初始显示为 fit（大图如 26% 而非 100%），此后缩放均基于原始尺寸，比例真实。 */
  const fitToView = useCallback(() => {
    const container = contentRef.current;
    if (!container || !naturalSize) return;
    const fit = computeFitScale(
      container.clientWidth,
      container.clientHeight,
      naturalSize.w,
      naturalSize.h,
    );
    setFitScale(fit);
    setScale(fit);
  }, [naturalSize]);

  // 图片元信息就绪后（容器已布局）计算 fit 比例；窗口尺寸变化（横竖屏/桌面缩放）时重算。
  useEffect(() => {
    if (naturalSize) fitToView();
  }, [naturalSize, fitToView]);

  useEffect(() => {
    const handleResize = () => {
      if (naturalSize) fitToView();
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [naturalSize, fitToView]);

  const resetZoom = () => fitToView();

  // Android 加固：缩放按钮主路径走 pointerdown（先于任何手势取消 click 触发），
  // 但仅响应主按键（右键/中键不缩放）；键盘激活无 pointerdown，经 click(detail===0) 兜底。
  const onZoomPointer = (action: () => void) => (e: React.PointerEvent) => {
    if (e.button === 0) action();
  };
  const onZoomKeyboard = (action: () => void) => (e: React.MouseEvent<HTMLButtonElement>) => {
    if (e.detail === 0) action();
  };

  // 安卓端手势：双指捏合缩放 + 双击切换（捏合进行中暂停 swipe 翻页，见 drag 门控）
  const { pinchActive } = useTouchZoom({
    elementRef: contentRef,
    scale,
    setScale,
    fitScale,
    fitToView,
    minScale: MIN_SCALE,
    maxScale: MAX_SCALE,
  });

  return {
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
    setScale,
    fitScale,
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
  };
}
