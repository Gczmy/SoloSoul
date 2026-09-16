import { useLayoutEffect, useRef, type CSSProperties, type ReactNode } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { observeTitlebarControls } from '@/lib/nativeTitlebarControls';
import './preview-window.css';

/** 文件预览/照片集共用整行原生玻璃顶栏，控件避开交通灯。 */
export function PreviewTitlebar({
  children,
  style,
  active = true,
}: {
  children: ReactNode;
  style?: CSSProperties;
  active?: boolean;
}) {
  const isMacOS = useNativeWindowStore((s) => s.isMacOS);
  const titlebarHeight = useNativeWindowStore((s) => s.titlebarHeight);
  const trafficLightsRight = useNativeWindowStore((s) => s.trafficLightsRight);
  const headerRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (!isMacOS || !active || !headerRef.current) return;
    return observeTitlebarControls(headerRef.current, { priority: 1, selector: 'button' });
  }, [isMacOS, active]);

  return (
    <div
      ref={headerRef}
      data-preview-titlebar
      data-active={active}
      data-tauri-drag-region="false"
      onClick={(event) => event.stopPropagation()}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        paddingTop: 10,
        paddingBottom: 10,
        paddingLeft: 14,
        paddingRight: 14,
        flexShrink: 0,
        ...style,
        ...(isMacOS && {
          minHeight: titlebarHeight || 52,
          paddingTop: 0,
          paddingBottom: 0,
          // 全屏隐藏交通灯时恢复普通内边距，不额外增加一行空白。
          paddingLeft:
            trafficLightsRight > 0 ? trafficLightsRight + 12 : (style?.paddingLeft ?? 14),
        }),
      }}
    >
      {children}
    </div>
  );
}
