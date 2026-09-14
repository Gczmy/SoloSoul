import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';
import type { ObjectSummary } from '@/stores/objectStore';
import { objectRulerAnchorId, prefersRulerReducedMotion, shellInset } from './objectRuler';

interface ObjectRulerPositionOptions {
  objects: ObjectSummary[];
  renderedCount: number;
  listRef: RefObject<HTMLDivElement | null>;
  revealObject: (index: number) => void;
}

export function useObjectRulerPosition({
  objects,
  renderedCount,
  listRef,
  revealObject,
}: ObjectRulerPositionOptions) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [pending, setPending] = useState<{ id: string; keyboard: boolean } | null>(null);
  const highlight = useRef<{ element: HTMLElement; timer: ReturnType<typeof setTimeout> } | null>(
    null,
  );

  useEffect(
    () => () => {
      if (highlight.current) {
        clearTimeout(highlight.current.timer);
        delete highlight.current.element.dataset.rulerTarget;
      }
    },
    [],
  );

  useLayoutEffect(() => {
    const list = listRef.current;
    const scroller = list?.closest('main');
    if (!list || !scroller) return;
    let anchors: { id: string; top: number }[] = [];
    let frame = 0;
    const updateActive = () => {
      frame = 0;
      const inset = Math.max(0, shellInset('top') - scroller.getBoundingClientRect().top);
      const readingLine = scroller.scrollTop + inset + (scroller.clientHeight - inset) * 0.25;
      if (
        scroller.scrollTop > 0 &&
        scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2
      ) {
        setActiveId(anchors.at(-1)?.id ?? null);
        return;
      }
      // 卡片位置只在布局变化时测量；滚动时二分查找，避免每帧读取所有卡片布局。
      let low = 0;
      let high = anchors.length - 1;
      while (low < high) {
        const mid = Math.ceil((low + high) / 2);
        if (anchors[mid].top <= readingLine) low = mid;
        else high = mid - 1;
      }
      setActiveId(anchors[low]?.id ?? null);
    };
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = 0;
      const origin = scroller.getBoundingClientRect().top;
      anchors = objects.slice(0, renderedCount).flatMap((object) => {
        const element = document.getElementById(objectRulerAnchorId(object.id));
        return element && list.contains(element)
          ? [
              {
                id: object.id,
                top: element.getBoundingClientRect().top - origin + scroller.scrollTop,
              },
            ]
          : [];
      });
      updateActive();
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(updateActive);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(list);
    observer.observe(scroller);
    scroller.addEventListener('scroll', onScroll, { passive: true });
    window.addEventListener('resize', measure);
    measure();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      scroller.removeEventListener('scroll', onScroll);
      window.removeEventListener('resize', measure);
    };
  }, [objects, renderedCount, listRef]);

  useLayoutEffect(() => {
    if (!pending) return;
    if (!objects.some((object) => object.id === pending.id)) {
      setPending(null);
      return;
    }
    const element = document.getElementById(objectRulerAnchorId(pending.id));
    const scroller = listRef.current?.closest('main');
    if (!element || !scroller || !listRef.current?.contains(element)) return;
    const origin = scroller.getBoundingClientRect().top;
    const inset = Math.max(0, shellInset('top') - origin);
    const top = scroller.scrollTop + element.getBoundingClientRect().top - origin - inset - 16;
    scroller.scrollTo({
      top: Math.max(0, top),
      behavior: prefersRulerReducedMotion() ? 'instant' : 'smooth',
    });
    if (highlight.current) {
      clearTimeout(highlight.current.timer);
      delete highlight.current.element.dataset.rulerTarget;
    }
    element.dataset.rulerTarget = 'true';
    highlight.current = {
      element,
      timer: setTimeout(() => {
        delete element.dataset.rulerTarget;
        highlight.current = null;
      }, 1800),
    };
    if (pending.keyboard)
      element.querySelector<HTMLElement>('[role="button"]')?.focus({ preventScroll: true });
    setActiveId(pending.id);
    setPending(null);
  }, [pending, objects, renderedCount, listRef]);

  const navigateTo = (index: number, keyboard = false) => {
    const object = objects[index];
    if (!object) return;
    revealObject(index);
    setPending({ id: object.id, keyboard });
  };
  return { activeId, navigateTo };
}
