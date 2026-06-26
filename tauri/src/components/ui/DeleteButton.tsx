import { memo } from 'react';
import { Trash2 } from 'lucide-react';
import { Button } from './Button';
import { BadgeIconButton } from './BadgeIconButton';
import { ICON_SIZE } from '@/lib/iconSizes';

interface DeleteButtonProps {
  /** Click handler — receives the mouse event for e.stopPropagation */
  onClick: React.MouseEventHandler<HTMLButtonElement>;
  /** Tooltip / aria label. Required for icon-only mode. */
  title: string;
  /** Render as a 28×28 icon-only button. Default false. */
  iconOnly?: boolean;
  /** Optional button text. Ignored when iconOnly is true. */
  children?: React.ReactNode;
  /** Disable interaction */
  disabled?: boolean;
}

/**
 * Reusable delete button using the danger-outline style.
 *
 * - Normal: transparent background, bright red border/icon/text
 * - Hover: bright red background, white icon/text
 *
 * Use `iconOnly` for compact row actions; omit it for batch toolbars.
 */
export const DeleteButton = memo(function DeleteButton({
  onClick,
  title,
  iconOnly = false,
  children,
  disabled = false,
}: DeleteButtonProps) {
  if (iconOnly) {
    return (
      <BadgeIconButton
        Icon={Trash2}
        onClick={onClick}
        title={title}
        dangerOutline
        iconSize={ICON_SIZE.sm}
        disabled={disabled}
      />
    );
  }

  return (
    <Button variant="danger-outline" size="sm" onClick={onClick} disabled={disabled} title={title}>
      <Trash2 size={ICON_SIZE.sm} />
      {children}
    </Button>
  );
});

DeleteButton.displayName = 'DeleteButton';
