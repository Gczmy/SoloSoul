import { useState, useRef, useCallback, useEffect } from 'react';
import type { CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import styles from './NavButton.module.css';

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
  const [cardStyle, setCardStyle] = useState<React.CSSProperties | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const isRight = position === 'right';

  const updatePosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      if (isHorizontal) {
        if (isBottom) {
          setCardStyle({
            top: 'auto',
            bottom: window.innerHeight - rect.top + 8,
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        } else {
          setCardStyle({
            top: rect.bottom + 8,
            bottom: 'auto',
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        }
      } else if (isRight) {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: 'auto',
          right: window.innerWidth - rect.left + 8,
          transform: 'translateY(-50%)',
        });
      } else {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: rect.right + 8,
          transform: 'translateY(-50%)',
        });
      }
    }
  }, [isHorizontal, isBottom, isRight]);

  const handleMouseEnter = useCallback(() => {
    setIsHovered(true);
    updatePosition();
  }, [updatePosition]);

  const handleMouseLeave = useCallback(() => {
    setIsHovered(false);
  }, []);

  useEffect(() => {
    if (!isHovered) return;
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [isHovered, updatePosition]);

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

  return (
    <div
      ref={wrapperRef}
      className={styles.navItemWrapper}
      style={isHorizontal ? { width: 40, height: 40 } : {}}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {path && (
        <div
          className={`${styles.activeIndicator} ${isActive ? styles.activeIndicatorVisible : ''}`}
          style={isHorizontal
            ? { left: '50%', top: 'auto', bottom: -4, transform: 'translateX(-50%)', width: 20, height: 3, borderRadius: '2px 2px 0 0' }
            : isRight
              ? { left: 'auto', right: -8, top: '50%', transform: 'translateY(-50%)', width: 3, height: 20, borderRadius: '2px 0 0 2px' }
              : {}}
        />
      )}
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={onClick}
        aria-label={label}
        aria-current={isActive ? 'page' : undefined}
        style={isHorizontal ? { width: 40, height: 40, borderRadius: 10 } : {}}
        data-tauri-drag-region="false"
      >
        <Icon size={20} />
      </button>
      {createPortal(nameCard, document.body)}
    </div>
  );
}
