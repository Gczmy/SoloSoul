import { lazy } from 'react';

/**
 * PhotoViewerOverlay 的共享懒加载封装（候选 2 第四步）。
 *
 * PhotoViewerOverlay 包含复杂手势动画（drag/pinch/AnimatePresence），
 * CSS 无法替代——通过 React.lazy 将其（含 framer-motion）移出 PageContainer 共享 chunk，
 * 仅在用户点击照片集查看器时按需加载。消费方单一（PhotoAlbumOverlay），无需漂移保护。
 */
export const LazyPhotoViewerOverlay = lazy(() =>
  import('./PhotoViewerOverlay').then((m) => ({
    default: m.PhotoViewerOverlay,
  })),
);
