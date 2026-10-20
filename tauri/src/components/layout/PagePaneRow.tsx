import { useEffect, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useScrollingNavLabel } from '@/hooks/useScrollingNavLabel';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import styles from './DesktopPagePane.module.css';
import navStyles from './NavButton.module.css';

interface PagePaneRowProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  name: string;
  enabled: boolean;
  leading: ReactNode;
  trailing?: ReactNode;
}

/** The fixed row owns input/focus; only the overflowing name is animated. */
export function PagePaneRow({ name, enabled, leading, trailing, ...props }: PagePaneRowProps) {
  const button = useRef<HTMLButtonElement>(null);
  const [focused, setFocused] = useState(false);
  const hover = useHoverCardPosition(button, {
    isHorizontal: false,
    isBottom: false,
    isRight: false,
  });
  const { viewportRef, textRef, isOverflowing, staticMode } = useScrollingNavLabel(
    name,
    enabled,
    enabled && (hover.isHovered || focused),
  );
  const { handleMouseLeave } = hover;
  useEffect(() => {
    if (enabled) return;
    setFocused(false);
    handleMouseLeave();
  }, [enabled, handleMouseLeave]);

  return (
    <>
      <button
        {...props}
        type="button"
        ref={button}
        aria-label={name}
        onMouseEnter={hover.handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={(event) => {
          if (!event.currentTarget.matches(':focus-visible')) return;
          setFocused(true);
          hover.updateCardPosition();
        }}
        onBlur={() => setFocused(false)}
      >
        {leading}
        <span
          ref={viewportRef}
          className={styles.name}
          data-nav-label
          data-overflow={isOverflowing}
        >
          <span ref={textRef} className={styles.labelTrack} data-label-track>
            {name}
          </span>
        </span>
        {trailing}
      </button>
      {enabled &&
        isOverflowing &&
        staticMode &&
        (hover.isHovered || focused) &&
        createPortal(
          <div
            role="tooltip"
            aria-hidden="true"
            className={navStyles.nameCardPortal}
            data-macos-glass="tooltip"
            style={{ position: 'fixed', ...hover.cardStyle, zIndex: 200, animation: 'none' }}
          >
            {name}
          </div>,
          document.body,
        )}
    </>
  );
}
