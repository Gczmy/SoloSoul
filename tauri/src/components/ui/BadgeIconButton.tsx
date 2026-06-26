import { memo } from 'react';
import type { MouseEventHandler } from 'react';
import type { LucideIcon } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';
import styles from './BadgeIconButton.module.css';

interface BadgeIconButtonProps {
  /** Icon from lucide-react */
  Icon: LucideIcon;
  /** Optional count to display as a badge */
  count?: number;
  /** Click handler */
  onClick: MouseEventHandler<HTMLButtonElement>;
  /** Tooltip title */
  title: string;
  /** If true, renders as a danger button (red hover) */
  danger?: boolean;
  /** If true, the button is disabled */
  disabled?: boolean;
  /** Icon size. Default ICON_SIZE.sm (14px). */
  iconSize?: number;
  /** Custom className */
  className?: string;
}

/**
 * A small icon button with an optional numeric badge.
 *
 * Design system alignment:
 * - 28×28 size (matches miniBtn)
 * - 6px border radius (radius-sm token)
 * - Badge uses tinted accent background (not solid white)
 * - Hover via .interactive-icon / .interactive-icon-danger CSS classes
 * - Transition uses project token variables
 */
export const BadgeIconButton = memo(function BadgeIconButton({
  Icon,
  count,
  onClick,
  title,
  danger = false,
  disabled = false,
  iconSize = ICON_SIZE.sm,
  className = '',
}: BadgeIconButtonProps) {
  const hasBadge = count !== undefined && count > 0;
  const label = hasBadge ? `${title} (${count})` : title;

  return (
    <div className={styles.wrapper}>
      <button
        type="button"
        onClick={onClick}
        title={title}
        disabled={disabled}
        aria-label={label}
        className={`${styles.button} ${danger ? styles.danger : styles.accent} ${className}`}
      >
        <Icon size={iconSize} />
      </button>
      {hasBadge && (
        <span className={styles.badge} data-testid={`count-badge-${title.toLowerCase()}`}>
          {count > 99 ? '99+' : count}
        </span>
      )}
    </div>
  );
});

BadgeIconButton.displayName = 'BadgeIconButton';
