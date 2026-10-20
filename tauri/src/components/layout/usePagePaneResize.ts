import {
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
  type PointerEvent,
  type KeyboardEvent,
} from 'react';
import { useUiStore } from '@/stores/uiStore';

export const PAGE_PANE_MIN = 200;
export const PAGE_PANE_MAX = 400;
export const PAGE_PANE_DEFAULT = 264;
const EDITOR_MIN = 280;

export function usePagePaneResize(
  bodyRef: RefObject<HTMLDivElement | null>,
  enabled: boolean,
  open: boolean,
) {
  const preferred = useUiStore((state) => state.desktopPagePaneWidth);
  const setPreferred = useUiStore((state) => state.setDesktopPagePaneWidth);
  const [bodyWidth, setBodyWidth] = useState(0);
  const [draft, setDraft] = useState<number | null>(null);
  const drag = useRef<{ pointer: number; x: number; width: number; current: number } | null>(null);
  const max = Math.max(PAGE_PANE_MIN, Math.min(PAGE_PANE_MAX, Math.floor(bodyWidth - EDITOR_MIN)));
  const bound = (value: number) => Math.max(PAGE_PANE_MIN, Math.min(max, Math.round(value)));
  const width = bodyWidth > 0 ? bound(draft ?? preferred) : preferred;

  useLayoutEffect(() => {
    const body = bodyRef.current;
    if (!enabled || !body) return;
    const measure = () => setBodyWidth(body.getBoundingClientRect().width);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(body);
    window.addEventListener('resize', measure);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', measure);
    };
  }, [bodyRef, enabled]);

  useLayoutEffect(() => {
    if (open && enabled) return;
    drag.current = null;
    setDraft(null);
  }, [open, enabled]);

  const finish = (event: PointerEvent<HTMLDivElement>) => {
    const current = drag.current;
    if (!current || current.pointer !== event.pointerId) return;
    drag.current = null;
    setPreferred(bound(current.current));
    setDraft(null);
  };

  return {
    width,
    max,
    resizing: draft !== null,
    separatorProps: {
      onPointerDown: (event: PointerEvent<HTMLDivElement>) => {
        if (event.button !== 0 || !open || !enabled) return;
        event.preventDefault();
        event.currentTarget.focus();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { pointer: event.pointerId, x: event.clientX, width, current: width };
        setDraft(width);
      },
      onPointerMove: (event: PointerEvent<HTMLDivElement>) => {
        const current = drag.current;
        if (!current || current.pointer !== event.pointerId) return;
        current.current = bound(current.width + event.clientX - current.x);
        setDraft(current.current);
      },
      onPointerUp: finish,
      onPointerCancel: finish,
      onLostPointerCapture: finish,
      onDoubleClick: () => setPreferred(PAGE_PANE_DEFAULT),
      onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => {
        const step = event.shiftKey ? 32 : 8;
        const target =
          event.key === 'Home'
            ? PAGE_PANE_MIN
            : event.key === 'End'
              ? max
              : event.key === 'ArrowLeft'
                ? width - step
                : event.key === 'ArrowRight'
                  ? width + step
                  : null;
        if (target === null) return;
        event.preventDefault();
        setPreferred(bound(target));
      },
    },
  };
}
