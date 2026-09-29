import { memo } from 'react';
import type { CSSProperties, MouseEventHandler } from 'react';
import type { LucideIcon } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
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
  /** If true, renders as a danger-outline button (red icon on transparent, solid red on hover) */
  dangerOutline?: boolean;
  /** If true, the button is disabled */
  disabled?: boolean;
  /** Optional toggle state; omitted for one-shot actions. */
  pressed?: boolean;
  /** Icon size. Default ICON_SIZE.sm (14px). */
  iconSize?: number;
  /** Custom className */
  className?: string;
}

/**
 * A small icon button with an optional numeric badge.
 *
 * Visual size and state are supplied by the shared icon-button tokens.
 */
export const BadgeIconButton = memo(function BadgeIconButton({
  Icon,
  count,
  onClick,
  title,
  danger = false,
  dangerOutline = false,
  disabled = false,
  pressed,
  iconSize = ICON_SIZE.sm,
  className = '',
}: BadgeIconButtonProps) {
  const hasBadge = count !== undefined && count > 0;
  const label = hasBadge ? `${title} (${count})` : title;

  const intent = dangerOutline ? 'danger-soft' : danger ? 'danger' : 'neutral';

  return (
    <div className={styles.wrapper}>
      <button
        data-ui-icon-button={dangerOutline ? 'danger-outline' : danger ? 'danger' : 'default'}
        data-ui-icon-intent={intent}
        type="button"
        onClick={onClick}
        title={title}
        disabled={disabled}
        aria-pressed={pressed}
        aria-label={label}
        className={`${styles.button} ${className}`}
        style={{ '--icon-symbol-size': `${iconSize}px` } as CSSProperties}
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
