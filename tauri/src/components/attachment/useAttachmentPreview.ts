/**
 * AttachmentPreviewOverlay 数据层 hook（P048 拆分：加载/缩放/手势逻辑与渲染分离）。
 * 含预览类型判定、文件加载（data URL / 自定义协议 / 文本）、缩放状态与安卓手势。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { syncStatusBarStyle } from '@/lib/theme';
import { useTouchZoom } from '@/hooks/useTouchZoom';
import { previewItemByMime, type AttachmentItem } from '@/lib/attachmentUtils';
import { isMobilePlatformSync, isWindowsSync } from '@/lib/platform';
import { MIN_SCALE, MAX_SCALE, ZOOM_STEP, clampScale, computeFitScale } from '@/lib/photoZoom';
import { invokeCommand as invoke } from '@/lib/ipcClient';

export type PreviewKind = 'image' | 'pdf' | 'text' | 'other';

export function isUriPath(path: string): boolean {
  return path.startsWith('content://') || path.startsWith('file://');
}

/**
 * 构造 `solosoul-pdf://` 自定义协议的 embed URL（仅 Windows 使用）。
 *
 * WebView2/Chromium 内建 PDF 查看器无法从 data:/blob: URL 可靠渲染 <embed>
 * （data: 无源标识 PDFium 拒绝加载；blob: 在 WebView2 子帧同样不稳定），且
 * `fs_read_file_as_data_url` 有 10 MiB 上限会拒绝真实大 PDF。改用自定义协议
 * 直出 application/pdf 字节（后端经 resolve_allowed_path 白名单校验 + 扩展名
 * 守卫），WebView2 按常规 HTTP 资源渲染，无 base64 膨胀、无大小上限。
 *
 * tauri 2.x 自定义协议在 Windows 的 URL 形态为 `http://<scheme>.localhost/<path>`
 * （其余桌面平台的 `<scheme>://localhost/<path>` 形态 WKWebView 渲染不确定，
 * 故 macOS/Linux 仍走原 data: URL 路径，不走本协议）。path 经
 * encodeURIComponent 编码（与后端 percent_decode_str 配对）。
 */
function buildPdfPreviewSrc(vaultPath: string): string {
  const encoded = encodeURIComponent(vaultPath);
  return `http://solosoul-pdf.localhost/${encoded}`;
}

export interface UseAttachmentPreviewParams {
  item: AttachmentItem | null;
}

