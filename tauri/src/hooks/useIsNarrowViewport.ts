import { useEffect, useState } from 'react';

const MOBILE_BREAKPOINT = 768;

/**
 * 检测当前视口是否为窄视口（< 768px，主要用于移动端布局）。
 * 基于 window.matchMedia，会在尺寸变化时自动更新。
 * 注意：这是布局用途，不是平台判定；平台判定请使用 lib/platform 中的 isMobilePlatform/isMobilePlatformSync。
 */
export function useIsNarrowViewport(): boolean {
  const [isNarrowViewport, setIsNarrowViewport] = useState(() => {
    if (typeof window === 'undefined') return false;
    return window.innerWidth < MOBILE_BREAKPOINT;
  });

  useEffect(() => {
    if (typeof window === 'undefined') return;

    const mediaQuery = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    const handleChange = (event: MediaQueryListEvent | MediaQueryList) => {
      setIsNarrowViewport('matches' in event ? event.matches : (event as MediaQueryList).matches);
    };

    handleChange(mediaQuery);
    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  return isNarrowViewport;
}
