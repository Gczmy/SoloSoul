import { useCallback, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

export type PopoverPlacement = 'top' | 'right' | 'bottom' | 'left';
export type NavCardPosition = { top: number } | null;

/** 快捷卡片在浏览器绘制前完成定位，避免先显示默认坐标再跳到按钮旁。 */
export function useNavCardPosition(
  isOpen: boolean,
  cardHeight: number,
  placement: PopoverPlacement,
) {
  const titlebarHeight = useNativeWindowStore((s) => s.titlebarHeight);
  const buttonRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<NavCardPosition>(null);
  const updatePosition = useCallback(() => {
    const button = buttonRef.current;
    if (!button) return;
    const rect = button.getBoundingClientRect();
    const preferredTop =
      placement === 'bottom'
        ? rect.bottom + 8
        : placement === 'top'
          ? rect.top - cardHeight - 8
          : rect.top;
    const top = Math.max(
      titlebarHeight + 8,
      Math.min(preferredTop, window.innerHeight - cardHeight - 8),
    );
    setPosition((previous) => (previous?.top === top ? previous : { top }));
  }, [cardHeight, placement, titlebarHeight]);

  useLayoutEffect(() => {
    if (!isOpen) return;
    updatePosition();
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [isOpen, updatePosition]);

  return { buttonRef, position };
}

/** 加载占位与卡片内容使用相同定位规则；尚未取得按钮坐标时不显示。 */
export function getNavCardStyle(
  position: NavCardPosition,
  placement: PopoverPlacement,
): CSSProperties {
  const sidebarInset = 'calc(var(--sidebar-width, 48px) + 4px)';
  return {
    ...(placement === 'top' || placement === 'bottom'
      ? { right: 12, left: 'auto' }
      : placement === 'right'
        ? { right: sidebarInset, left: 'auto' }
        : { left: sidebarInset, right: 'auto' }),
    top: position?.top,
    visibility: position ? undefined : 'hidden',
  };
}
