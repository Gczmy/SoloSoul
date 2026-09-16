import { memo, forwardRef } from 'react';
import styles from './Card.module.css';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  interactive?: boolean;
  /** 浮层外壳与正文卡片分开选材质，保留同一布局组件。 */
  surface?: 'default' | 'floating';
  onClick?: () => void;
  onDoubleClick?: () => void;
  onMouseDown?: (e: React.MouseEvent) => void;
  onMouseUp?: (e: React.MouseEvent) => void;
  onMouseLeave?: (e: React.MouseEvent) => void;
  onTouchStart?: (e: React.TouchEvent) => void;
  onTouchEnd?: (e: React.TouchEvent) => void;
  style?: React.CSSProperties;
}

export const Card = memo(
  forwardRef<HTMLDivElement, CardProps>(function Card(
    {
      children,
      className,
      interactive,
      surface = 'default',
      onClick,
      onDoubleClick,
      onMouseDown,
      onMouseUp,
      onMouseLeave,
      onTouchStart,
      onTouchEnd,
      style,
    }: CardProps,
    ref,
  ) {
    return (
      <div
        ref={ref}
        data-ui-card
        data-macos-glass={surface === 'floating' ? 'panel' : undefined}
        className={`${styles.card} ${interactive ? styles.interactive : ''} ${className || ''}`}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
        onMouseDown={onMouseDown}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseLeave}
        onTouchStart={onTouchStart}
        onTouchEnd={onTouchEnd}
        role={onClick ? 'button' : undefined}
        tabIndex={onClick ? 0 : undefined}
        style={style}
        onKeyDown={
          onClick
            ? (e) => {
                if (e.key === 'Enter' || e.key === ' ') onClick();
              }
            : undefined
        }
      >
        {children}
      </div>
    );
  }),
);
