import { useCallback, useLayoutEffect, type RefObject } from 'react';

/** 圆角框内只显示完整刻度；滚轮、键盘和当前位置跟随使用同一行高。 */
export function useRulerRail(
  containerRef: RefObject<HTMLElement | null>,
  railRef: RefObject<HTMLDivElement | null>,
  count: number,
) {
  const step = Math.max(6, Math.min(18, Math.floor(320 / Math.max(1, count))));

  useLayoutEffect(() => {
    const container = containerRef.current;
    const rail = railRef.current;
    if (!container || !rail) return;
    let remainder = 0;
    const scrollToRow = (row: number) => {
      rail.scrollTo({
        top: Math.max(0, Math.min(rail.scrollHeight - rail.clientHeight, row * step)),
        behavior: 'instant',
      });
    };
    const resize = () => {
      // 外框上下各留 12px，使滚动区域完全落在圆角内部的直边区。
      const rows = Math.max(1, Math.floor((Math.min(360, container.clientHeight) - 24) / step));
      rail.style.height = `${Math.min(count, rows) * step}px`;
      scrollToRow(Math.round(rail.scrollTop / step));
      remainder = 0;
    };
    const onWheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.deltaY === 0) return;
      event.preventDefault();
      const unit = event.deltaMode === 1 ? step : event.deltaMode === 2 ? rail.clientHeight : 1;
      const delta = event.deltaY * unit;
      if (Math.sign(delta) !== Math.sign(remainder)) remainder = 0;
      remainder += delta;
      const rows = Math.trunc(remainder / step);
      if (rows === 0) return;
      remainder -= rows * step;
      scrollToRow(Math.round(rail.scrollTop / step) + rows);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(container);
    // 显式取消原生像素滚动，触控板的小幅输入累积满一格后再移动。
    rail.addEventListener('wheel', onWheel, { passive: false });
    resize();
    return () => {
      observer.disconnect();
      rail.removeEventListener('wheel', onWheel);
    };
  }, [containerRef, railRef, count, step]);

  const scrollToIndex = useCallback(
    (index: number) => {
      const rail = railRef.current;
      if (!rail) return;
      const visibleRows = Math.floor(rail.clientHeight / step);
      rail.scrollTo({
        top: Math.max(
          0,
          Math.min(
            rail.scrollHeight - rail.clientHeight,
            (index - Math.floor(visibleRows / 2)) * step,
          ),
        ),
        behavior: 'instant',
      });
    },
    [railRef, step],
  );

  return { step, scrollToIndex };
}
