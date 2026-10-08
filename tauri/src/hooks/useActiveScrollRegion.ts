import { useEffect } from 'react';

const X = 'data-scrollbar-region-x';
const Y = 'data-scrollbar-region-y';
const MODE = 'data-scrollbar-region-mode';

/** 只登记实际可滚动的轴；高亮由 CSS :hover 决定，不依赖鼠标事件或焦点。 */
export function useActiveScrollRegion() {
  useEffect(() => {
    const root = document.documentElement;
    const observed = new Set<HTMLElement>();
    const pending = new Set<HTMLElement>();
    const subtrees = new Set<HTMLElement>();
    let frame = 0;
    let fullScan = true;
    const mark = (node: HTMLElement, attr: string, enabled: boolean) => {
      if (enabled) {
        if (!node.hasAttribute(attr)) node.setAttribute(attr, 'true');
      } else node.removeAttribute(attr);
    };
    const enqueue = (target: Node | null, subtree = false) => {
      const element = target instanceof HTMLElement ? target : target?.parentElement;
      if (element) {
        if (subtree) subtrees.add(element);
        for (let node: HTMLElement | null = element; node; node = node.parentElement)
          pending.add(node);
      }
      if (!frame) frame = requestAnimationFrame(update);
    };
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) enqueue(entry.target);
    });
    const update = () => {
      frame = 0;
      // 运行平台已在 React 挂载前初始化，不依赖原生材质请求成功。
      const platform = root.dataset.platform ?? root.dataset.desktopPlatform;
      if (platform === 'macos' || platform === 'windows') root.setAttribute(MODE, 'hover');
      else root.removeAttribute(MODE);
      if (fullScan) subtrees.add(root);
      fullScan = false;
      for (const tree of subtrees) {
        pending.add(tree);
        for (const child of tree.querySelectorAll<HTMLElement>('*')) pending.add(child);
      }
      subtrees.clear();
      for (const node of pending) {
        if (!node.isConnected) continue;
        if (!observed.has(node)) {
          observed.add(node);
          // 也观察内容尺寸，容器高度不变时 scrollHeight 仍可能增长。
          resizeObserver.observe(node);
        }
        const style = getComputedStyle(node);
        const excluded = node.matches('input, select');
        let x = !excluded && /^(auto|scroll|overlay)$/.test(style.overflowX);
        let y = !excluded && /^(auto|scroll|overlay)$/.test(style.overflowY);
        if (node === document.scrollingElement) {
          const body = getComputedStyle(document.body);
          x =
            !['hidden', 'clip'].includes(style.overflowX) &&
            !['hidden', 'clip'].includes(body.overflowX);
          y =
            !['hidden', 'clip'].includes(style.overflowY) &&
            !['hidden', 'clip'].includes(body.overflowY);
        }
        mark(node, X, x && node.scrollWidth - node.clientWidth > 1);
        mark(node, Y, y && node.scrollHeight - node.clientHeight > 1);
      }
      pending.clear();
      for (const node of observed) {
        if (!node.isConnected) {
          resizeObserver.unobserve(node);
          observed.delete(node);
          node.removeAttribute(X);
          node.removeAttribute(Y);
        }
      }
    };
    const observer = new MutationObserver((records) => {
      for (const record of records) {
        enqueue(record.target, record.type === 'attributes');
        for (const node of record.addedNodes) enqueue(node, true);
      }
    });
    observer.observe(root, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      // 不监听登记属性，防止扫描自己触发下一轮扫描。
      attributeFilter: [
        'class',
        'style',
        'hidden',
        'open',
        'aria-hidden',
        'data-platform',
        'data-desktop-platform',
      ],
    });
    const refresh = () => {
      fullScan = true;
      enqueue(root);
    };
    const contentChanged = (event: Event) =>
      enqueue(event.target instanceof Node ? event.target : null);
    window.addEventListener('resize', refresh);
    document.addEventListener('load', contentChanged, true);
    document.addEventListener('input', contentChanged, true);
    document.fonts?.addEventListener('loadingdone', refresh);
    refresh();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      resizeObserver.disconnect();
      window.removeEventListener('resize', refresh);
      document.removeEventListener('load', contentChanged, true);
      document.removeEventListener('input', contentChanged, true);
      document.fonts?.removeEventListener('loadingdone', refresh);
      root.removeAttribute(MODE);
      for (const node of observed) {
        node.removeAttribute(X);
        node.removeAttribute(Y);
      }
    };
  }, []);
}
