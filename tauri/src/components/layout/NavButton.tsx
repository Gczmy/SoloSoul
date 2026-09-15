import { useRef, useContext } from 'react';
import { DesktopSidebarContext } from './DesktopSidebarContext';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import styles from './NavButton.module.css';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';

export type NavPosition = 'left' | 'right' | 'top' | 'bottom';

interface NavButtonProps {
  path?: string;
  Icon: LucideIcon;
  label: string;
  isActive?: boolean;
  onClick: () => void;
  position?: NavPosition;
}

export function NavButton({
  path,
  Icon,
  label,
  isActive,
  onClick,
  position = 'left',
}: NavButtonProps) {
  const wrapperRef = useRef<HTMLDivElement>(null);

  const isHorizontal = position === 'top' || position === 'bottom';
  const sidebarExpanded = useContext(DesktopSidebarContext) && !isHorizontal;
  const isBottom = position === 'bottom';
  const isRight = position === 'right';

  const { cardStyle, isHovered, handleMouseEnter, handleMouseLeave } = useHoverCardPosition(
    wrapperRef,
    { isHorizontal, isBottom, isRight },
  );

  const nameCard =
    isHovered && !sidebarExpanded ? (
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
      >
        <Icon size={20} />
        <span className={styles.label}>{label}</span>
      </button>
      {createPortal(nameCard, document.body)}
    </div>
  );
}