export function useAttachmentPreview({ item }: UseAttachmentPreviewParams) {
  const [previewKind, setPreviewKind] = useState<PreviewKind | null>(null);
  const [previewUrl, setPreviewUrl] = useState('');
  const [textContent, setTextContent] = useState('');
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(false);
  const [naturalSize, setNaturalSize] = useState<{ width: number; height: number } | null>(null);
  const [scale, setScale] = useState(1);
  /** 适应视口比例（相对原始尺寸）；双击以此为基准放大、捏合回落低于它时回弹。 */
  const [fitScale, setFitScale] = useState(1);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!item) {
      setPreviewKind(null);
      setPreviewUrl('');
      setTextContent('');
      setError(false);
      setLoading(false);
      setNaturalSize(null);
      setScale(1);
      setFitScale(1);
      // 关闭预览遮罩后恢复应用主题对应的状态栏样式
      const currentTheme = document.documentElement.getAttribute('data-theme');
      void syncStatusBarStyle(currentTheme === 'dark' ? 'dark' : 'light');
      return;
    }

    // 打开预览遮罩时使用深色背景配浅色状态栏图标/文字
    void syncStatusBarStyle('dark');

    const rawKind = previewItemByMime(item);
    // Android/iOS WebView 无法通过 <embed> 直接渲染本地 PDF data URL，
    // 统一交给系统应用打开。
    const kind = rawKind === 'pdf' && isMobilePlatformSync() ? 'other' : rawKind;
    setPreviewKind(kind);
    setPreviewUrl('');
    setTextContent('');
    setError(false);
    setLoading(true);
    setNaturalSize(null);
    setScale(1);
    setFitScale(1);

    const filePath = item.vaultPath;
    if (!filePath || isUriPath(filePath)) {
      setError(true);
      setLoading(false);
      return;
    }

    if (kind === 'image') {
      invoke<string>('fs_read_file_as_data_url', { path: filePath })
        .then((url) => {
          // P017：图片走 data URL（img-src data: 放行），保留代码层守卫
          setPreviewUrl(url);
        })
        .catch(() => setError(true))
        .finally(() => setLoading(false));
    } else if (kind === 'pdf') {
      if (isWindowsSync()) {
        // Windows（WebView2/PDFium）：无法渲染 data:/blob: URL 的 embed，且 10 MiB
        // 上限会拒绝真实 PDF。经自定义协议 solosoul-pdf:// 直出 application/pdf
        // 字节（后端白名单校验），同步设置 URL 即可，无需 invoke。
        setPreviewUrl(buildPdfPreviewSrc(filePath));
        setLoading(false);
      } else {
        // macOS/Linux（WKWebView/WebKit）：data: URL 原生可渲染，保持既有路径
        //（自定义协议形态在 WebKit 的渲染支持不确定，不做回归风险）。
        invoke<string>('fs_read_file_as_data_url', { path: filePath })
          .then((url) => {
            // P017 守卫——仅 application/pdf data URL 允许进入 <embed>
            if (!url.startsWith('data:application/pdf')) {
              setError(true);
              return;
            }
            setPreviewUrl(url);
          })
          .catch(() => setError(true))
          .finally(() => setLoading(false));
      }
    } else if (kind === 'text') {
      invoke<string>('fs_read_file_as_text', { path: filePath })
        .then(setTextContent)
        .catch(() => setError(true))
        .finally(() => setLoading(false));
    } else {
      // 'other' files are not loaded automatically.
      setLoading(false);
    }
  }, [item]);

  // Calculate an initial scale that fits the image inside the viewport.
  const fitToView = useCallback(() => {
    const container = scrollRef.current;
    if (!container || !naturalSize) return;
    if (naturalSize.width === 0 || naturalSize.height === 0) return;
    // P049: 共享 computeFitScale（与 PhotoViewerOverlay 逐字同构，收敛消除重复）
    const fit = computeFitScale(
      container.clientWidth,
      container.clientHeight,
      naturalSize.width,
      naturalSize.height,
    );
    setFitScale(fit);
    setScale(fit);
  }, [naturalSize]);

  useEffect(() => {
    if (naturalSize) {
      fitToView();
    }
  }, [naturalSize, fitToView]);

  useEffect(() => {
    const handleResize = () => {
      if (naturalSize) {
        fitToView();
      }
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [naturalSize, fitToView]);

  // 安卓端手势：双指捏合缩放 + 双击切换（绑定图片滚动容器，单指拖动仍是原生滚动平移）
  useTouchZoom({
    elementRef: scrollRef,
    scale,
    setScale,
    fitScale,
    fitToView,
    minScale: MIN_SCALE,
    maxScale: MAX_SCALE,
  });

  const zoomIn = () => setScale((s) => clampScale(s * ZOOM_STEP));
  const zoomOut = () => setScale((s) => clampScale(s / ZOOM_STEP));
  const resetZoom = () => fitToView();

  /** 图片渲染尺寸是否未超出视口（与 PhotoViewerOverlay 一致，带 epsilon 容忍浮点回环）。
   *  未超出时容器 touch-action 覆写为 pan-y——全局 `touch-action: manipulation`
   *  允许 pinch-zoom，浏览器会原生抢走双指手势导致捏合失效；pan-y 不含 pinch-zoom，
   *  事件完整派发给 useTouchZoom。超出后恢复 auto 让浏览器处理双向滚动平移。 */
  const fitsViewport =
    previewKind === 'image' && naturalSize !== null ? scale <= fitScale + 0.001 : true;

  const handleWheel = (e: React.WheelEvent) => {
    // Ctrl/Cmd + wheel zooms; plain wheel scrolls the container normally.
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const factor = e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
      setScale((s) => clampScale(s * factor));
    }
  };

  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setNaturalSize({ width: img.naturalWidth, height: img.naturalHeight });
  };

  return {
    previewKind,
    previewUrl,
    textContent,
    error,
    loading,
    naturalSize,
    scale,
    fitScale,
    scrollRef,
    fitsViewport,
    zoomIn,
    zoomOut,
    resetZoom,
    handleWheel,
    handleImageLoad,
  };
}
