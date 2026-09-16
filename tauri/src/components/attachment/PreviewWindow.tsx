import type { CSSProperties, HTMLAttributes } from 'react';
import { createPortal } from 'react-dom';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import './preview-window.css';

/** macOS 独立预览层：原页面可以整体避开顶栏，预览控件仍在同一行显示。 */
export function PreviewWindow({
  portal = true,
  style,
  ...props
}: HTMLAttributes<HTMLDivElement> & { portal?: boolean }) {
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const surface = (
    <div
      {...props}
      data-preview-window
      style={
        {
          ...style,
          '--preview-surface': style?.background ?? 'var(--bg-elevated)',
          '--preview-blur': style?.backdropFilter ?? 'none',
        } as CSSProperties
      }
    />
  );
  // 照片查看器及其加载态留在相册内，保留相册 → 查看器的层级和返回行为。
  return isMacOS && portal ? createPortal(surface, document.body) : surface;
}
