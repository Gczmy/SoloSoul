import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { createPortal } from 'react-dom';
import { useScrollingNavLabel } from '@/hooks/useScrollingNavLabel';
import { useHoverCardPosition } from '@/hooks/useHoverCardPosition';
import styles from './DesktopPagePane.module.css';
import navStyles from './NavButton.module.css';
import { supportsHover } from '@/lib/platform';
import { PagePanePreviewFrame } from './PagePanePreviewFrame';

interface PagePaneRowProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  name: string;
  enabled: boolean;
  leading: ReactNode;
  trailing?: ReactNode;
  preview?: ReactNode;
  previewLabel?: string;
  previewKey?: string;
  activePreview?: string | null;
  onPreviewChange?: Dispatch<SetStateAction<string | null>>;
}

/** The fixed row owns input/focus; only the overflowing name is animated. */
export function PagePaneRow({
  name,
  enabled,
  leading,
  trailing,
  preview,
  previewLabel,
  previewKey,
  activePreview,
  onPreviewChange,
  ...props
}: PagePaneRowProps) {
  const button = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const suppressFocusOpen = useRef(false);
  const tooltipId = useId();
  const previewOpen = Boolean(preview && previewKey && activePreview === previewKey);
  const cancelTimer = useCallback(() => clearTimeout(timer.current), []);
  const closePreview = useCallback(() => {
    clearTimeout(timer.current);
    onPreviewChange?.((previous) => (previous === previewKey ? null : previous));
  }, [onPreviewChange, previewKey]);
  const leavePreview = () => {
    cancelTimer();
    timer.current = setTimeout(closePreview, 120);
  };
  const dismissPreview = useCallback(() => {
    if (card.current?.contains(document.activeElement)) {
      suppressFocusOpen.current = true;
      button.current?.focus();
    }
    closePreview();
  }, [closePreview]);
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
    closePreview();
  }, [enabled, handleMouseLeave, closePreview]);
  useEffect(() => () => clearTimeout(timer.current), []);

  return (
    <>
      <button
        {...props}
        type="button"
        ref={button}
        aria-label={name}
        aria-describedby={enabled && previewOpen ? tooltipId : undefined}
        onMouseEnter={() => {
          hover.handleMouseEnter();
          if (!enabled || !supportsHover() || !previewKey) return;
          suppressFocusOpen.current = false;
          cancelTimer();
          timer.current = setTimeout(() => onPreviewChange?.(previewKey), 180);
        }}
        onMouseLeave={() => {
          handleMouseLeave();
          leavePreview();
        }}
        onFocus={(event) => {
          if (!event.currentTarget.matches(':focus-visible')) return;
          setFocused(true);
          hover.updateCardPosition();
          cancelTimer();
          if (suppressFocusOpen.current) suppressFocusOpen.current = false;
          else if (enabled && previewKey) onPreviewChange?.(previewKey);
        }}
        onBlur={(event) => {
          setFocused(false);
          if (!(event.relatedTarget instanceof Node && card.current?.contains(event.relatedTarget)))
            closePreview();
        }}
        onClick={(event) => {
          closePreview();
          props.onClick?.(event);
        }}
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
      {enabled && previewOpen && (
        <PagePanePreviewFrame
          anchor={button}
          cardRef={card}
          id={tooltipId}
          label={previewLabel ?? name}
          onClose={closePreview}
          onDismiss={dismissPreview}
          onMouseEnter={cancelTimer}
          onMouseLeave={leavePreview}
        >
          {preview}
        </PagePanePreviewFrame>
      )}
      {enabled &&
        !preview &&
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
