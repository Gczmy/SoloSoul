import { useState, useRef, useCallback, useEffect } from 'react';
import type { CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import styles from './NavButton.module.css';
import { ICON_SIZE } from '@/lib/constants';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import { prefetchRoute } from '@/App/routeLoaders';

export type NavPosition = 'left' | 'right' | 'top' | 'bottom';

interface NavButtonProps {
  path?: string;
  /** 悬停/触摸预取目标路径；缺省时回退到 path（避免 ai_chat 卡片模式为预取而影响 active 指示点）。 */
  prefetchPath?: string;
  Icon: LucideIcon;
  label: string;
  isActive?: boolean;
  onClick: () => void;
  position?: NavPosition;
}

export function NavButton({
  path,
  prefetchPath,
  Icon,
  label,
  isActive,
  onClick,
  position = 'left',
}: NavButtonProps) {
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [indicatorStyle, setIndicatorStyle] = useState<CSSProperties | null>(null);

  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';

  const { cardStyle, isHovered, handleMouseEnter, handleMouseLeave } = useHoverCardPosition(
    wrapperRef,
    { isHorizontal, isBottom, isRight },
  );

  const updateIndicator = useCallback(() => {
    if (!wrapperRef.current || !isActive) return;
    const rect = wrapperRef.current.getBoundingClientRect();
    if (position === 'top') {
      setIndicatorStyle({
        top: rect.bottom + 4,
        left: rect.left + rect.width / 2,
        transform: 'translateX(-50%)',
        width: 20,
        height: 3,
      });
    } else if (position === 'bottom') {
      setIndicatorStyle({
        bottom: window.innerHeight - rect.top + 4,
        left: rect.left + rect.width / 2,
        transform: 'translateX(-50%)',
        width: 20,
        height: 3,
      });
    } else if (position === 'left') {
      setIndicatorStyle({
        top: rect.top + rect.height / 2,
        left: rect.right + 4,
        transform: 'translateY(-50%)',
        width: 3,
        height: 20,
      });
    } else {
      // right
      setIndicatorStyle({
        top: rect.top + rect.height / 2,
        right: window.innerWidth - rect.left + 4,
        transform: 'translateY(-50%)',
        width: 3,
        height: 20,
      });
    }
  }, [isActive, position]);

  useEffect(() => {
    if (!isActive) return;
    updateIndicator();
    window.addEventListener('scroll', updateIndicator, true);
    window.addEventListener('resize', updateIndicator);
    return () => {
      window.removeEventListener('scroll', updateIndicator, true);
      window.removeEventListener('resize', updateIndicator);
    };
  }, [isActive, updateIndicator]);

  const nameCard = isHovered ? (
    <div
      className={isHorizontal ? styles.nameCardPortalHorizontal : styles.nameCardPortal}
      style={{
        position: 'fixed',
        ...cardStyle,
        zIndex: 200,
      }}
      role="tooltip"
      aria-hidden="true"
    >
      {label}
    </div>
  ) : null;

  // P015-R5: 悬停（pointerenter）/按下（pointerdown）/聚焦（focus）时预热目标页面 chunk，
  // 点击导航时 chunk 已命中缓存，消除切页骨架屏。prefetchPath 优先于 path，两者皆空
  // （动作型按钮）时静默跳过。
  const handlePrefetch = useCallback(() => {
    const target = prefetchPath ?? path;
    if (target) prefetchRoute(target);
  }, [path, prefetchPath]);

  const activeIndicator =
    path && isActive ? (
      <div
        className={styles.activeIndicatorPortal}
        style={{
          position: 'fixed',
          ...indicatorStyle,
          zIndex: 199,
        }}
        aria-hidden="true"
      />
    ) : null;

  return (
    <div
      ref={wrapperRef}
      className={styles.navItemWrapper}
      style={isHorizontal ? { width: 40, height: 40 } : {}}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onPointerEnter={handlePrefetch}
      onPointerDown={handlePrefetch}
    >
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={onClick}
        onFocus={handlePrefetch}
        aria-label={label}
        aria-current={isActive ? 'page' : undefined}
        style={isHorizontal ? { width: 40, height: 40, borderRadius: 10 } : {}}
        data-tauri-drag-region="false"
      >
        <Icon size={ICON_SIZE.xl} />
      </button>
      {createPortal(nameCard, document.body)}
      {createPortal(activeIndicator, document.body)}
    </div>
  );
}
