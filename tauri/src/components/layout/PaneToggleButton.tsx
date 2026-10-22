import { useEffect, useId, useRef, useState, type ComponentProps } from 'react';
import { createPortal } from 'react-dom';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import styles from './PaneToggleButton.module.css';
import navStyles from './NavButton.module.css';

type PaneToggleButtonProps = ComponentProps<'button'> & {
  tooltip: string;
  tooltipMarker: 'page-pane' | 'object-inspector';
  align?: 'start' | 'end';
};

/** 两侧面板开关共享悬停提示，避免浏览器原生 title 与应用卡片混用。 */
export function PaneToggleButton({
  tooltip,
  tooltipMarker,
  align = 'start',
  className,
  children,
  ...props
}: PaneToggleButtonProps) {
  const ref = useRef<HTMLButtonElement>(null);
  const hintId = useId();
  const [focused, setFocused] = useState(false);
  const hint = useHoverCardPosition(ref, {
    isHorizontal: true,
    isBottom: false,
    isRight: false,
  });
  const visible = hint.isHovered || focused;
  const hide = hint.handleMouseLeave;
  useEffect(() => {
    if (!visible) return;
    const close = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setFocused(false);
      hide();
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [visible, hide]);

  return (
    <>
      <button
        {...props}
        ref={ref}
        type="button"
        className={[styles.button, className].filter(Boolean).join(' ')}
        data-desktop-control="icon"
        aria-describedby={visible ? hintId : undefined}
        onMouseEnter={hint.handleMouseEnter}
        onMouseLeave={hide}
        onFocus={(event) => {
          if (event.currentTarget.matches(':focus-visible')) {
            setFocused(true);
            hint.updateCardPosition();
          }
        }}
        onBlur={() => setFocused(false)}
      >
        {children}
      </button>
      {visible &&
        hint.cardStyle &&
        createPortal(
          <div
            id={hintId}
            role="tooltip"
            data-page-pane-toggle-tooltip={tooltipMarker === 'page-pane' || undefined}
            data-object-inspector-toggle-tooltip={tooltipMarker === 'object-inspector' || undefined}
            data-macos-glass="tooltip"
            className={navStyles.nameCardPortalHorizontal}
            style={{
              position: 'fixed',
              ...hint.cardStyle,
              ...(align === 'end'
                ? {
                    left: 'auto',
                    right: Math.max(8, window.innerWidth - Number(hint.cardStyle.left) - 16),
                  }
                : { left: Math.max(8, Number(hint.cardStyle.left) - 20) }),
              transform: 'none',
              zIndex: 200,
            }}
          >
            {tooltip}
          </div>,
          document.body,
        )}
    </>
  );
}
