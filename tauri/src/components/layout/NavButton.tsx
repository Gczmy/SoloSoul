import { useState, useRef, useCallback, useEffect } from 'react';
import type { CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import styles from './NavButton.module.css';
import { ICON_SIZE } from '@/lib/constants';

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
  const [indicatorStyle, setIndicatorStyle] = useState<CSSProperties | null>(null);
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
    >
      <button
        className={`${styles.navButton} ${isActive ? styles.activeButton : ''}`}
        onClick={onClick}
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
