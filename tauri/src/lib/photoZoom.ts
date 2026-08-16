/**
 * 图片缩放共享常量与纯函数（P049：AttachmentPreviewOverlay 与 PhotoViewerOverlay
 * 两处逐字重复的缩放/平移逻辑收敛于此）。
 */

export const MIN_SCALE = 0.1;
export const MAX_SCALE = 5;
export const ZOOM_STEP = 1.2;

/** 钳制缩放比例到 [MIN_SCALE, MAX_SCALE]。 */
export function clampScale(value: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));
}

/**
 * 纯函数：计算「适应视口」缩放比例（相对图片原始尺寸）。
 * 取宽/高两个方向的适配比中较小者，且不超过 1（小图不放大）。
 */
export function computeFitScale(
  clientWidth: number,
  clientHeight: number,
  naturalWidth: number,
  naturalHeight: number,
): number {
  if (clientWidth <= 0 || clientHeight <= 0 || naturalWidth <= 0 || naturalHeight <= 0) {
    return 1;
  }
  return Number(Math.min(clientWidth / naturalWidth, clientHeight / naturalHeight, 1).toFixed(3));
}
