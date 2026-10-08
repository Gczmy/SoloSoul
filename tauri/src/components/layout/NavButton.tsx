import { useRef, useContext, useState } from 'react';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import styles from './NavButton.module.css';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import { useScrollingNavLabel } from '@/hooks/useScrollingNavLabel';

export type NavPosition = 'left' | 'right' | 'top' | 'bottom';

interface NavButtonProps {
  path?: string;
  Icon: LucideIcon;
  label: string;
  isActive?: boolean;
  onClick: () => void;
  position?: NavPosition;
  scrollOverflowLabel?: boolean;
}

export function NavButton({
  path,
  Icon,
  label,
  isActive,
  onClick,
  position = 'left',
  scrollOverflowLabel = false,
}: NavButtonProps) {
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);

  const isHorizontal = position === 'top' || position === 'bottom';
  const sidebarExpanded = useContext(DesktopSidebarContext) && !isHorizontal;
  const isBottom = position === 'bottom';
  const isRight = position === 'right';

  const { cardStyle, isHovered, handleMouseEnter, handleMouseLeave, updateCardPosition } =
    useHoverCardPosition(wrapperRef, { isHorizontal, isBottom, isRight });
  const scrollingEnabled = sidebarExpanded && scrollOverflowLabel;
  const { viewportRef, textRef, isOverflowing, staticMode } = useScrollingNavLabel(
    label,
    scrollingEnabled,
    isHovered || isFocused,
  );

  const nameCard =
    (isHovered || isFocused) &&
    (!sidebarExpanded || (scrollingEnabled && isOverflowing && staticMode)) ? (
      <div
        className={isHorizontal ? styles.nameCardPortalHorizontal : styles.nameCardPortal}
        style={{
          position: 'fixed',
          ...cardStyle,
          zIndex: 200,
          animation: staticMode ? 'none' : undefined,
        }}
        role="tooltip"
        data-macos-glass="tooltip"
        aria-hidden="true"
      >
        {label}
      </div>
    ) : null;

  return (
    <div
      ref={wrapperRef}
      className={`${styles.navItemWrapper} ${sidebarExpanded ? styles.expanded : !isHorizontal ? styles.compactLabels : ''}`}
      style={isHorizontal ? { width: 40, height: 40 } : {}}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={onClick}
        aria-label={label}
        aria-current={path && isActive ? 'page' : undefined}
        aria-pressed={!path ? !!isActive : undefined}
        style={isHorizontal ? { width: 40, height: 40, borderRadius: 10 } : {}}
        data-tauri-drag-region="false"
        onFocus={(event) => {
          if (!event.currentTarget.matches(':focus-visible')) return;
          setIsFocused(true);
          updateCardPosition();
        }}
        onBlur={() => setIsFocused(false)}
      >
        <Icon size={20} />
        {scrollingEnabled ? (
          <span
            ref={viewportRef}
            className={`${styles.label} ${styles.scrollingLabel}`}
            data-nav-label
            data-overflow={isOverflowing}
          >
            <span ref={textRef} className={styles.labelTrack} data-label-track>
              {label}
            </span>
          </span>
        ) : (
          <span className={styles.label}>{label}</span>
        )}
      </button>
      {createPortal(nameCard, document.body)}
    </div>
  );
}
