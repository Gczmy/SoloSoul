import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import styles from './ValueContainer.module.css';

type WrapState = 'inline' | 'full' | 'full-wrapped';

/** 只计文本基线，避免把 Android 48px 按钮高度误认为正文换行。 */
function textWraps(element: HTMLElement): boolean {
  const walker = element.ownerDocument.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  const range = element.ownerDocument.createRange();
  // jsdom 没有 Range 几何实现；该环境无真实换行可测，交给浏览器布局回归验证。
  if (typeof range.getClientRects !== 'function') return false;
  const lineHeight =
    parseFloat(getComputedStyle(element).lineHeight) ||
    parseFloat(getComputedStyle(element).fontSize) * 1.2;
  let firstTop = Number.POSITIVE_INFINITY;
  let lastTop = Number.NEGATIVE_INFINITY;
  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (!node.textContent?.trim()) continue;
    range.selectNodeContents(node);
    for (const rect of range.getClientRects()) {
      if (rect.width <= 0) continue;
      firstTop = Math.min(firstTop, rect.top);
      lastTop = Math.max(lastTop, rect.top);
    }
  }
  return lastTop - firstTop > lineHeight / 2;
}

function useFieldWrapState(value: string) {
  const containerRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<WrapState>('inline');

  useLayoutEffect(() => {
    const container = containerRef.current;
    const text = textRef.current;
    const parent = container?.parentElement;
    if (!container || !text || !parent) return;

    const measure = () => {
      const previousFlex = container.style.flex;
      let next: WrapState = 'inline';
      try {
        // 比较同一帧中的两种可用宽度，不依赖上一次的布局状态。
        container.style.flex = '1 1 0%';
        if (textWraps(text)) {
          container.style.flex = '0 0 100%';
          next = textWraps(text) ? 'full-wrapped' : 'full';
        }
      } finally {
        container.style.flex = previousFlex;
      }
      setState((current) => (current === next ? current : next));
    };

    measure();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    observer?.observe(parent);
    observer?.observe(text);
    for (const sibling of parent.children) {
      if (sibling !== container) observer?.observe(sibling);
    }
    window.addEventListener('resize', measure);
    document.fonts?.addEventListener('loadingdone', measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', measure);
      document.fonts?.removeEventListener('loadingdone', measure);
    };
  }, [value]);

  return { containerRef, textRef, state };
}

export function ValueContainer({
  value,
  children,
  action,
}: {
  value: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  const { containerRef, textRef, state } = useFieldWrapState(value);
  return (
    <div ref={containerRef} data-field-value data-value-layout={state} className={styles.container}>
      <div ref={textRef} data-field-value-text className={styles.text}>
        {children}
      </div>
      {action}
    </div>
  );
}
