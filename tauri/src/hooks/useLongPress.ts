import { useRef, useCallback } from 'react';

interface UseLongPressOptions {
  /** Callback fired when a long press is detected */
  onLongPress: (event: React.TouchEvent | React.MouseEvent) => void;
  /** Callback fired on normal click/tap (optional) */
  onClick?: () => void;
  /** Duration in ms to consider a press as long press (default: 500) */
  threshold?: number;
}

export interface LongPressHandlers {
  onMouseDown: (event: React.MouseEvent) => void;
  onMouseUp: () => void;
  onMouseLeave: () => void;
  onTouchStart: (event: React.TouchEvent) => void;
  onTouchEnd: () => void;
  onClick: () => void;
}

/**
 * Detect long press gestures on touch devices and mouse.
 * Prevents the click event from firing when a long press is detected.
 */
export function useLongPress({
  onLongPress,
  onClick,
  threshold = 500,
}: UseLongPressOptions): LongPressHandlers {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isLongPressRef = useRef(false);

  const start = useCallback(
    (event: React.TouchEvent | React.MouseEvent) => {
      isLongPressRef.current = false;
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        isLongPressRef.current = true;
        onLongPress(event);
      }, threshold);
    },
    [onLongPress, threshold],
  );

  const cancel = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const handleClick = useCallback(() => {
    if (isLongPressRef.current) {
      isLongPressRef.current = false;
      return;
    }
    onClick?.();
  }, [onClick]);

  return {
    onMouseDown: start,
    onMouseUp: cancel,
    onMouseLeave: cancel,
    onTouchStart: start,
    onTouchEnd: cancel,
    onClick: handleClick,
  };
}
